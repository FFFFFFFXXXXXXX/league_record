// 'windows_subsystem = "windows/console"' decides if the executable should launch in a console window or not
// but only add this for release builds (debug_assertions disabled)
// gets ignored on all other targets
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use commands::*;
use handlers::*;
use state::*;

mod commands;
mod fileserver;
mod filewatcher;
mod handlers;
mod helpers;
mod recorder;
mod state;

fn main() {
    // Only check if this is the only instance of LeagueRecord if the check succeeds (= true|false).
    // It is better to accidentally open two instances instead of none because something went wrong
    //
    // Don't drop single_instance around until the end of main()
    let single_instance = single_instance::SingleInstance::new("LEAGUE_RECORD_APPLICATION");
    if let Ok(single_instance) = single_instance.as_ref() {
        if !single_instance.is_single() {
            println!("There is already an instance of LeagueRecord running!");
            return;
        }
    } else {
        println!("Something went wrong when checking for other instances of LeagueRecord");
    }

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(WindowState::init())
        .manage(AssetPort::init())
        .manage(SettingsFile::default())
        .manage(SettingsWrapper::default())
        .manage(FileWatcher::default())
        .invoke_handler(tauri::generate_handler![
            show_app_window,
            get_marker_flags,
            set_marker_flags,
            get_asset_port,
            get_recordings_size,
            get_recordings_list,
            open_recordings_folder,
            delete_video,
            rename_video,
            get_metadata
        ])
        .system_tray(create_system_tray())
        .on_system_tray_event(system_tray_event_handler)
        .setup(setup_handler)
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(run_handler);
}

#[test]
fn generate_command_bindings() {
    tauri_specta::ts::export(
        specta::collect_types![
            show_app_window,
            get_marker_flags,
            set_marker_flags,
            get_asset_port,
            get_recordings_size,
            get_recordings_list,
            open_recordings_folder,
            delete_video,
            rename_video,
            get_metadata
        ],
        "../src/bindings.ts",
    )
    .unwrap();
}

#[test]
fn generate_type_bindings() {
    specta::export::ts("../src/settings.ts").unwrap();
}
