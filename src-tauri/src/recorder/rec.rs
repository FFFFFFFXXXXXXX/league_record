use std::{fmt::Display, path::PathBuf, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use futures_util::StreamExt;
use libobs_recorder::settings::{RateControl, Resolution, StdResolution, Window};
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
use super::window::{self, WINDOW_CLASS, WINDOW_PROCESS, WINDOW_TITLE};
use crate::cancellable;
use crate::game_data::{self, GameId};
use crate::helpers::cleanup_recordings;
use crate::helpers::set_recording_tray_item;
use crate::state::{CurrentlyRecording, SettingsWrapper};

const RECORDINGS_CHANGED_EVENT: &str = "recordings_changed";

pub struct LeagueRecorder {
    cancel_token: CancellationToken,
    task: async_runtime::Mutex<JoinHandle<()>>,
}

impl LeagueRecorder {
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
            if timeout(Duration::from_secs(1), &mut *task).await.is_err() {
                log::warn!("RecordingTask stop() ran into timeout - aborting task");
                task.abort();
            }
        });
    }
}

enum State {
    Idle,
    Recording(RecordingTask),
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

#[derive(Debug)]
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
            state = state_transition(state, SubscriptionResponse::Session(init_event_data), ctx).await
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
        // wait for game to record
        State::Idle => match sub_resp {
            SubscriptionResponse::Session(SessionEventData {
                phase: GamePhase::GameStart | GamePhase::InProgress,
                game_data: GameData { queue, game_id },
            }) if queue.is_ranked || !ctx.app_handle.state::<SettingsWrapper>().only_record_ranked() => {
                State::Recording(RecordingTask::start(game_id, ctx))
            }
            _ => State::Idle,
        },

        // wait for game to end => stop recording
        State::Recording(recording_task) => match sub_resp {
            SubscriptionResponse::Session(SessionEventData {
                phase:
                    phase @ (GamePhase::FailedToLaunch
                    | GamePhase::Reconnect
                    | GamePhase::WaitingForStats
                    | GamePhase::PreEndOfGame),
                ..
            }) => {
                log::info!("stopping recording due to session event phase: {phase:?}");

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

        // wait for game-data to become available
        State::EndOfGame(metadata) => match sub_resp {
            ws_msg @ (SubscriptionResponse::EogStatsBlock {}
            | SubscriptionResponse::Session(SessionEventData {
                phase:
                    GamePhase::EndOfGame | GamePhase::TerminatedInError | GamePhase::ChampSelect | GamePhase::GameStart,
                ..
            })) => {
                log::info!("triggered game-data collection due to msg: {ws_msg:?}");

                // spawn task to handle collecting data so we don't block the recorder for too long
                // because game_data::process_data(...) can take up to a minute to finish when re-trying
                let ctx = CtxOwned::from(ctx);
                async_runtime::spawn(async move {
                    let Metadata {
                        game_id,
                        output_filepath,
                        ingame_time_rec_start_offset,
                    } = metadata;

                    let mut metadata_filepath = output_filepath;
                    metadata_filepath.set_extension("json");

                    match game_data::process_data_with_retry(
                        ingame_time_rec_start_offset,
                        game_id,
                        &ctx.credentials,
                        &ctx.cancel_token,
                    )
                    .await
                    {
                        Ok(game_metadata) => {
                            log::info!("writing game metadata to file: {metadata_filepath:?}");

                            if let Ok(file) = std::fs::File::create(&metadata_filepath) {
                                let result = serde_json::to_writer(&file, &game_metadata);
                                log::info!("metadata saved: {result:?}");
                            }
                        }
                        Err(e) => log::error!("unable to process data: {e}"),
                    }

                    if let Err(e) = ctx.app_handle.emit_all(RECORDINGS_CHANGED_EVENT, ()) {
                        log::error!("failed to send 'RECORDINGS_CHANGED_EVENT' to UI: {e}");
                    }
                });

                State::Idle
            }
            _ => State::EndOfGame(metadata),
        },
    };

    log::info!("recorder state: {next_state}");
    next_state
}

struct RecordingTask {
    join_handle: JoinHandle<Result<Rec>>,
    ctx: CtxOwned,
}

impl RecordingTask {
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

        cleanup_recordings(&self.ctx.app_handle);
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

        // save (GameId, rec_start_offset) tuple from which we can later fetch the data if we don't succeed on the first try
        let mut metadata_filepath = output_filepath.clone();
        metadata_filepath.set_extension("json");
        if let Err(e) = std::fs::File::create(&metadata_filepath)
            .map_err(anyhow::Error::msg)
            .and_then(|file| {
                serde_json::to_writer(&file, &(game_id, ingame_time_rec_start_offset)).map_err(anyhow::Error::msg)
            })
        {
            log::info!("failed to save (game_id, rec_offset) tuple: {e}")
        }

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
        let settings_state = ctx.app_handle.state::<SettingsWrapper>();

        let window_size = Self::get_window_size().await?;
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

    async fn get_window_size() -> Result<Resolution> {
        let mut window_handle = None;
        for _ in 0..30 {
            window_handle = window::get_lol_window();
            if window_handle.is_some() {
                break;
            }

            sleep(Duration::from_millis(500)).await;
        }

        let Some(window_handle) = window_handle else { bail!("unable to get window_handle") };
        for _ in 0..30 {
            if let Ok(window_size) = window::get_window_size(window_handle) {
                return Ok(window_size);
            }

            sleep(Duration::from_millis(500)).await;
        }

        bail!("unable to get window size");
    }
}
