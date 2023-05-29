use std::{error::Error, process::Command, thread, time::Duration};

use tauri::{
    api::{
        path::{app_config_dir, video_dir},
        shell,
    },
    App, AppHandle, Manager, RunEvent, SystemTray, SystemTrayEvent, WindowEvent, Wry,
};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE};

use crate::{
    fileserver,
    helpers::{check_updates, create_tray_menu, create_window, ensure_settings_exist, save_window_state},
    recorder,
    state::{Settings, SettingsFile},
    AssetPort,
};

pub fn create_system_tray() -> SystemTray {
    SystemTray::new().with_menu(create_tray_menu())
}

pub fn system_tray_event_handler(app_handle: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::DoubleClick { .. } => {
            create_window(app_handle);
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "settings" => {
                // spawn a seperate thread to avoid blocking the main thread with .status()
                thread::spawn({
                    let app_handle = app_handle.clone();
                    move || {
                        let path = app_handle.state::<SettingsFile>().get();

                        if ensure_settings_exist(&path) {
                            let settings = app_handle.state::<Settings>();
                            let old_recordings_path = settings.get_recordings_path();
                            let old_marker_flags = settings.get_marker_flags();

                            // hardcode 'notepad' since league_record currently only works on windows anyways
                            Command::new("notepad")
                                .arg(&path)
                                .status()
                                .expect("failed to start text editor");
                            // update markerflags in UI
                            settings.load_from_file(&path);

                            let marker_flags = settings.get_marker_flags();
                            let recordings_path = settings.get_recordings_path();
                            let marker_flags_changed = marker_flags != old_marker_flags;
                            let recordings_path_changed = recordings_path != old_recordings_path;

                            if recordings_path_changed {
                                // send stop fileserver signal
                                app_handle.trigger_global("shutdown_fileserver", None);
                                // when fileserver stopped restart with new folder on the same port as before
                                app_handle.once_global("fileserver_shutdown", {
                                    let app_handle = app_handle.clone();
                                    move |_| {
                                        let port = app_handle.state::<AssetPort>().get();
                                        fileserver::start(app_handle, recordings_path, port);
                                    }
                                });
                            }

                            if marker_flags_changed || recordings_path_changed {
                                let _ = app_handle.emit_all("reload_ui", ());
                            }
                        }
                    }
                });
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
                let _ = shell::open(
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
    #[cfg(target_os = "windows")]
    unsafe {
        // Get correct window size from GetClientRect
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE)
    };

    let app_handle = app.app_handle();

    let settings = app_handle.state::<Settings>();

    // get path to config directory
    let mut settings_path = app_config_dir(app_handle.config().as_ref()).expect("Error getting app directory");
    settings_path.push("settings.json");
    // create settings.json file if missing
    ensure_settings_exist(&settings_path);
    // load settings and set state
    settings.load_from_file(&settings_path);
    app_handle.state::<SettingsFile>().set(settings_path);

    let debug_log = settings.debug_log();

    println!("debug_log: {}\n", if debug_log { "enabled" } else { "disabled" });

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
    let recordings_path = settings.get_recordings_path();

    if debug_log {
        println!("video folder: {:?}\n", recordings_path);
        println!("fileserver port: {}\n", port);
    }

    fileserver::start(app_handle.clone(), recordings_path, port);
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
