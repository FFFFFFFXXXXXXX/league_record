use std::error::Error;

use tauri::api::path::{app_config_dir, video_dir};
use tauri::api::shell;
use tauri::{App, AppHandle, Manager, RunEvent, SystemTray, SystemTrayEvent, WindowEvent, Wry};

use crate::helpers::*;
use crate::state::{SettingsFile, SettingsWrapper};
use crate::{filewatcher, recorder::LeagueRecorder};

pub fn create_system_tray() -> SystemTray {
    SystemTray::new()
        .with_tooltip("LeagueRecord")
        .with_menu(create_tray_menu())
}

pub fn system_tray_event_handler(app_handle: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::DoubleClick { .. } => create_window(app_handle),
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "settings" => let_user_edit_settings(app_handle),
            "open" => create_window(app_handle),
            "quit" => {
                app_handle.windows().into_values().for_each(|window| _ = window.close());
                app_handle.state::<LeagueRecorder>().stop();
                app_handle.exit(0);
            }
            "update" => {
                _ = shell::open(
                    &app_handle.shell_scope(),
                    "https://github.com/FFFFFFFXXXXXXX/league_record/releases/latest",
                    None,
                );
            }
            _ => {}
        },
        _ => {}
    }
}

pub fn setup_handler(app: &mut App<Wry>) -> Result<(), Box<dyn Error>> {
    let app_handle = app.app_handle();

    // get path to config directory
    let config_path = app_config_dir(&app_handle.config()).expect("Error getting app directory");

    let settings_path = config_path.join("settings.json");
    let settings = app_handle.state::<SettingsWrapper>();
    // create settings.json file if missing
    ensure_settings_exist(&settings_path);
    // load settings and set state
    settings.load_from_file(&settings_path);
    app_handle.manage::<SettingsFile>(SettingsFile::new(settings_path));

    let debug_log = settings.debug_log();

    if debug_log {
        add_log_plugin(&app_handle)?;
    }

    log::info!("LeagueRecord v{}", env!("CARGO_PKG_VERSION"));
    log::info!("{}", chrono::Local::now().format("%d-%m-%Y %H:%M"));
    log::info!("debug_log: {}", if debug_log { "enabled" } else { "disabled" });

    if settings.check_for_updates() {
        check_updates(&app_handle);
    }

    log::info!("Settings: {:?}", settings.inner());

    sync_autostart(&app_handle);

    // only start app if video directory exists
    if video_dir().is_none() {
        log::error!("Error: No video folder available");
        app_handle.exit(-1);
    }

    // don't show window on startup and set initial window state
    if let Some(window) = app_handle.get_window("main") {
        save_window_state(&app_handle, &window);
        _ = window.close();
    }

    // start watching recordings folder for changes
    let recordings_path = settings.get_recordings_path();
    log::info!("video folder: {:?}", recordings_path);
    filewatcher::replace(&app_handle, &recordings_path);

    // start checking for LoL games to record
    app_handle.manage(LeagueRecorder::new(app_handle.clone()));

    // cleanup recordings if they are too old or the total size of the recordings gets too big
    // this only happens if 'maxRecordingAge' or 'maxRecordingsSize' is configured in the settings
    tauri::async_runtime::spawn_blocking(move || cleanup_recordings(&app_handle));

    Ok(())
}

pub fn run_handler(app_handle: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::WindowEvent {
            event: WindowEvent::CloseRequested { .. },
            ..
        } => {
            // triggered on window close (X Button)
            if let Some(window) = app_handle.get_window("main") {
                save_window_state(app_handle, &window);
            }
        }
        RunEvent::ExitRequested { api, .. } => {
            // triggered when no windows remain
            // prevent complete shutdown of program so that just the tray icon stays
            api.prevent_exit();
        }
        _ => {}
    }
}
