use std::{error::Error, path::PathBuf, thread, time::Duration};

use tauri::{
    api::{path::video_dir, shell::open},
    App, AppHandle, Manager, RunEvent, SystemTray, SystemTrayEvent, WindowEvent, Wry,
};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
};

use crate::{
    fileserver,
    helpers::{check_updates, create_tray_menu, create_window, save_window_state},
    recorder,
    state::Settings,
    AssetPort,
};

pub fn create_system_tray() -> SystemTray {
    SystemTray::new().with_menu(create_tray_menu())
}

pub fn system_tray_event_handler(app_handle: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "open" => create_window(app_handle),
            "quit" => {
                app_handle.trigger_global("shutdown", Some("".into()));
                // normally recorder should call app_handle.exit() after shutting down
                // if that doesn't happen within 3s force shutdown here
                thread::sleep(Duration::from_secs(3));
                app_handle.exit(0);
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
    let folder = settings
        .recordings_folder_as_string()
        .expect("invalid video folder");

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
