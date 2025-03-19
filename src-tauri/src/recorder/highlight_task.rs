use shaco::ingame::IngameClient;
use tauri::{async_runtime::JoinHandle, AppHandle, Listener};
use tokio_util::sync::CancellationToken;

use crate::cancellable;

pub struct HighlightTask {
    join_handle: JoinHandle<Vec<f64>>,
    cancel_token: CancellationToken,
}

impl HighlightTask {
    pub fn new(app_handle: AppHandle) -> Self {
        let cancel_token = CancellationToken::new();

        let join_handle = tauri::async_runtime::spawn({
            let cancel_token = cancel_token.clone();

            async move {
                let (tx, mut rx) = tauri::async_runtime::channel(128);
                app_handle.listen("shortcut-event", {
                    let app_handle = app_handle.clone();
                    move |event| {
                        let sent = tx.blocking_send(());
                        if tx.is_closed() || sent.is_err() {
                            app_handle.unlisten(event.id());
                        }
                    }
                });

                let ingame_client = IngameClient::new();
                let mut highlight_timestamps = Vec::new();
                loop {
                    match cancellable!(rx.recv(), cancel_token, Option) {
                        Some(()) => {
                            if let Ok(timestamp) =
                                ingame_client.game_stats().await.map(|stats| stats.game_time * 1000.0)
                            {
                                highlight_timestamps.push(timestamp);
                            }
                        }
                        _ => {
                            rx.close();
                            break;
                        }
                    }
                }

                highlight_timestamps
            }
        });

        Self { join_handle, cancel_token }
    }

    pub async fn stop(self) -> Vec<f64> {
        self.cancel_token.cancel();
        match self.join_handle.await {
            Ok(highlight_data) => highlight_data,
            Err(e) => {
                log::warn!("failed to collect highlight data: {e}");
                vec![]
            }
        }
    }
}
