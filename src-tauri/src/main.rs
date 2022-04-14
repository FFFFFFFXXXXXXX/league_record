#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

extern crate libobs_recorder;
use chrono::Local;
use libobs_recorder::{framerate::Framerate, rate_control::Cqp, resolution::Resolution, *};

use std::{
    cmp::min,
    env,
    fs::create_dir,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    thread,
    time::Duration,
};

use tauri::{
    api::path::video_dir,
    http::{HttpRange, ResponseBuilder},
    CustomMenuItem, Manager, RunEvent, SystemTray, SystemTrayEvent, SystemTrayMenu, WindowEvent,
};

// use the mutex to let only one recording be active at a time.
// the bool in the mutex is unused.
#[derive(Default)]
struct RecordState {
    recording: std::sync::Mutex<bool>,
}
#[tauri::command]
async fn record(state: tauri::State<'_, RecordState>) -> Result<(), String> {
    // get mutex and store it in _recording_lock which is only dropped when the function returns
    // _recording_lock gets dropped after recorder (dropped in reverse order of declaration)
    // => the lock is always released after the recorder has completely shutdown
    let _recording_lock = match state.recording.try_lock() {
        Ok(lock) => lock,
        _ => return Err("Already recording!".into()),
    };

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
        thread::sleep(Duration::from_secs(60));
        recorder.stop_recording();
    }

    Ok(())
}

fn main() {
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("open", "Open"))
        .add_item(CustomMenuItem::new("quit", "Quit"));
    let system_tray = SystemTray::new().with_menu(tray_menu);
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![record])
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "open" => {
                    let window = app.get_window("main").unwrap();
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
                "quit" => {
                    Recorder::shutdown();
                    app.exit(0);
                }
                _ => {}
            },
            SystemTrayEvent::DoubleClick {
                position: _,
                size: _,
                ..
            } => {
                let window = app.get_window("main").unwrap();
                window.show().unwrap();
                window.set_focus().unwrap();
            }
            _ => {}
        })
        .register_uri_scheme_protocol("stream", move |_app, request| {
            let mut response = ResponseBuilder::new();
            #[cfg(target_os = "windows")]
            let path_str = request.uri().replace("stream://localhost/", "");
            #[cfg(not(target_os = "windows"))]
            let path = request.uri().replace("stream://", "");

            if !path_str.ends_with(".mp4") {
                return response.mimetype("text/plain").status(404).body(Vec::new());
            }

            let mut path = PathBuf::from("../");
            path.push(path_str);

            let content = File::open(path);
            let mut content = match content {
                Ok(c) => c,
                Err(_) => return response.mimetype("text/plain").status(404).body(Vec::new()),
            };

            let mut buf = Vec::new();
            let mut status_code = 200;

            // if the webview sent a range header, we need to send a 206 in return
            // Actually only macOS and Windows are supported. Linux will ALWAYS return empty headers.
            if let Some(range) = request.headers().get("range") {
                let file_size = content.metadata().unwrap().len();
                let range = HttpRange::parse(range.to_str().unwrap(), file_size).unwrap();
                let first_range = range.first();
                if let Some(range) = first_range {
                    let mut real_length = range.length;

                    if range.length > file_size / 3 {
                        real_length = min(file_size - range.start, 1024 * 400);
                    }

                    let last_byte = range.start + real_length - 1;
                    status_code = 206;

                    // Only macOS and Windows are supported, if you set headers in linux they are ignored
                    response = response
                        .header("Connection", "Keep-Alive")
                        .header("Accept-Ranges", "bytes")
                        .header("Content-Length", real_length)
                        .header(
                            "Content-Range",
                            format!("bytes {}-{}/{}", range.start, last_byte, file_size),
                        );

                    content.seek(SeekFrom::Start(range.start))?;
                    content.take(real_length).read_to_end(&mut buf)?;
                } else {
                    content.read_to_end(&mut buf)?;
                }
            }

            response.mimetype("video/mp4").status(status_code).body(buf)
        })
        .manage(RecordState::default())
        .setup(|_app| {
            Recorder::init().unwrap();
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, e| match e {
        RunEvent::WindowEvent {
            label,
            event: WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            api.prevent_close();
            app_handle.get_window(&label).unwrap().hide().unwrap();
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        _ => {}
    });
}

fn get_new_filepath() -> String {
    let filename = format!("{}", Local::now().format("%Y-%m-%d_%H-%M-%S.mp4"));
    let mut vid_dir = video_dir().unwrap();
    vid_dir.push(PathBuf::from("league_recordings"));
    if !vid_dir.exists() {
        let _ = create_dir(vid_dir.as_path());
    }
    vid_dir.push(PathBuf::from(filename));
    vid_dir.to_str().unwrap().into()
}
