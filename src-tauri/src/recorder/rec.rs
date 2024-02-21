use std::{fmt::Display, path::PathBuf, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use futures_util::StreamExt;
use libobs_recorder::settings::{RateControl, StdResolution, Window};
use libobs_recorder::{Recorder, RecorderSettings};
use riot_local_auth::Credentials;
use shaco::ingame::IngameClient;
use shaco::model::ws::{EventType, LcuSubscriptionType};
use shaco::rest::LcuRestClient;
use shaco::ws::LcuWebsocketClient;
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Manager};
use tokio::select;
use tokio::time::{interval, sleep, timeout};
use tokio_util::sync::CancellationToken;

use super::session_event::{GameData, GamePhase, SessionEventData, SubscriptionResponse};
use super::window::{get_lol_window, get_window_size, WINDOW_CLASS, WINDOW_PROCESS, WINDOW_TITLE};
use crate::cancellable;
use crate::game_data::{self, GameId};
use crate::state::{CurrentlyRecording, SettingsWrapper};
use crate::{helpers::set_recording_tray_item, recorder::session_event::Queue};

const RECORDINGS_CHANGED_EVENT: &str = "recordings_changed";

pub struct RecordLeagueGames {
    cancel_token: CancellationToken,
    task: async_runtime::Mutex<JoinHandle<()>>,
}

impl RecordLeagueGames {
    pub fn start(app_handle: AppHandle) -> Self {
        let cancel_token = CancellationToken::new();
        let task = async_runtime::spawn(wait_for_api(app_handle, cancel_token.child_token()));
        Self {
            cancel_token,
            task: async_runtime::Mutex::new(task),
        }
    }

    pub fn stop(&self) {
        async_runtime::block_on(async {
            self.cancel_token.cancel();

            let mut task = self.task.lock().await;
            if let Err(_) = timeout(Duration::from_secs(1), &mut *task).await {
                log::warn!("RecordingTask stop() ran into timeout - aborting task");
                task.abort();
            }
        });
    }
}

enum State {
    Idle,
    Recording(RecordingTaskInner),
    EndOfGame(Metadata),
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Idle => f.write_str("Idle"),
            State::Recording(_) => f.write_str("Recording"),
            State::EndOfGame(metadata) => f.write_fmt(format_args!("EndOfGame({metadata})")),
        }
    }
}

struct Rec {
    recorder: Recorder,
    metadata: Metadata,
}

struct Metadata {
    game_id: GameId,
    output_filepath: PathBuf,
    ingame_time_rec_start_offset: f64,
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "game_id={}, filepath={}, rec_offset={}",
            self.game_id,
            self.output_filepath.display(),
            self.ingame_time_rec_start_offset
        ))
    }
}

#[derive(Clone, Copy)]
struct Ctx<'a> {
    app_handle: &'a AppHandle,
    credentials: &'a Credentials,
    cancel_token: &'a CancellationToken,
}

impl<'a> From<&'a CtxOwned> for Ctx<'a> {
    fn from(ctx: &'a CtxOwned) -> Self {
        Self {
            app_handle: &ctx.app_handle,
            credentials: &ctx.credentials,
            cancel_token: &ctx.cancel_token,
        }
    }
}

#[derive(Clone)]
struct CtxOwned {
    app_handle: AppHandle,
    credentials: Credentials,
    cancel_token: CancellationToken,
}

impl<'a> From<Ctx<'a>> for CtxOwned {
    fn from(ctx: Ctx<'a>) -> Self {
        Self {
            app_handle: ctx.app_handle.clone(),
            credentials: ctx.credentials.clone(),
            cancel_token: ctx.cancel_token.clone(),
        }
    }
}

async fn wait_for_api(app_handle: AppHandle, cancel_token: CancellationToken) {
    log::info!("waiting for LCU API");

    loop {
        if let Ok(credentials) = riot_local_auth::lcu::try_get_credentials() {
            let ctx = Ctx {
                credentials: &credentials,
                app_handle: &app_handle,
                cancel_token: &cancel_token,
            };

            if let Err(e) = listen_for_games(ctx).await {
                log::error!("stopped listening for games: {e}");
            }
        }

        let cancelled = cancellable!(sleep(Duration::from_secs(1)), cancel_token, ());
        if cancelled {
            log::info!("task cancelled (wait_for_api)");
            return;
        }
    }
}

