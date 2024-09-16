use std::cmp::Ordering;
use std::fs::metadata;
use std::path::PathBuf;
use std::process::Command;

use tauri::{AppHandle, State};

use crate::app::{action, RecordingManager};
use crate::recorder::MetadataFile;
use crate::state::{MarkerFlags, SettingsFile, SettingsWrapper};
use crate::util::compare_time;

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_marker_flags(settings: State<SettingsWrapper>) -> MarkerFlags {
    settings.get_marker_flags()
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn set_marker_flags(
    marker_flags: MarkerFlags,
    settings: State<SettingsWrapper>,
    settings_file: State<SettingsFile>,
) {
    settings.set_marker_flags(marker_flags);
    settings.write_to_file(settings_file.get());
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_recordings_path(settings: State<SettingsWrapper>) -> PathBuf {
    settings.get_recordings_path().to_path_buf()
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_recordings_size(app_handle: AppHandle) -> f32 {
    let mut size = 0;
    for file in app_handle.get_recordings() {
        if let Ok(metadata) = metadata(file) {
            size += metadata.len();
        }
    }
    size as f32 / 1_000_000_000.0 // in Gigabyte
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    video_id: String,
    metadata: Option<MetadataFile>,
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_recordings_list(app_handle: AppHandle) -> Vec<Recording> {
    let mut recordings = app_handle.get_recordings();
    // sort by time created (index 0 is newest)
    recordings.sort_by(|a, b| compare_time(a, b).unwrap_or(Ordering::Equal));
    let mut ret = Vec::new();
    for path in recordings {
        if let Some(video_id) = path
            .file_name()
            .and_then(|fname| fname.to_os_string().into_string().ok())
        {
            let metadata = action::get_recording_metadata(&path, true).ok();
            ret.push(Recording { video_id, metadata });
        }
    }
    ret
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn open_recordings_folder(state: State<SettingsWrapper>) {
    if let Err(e) = state
        .get_recordings_path()
        .canonicalize()
        .and_then(|path| Command::new("explorer").arg(path).spawn())
    {
        log::error!("failed to open recordings-folder: {e:?}");
    }
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn rename_video(video_id: String, new_video_id: String, state: State<SettingsWrapper>) -> bool {
    let recording = state.get_recordings_path().join(video_id);
    action::rename_recording(recording, new_video_id).unwrap_or_else(|e| {
        log::error!("failed to rename video: {e}");
        false
    })
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn delete_video(video_id: String, state: State<SettingsWrapper>) -> bool {
    let recording = state.get_recordings_path().join(video_id);

    match action::delete_recording(recording) {
        Ok(_) => true,
        Err(e) => {
            log::error!("failed to delete video: {e}");
            false
        }
    }
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_metadata(video_id: String, state: State<SettingsWrapper>) -> Option<MetadataFile> {
    let path = state.get_recordings_path().join(video_id);
    action::get_recording_metadata(&path, true).ok()
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn toggle_favorite(video_id: String, state: State<SettingsWrapper>) -> Option<bool> {
    let path = state.get_recordings_path().join(video_id);

    let mut metadata = action::get_recording_metadata(&path, true).ok()?;
    let favorite = !metadata.is_favorite();
    metadata.set_favorite(favorite);
    action::save_recording_metadata(&path, &metadata).ok()?;

    Some(favorite)
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn confirm_delete(settings: State<SettingsWrapper>) -> bool {
    settings.confirm_delete()
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn disable_confirm_delete(settings: State<SettingsWrapper>, settings_file: State<SettingsFile>) {
    settings.set_confirm_delete(false);
    settings.write_to_file(settings_file.get());
}
