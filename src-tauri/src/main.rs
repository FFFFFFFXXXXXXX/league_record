// 'windows_subsystem = "windows/console"' decides if the executable should launch in a console window or not
// but only add this for release builds (debug_assertions disabled)
// gets ignored on all other targets
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod commands;
mod constants;
mod filewatcher;
mod generate_bindings;
mod recorder;
mod state;
mod util;

fn main() {
    use app::{AppManager, AppWindow, WindowManager};
    use state::{CurrentlyRecording, Shutdown, TrayState, WindowState};
    use tauri::Manager;

    #[cfg(feature = "tokio-console")]
    console_subscriber::init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            app.open_window(AppWindow::Main)
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(WindowState::default())
        .manage(CurrentlyRecording::default())
        .manage(TrayState::default())
        .manage(Shutdown::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_marker_flags,
            commands::set_marker_flags,
            commands::get_recordings_path,
            commands::get_recordings_size,
            commands::get_recordings_list,
            commands::open_recordings_folder,
            commands::delete_video,
            commands::rename_video,
            commands::get_metadata,
            commands::toggle_favorite,
            commands::confirm_delete,
            commands::disable_confirm_delete
        ])
        .setup(|app| app.app_handle().setup().map_err(anyhow::Error::into))
        .build(tauri::generate_context!());

    match app {
        Ok(app) => app.run(app::process_app_event),
        Err(e) => {
            println!("error starting LeagueRecord: {e:?}");
            log::error!("error starting LeagueRecord: {e:?}");
        }
    }
}
