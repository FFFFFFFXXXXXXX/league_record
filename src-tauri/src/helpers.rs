use std::{
    cmp::Ordering,
    fs, io,
    path::{Path, PathBuf},
};

use reqwest::{blocking::Client, redirect::Policy, StatusCode};
use tauri::{api::version::compare, AppHandle, CustomMenuItem, Manager, SystemTrayMenu, SystemTrayMenuItem, Window};

use crate::state::{Settings, WindowState};

const GITHUB_LATEST: &str = "https://github.com/FFFFFFFXXXXXXX/league_record/releases/latest";

pub fn create_tray_menu() -> SystemTrayMenu {
    SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("rec", "Recording").disabled())
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("settings", "Settings"))
        .add_item(CustomMenuItem::new("open", "Open"))
        .add_item(CustomMenuItem::new("quit", "Quit"))
}

pub fn check_updates(app_handle: &AppHandle, debug_log: bool) {
    let config = app_handle.config();
    let version = config.package.version.as_ref().unwrap();

    let client = match Client::builder().redirect(Policy::none()).build() {
        Ok(c) => c,
        Err(_) => {
            if debug_log {
                println!("Error creating HTTP Client in 'check_updates'");
            }
            return;
        }
    };

    let Ok(result) = client.get(GITHUB_LATEST).send() else {
        if debug_log {
            println!("couldn't GET http result in 'check_updates");
        }
        return;
    };

    if result.status() == StatusCode::FOUND {
        let url = result.headers().get("location").unwrap();
        if let Ok(url) = url.to_str() {
            let new_version = url.rsplit_once("/v").unwrap().1;

            if debug_log {
                println!("Checking for update: {}/{} (current/newest)\n", version, new_version);
            }

            if let Ok(res) = compare(version, new_version) {
                if res == 1 {
                    let tray_menu = create_tray_menu()
                        .add_native_item(SystemTrayMenuItem::Separator)
                        .add_item(CustomMenuItem::new("update", "Update Available!"));
                    _ = app_handle.tray_handle().set_menu(tray_menu);
                }
            }
            return; // skip last log when there was a version to check against
        }
    }

    if debug_log {
        println!("Error somewhere in the HTTP response from {}", GITHUB_LATEST);
    }
}

pub fn get_recordings(rec_folder: &Path) -> Vec<PathBuf> {
    // get all mp4 files in ~/Videos/%folder-name%
    let mut recordings = Vec::<PathBuf>::new();
    let Ok(read_dir) = rec_folder.read_dir() else { return vec![] };

    for entry in read_dir.flatten() {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if let Some(ext) = path.extension() {
            if ext == "mp4" {
                recordings.push(path);
            }
        }
    }
    recordings
}

pub fn path_to_string(path: &PathBuf) -> String {
    path.to_owned().into_os_string().into_string().expect("invalid path")
}

pub fn compare_time(a: &Path, b: &Path) -> io::Result<Ordering> {
    let a_time = a.metadata()?.created()?;
    let b_time = b.metadata()?.created()?;
    Ok(a_time.cmp(&b_time).reverse())
}

pub fn show_window(window: &Window) {
    _ = window.show();
    _ = window.unminimize();
    _ = window.set_focus();
}

pub fn create_window(app_handle: &AppHandle) {
    if let Some(main) = app_handle.windows().get("main") {
        show_window(main);
    } else {
        let window_state = app_handle.state::<WindowState>();

        let builder = tauri::Window::builder(app_handle, "main", tauri::WindowUrl::App(PathBuf::from("/")));

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

pub fn save_window_state(app_handle: &AppHandle, window: &Window) {
    let debug_log = app_handle.state::<Settings>().debug_log();
    let scale_factor = window.scale_factor().expect("Error getting window scale factor");
    let window_state = app_handle.state::<WindowState>();

    if let Ok(size) = window.inner_size() {
        let size = ((size.width as f64) / scale_factor, (size.height as f64) / scale_factor);
        *window_state.size.lock().expect("win-state mutex error") = size;

        if debug_log {
            println!("saved window size: {}x{}", size.0, size.1);
        }
    }
    if let Ok(position) = window.outer_position() {
        let position = ((position.x as f64) / scale_factor, (position.y as f64) / scale_factor);
        *window_state.position.lock().expect("win-state mutex error") = position;

        if debug_log {
            println!("saved window position: {}x {}y", position.0, position.1);
        }
    }
}

pub fn ensure_settings_exist(settings_file: &PathBuf) -> bool {
    if !settings_file.is_file() {
        // get directory of settings file
        let Some(parent) = settings_file.parent() else { return false };
        // create the whole settings_file to the directory
        let Ok(_) = fs::create_dir_all(parent) else { return false };
        // create the settings file with the default settings json
        let Ok(_) = fs::write(settings_file, include_str!("../default-settings.json")) else { return false };
    }
    true
}
