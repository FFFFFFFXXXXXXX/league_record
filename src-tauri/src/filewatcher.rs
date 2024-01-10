use std::path::PathBuf;

use notify::Watcher;
use tauri::{AppHandle, Manager};

use crate::state::FileWatcher;

pub fn replace_filewatcher(app_handle: &AppHandle, recordings_path: &PathBuf) {
    let watcher = notify::recommended_watcher({
        let app_handle = app_handle.clone();
        move |res: notify::Result<notify::Event>| {
            log::info!("filewatcher event: {:?}", res);

            // only trigger UI reload if one of the changed paths is a video (.mp4) file
            if let Ok(event) = res {
                let contains_mp4_path = event
                    .paths
                    .iter()
                    .find(|p| p.extension().is_some_and(|ext| ext == "mp4"))
                    .is_some();

                if contains_mp4_path {
                    _ = app_handle.emit_all("reload_recordings", ());
                }
            }
        }
    });

    match watcher {
        Ok(mut watcher) => {
            _ = watcher.watch(&recordings_path, notify::RecursiveMode::NonRecursive);

            // store Watcher so it doesn't drop and stop watching
            // also drop old watcher
            app_handle.state::<FileWatcher>().set(watcher);
        }
        Err(e) => log::error!("failed to start filewatcher: {e}"),
    }
}
