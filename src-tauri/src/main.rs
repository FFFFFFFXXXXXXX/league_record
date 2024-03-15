// 'windows_subsystem = "windows/console"' decides if the executable should launch in a console window or not
// but only add this for release builds (debug_assertions disabled)
// gets ignored on all other targets
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use commands::*;
use handlers::*;
use state::*;

mod commands;
mod filewatcher;
mod game_data;
mod handlers;
mod helpers;
mod recorder;
mod state;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            helpers::create_window(app)
        }))
        .manage(WindowState::default())
        .manage(SettingsWrapper::default())
        .manage(CurrentlyRecording::default())
        .invoke_handler(tauri::generate_handler![
            show_app_window,
            get_marker_flags,
            set_marker_flags,
            get_recordings_path,
            get_recordings_size,
            get_recordings_list,
            open_recordings_folder,
            delete_video,
            rename_video,
            get_metadata,
            toggle_favorite
        ])
        .system_tray(create_system_tray())
        .on_system_tray_event(system_tray_event_handler)
        .setup(setup_handler)
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(run_handler);
}

#[test]
fn generate_command_bindings() {
    tauri_specta::ts::export_with_cfg(
        specta::collect_types![
            show_app_window,
            get_marker_flags,
            set_marker_flags,
            get_recordings_path,
            get_recordings_size,
            get_recordings_list,
            open_recordings_folder,
            delete_video,
            rename_video,
            get_metadata,
            toggle_favorite
        ]
        .unwrap(),
        specta::ts::ExportConfiguration::new()
            .bigint(specta::ts::BigIntExportBehavior::Number)
            .export_by_default(Some(false)),
        "../src/bindings.ts",
    )
    .unwrap();
}

#[test]
fn generate_type_bindings() {
    use specta::ts::{BigIntExportBehavior, ExportConfiguration};

    specta::export::ts_with_cfg(
        "../league_record_types/index.d.ts",
        &ExportConfiguration::new().bigint(BigIntExportBehavior::Number),
    )
    .unwrap();
}
