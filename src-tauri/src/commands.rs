use std::{
    cmp::Ordering,
    fs::{metadata, remove_file, File},
    io::BufReader,
    path::PathBuf,
};

use crate::helpers::{compare_time, get_recordings, get_recordings_folder as get_rec_folder};
use serde_json::Value;

#[tauri::command]
pub async fn get_recordings_size() -> f64 {
    let mut size = 0;
    for file in get_recordings() {
        if let Ok(metadata) = metadata(file) {
            size += metadata.len();
        }
    }
    size as f64 / 1_000_000_000.0 // in Gigabyte
}

#[tauri::command]
pub async fn delete_video(video: String) -> bool {
    // remove video
    let mut path = get_rec_folder();
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
pub async fn get_recordings_folder() -> String {
    let folder: PathBuf = get_rec_folder();
    if let Ok(string) = folder.into_os_string().into_string() {
        string
    } else {
        String::new()
    }
}

#[tauri::command]
pub async fn get_recordings_list() -> Vec<String> {
    let mut recordings = get_recordings();
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
pub async fn get_metadata(video: String) -> Option<Value> {
    let mut path = get_rec_folder();
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
