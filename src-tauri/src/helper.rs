use std::{cmp::Ordering, fs::create_dir_all, io, path::PathBuf};

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

    // get all mp4 files in ~/Videos/league_recordings
    let rec_folder = get_recordings_folder();
    let rd_dir = rec_folder.read_dir().unwrap();
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

    // sort by time created (index 0 is newest)
    recordings.sort_by(|a, b| {
        if let Ok(result) = compare_time(a, b) {
            result
        } else {
            Ordering::Equal
        }
    });
    return recordings;
}

fn compare_time(a: &PathBuf, b: &PathBuf) -> io::Result<Ordering> {
    let a_time = a.metadata()?.created()?;
    let b_time = b.metadata()?.created()?;
    Ok(a_time.cmp(&b_time).reverse())
}