async fn listen_for_games(ctx: Ctx<'_>) -> Result<()> {
    const GAMEFLOW_SESSION: &str = "/lol-gameflow/v1/session";
    const EOG_STATS_BLOCK: &str = "/lol-end-of-game/v1/eog-stats-block";

    let mut lcu_ws_client = LcuWebsocketClient::connect_with(ctx.credentials).await?;
    lcu_ws_client
        .subscribe(LcuSubscriptionType::JsonApiEvent(GAMEFLOW_SESSION.into()))
        .await?;
    lcu_ws_client
        .subscribe(LcuSubscriptionType::JsonApiEvent(EOG_STATS_BLOCK.into()))
        .await?;

    let mut state = State::Idle;

    match LcuRestClient::from(ctx.credentials)
        .get::<SessionEventData>(GAMEFLOW_SESSION)
        .await
    {
        Ok(init_event_data) => {
            state = state_transition(state, SubscriptionResponse::Session(init_event_data), ctx).await;
        }
        Err(e) => log::info!("no initial event-data: {e}"),
    }

    while let Some(event) = cancellable!(lcu_ws_client.next(), ctx.cancel_token, Option) {
        if event.payload.event_type != EventType::Update {
            continue;
        }

        state = match serde_json::from_value::<SubscriptionResponse>(event.payload.data) {
            Ok(event_data) => state_transition(state, event_data, ctx).await,
            Err(e) => {
                log::error!("failed to deserialize event: {e}");
                continue;
            }
        }
    }

    if let State::Recording(recording_task) = state {
        _ = recording_task.stop().await;
    }

    Ok(())
}

async fn state_transition(state: State, sub_resp: SubscriptionResponse, ctx: Ctx<'_>) -> State {
    let next_state = match state {
        State::Idle => match sub_resp {
            SubscriptionResponse::Session(SessionEventData {
                phase: GamePhase::GameStart | GamePhase::InProgress,
                game_data: GameData { queue, game_id },
            }) if queue.is_ranked || !ctx.app_handle.state::<SettingsWrapper>().only_record_ranked() => {
                State::Recording(RecordingTaskInner::start(game_id, ctx))
            }
            _ => State::Idle,
        },

        State::Recording(recording_task) => match sub_resp {
            SubscriptionResponse::Session(SessionEventData {
                phase: GamePhase::FailedToLaunch | GamePhase::Reconnect,
                ..
            }) => {
                _ = recording_task.stop().await;
                State::Idle
            }
            SubscriptionResponse::Session(SessionEventData {
                phase: GamePhase::WaitingForStats,
                ..
            }) => {
                // make sure the task stops e.g. maybe IngameAPI didn't start => caught in waiting for game loop
                match recording_task.stop().await {
                    Ok(metadata) => State::EndOfGame(metadata),
                    Err(e) => {
                        log::error!("failed to stop recording: {e}");
                        State::Idle
                    }
                }
            }
            _ => State::Recording(recording_task),
        },

        State::EndOfGame(metadata) => match sub_resp {
            SubscriptionResponse::Eog {}
            | SubscriptionResponse::Session(SessionEventData {
                phase: GamePhase::TerminatedInError,
                ..
            })
            | SubscriptionResponse::Session(SessionEventData {
                phase: GamePhase::EndOfGame,
                game_data: GameData {
                    queue: Queue { id: -1, .. }, ..
                },
            }) => {
                let Metadata {
                    game_id,
                    output_filepath,
                    ingame_time_rec_start_offset,
                } = metadata;

                let mut metadata_filepath = output_filepath;
                metadata_filepath.set_extension("json");

                match game_data::process_data(ingame_time_rec_start_offset, game_id, ctx.credentials, ctx.cancel_token)
                    .await
                {
                    Ok(game_metadata) => {
                        log::info!("writing game metadata to file: {metadata_filepath:?}");

                        // serde_json requires a std::fs::File
                        if let Ok(file) = std::fs::File::create(&metadata_filepath) {
                            let result = serde_json::to_writer(&file, &game_metadata);
                            log::info!("metadata saved: {result:?}");

                            _ = ctx.app_handle.emit_all(RECORDINGS_CHANGED_EVENT, ());
                        }
                    }
                    Err(e) => log::error!("unable to process data: {e}"),
                }

                State::Idle
            }
            _ => State::EndOfGame(metadata),
        },
    };

    log::info!("recorder state: {next_state}");
    next_state
}

struct RecordingTaskInner {
    join_handle: JoinHandle<Result<Rec>>,
    ctx: CtxOwned,
}

impl RecordingTaskInner {
    fn start(game_id: GameId, ctx: Ctx<'_>) -> Self {
        let ctx = CtxOwned {
            cancel_token: ctx.cancel_token.child_token(),
            ..ctx.into()
        };
        let join_handle = async_runtime::spawn(Self::record(game_id, ctx.clone()));
        Self { join_handle, ctx }
    }

