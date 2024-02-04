use std::{ffi::OsStr, path::Path};

use notify::Watcher;
use tauri::{AppHandle, Manager};

use crate::{state::FileWatcher, CurrentlyRecording};

pub fn replace(app_handle: &AppHandle, recordings_path: &Path) {
    let watcher = notify::recommended_watcher({
        let app_handle = app_handle.clone();
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                let currently_recording = app_handle.state::<CurrentlyRecording>().get();

                log::info!("filewatcher event: {:?}", event.paths);
                log::info!("currently recording: {:?}", currently_recording);

                // only trigger UI sidebar reload if one of the changed paths is a video (.mp4) file
                let contains_mp4_path = event.paths.iter().any(|p| {
                    p.extension().and_then(OsStr::to_str) == Some("mp4")
                        && !currently_recording.as_ref().is_some_and(|curr_rec| curr_rec == p)
                });

                // only trigger UI metadata reload if one of the changed paths is a metadata file (.json) file
                let json_paths: Vec<_> = event
                    .paths
                    .iter()
                    .filter_map(|p| {
                        if p.extension().and_then(OsStr::to_str) == Some("json") {
                            return p.file_stem().and_then(OsStr::to_str);
                        } else {
                            None
                        }
                    })
                    .collect();

                log::info!("filewatcher event contains .mp4 path: {contains_mp4_path}");
                log::info!("filewatcher event json paths: {:?}", json_paths);

                if contains_mp4_path {
                    _ = app_handle.emit_all("recordings_changed", ());
                }

                if !json_paths.is_empty() {
                    _ = app_handle.emit_all("metadata_changed", json_paths);
                }
            }
        }
    });

    match watcher {
        Ok(mut watcher) => {
            _ = watcher.watch(recordings_path, notify::RecursiveMode::NonRecursive);

            // store Watcher so it doesn't drop and stop watching
            // also drop old watcher
            if let Some(fw_state) = app_handle.try_state::<FileWatcher>() {
                fw_state.set(watcher);
            } else {
                app_handle.manage::<FileWatcher>(FileWatcher::new(watcher));
            }
        }
        Err(e) => log::error!("failed to start filewatcher: {e}"),
    }
}
