// 'windows_subsystem = "windows/console"' decides if the executable should launch in a console window or not
// but only add this for release builds (debug_assertions disabled)
// gets ignored on all other targets
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use commands::*;
use handlers::*;
use state::*;

mod commands;
mod fileserver;
mod handlers;
mod helpers;
mod recorder;
mod state;

fn main() {
    println!("LeagueRecord v{}", env!("CARGO_PKG_VERSION"));

    // Only check if this is the only instance of LeagueRecord if the check succeeds (= true|false).
    // It is better to accidentally open two instances instead of none because something went wrong
    if let Ok(single_instance) = single_instance::SingleInstance::new("LEAGUE_RECORD_APPLICATION") {
        if !single_instance.is_single() {
            println!("An instance of LeagueRecord is already open!");
            return;
        }

        // leak the SingleInstance so Drop doesn't get called which would destroy the underlying Mutex (on Windows)
        Box::leak(Box::new(single_instance));
    } else {
        println!("Something went wrong when checking for other instances of LeagueRecord");
    }

    let app = tauri::Builder::default()
        .manage(WindowState::init())
        .manage(AssetPort::init())
        .manage(SettingsFile::default())
        .manage(Settings::default())
        .invoke_handler(tauri::generate_handler![
            show_app_window,
            get_default_marker_flags,
            get_current_marker_flags,
            set_current_marker_flags,
            get_asset_port,
            get_recordings_size,
            get_recordings_list,
            open_recordings_folder,
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
