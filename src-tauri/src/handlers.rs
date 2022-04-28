use std::{collections::HashMap, error::Error};

use libobs_recorder::Recorder;
use tauri::{
    api::{path::video_dir, process::Command},
    App, AppHandle, CustomMenuItem, Manager, RunEvent, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, WindowEvent, Wry,
};

use crate::{helpers::show_window, recorder, state::RecordingsFolder, AssetPort};

pub fn create_system_tray() -> SystemTray {
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("rec", "Recording").disabled())
        .add_native_item(SystemTrayMenuItem::from(SystemTrayMenuItem::Separator))
        .add_item(CustomMenuItem::new("open", "Open"))
        .add_item(CustomMenuItem::new("quit", "Quit"));
    SystemTray::new().with_menu(tray_menu)
}

pub fn system_tray_event_handler(app_handle: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "open" => show_window(app_handle),
            "quit" => {
                app_handle.trigger_global("shutdown", Some("".into()));
            }
            _ => {}
        },
        SystemTrayEvent::DoubleClick {
            position: _,
            size: _,
            ..
        } => show_window(app_handle),
        _ => {}
    }
}

pub fn setup_handler(app: &mut App<Wry>) -> Result<(), Box<dyn Error>> {
    let app_handle = app.app_handle();

    // only start app if video directory exists
    if video_dir().is_none() {
        app_handle.exit(-1);
    }

    let port = app_handle.state::<AssetPort>().get();
    let folder = app_handle.state::<RecordingsFolder>().get_as_string();
    let (_, child) = Command::new("static-file-server")
        .envs(HashMap::from([
            ("PORT".into(), port.to_string()),
            ("FOLDER".into(), folder.clone().unwrap()),
        ]))
        .spawn()
        .unwrap();

    println!(
        "PID: {}, PORT: {}, FOLDER: {}",
        child.pid(),
        port,
        folder.unwrap()
    );
    app_handle.once_global("shutdown", move |_| {
        let _ = child.kill();
    });

    std::thread::spawn(move || {
        // let libobs_data_path = Some(String::from("./data/libobs/"));
        // let plugin_bin_path = Some(String::from("./obs-plugins/64bit/"));
        // let plugin_data_path = Some(String::from("./data/obs-plugins/%module%/"));

        let libobs_data_path = Some(String::from("./libobs/data/libobs/"));
        let plugin_bin_path = Some(String::from("./libobs/obs-plugins/64bit/"));
        let plugin_data_path = Some(String::from("./libobs/data/obs-plugins/%module%/"));

        if Recorder::init(libobs_data_path, plugin_bin_path, plugin_data_path).is_ok() {
            recorder::start_polling(app_handle);
        }
    });

    Ok(())
}

pub fn run_handler(app_handle: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::WindowEvent {
            label,
            event: WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            api.prevent_close();
            if let Some(window) = app_handle.get_window(&label) {
                let _ = window.hide();
                let _ = window.emit::<_>("close_pause", ());
            }
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        _ => {}
    }
}
