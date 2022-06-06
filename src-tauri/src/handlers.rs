use std::{collections::HashMap, error::Error, thread, time::Duration};

use tauri::{
    api::{path::video_dir, process::Command},
    App, AppHandle, CustomMenuItem, Manager, RunEvent, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, WindowEvent, Wry,
};

use crate::{
    helpers::{create_window, set_window_state},
    recorder,
    state::Settings,
    AssetPort,
};

pub fn create_system_tray() -> SystemTray {
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("rec", "Recording").disabled())
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("open", "Open"))
        .add_item(CustomMenuItem::new("quit", "Quit"));
    SystemTray::new().with_menu(tray_menu)
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
    let app_handle = app.app_handle();

    // only start app if video directory exists
    if video_dir().is_none() {
        app_handle.exit(-1);
    }

    // don't show window on startup and set initial window state
    if let Some(window) = app_handle.get_window("main") {
        set_window_state(&app_handle, &window);
        let _ = window.close();
    }

    // launch static-file-server as a replacement for the broken asset protocol
    let port = app_handle.state::<AssetPort>().get();
    let folder = app_handle.state::<Settings>().recordings_folder_as_string();
    let (_, sfs) = Command::new_sidecar("static-file-server")
        .expect("missing static-file-server")
        .envs(HashMap::from([
            ("PORT".into(), port.to_string()),
            ("FOLDER".into(), folder.expect("invalid sfs folder")),
        ]))
        .spawn()
        .expect("error spawing static-file-server");

    std::thread::spawn(|| recorder::start_polling(app_handle, sfs));
    Ok(())
}

pub fn run_handler(app_handle: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::WindowEvent {
            label: _,
            event: WindowEvent::CloseRequested { api: _, .. },
            ..
        } => {
            if let Some(window) = app_handle.get_window("main") {
                set_window_state(app_handle, &window);
            }
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        _ => {}
    }
}
