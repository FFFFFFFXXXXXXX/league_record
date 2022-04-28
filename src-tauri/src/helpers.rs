use std::{cmp::Ordering, io, path::PathBuf};

use reqwest::blocking::Client;
use tauri::{AppHandle, Manager, Runtime};

use crate::state::RecordingsFolder;

pub fn create_client() -> Client {
    let pem = include_bytes!("../riotgames.pem");
    let cert = reqwest::Certificate::from_pem(pem).unwrap();
    let client = Client::builder()
        .add_root_certificate(cert)
        .build()
        .unwrap();
    return client;
}

pub fn get_recordings_folder<R: Runtime>(app_handle: &AppHandle<R>) -> PathBuf {
    app_handle.state::<RecordingsFolder>().get()
}

pub fn get_recordings<R: Runtime>(app_handle: &AppHandle<R>) -> Vec<PathBuf> {
    let mut recordings = Vec::<PathBuf>::new();
    // get all mp4 files in ~/Videos/league_recordings
    let rec_folder = get_recordings_folder(app_handle);
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

pub fn show_window(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
