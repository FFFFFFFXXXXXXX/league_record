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
    helpers::{compare_time, get_recordings},
    state::{AssetPort, Settings},
};
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn get_polling_interval(state: State<'_, Settings>) -> Result<u32, ()> {
    Ok(state.polling_interval())
}

#[tauri::command]
pub async fn get_marker_flags(state: State<'_, Settings>) -> Result<Value, ()> {
    Ok(state.marker_flags())
}

#[tauri::command]
pub fn get_asset_port(state: State<'_, AssetPort>) -> u16 {
    state.get()
}

#[tauri::command]
pub async fn get_recordings_size(state: State<'_, Settings>) -> Result<f64, ()> {
    let mut size = 0;
    for file in get_recordings(state.recordings_folder()) {
        if let Ok(metadata) = metadata(file) {
            size += metadata.len();
        }
    }
    Ok(size as f64 / 1_000_000_000.0) // in Gigabyte
}

#[tauri::command]
pub async fn get_recordings_list(state: State<'_, Settings>) -> Result<Vec<String>, ()> {
    let mut recordings = get_recordings(state.recordings_folder());
    // sort by time created (index 0 is newest)
    recordings.sort_by(|a, b| {
        if let Ok(result) = compare_time(a, b) {
            result
        } else {
            Ordering::Equal
        }
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
pub fn get_recordings_folder(state: State<'_, Settings>) -> Result<String, ()> {
    state.recordings_folder_as_string()
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
    let reader = if let Ok(file) = File::open(path) {
        BufReader::new(file)
    } else {
        return Ok(Value::Null);
    };

    if let Ok(json) = serde_json::from_reader::<BufReader<File>, Value>(reader) {
        Ok(json)
    } else {
        Ok(Value::Null)
    }
}
