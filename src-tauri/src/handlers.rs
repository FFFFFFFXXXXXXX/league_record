use std::{
    cmp::min,
    error::Error,
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use libobs_recorder::Recorder;
use tauri::{
    http::{HttpRange, Request, Response, ResponseBuilder},
    App, AppHandle, Manager, RunEvent, SystemTrayEvent, WindowEvent, Wry,
};

use crate::helper::get_recordings_folder;

pub fn system_tray_event_handler(app: &AppHandle, event: SystemTrayEvent) {
    match event {
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
    }
}

pub fn video_protocol_handler(
    _: &AppHandle,
    request: &Request,
) -> core::result::Result<Response, Box<dyn Error>> {
    let mut response = ResponseBuilder::new();
    #[cfg(target_os = "windows")]
    let uri = urlencoding::decode(request.uri()).unwrap();
    let path_str = uri.replace("video://localhost/", "");
    #[cfg(not(target_os = "windows"))]
    let path = request.uri().replace("video://", "");

    if !path_str.ends_with(".mp4") {
        return response.mimetype("text/plain").status(404).body(Vec::new());
    }

    let mut path = get_recordings_folder();
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
}

pub fn setup_handler(_app: &mut App<Wry>) -> Result<(), Box<dyn Error>> {
    let libobs_data_path = Some(String::from("./libobs/data/libobs/"));
    let plugin_bin_path = Some(String::from("./obs-plugins/64bit/"));
    let plugin_data_path = Some(String::from("./libobs/data/obs-plugins/%module%/"));

    // Recorder::init(libobs_data_path, plugin_bin_path, plugin_data_path).unwrap();
    Recorder::init(libobs_data_path, plugin_bin_path, plugin_data_path).unwrap();
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
            let window = app.get_window(&label).unwrap();
            window.hide().unwrap();
            window.emit::<_>("close_pause", ()).unwrap();
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        _ => {}
    }
}
