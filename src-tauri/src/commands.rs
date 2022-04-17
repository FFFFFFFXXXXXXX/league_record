use std::{fs::File, io::Read, path::PathBuf, sync::mpsc::channel, time::Duration};

use crate::helpers::{get_new_filepath, get_recordings, get_recordings_folder as get_rec_folder};
use libobs_recorder::{
    framerate::Framerate, rate_control::Cqp, resolution::Resolution, Recorder, RecorderSettings,
};
use reqwest::header::ACCEPT;
use tauri::Runtime;

#[tauri::command]
pub async fn get_recordings_folder() -> String {
    let folder: PathBuf = get_rec_folder();
    if let Ok(string) = folder.into_os_string().into_string() {
        string
    } else {
        String::new()
    }
}

#[tauri::command]
pub async fn get_recordings_list() -> Vec<String> {
    let recordings = get_recordings();
    let mut ret = Vec::<String>::new();
    for path in recordings {
        if let Some(os_str_ref) = path.file_name() {
            if let Ok(filename) = os_str_ref.to_os_string().into_string() {
                ret.push(filename);
            }
        }
    }
    return ret;
}

#[tauri::command]
pub async fn get_league_events() -> Vec<String> {
    let mut buf = Vec::new();
    File::open("riotgames.pem")
        .unwrap()
        .read_to_end(&mut buf)
        .unwrap();
    let cert = reqwest::Certificate::from_pem(&buf).unwrap();
    let client = reqwest::Client::builder()
        .add_root_certificate(cert)
        .build()
        .unwrap();
    let result = client
        .get("https://127.0.0.1:2999/liveclientdata/eventdata")
        .header(ACCEPT, "application/json")
        .send()
        .await
        .unwrap();
    println!("{}: {}", result.status(), result.text().await.unwrap());
    vec!["".into()]
}

// use the mutex to let only one recording be active at a time.
// the bool in the mutex is unused.
#[derive(Default)]
pub struct RecordState {
    recording: std::sync::Mutex<bool>,
}
#[tauri::command]
pub async fn record<R: Runtime>(
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

    let _ = window.emit("recordings_changed", ());
    Ok(())
}
