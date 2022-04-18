use std::{
    cmp::Ordering,
    fs::{metadata, remove_file},
    path::PathBuf,
    sync::mpsc::channel,
    time::Duration,
};

use crate::helpers::{
    compare_time, create_client, get_new_filepath, get_recordings,
    get_recordings_folder as get_rec_folder,
};
use libobs_recorder::{
    framerate::Framerate, rate_control::Cqp, resolution::Resolution, Recorder, RecorderSettings,
};
use reqwest::header::ACCEPT;
use serde::Deserialize;
use tauri::Runtime;

#[tauri::command]
pub async fn get_recordings_size() -> f64 {
    let mut size = 0;
    for file in get_recordings() {
        size += metadata(file).unwrap().len();
    }
    size as f64 / 1_000_000_000.0 // in Gigabyte
}

#[tauri::command]
pub async fn delete_video(video: String) -> bool {
    let mut path = get_rec_folder();
    path.push(PathBuf::from(video));
    match remove_file(path) {
        Ok(_) => true,
        Err(_) => false,
    }
}

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
    let mut recordings = get_recordings();
    // sort by time created (index 0 is newest)
    recordings.sort_by(|a, b| {
        if let Ok(result) = compare_time(a, b) {
            result
        } else {
            Ordering::Equal
        }
    });
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

#[derive(Deserialize, Debug)]
struct Events {
    #[serde(rename = "Events")]
    events: Vec<Event>,
}

#[derive(Deserialize, Debug)]
struct Event {
    #[serde(rename = "EventID")]
    id: u32,
    #[serde(rename = "EventName")]
    name: String,
    #[serde(rename = "EventTime")]
    time: f32,
}

#[tauri::command]
pub async fn get_league_events() -> Vec<String> {
    let client = create_client();

    // let result = client
    //     .get("https://127.0.0.1:2999/liveclientdata/activeplayername")
    //     .header(ACCEPT, "application/json")
    //     .timeout(Duration::from_secs(1))
    //     .send()
    //     .await;
    // let player_name = if let Ok(result) = result {
    //     result.text()
    // } else {
    //     return Vec::new();
    // };

    let result = client
        .get("https://127.0.0.1:2999/liveclientdata/eventdata")
        .header(ACCEPT, "application/json")
        .timeout(Duration::from_secs(1))
        .send()
        .await;
    let events = if let Ok(result) = result {
        // let text = result.text().await.unwrap();
        // println!("before parse: {}", text);
        result.json::<Events>().await
    } else {
        return Vec::new();
    };
    println!("after parse");
    let events = if let Ok(e) = events {
        e
    } else {
        return Vec::new();
    };

    let mut vec = Vec::<String>::new();
    for event in events.events {
        println!("{:?}", event);
        vec.push(event.name);
    }
    return vec;
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
