use std::{
    cmp::Ordering,
    io,
    path::{Path, PathBuf},
};

use tauri::{AppHandle, Manager, Window};

use crate::state::WindowState;

pub fn get_recordings(rec_folder: &PathBuf) -> Vec<PathBuf> {
    // get all mp4 files in ~/Videos/%folder-name%
    let mut recordings = Vec::<PathBuf>::new();
    let rd_dir = match rec_folder.read_dir() {
        Ok(rd_dir) => rd_dir,
        Err(_) => return vec![],
    };
    for entry in rd_dir.flatten() {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "mp4" {
                recordings.push(path);
            }
        }
    }
    recordings
}

pub fn compare_time(a: &Path, b: &Path) -> io::Result<Ordering> {
    let a_time = a.metadata()?.created()?;
    let b_time = b.metadata()?.created()?;
    Ok(a_time.cmp(&b_time).reverse())
}

pub fn create_window(app_handle: &AppHandle) {
    let windows = app_handle.windows();
    if let Some(main) = windows.get("main") {
        let _ = main.show();
    } else {
        let window_state = app_handle.state::<WindowState>();

        let builder = tauri::Window::builder(
            app_handle,
            "main",
            tauri::WindowUrl::App(PathBuf::from("/")),
        );

        let size = *window_state.size.lock().unwrap();
        let position = *window_state.position.lock().unwrap();
        builder
            .title("LeagueRecord")
            .inner_size(size.0, size.1)
            .position(position.0, position.1)
            .min_inner_size(800.0, 450.0)
            .visible(false)
            .build()
            .expect("error creating window");
    }
}

pub fn set_window_state(app_handle: &AppHandle, window: &Window) {
    let scale_factor = window.scale_factor().unwrap();
    let window_state = app_handle.state::<WindowState>();

    if let Ok(size) = window.inner_size() {
        *window_state.size.lock().unwrap() = (
            (size.width as f64) / scale_factor,
            (size.height as f64) / scale_factor,
        );
    }
    if let Ok(position) = window.outer_position() {
        *window_state.position.lock().unwrap() = (
            (position.x as f64) / scale_factor,
            (position.y as f64) / scale_factor,
        );
    }
}
