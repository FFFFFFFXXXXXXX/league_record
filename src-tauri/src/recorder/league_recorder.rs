use std::time::Duration;

use shaco::rest::LcuRestClient;
use tauri::async_runtime::{self, JoinHandle, Mutex};
use tauri::AppHandle;
use tokio::select;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;

use super::game_listener::{ApiCtx, GameListener};
use crate::cancellable;

pub struct LeagueRecorder {
    cancel_token: CancellationToken,
    task: Mutex<JoinHandle<()>>,
}

impl LeagueRecorder {
    const PLATFORM_ID: &'static str = "/lol-platform-config/v1/namespaces/LoginDataPacket/platformId";

    pub fn new(app_handle: AppHandle) -> Self {
        let cancel_token = CancellationToken::new();

        let task = async_runtime::spawn({
            let cancel_token = cancel_token.child_token();

            async move {
                log::info!("waiting for LCU API");

                loop {
                    if let Ok(credentials) = riot_local_auth::lcu::try_get_credentials() {
                        let lcu_rest_client = LcuRestClient::from(&credentials);

                        if let Ok(platform_id) = lcu_rest_client.get::<String>(Self::PLATFORM_ID).await {
                            let ctx = ApiCtx {
                                app_handle: app_handle.clone(),
                                credentials,
                                platform_id,
                                cancel_token: cancel_token.clone(),
                            };

                            if let Err(e) = GameListener::new(ctx).run().await {
                                log::error!("stopped listening for games: {e}");
                            }
                        }
                    }

                    let cancelled = cancellable!(sleep(Duration::from_secs(1)), cancel_token, ());
                    if cancelled {
                        log::info!("task cancelled (wait_for_api)");
                        return;
                    }
                }
            }
        });

        Self {
            cancel_token,
            task: Mutex::new(task),
        }
    }

    pub async fn stop(&self) {
        self.cancel_token.cancel();

        let Ok(mut task) = self.task.try_lock() else { return };
        if timeout(Duration::from_secs(2), &mut *task).await.is_err() {
            log::warn!("RecordingTask stop() ran into timeout - aborting task");
            task.abort();
        }
    }
}
