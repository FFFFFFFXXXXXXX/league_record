use std::{error::Error, fs, path::PathBuf, thread, time::Duration};

use tauri::{
    api::{
        path::{app_config_dir, video_dir},
        shell::{self, open},
    },
    App, AppHandle, Manager, RunEvent, SystemTray, SystemTrayEvent, WindowEvent, Wry,
};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
};

use crate::{
    fileserver,
    helpers::{self, check_updates, create_tray_menu, create_window, save_window_state},
    recorder,
    state::{Settings, SettingsFile},
    AssetPort,
};

pub fn create_system_tray() -> SystemTray {
    SystemTray::new().with_menu(create_tray_menu())
}

pub fn system_tray_event_handler(app_handle: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "settings" => {
                let path = app_handle.state::<SettingsFile>().get();
                if path.is_file() {
                    let _ = shell::open(
                        &app_handle.shell_scope(),
                        helpers::path_to_string(&path),
                        None,
                    );
                } else {
                    if let Some(parent) = path.parent() {
                        if fs::create_dir_all(parent).is_ok() {
                            if fs::write(&path, include_str!("../default-settings.json")).is_ok() {
                                let _ = shell::open(
                                    &app_handle.shell_scope(),
                                    helpers::path_to_string(&path),
                                    None,
                                );
                            }
                        }
                    }
                }
            }
            "open" => create_window(app_handle),
            "quit" => {
                if let Some(main) = app_handle.windows().get("main") {
                    let _ = main.close();
                }
                app_handle.trigger_global("shutdown_fileserver", None);
                app_handle.trigger_global("shutdown_recorder", None);

                // normally recorder should call app_handle.exit() after shutting down
                // if that doesn't happen within 3s force shutdown here
                std::thread::spawn({
                    let app_handle = app_handle.clone();
                    move || {
                        thread::sleep(Duration::from_secs(3));
                        app_handle.exit(0);
                    }
                });
            }
            "update" => {
                let _ = open(
                    &app_handle.shell_scope(),
                    "https://github.com/FFFFFFFXXXXXXX/league_record/releases/latest",
                    None,
                );
            }
            _ => {}
        },
        SystemTrayEvent::DoubleClick {
            position: _,
            size: _,
            ..
        } => create_window(app_handle),
        _ => {}
    }
}

pub fn setup_handler(app: &mut App<Wry>) -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "windows")]
    unsafe {
        // Get correct window size from GetClientRect
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE)
    };

    let app_handle = app.app_handle();

    let settings = app_handle.state::<Settings>();
    // Load settings from settings.json file if it exists and save settings folder
    let mut settings_path =
        app_config_dir(app_handle.config().as_ref()).expect("Error getting app directory");
    settings_path.push("settings.json");
    settings.load_settings_file(&settings_path);
    app_handle.state::<SettingsFile>().set(settings_path);

    let debug_log = settings.debug_log();

    println!(
        "debug_log: {}\n",
        if debug_log { "enabled" } else { "disabled" }
    );

    if debug_log {
        println!("Settings: {:?}\n", settings);
    }

    if settings.check_for_updates() {
        check_updates(&app_handle, debug_log);
    }

    // only start app if video directory exists
    if video_dir().is_none() {
        if debug_log {
            println!("Error: No video folder available");
        }
        app_handle.exit(-1);
    }

    // don't show window on startup and set initial window state
    if let Some(window) = app_handle.get_window("main") {
        save_window_state(&app_handle, &window);
        let _ = window.close();
    }

    // launch static-file-server as a replacement for the broken asset protocol
    let port = app_handle.state::<AssetPort>().get();
    let folder = helpers::path_to_string(&settings.recordings_folder());

    if debug_log {
        println!("fileserver port: {}\n", port);
        println!("video folder: {}\n", folder);
    }

    fileserver::start(app_handle.clone(), PathBuf::from(folder), port);
    recorder::start(app_handle);
    Ok(())
}

pub fn run_handler(app_handle: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::WindowEvent {
            label: _,
            event: WindowEvent::CloseRequested { api: _, .. },
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
