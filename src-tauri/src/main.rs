#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod fileserver;
mod handlers;
mod helpers;
mod recorder;
mod state;

use commands::*;
use handlers::*;
use state::*;
use tauri::{generate_handler, Builder};

fn main() {
    let app = Builder::default()
        .manage(WindowState::init())
        .manage(MarkerFlagsState::init())
        .manage(AssetPort::init())
        .manage(Settings::init())
        .invoke_handler(generate_handler![
            show_app_window,
            get_default_marker_flags,
            get_current_marker_flags,
            set_current_marker_flags,
            get_asset_port,
            get_recordings_size,
            get_recordings_list,
            get_recordings_folder,
            delete_video,
            get_metadata
        ])
        .system_tray(create_system_tray())
        .on_system_tray_event(system_tray_event_handler)
        .setup(setup_handler)
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(run_handler);
}
