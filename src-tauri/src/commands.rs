/*
    Notice: Some commands return a Result even though it is not necessary,
    because async tauri::commands have some kind of bug where they don't compile if they
    return just a value
*/

use std::{
    cmp::Ordering,
    fs::{metadata, remove_file, File},
    io::BufReader,
    path::PathBuf,
};

use crate::{
    helpers::{self, compare_time, get_recordings, show_window},
    state::{AssetPort, MarkerFlags, MarkerFlagsState, Settings},
};
use serde_json::Value;
use tauri::{api::shell, AppHandle, Manager, State};

#[tauri::command]
pub async fn show_app_window(app_handle: AppHandle) {
    if let Some(main) = app_handle.windows().get("main") {
        show_window(main);
    }
}

#[tauri::command]
pub async fn get_default_marker_flags(settings_state: State<'_, Settings>) -> Result<Value, ()> {
    Ok(settings_state.marker_flags())
}

#[tauri::command]
pub async fn get_current_marker_flags(
    flag_state: State<'_, MarkerFlagsState>,
) -> Result<Value, ()> {
    let Ok(flags) = flag_state.0.lock() else {
        return Err(());
    };
    match &*flags {
        Some(f) => Ok(f.to_json_value()),
        None => Ok(Value::Null),
    }
}

#[tauri::command]
pub async fn set_current_marker_flags(
    marker_flags: MarkerFlags,
    flag_state: State<'_, MarkerFlagsState>,
) -> Result<(), ()> {
    if let Ok(mut flags) = flag_state.0.lock() {
        *flags = Some(marker_flags);
        return Ok(());
    } else {
        return Err(());
    }
}

#[tauri::command]
pub fn get_asset_port(port_state: State<'_, AssetPort>) -> u16 {
    port_state.get()
}

#[tauri::command]
pub async fn get_recordings_size(settings_state: State<'_, Settings>) -> Result<f32, ()> {
    let mut size = 0;
    for file in get_recordings(&settings_state.recordings_folder()) {
        if let Ok(metadata) = metadata(file) {
            size += metadata.len();
        }
    }
    Ok(size as f32 / 1_000_000_000.0) // in Gigabyte
}

#[tauri::command]
pub async fn get_recordings_list(settings_state: State<'_, Settings>) -> Result<Vec<String>, ()> {
    let mut recordings = get_recordings(&settings_state.recordings_folder());
    // sort by time created (index 0 is newest)
    recordings.sort_by(|a, b| match compare_time(a, b) {
        Ok(result) => result,
        Err(_) => Ordering::Equal,
    });
    let mut ret = Vec::<String>::new();
    for path in recordings {
        if let Some(os_str_ref) = path.file_name() {
            if let Ok(filename) = os_str_ref.to_os_string().into_string() {
                ret.push(filename);
            }
        }
    }
    Ok(ret)
}

#[tauri::command]
pub fn open_recordings_folder(app_handle: AppHandle, state: State<'_, Settings>) {
    let _ = shell::open(
        &app_handle.shell_scope(),
        helpers::path_to_string(&state.recordings_folder()),
        None,
    );
}

#[tauri::command]
pub async fn delete_video(video: String, state: State<'_, Settings>) -> Result<bool, ()> {
    // remove video
    let mut path = state.recordings_folder();
    path.push(PathBuf::from(&video));
    if remove_file(&path).is_err() {
        // if video delete fails return and dont delete json file
        return Ok(false);
    }

    // remove json file if it exists
    path.set_extension("json");
    let _ = remove_file(path);
    Ok(true)
}

#[tauri::command]
pub async fn get_metadata(video: String, state: State<'_, Settings>) -> Result<Value, ()> {
    let mut path = state.recordings_folder();
    path.push(PathBuf::from(video));
    path.set_extension("json");
    let reader = match File::open(path) {
        Ok(file) => BufReader::new(file),
        Err(_) => return Ok(Value::Null),
    };

    match serde_json::from_reader::<BufReader<File>, Value>(reader) {
        Ok(json) => Ok(json),
        Err(_) => Ok(Value::Null),
    }
}
