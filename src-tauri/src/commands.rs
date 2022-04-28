use std::{
    cmp::Ordering,
    fs::{metadata, remove_file, File},
    io::BufReader,
    path::PathBuf,
};

use crate::{
    helpers::{compare_time, get_recordings, get_recordings_folder as get_rec_folder},
    state::{AssetPort, RecordingsFolder},
};
use serde_json::Value;
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub fn get_asset_port(state: tauri::State<'_, AssetPort>) -> u16 {
    state.get()
}

#[tauri::command]
pub async fn get_recordings_size<R: Runtime>(app_handle: AppHandle<R>) -> f64 {
    let mut size = 0;
    for file in get_recordings(&app_handle) {
        if let Ok(metadata) = metadata(file) {
            size += metadata.len();
        }
    }
    size as f64 / 1_000_000_000.0 // in Gigabyte
}

#[tauri::command]
pub async fn get_recordings_list<R: Runtime>(app_handle: AppHandle<R>) -> Vec<String> {
    let mut recordings = get_recordings(&app_handle);
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
    return ret;
}

#[tauri::command]
pub fn get_recordings_folder(state: tauri::State<'_, RecordingsFolder>) -> String {
    let folder = state.get_as_string();
    if let Ok(string) = folder {
        string
    } else {
        String::new()
    }
}

#[tauri::command]
pub async fn delete_video<R: Runtime>(video: String, app_handle: AppHandle<R>) -> bool {
    // remove video
    let mut path = get_rec_folder(&app_handle);
    path.push(PathBuf::from(&video));
    let ok = match remove_file(&path) {
        Ok(_) => true,
        Err(_) => false,
    };

    // remove json file if it exists
    path.set_extension("json");
    let _ = remove_file(path);
    return ok;
}

#[tauri::command]
pub async fn get_metadata<R: Runtime>(video: String, app_handle: AppHandle<R>) -> Option<Value> {
    let mut path = get_rec_folder(&app_handle);
    path.push(PathBuf::from(video));
    path.set_extension("json");
    let reader = if let Ok(file) = File::open(path) {
        BufReader::new(file)
    } else {
        return None;
    };

    if let Ok(json) = serde_json::from_reader::<BufReader<File>, Value>(reader) {
        Some(json)
    } else {
        None
    }
}
