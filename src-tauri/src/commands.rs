/*
    Notice: Some commands return a Result even though it is not necessary,
    because async tauri::commands have some kind of bug where they don't compile if they
    return just a value
*/

use std::cmp::Ordering;
use std::fs::{metadata, read_to_string, rename, write, File};
use std::io::BufReader;
use std::path::PathBuf;

use tauri::{api::shell, AppHandle, Manager, State};

use crate::game_data::GameMetadata;
use crate::helpers::{self, compare_time, delete_recording, get_recordings, show_window};
use crate::state::{MarkerFlags, SettingsFile, SettingsWrapper};

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub async fn show_app_window(app_handle: AppHandle) {
    if let Some(main) = app_handle.windows().get("main") {
        show_window(main);
    }
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_marker_flags(settings: State<'_, SettingsWrapper>) -> MarkerFlags {
    settings.get_marker_flags()
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn set_marker_flags(
    marker_flags: MarkerFlags,
    settings: State<'_, SettingsWrapper>,
    settings_file: State<'_, SettingsFile>,
) {
    settings.set_marker_flags(marker_flags);
    settings.write_to_file(settings_file.get());
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_recordings_path(settings: State<'_, SettingsWrapper>) -> PathBuf {
    settings.get_recordings_path().to_path_buf()
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_recordings_size(app_handle: AppHandle) -> f32 {
    let mut size = 0;
    for file in get_recordings(&app_handle) {
        if let Ok(metadata) = metadata(file) {
            size += metadata.len();
        }
    }
    size as f32 / 1_000_000_000.0 // in Gigabyte
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Recording {
    video_id: String,
    metadata: Option<GameMetadata>,
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_recordings_list(app_handle: AppHandle) -> Vec<Recording> {
    let mut recordings = get_recordings(&app_handle);
    // sort by time created (index 0 is newest)
    recordings.sort_by(|a, b| compare_time(a, b).unwrap_or(Ordering::Equal));
    let mut ret = Vec::new();
    for path in recordings {
        if let Some(video_id) = path
            .file_name()
            .and_then(|fname| fname.to_os_string().into_string().ok())
        {
            let metadata = helpers::get_metadata(&path).ok();
            ret.push(Recording { video_id, metadata });
        }
    }
    ret
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn open_recordings_folder(app_handle: AppHandle, state: State<'_, SettingsWrapper>) {
    _ = shell::open(
        &app_handle.shell_scope(),
        helpers::path_to_string(&state.get_recordings_path()),
        None,
    );
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn rename_video(video_id: String, new_video_id: String, state: State<'_, SettingsWrapper>) -> bool {
    let new = PathBuf::from(&new_video_id);
    let Some(new_filename) = new.file_name() else { return false };

    let mut path = state.get_recordings_path().join(video_id);
    let mut new_path = path.clone();
    new_path.set_file_name(new_filename);

    if new_path.exists() {
        return false;
    }

    if rename(&path, &new_path).is_err() {
        return false;
    }

    path.set_extension("json");
    new_path.set_extension("json");
    _ = rename(&path, &new_path);
    true
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn delete_video(video_id: String, state: State<'_, SettingsWrapper>) -> bool {
    let recording = state.get_recordings_path().join(video_id);
    if let Err(e) = delete_recording(recording) {
        log::error!("deleting video failed: {e}");
    }

    true
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_metadata(video_id: String, state: State<'_, SettingsWrapper>) -> Option<GameMetadata> {
    let mut path = state.get_recordings_path().join(video_id);
    path.set_extension("json");

    let reader = BufReader::new(File::open(path).ok()?);
    serde_json::from_reader::<_, GameMetadata>(reader).ok()
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn toggle_favorite(video_id: String, state: State<'_, SettingsWrapper>) -> Option<bool> {
    let mut path = state.get_recordings_path().join(video_id);
    path.set_extension("json");

    let metadata_json = read_to_string(&path).ok()?;
    let mut metadata = serde_json::from_str::<GameMetadata>(&metadata_json).ok()?;
    metadata.favorite = !metadata.favorite;

    let metadata_json = serde_json::to_string(&metadata).ok()?;
    write(&path, metadata_json).ok()?;

    Some(metadata.favorite)
}
