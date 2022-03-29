#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

extern crate wgc_recorder;
use std::{thread, time::Duration};
use wgc_recorder::{
    bitrate::Bitrate, framerate::Framerate, resolution::Resolution, Recorder, RecorderSettings,
};

use std::{
    cmp::min,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use tauri::{
    http::{HttpRange, ResponseBuilder},
    CustomMenuItem, Manager, RunEvent, Runtime, SystemTray, SystemTrayEvent, SystemTrayMenu,
};

#[tauri::command]
async fn record<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
) -> Result<(), String> {
    let settings = RecorderSettings {
        window_title: String::from("Mozilla Firefox"),
        output_resolution: Resolution::_1080p,
        framerate: Framerate::new(30),
        bitrate: Bitrate::mbit(8),
        capture_cursor: true,
    };
    match Recorder::new(settings) {
        Ok(mut recorder) => {
            let duration = Duration::from_secs(10);
            match recorder.start(Some(duration)) {
                Ok(_) => (),
                Err(e) => {
                    println!("{}", e);
                }
            }
        }
        Err(e) => println!("{}", e),
    };
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

            let content = std::fs::File::open(path);
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
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, e| match e {
        RunEvent::CloseRequested { label, api, .. } => {
            let app_handle = app_handle.clone();
            api.prevent_close();
            app_handle.get_window(&label).unwrap().hide().unwrap();
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        _ => {}
    });
}
