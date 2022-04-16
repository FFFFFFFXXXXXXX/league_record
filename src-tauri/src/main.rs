#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod handlers;
mod helper;

extern crate libobs_recorder;

use std::{sync::mpsc::channel, time::Duration};

use handlers::*;
use helper::{get_new_filepath, get_recordings};
use libobs_recorder::{
    framerate::Framerate, rate_control::Cqp, resolution::Resolution, Recorder, RecorderSettings,
};
use tauri::{CustomMenuItem, Runtime, SystemTray, SystemTrayMenu};

#[tauri::command]
async fn get_recordings_list() -> Vec<String> {
    let recordings = get_recordings();
    let mut ret = Vec::<String>::new();
    for path in recordings {
        if let Some(os_str_ref) = path.file_name() {
            if let Ok(filename) = os_str_ref.to_os_string().into_string() {
                println!("{}", filename);
                ret.push(filename);
            }
        }
    }
    return ret;
}

// use the mutex to let only one recording be active at a time.
// the bool in the mutex is unused.
#[derive(Default)]
struct RecordState {
    recording: std::sync::Mutex<bool>,
}
#[tauri::command]
async fn record<R: Runtime>(
    state: tauri::State<'_, RecordState>,
    window: tauri::Window<R>,
) -> Result<(), String> {
    // get mutex and store it in _recording_lock which is only dropped when the function returns
    // _recording_lock gets dropped after recorder (dropped in reverse order of declaration)
    // => the lock is always released after the recorder has completely shutdown
    let _recording_lock = match state.recording.try_lock() {
        Ok(lock) => lock,
        _ => return Err("Already recording!".into()),
    };

    let (sender, receiver) = channel::<_>();
    window.once("stop_record", move |_| sender.send(()).unwrap());

    let mut settings = RecorderSettings::new();
    settings
        .set_window_title("League of Legends (TM) Client:RiotWindowClass:League of Legends.exe");
    settings.set_input_resolution(Resolution::_1440p);
    settings.set_output_resolution(Resolution::_1080p);
    settings.set_framerate(Framerate::new(30));
    settings.set_cqp(Cqp::new(16));
    settings.record_audio(true);
    settings.set_output_path(get_new_filepath());

    let mut recorder = Recorder::get(settings);
    if recorder.start_recording() {
        let _ = receiver.recv_timeout(Duration::from_secs(5000));
        recorder.stop_recording();
    }

    let _ = window.emit("new_recording", ());
    Ok(())
}

fn main() {
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("open", "Open"))
        .add_item(CustomMenuItem::new("quit", "Quit"));
    let system_tray = SystemTray::new().with_menu(tray_menu);
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_recordings_list, record])
        .system_tray(system_tray)
        .on_system_tray_event(system_tray_event_handler)
        .register_uri_scheme_protocol("video", video_protocol_handler)
        .manage(RecordState::default())
        .setup(setup_handler)
        .build(tauri::generate_context!())
        .expect("error while running tauri application");
    app.run(run_handler);
}
