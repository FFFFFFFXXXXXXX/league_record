use std::{collections::HashMap, error::Error};

use libobs_recorder::Recorder;
use tauri::{
    api::{path::video_dir, process::Command},
    App, AppHandle, Manager, RunEvent, SystemTrayEvent, WindowEvent, Wry,
};

use crate::{helpers::show_window, recorder};

pub fn system_tray_event_handler(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "open" => show_window(app),
            "quit" => {
                app.trigger_global("shutdown", Some("".into()));
            }
            _ => {}
        },
        SystemTrayEvent::DoubleClick {
            position: _,
            size: _,
            ..
        } => show_window(app),
        _ => {}
    }
}

pub fn setup_handler(app: &mut App<Wry>) -> Result<(), Box<dyn Error>> {
    let app_handle = app.app_handle();

    // only start app if video directory exists
    if video_dir().is_none() {
        app_handle.exit(-1);
    }

    let (_, child) = Command::new("static-file-server")
        .envs(HashMap::from([
            ("PORT".into(), "1234".to_string()),
            (
                "FOLDER".into(),
                crate::helpers::get_recordings_folder()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        ]))
        .spawn()
        .unwrap();

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

pub fn run_handler(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::WindowEvent {
            label,
            event: WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            api.prevent_close();
            if let Some(window) = app.get_window(&label) {
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
