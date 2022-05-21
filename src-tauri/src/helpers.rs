use std::{cmp::Ordering, io, path::PathBuf};

use tauri::{AppHandle, Manager};

pub fn get_recordings(rec_folder: PathBuf) -> Vec<PathBuf> {
    // get all mp4 files in ~/Videos/%folder-name%
    let mut recordings = Vec::<PathBuf>::new();
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

pub fn create_window(app_handle: &AppHandle) {
    let windows = app_handle.windows();
    if let Some(main) = windows.get("main") {
        let _ = main.show();
    } else {
        let builder = tauri::Window::builder(
            app_handle,
            "main",
            tauri::WindowUrl::App(PathBuf::from("/")),
        );
        builder
            .title("LeagueRecord")
            .inner_size(1298.0, 702.0)
            .center()
            .visible(false)
            .theme(None)
            .build()
            .expect("error creating window");
    }
}
