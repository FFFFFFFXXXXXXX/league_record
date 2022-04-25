#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

extern crate libobs_recorder;

mod commands;
mod handlers;
mod helpers;
mod recorder;

use commands::*;
use handlers::*;
use tauri::{CustomMenuItem, SystemTray, SystemTrayMenu};

fn main() {
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("open", "Open"))
        .add_item(CustomMenuItem::new("quit", "Quit"));
    let system_tray = SystemTray::new().with_menu(tray_menu);
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            delete_video,
            get_recordings_size,
            get_recordings_list,
            get_recordings_folder,
            get_metadata
        ])
        .system_tray(system_tray)
        .on_system_tray_event(system_tray_event_handler)
        .register_uri_scheme_protocol("video", video_protocol_handler)
        .setup(setup_handler)
        .build(tauri::generate_context!())
        .expect("error while running tauri application");
    app.run(run_handler);
}
