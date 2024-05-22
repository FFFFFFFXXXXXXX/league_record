use std::ffi::OsStr;
use std::fmt::Display;

use anyhow::Result;
use futures_util::StreamExt;
use riot_datatypes::lcu::{GameData, GamePhase, SessionEventData, SubscriptionResponse};
use riot_datatypes::{GameId, MatchId};
use riot_local_auth::Credentials;
use shaco::model::ws::{EventType, LcuSubscriptionType};
use shaco::{rest::LcuRestClient, ws::LcuWebsocketClient};
use tauri::async_runtime;
use tauri::{AppHandle, Manager};
use tokio::select;
use tokio_util::sync::CancellationToken;

use super::metadata;
use super::recording_task::{GameCtx, Metadata, RecordingTask};
use crate::app::{AppEvent, EventManager, RecordingManager};
use crate::cancellable;
use crate::state::SettingsWrapper;

#[derive(Clone)]
pub struct ApiCtx {
    pub app_handle: AppHandle,
    pub credentials: Credentials,
    pub platform_id: String,
    pub cancel_token: CancellationToken,
}

impl ApiCtx {
    fn game_ctx(&self, game_id: GameId) -> GameCtx {
        GameCtx {
            app_handle: self.app_handle.clone(),
            match_id: MatchId {
                game_id,
                platform_id: self.platform_id.clone(),
            },
            cancel_token: self.cancel_token.child_token(),
        }
    }
}

#[derive(Default)]
enum State {
    #[default]
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

pub struct GameListener {
    ctx: ApiCtx,
    state: State,
}

impl GameListener {
    const GAMEFLOW_SESSION: &'static str = "/lol-gameflow/v1/session";
    const EOG_STATS_BLOCK: &'static str = "/lol-end-of-game/v1/eog-stats-block";

    pub fn new(ctx: ApiCtx) -> Self {
        Self { ctx, state: State::Idle }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut lcu_ws_client = LcuWebsocketClient::connect_with(&self.ctx.credentials).await?;
        lcu_ws_client
            .subscribe(LcuSubscriptionType::JsonApiEvent(Self::GAMEFLOW_SESSION.into()))
            .await?;
        lcu_ws_client
            .subscribe(LcuSubscriptionType::JsonApiEvent(Self::EOG_STATS_BLOCK.into()))
            .await?;

        let lcu_rest_client = LcuRestClient::from(&self.ctx.credentials);
        match lcu_rest_client.get::<SessionEventData>(Self::GAMEFLOW_SESSION).await {
            Ok(init_event_data) => {
                self.state_transition(SubscriptionResponse::Session(init_event_data))
                    .await
            }
            Err(e) => log::info!("no initial event-data: {e}"),
        }

        match lcu_rest_client.get::<SessionEventData>(Self::GAMEFLOW_SESSION).await {
            Ok(init_event_data) => {
                self.state_transition(SubscriptionResponse::Session(init_event_data))
                    .await
            }
            Err(e) => log::info!("no initial event-data: {e}"),
        }

        while let Some(event) = cancellable!(lcu_ws_client.next(), self.ctx.cancel_token, Option) {
            if event.payload.event_type != EventType::Update {
                continue;
            }

            match serde_json::from_value::<SubscriptionResponse>(event.payload.data) {
                Ok(event_data) => self.state_transition(event_data).await,
                Err(e) => {
                    log::error!("failed to deserialize event: {e}");
                    continue;
                }
            }
        }

        if let State::Recording(recording_task) = std::mem::take(&mut self.state) {
            _ = recording_task.stop().await;
        }

        Ok(())
    }

    async fn state_transition(&mut self, sub_resp: SubscriptionResponse) {
        self.state = match std::mem::take(&mut self.state) {
            // wait for game to record
            State::Idle => match sub_resp {
                SubscriptionResponse::Session(SessionEventData {
                    phase: GamePhase::GameStart | GamePhase::InProgress,
                    game_data: GameData { queue, game_id },
                }) if queue.is_ranked || !self.ctx.app_handle.state::<SettingsWrapper>().only_record_ranked() => {
                    State::Recording(RecordingTask::new(self.ctx.game_ctx(game_id)))
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
                    let ctx = self.ctx.clone();
                    async_runtime::spawn(async move {
                        let Metadata {
                            match_id,
                            output_filepath,
                            ingame_time_rec_start_offset,
                        } = metadata;

                        let mut metadata_filepath = output_filepath;
                        let video_id = metadata_filepath.file_name().and_then(OsStr::to_str).map(str::to_owned);
                        metadata_filepath.set_extension("json");

                        match metadata::process_data_with_retry(
                            ingame_time_rec_start_offset,
                            match_id,
                            &ctx.credentials,
                            &ctx.cancel_token,
                        )
                        .await
                        {
                            Ok(game_metadata) => {
                                let result = AppHandle::save_recording_metadata(
                                    &metadata_filepath,
                                    &crate::recorder::MetadataFile::Metadata(game_metadata),
                                );
                                log::info!("writing game metadata to ({metadata_filepath:?}): {result:?}");
                            }
                            Err(e) => log::error!("unable to process data: {e}"),
                        }

                        if let Some(video_id) = video_id {
                            if let Err(e) = ctx
                                .app_handle
                                .send_event(AppEvent::MetadataChanged { payload: vec![video_id] })
                            {
                                log::error!("GameListener failed to send event: {e}");
                            }
                        }
                    });

                    State::Idle
                }
                _ => State::EndOfGame(metadata),
            },
        };

        log::info!("recorder state: {}", self.state);
    }
}
