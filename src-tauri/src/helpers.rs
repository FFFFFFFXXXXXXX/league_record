use std::{cmp::Ordering, fs::create_dir_all, io, path::PathBuf};

use reqwest::blocking::Client;
use tauri::{api::path::video_dir, AppHandle, Manager};

pub fn create_client() -> Client {
    let pem = include_bytes!("../riotgames.pem");
    let cert = reqwest::Certificate::from_pem(pem).unwrap();
    let client = Client::builder()
        .add_root_certificate(cert)
        .build()
        .unwrap();
    return client;
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
    // get all mp4 files in ~/Videos/league_recordings
    let rec_folder = get_recordings_folder();
    let rd_dir = if let Ok(rd_dir) = rec_folder.read_dir() {
        rd_dir
    } else {
        return vec![];
    };
    for entry in rd_dir {
        if let Ok(entry) = entry {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "mp4" {
                    recordings.push(path);
                }
            }
        }
    }
    return recordings;
}

pub fn compare_time(a: &PathBuf, b: &PathBuf) -> io::Result<Ordering> {
    let a_time = a.metadata()?.created()?;
    let b_time = b.metadata()?.created()?;
    Ok(a_time.cmp(&b_time).reverse())
}

pub fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