    async fn stop(self) -> Result<Metadata> {
        self.ctx.cancel_token.cancel();
        let Rec { mut recorder, metadata } = self.join_handle.await??;

        let stopped = recorder.stop_recording();
        let shutdown = recorder.shutdown();
        log::info!("stopping recording: stopped={stopped:?}, shutdown={shutdown:?}");

        self.ctx.app_handle.state::<CurrentlyRecording>().set(None);
        set_recording_tray_item(&self.ctx.app_handle, false);

        Ok(metadata)
    }

    async fn record(game_id: GameId, ctx: CtxOwned) -> Result<Rec> {
        let (mut recorder, output_filepath) =
            cancellable!(Self::setup_recorder(Ctx::from(&ctx)), ctx.cancel_token, Result)?;

        // ingame_client timeout is 200ms, so no need to make cancellable with token
        let ingame_client = IngameClient::new();

        log::info!("waiting for game to start");
        let mut timer = interval(Duration::from_millis(500));
        while !ingame_client.active_game().await {
            let cancelled = cancellable!(timer.tick(), ctx.cancel_token, ());
            if cancelled {
                let shutdown = recorder.shutdown();
                bail!("waiting for game cancelled - recorder shutdown: {shutdown:?}");
            }
        }

        ctx.app_handle
            .state::<CurrentlyRecording>()
            .set(Some(output_filepath.clone()));
        set_recording_tray_item(&ctx.app_handle, true);

        // if initial game_data is successful => start recording
        if let Err(e) = recorder.start_recording() {
            ctx.app_handle.state::<CurrentlyRecording>().set(None);
            set_recording_tray_item(&ctx.app_handle, false);

            // if recording start failed stop recording just in case and retry next 'recorder loop
            let stop_recording = recorder.stop_recording();
            let shutdown = recorder.shutdown();
            bail!("failed to start recording: {e:?} (stopped={stop_recording:?}, shutdown={shutdown:?})");
        }

        // the ingame time when we start recording
        // this is important when the app gets started and starts recording in the middle of a game
        let ingame_time_rec_start_offset = ingame_client
            .game_stats()
            .await
            .map(|stats| stats.game_time)
            .unwrap_or_default();

        Ok(Rec {
            recorder,
            metadata: Metadata {
                game_id,
                output_filepath,
                ingame_time_rec_start_offset,
            },
        })
    }

    async fn setup_recorder(ctx: Ctx<'_>) -> Result<(Recorder, PathBuf)> {
        // try to get window handle for 15s
        let mut window_handle = None;
        for _ in 0..30 {
            window_handle = get_lol_window();

            if window_handle.is_some() {
                break;
            }

            sleep(Duration::from_millis(500)).await;
        }

        let window_size = get_window_size(window_handle.context("no LoL ingame window found")?)?;

        let settings_state = ctx.app_handle.state::<SettingsWrapper>();

        // either get the explicitly set resolution or choose the default resolution for the LoL window aspect ratio
        let output_resolution = settings_state
            .get_output_resolution()
            .unwrap_or_else(|| StdResolution::closest_std_resolution(&window_size));

        log::info!("Using resolution ({output_resolution:?}) for window ({window_size:?})");

        let mut settings = RecorderSettings::new();
        settings.set_window(Window::new(
            WINDOW_TITLE,
            Some(WINDOW_CLASS.into()),
            Some(WINDOW_PROCESS.into()),
        ));
        settings.set_input_resolution(window_size);
        settings.set_output_resolution(output_resolution);
        settings.set_framerate(settings_state.get_framerate());
        settings.set_rate_control(RateControl::CQP(settings_state.get_encoding_quality()));
        settings.record_audio(settings_state.get_audio_source());

        let mut filename = settings_state.get_filename_format();
        if !filename.ends_with(".mp4") {
            filename.push_str(".mp4");
        }
        let filename_path = settings_state
            .get_recordings_path()
            .join(format!("{}", chrono::Local::now().format(&filename)));
        settings.set_output_path(
            filename_path
                .to_str()
                .context("filename_path is not a valid UTF-8 string")?,
        );

        let mut recorder = Recorder::new_with_paths(
            ctx.app_handle
                .path_resolver()
                .resolve_resource("libobs/extprocess_recorder.exe"),
            None,
            None,
            None,
        )?;

        recorder.configure(&settings)?;
        log::info!("recorder configured");
        log::info!("Available encoders: {:?}", recorder.available_encoders());
        log::info!("Selected encoder: {:?}", recorder.selected_encoder());

        Ok((recorder, filename_path))
    }
}
