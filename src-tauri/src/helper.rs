use std::{fs::create_dir_all, path::PathBuf};

use chrono::Local;
use tauri::api::path::video_dir;

pub fn get_new_filepath() -> String {
    let filename = format!("{}", Local::now().format("%Y-%m-%d_%H-%M-%S.mp4"));
    let mut vid_dir = get_recordings_folder();
    vid_dir.push(PathBuf::from(filename));
    vid_dir.to_str().unwrap().into()
}

pub fn get_recordings_folder() -> PathBuf {
    let mut rec_dir = video_dir().unwrap();
    rec_dir.push(PathBuf::from("league_recordings"));
    if !rec_dir.exists() {
        let _ = create_dir_all(rec_dir.as_path());
    }
    return rec_dir;
}

pub fn get_recordings() -> Vec<PathBuf> {
    let mut recordings = Vec::<PathBuf>::new();
    let rec_folder = get_recordings_folder();
    let rd_dir = rec_folder.read_dir().unwrap();
    for entry in rd_dir {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
                println!(
                    "{} {}",
                    path.file_name().unwrap().to_str().unwrap(),
                    path.extension().unwrap().to_str().unwrap()
                );
            }
            if let Some(ext) = path.extension() {
                if ext == "mp4" {
                    recordings.push(path);
                }
            }
        }
    }
    recordings
}
