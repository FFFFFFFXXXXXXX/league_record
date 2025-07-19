use std::path::PathBuf;
use std::{ffi::OsStr, path::Path};

use notify::event::{ModifyKind, RenameMode};
use notify::{EventKind, Watcher};
use tauri::{AppHandle, Manager};

use crate::app::{AppEvent, EventManager};
use crate::state::CurrentlyRecording;
use crate::state::FileWatcher;

pub fn replace(app_handle: &AppHandle, recordings_path: &Path) {
    let watcher = notify::recommended_watcher({
        let app_handle = app_handle.clone();
        move |res: notify::Result<notify::Event>| {
            let Ok(event) = res else { return };

            let currently_recording: Option<PathBuf> = app_handle.state::<CurrentlyRecording>().get();

            let mut contains_mp4_path: bool = false;
            let mut json_paths: Vec<String> = Vec::new();

            for path in event.paths {
                if Some(&path) == currently_recording.as_ref() {
                    continue;
                }

                let ext = path.extension().and_then(OsStr::to_str);

                contains_mp4_path |= ext == Some("mp4");

                if ext == Some("json") {
                    if let Some(video_id) = path.file_name().and_then(OsStr::to_str).map(str::to_owned) {
                        json_paths.push(video_id);
                    }
                }
            }

            match event.kind {
                EventKind::Create(_) => {
                    if contains_mp4_path {
                        log::info!("filewatcher event contains .mp4 path: {contains_mp4_path}");
                        if let Err(e) = app_handle.send_event(AppEvent::RecordingsChanged { payload: () }) {
                            log::warn!("filewatcher failed to send event: {e:?}");
                        }
                    }

                    if !json_paths.is_empty() {
                        log::info!("filewatcher event json paths: {json_paths:?}");
                        if let Err(e) = app_handle.send_event(AppEvent::MetadataChanged { payload: json_paths }) {
                            log::warn!("filewatcher failed to send event: {e:?}");
                        }
                    }
                }
                EventKind::Remove(_) => {
                    if contains_mp4_path {
                        log::info!("filewatcher event contains .mp4 path: {contains_mp4_path}");
                        if let Err(e) = app_handle.send_event(AppEvent::RecordingsChanged { payload: () }) {
                            log::warn!("filewatcher failed to send event: {e:?}");
                        }
                    }

                    if !json_paths.is_empty() {
                        log::info!("filewatcher event json paths: {json_paths:?}");
                        if let Err(e) = app_handle.send_event(AppEvent::MetadataChanged { payload: json_paths }) {
                            log::warn!("filewatcher failed to send event: {e:?}");
                        }
                    }
                }
                EventKind::Modify(ModifyKind::Name(
                    RenameMode::To | RenameMode::Both | RenameMode::Any | RenameMode::Other,
                )) => {
                    if contains_mp4_path {
                        log::info!("filewatcher event contains .mp4 path: {contains_mp4_path}");
                        if let Err(e) = app_handle.send_event(AppEvent::RecordingsChanged { payload: () }) {
                            log::warn!("filewatcher failed to send event: {e:?}");
                        }
                    }

                    if !json_paths.is_empty() {
                        log::info!("filewatcher event json paths: {json_paths:?}");
                        if let Err(e) = app_handle.send_event(AppEvent::MetadataChanged { payload: json_paths }) {
                            log::warn!("filewatcher failed to send event: {e:?}");
                        }
                    }
                }
                _ => {}
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
