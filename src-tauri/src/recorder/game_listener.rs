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

use super::recording_task::{GameCtx, Metadata, RecordingTask};
use super::{api, metadata};
use crate::cancellable;
use crate::recorder::RECORDINGS_CHANGED_EVENT;
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
    pub fn new(ctx: ApiCtx) -> Self {
        Self { ctx, state: State::Idle }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut lcu_ws_client = LcuWebsocketClient::connect_with(&self.ctx.credentials).await?;
        lcu_ws_client
            .subscribe(LcuSubscriptionType::JsonApiEvent(api::GAMEFLOW_SESSION.into()))
            .await?;
        lcu_ws_client
            .subscribe(LcuSubscriptionType::JsonApiEvent(api::EOG_STATS_BLOCK.into()))
            .await?;

        let lcu_rest_client = LcuRestClient::from(&self.ctx.credentials);
        match lcu_rest_client.get::<SessionEventData>(api::GAMEFLOW_SESSION).await {
            Ok(init_event_data) => {
                self.state_transition(SubscriptionResponse::Session(init_event_data))
                    .await
            }
            Err(e) => log::info!("no initial event-data: {e}"),
        }

        match lcu_rest_client.get::<SessionEventData>(api::GAMEFLOW_SESSION).await {
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
                            game_id,
                            output_filepath,
                            ingame_time_rec_start_offset,
                        } = metadata;

                        let mut metadata_filepath = output_filepath;
                        metadata_filepath.set_extension("json");

                        match metadata::process_data_with_retry(
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

        log::info!("recorder state: {}", self.state);
    }
}
