use std::{error::Error, process::Command, thread, time::Duration};

use tauri::{
    api::{
        path::{app_config_dir, video_dir},
        shell,
    },
    async_runtime, App, AppHandle, Manager, RunEvent, SystemTray, SystemTrayEvent, WindowEvent, Wry,
};

use crate::filewatcher;
use crate::helpers::*;
use crate::recorder;
use crate::state::{SettingsFile, SettingsWrapper};

pub fn create_system_tray() -> SystemTray {
    SystemTray::new()
        .with_tooltip("LeagueRecord")
        .with_menu(create_tray_menu())
}

pub fn system_tray_event_handler(app_handle: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::DoubleClick { .. } => {
            create_window(app_handle);
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "settings" => {
                // spawn a separate thread to avoid blocking the main thread with .status()
                thread::spawn({
                    let app_handle = app_handle.clone();
                    move || {
                        let settings_file = app_handle.state::<SettingsFile>();
                        let settings_file = settings_file.get();

                        if ensure_settings_exist(settings_file) {
                            let settings = app_handle.state::<SettingsWrapper>();
                            let old_recordings_path = settings.get_recordings_path();
                            let old_log = settings.debug_log();

                            // hardcode 'notepad' since league_record currently only works on windows anyways
                            Command::new("notepad")
                                .arg(settings_file)
                                .status()
                                .expect("failed to start text editor");

                            // reload settings from settings.json
                            settings.load_from_file(settings_file);
                            log::info!("Settings updated: {:?}", settings.inner());

                            // check and update autostart if necessary
                            sync_autostart(&app_handle);

                            // add / remove logs plugin if needed
                            if old_log != settings.debug_log() {
                                if settings.debug_log() {
                                    if add_log_plugin(&app_handle).is_err() {
                                        // retry
                                        remove_log_plugin(&app_handle);
                                        _ = add_log_plugin(&app_handle);
                                    }
                                } else {
                                    remove_log_plugin(&app_handle);
                                }
                            }

                            // check if UI window needs to be updated
                            let recordings_path = settings.get_recordings_path();
                            if recordings_path != old_recordings_path {
                                filewatcher::replace(&app_handle, &recordings_path);
                                _ = app_handle.emit_all("recordings_changed", ());
                            }
                        }
                    }
                });
            }
            "open" => create_window(app_handle),
            "quit" => {
                // close UI window
                if let Some(main) = app_handle.windows().get("main") {
                    _ = main.close();
                }

                // always exit after 3s
                tauri::async_runtime::spawn({
                    let app_handle = app_handle.clone();
                    async move {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        app_handle.exit(1);
                    }
                });

                // stop recorder, recorder calls app_handle.exit(0) after shutting down
                app_handle.trigger_global("shutdown_recorder", None);
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

    let recordings_path = settings.get_recordings_path();
    log::info!("video folder: {:?}", recordings_path);

    filewatcher::replace(&app_handle, &recordings_path);
    async_runtime::spawn(recorder::start_recorder(app_handle));
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
