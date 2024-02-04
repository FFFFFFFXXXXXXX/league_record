/*
    Notice: Some commands return a Result even though it is not necessary,
    because async tauri::commands have some kind of bug where they don't compile if they
    return just a value
*/

use std::{
    cmp::Ordering,
    fs::{metadata, remove_file, rename, File},
    io::BufReader,
    path::PathBuf,
};

use crate::{
    helpers::{self, compare_time, get_recordings, show_window},
    recorder::data::GameData,
    state::{AssetPort, MarkerFlags, SettingsFile, SettingsWrapper},
};
use tauri::{api::shell, AppHandle, Manager, State};

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
    settings.write_to_file(&settings_file.get());
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_asset_port(port_state: State<'_, AssetPort>) -> u16 {
    port_state.get()
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_recordings_size(settings_state: State<'_, SettingsWrapper>) -> f32 {
    let mut size = 0;
    for file in get_recordings(&settings_state.get_recordings_path()) {
        if let Ok(metadata) = metadata(file) {
            size += metadata.len();
        }
    }
    size as f32 / 1_000_000_000.0 // in Gigabyte
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_recordings_list(settings_state: State<'_, SettingsWrapper>) -> Vec<String> {
    let mut recordings = get_recordings(&settings_state.get_recordings_path());
    // sort by time created (index 0 is newest)
    recordings.sort_by(|a, b| compare_time(a, b).unwrap_or(Ordering::Equal));
    let mut ret = Vec::<String>::new();
    for path in recordings {
        if let Some(os_str_ref) = path.file_name() {
            if let Ok(filename) = os_str_ref.to_os_string().into_string() {
                ret.push(filename);
            }
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
    // remove video
    let mut path = state.get_recordings_path();
    path.push(PathBuf::from(&video_id));
    if remove_file(&path).is_err() {
        // if video delete fails return and dont delete json file
        return false;
    }

    // remove json file if it exists
    path.set_extension("json");
    _ = remove_file(path);
    true
}

#[cfg_attr(test, specta::specta)]
#[tauri::command]
pub fn get_metadata(video_id: String, state: State<'_, SettingsWrapper>) -> Option<GameData> {
    let mut path = state.get_recordings_path().join(video_id);
    path.set_extension("json");

    let reader = BufReader::new(File::open(path).ok()?);
    serde_json::from_reader::<_, GameData>(reader).ok()
}
