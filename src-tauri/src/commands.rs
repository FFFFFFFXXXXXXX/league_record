use std::{
    cmp::Ordering,
    fs::{metadata, remove_file, File},
    io::BufReader,
    path::PathBuf,
    sync::mpsc::channel,
    time::Duration,
};

use crate::helpers::{
    compare_time, create_client, get_recordings, get_recordings_folder as get_rec_folder,
};
use chrono::Local;
use libobs_recorder::{
    framerate::Framerate,
    rate_control::{Cqp, Icq},
    resolution::Resolution,
    window::Window,
    Recorder, RecorderSettings,
};
use reqwest::{header::ACCEPT, StatusCode};
use serde_json::{json, Value};
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
    // remove video
    let mut path = get_rec_folder();
    path.push(PathBuf::from(&video));
    let ok1 = match remove_file(path) {
        Ok(_) => true,
        Err(_) => false,
    };
    // remove json file
    let mut path = get_rec_folder();
    let mut json = video.clone();
    json.replace_range(json.len() - 4.., ".json");
    path.push(PathBuf::from(json));
    let ok2 = match remove_file(path) {
        Ok(_) => true,
        Err(_) => false,
    };
    return ok1 && ok2;
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

#[tauri::command]
pub async fn save_metadata(mut filename: String, mut json: Value) -> Result<(), String> {
    let player_name = json["playerName"].clone();
    let events = json.get_mut("events").unwrap();

    let new_events: Vec<Value> = events
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| match event["EventName"].as_str().unwrap() {
            "DragonKill" => Some(json!({
                "eventName": "Dragon",
                "eventTime": event["EventTime"]
            })),
            "HeraldKill" => Some(json!({
                "eventName": "Herald",
                "eventTime": event["EventTime"]
            })),
            "BaronKill" => Some(json!({
                "eventName": "Baron",
                "eventTime": event["EventTime"]
            })),
            "ChampionKill" => {
                let assisters = event["Assisters"].as_array().unwrap();
                if event["VictimName"] == player_name {
                    Some(json!({
                        "eventName": "Death",
                        "eventTime": event["EventTime"]
                    }))
                } else if event["KillerName"] == player_name || assisters.contains(&player_name) {
                    Some(json!({
                        "eventName": "Kill",
                        "eventTime": event["EventTime"]
                    }))
                } else {
                    None
                }
            }
            "TurretKilled" => Some(json!({
                "eventName": "Turret",
                "eventTime": event["EventTime"]
            })),
            "InhibKilled" => Some(json!({
                "eventName": "Inhibitor",
                "eventTime": event["EventTime"]
            })),
            _ => None,
        })
        .collect();

    println!("old events: {:?}\nnew events: {:?}", events, new_events);
    // replace old events with new events
    *events = Value::Array(new_events);

    filename.replace_range(filename.len() - 4.., ".json");
    if let Ok(file) = File::create(filename) {
        let _ = serde_json::to_writer(file, &json);
        Ok(())
    } else {
        Err("Could not create json file!".into())
    }
}

#[tauri::command]
pub async fn get_metadata(video: String) -> Option<Value> {
    let mut filename = video.clone();
    filename.replace_range(filename.len() - 4.., ".json");
    let mut path = get_rec_folder();
    path.push(PathBuf::from(filename));
    let reader = if let Ok(file) = File::open(path) {
        BufReader::new(file)
    } else {
        return None;
    };

    if let Ok(json) = serde_json::from_reader::<BufReader<File>, Value>(reader) {
        Some(json)
    } else {
        None
    }
}

#[tauri::command]
pub async fn get_league_data() -> Option<Value> {
    let client = create_client();

    let result = client
        .get("https://127.0.0.1:2999/liveclientdata/allgamedata")
        .header(ACCEPT, "application/json")
        .timeout(Duration::from_secs(1))
        .send()
        .await;
    let data = if let Ok(result) = result {
        if result.status() == StatusCode::OK {
            result.json::<Value>().await.unwrap()
        } else {
            return None;
        }
    } else {
        return None;
    };
    let mut player_info: Option<&Value> = None;
    for player in data["allPlayers"].as_array().unwrap() {
        if player["summonerName"] == data["activePlayer"]["summonerName"] {
            player_info = Some(player);
            break;
        }
    }
    if let Some(info) = player_info {
        Some(json!({
            "playerName": data["activePlayer"]["summonerName"],
            "championName": info["championName"],
            "stats": info["scores"],
            "events": data["events"]["Events"],
            "gameMode": data["gameData"]["gameMode"]
        }))
    } else {
        return None;
    }
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
) -> Result<String, String> {
    // get mutex and store it in _recording_lock which is only dropped when the function returns
    // _recording_lock gets dropped after recorder (dropped in reverse order of declaration)
    // => the lock is always released after the recorder has completely shutdown
    let _recording_lock = match state.recording.try_lock() {
        Ok(lock) => lock,
        _ => return Err("Already recording!".into()),
    };

    let (sender, receiver) = channel::<_>();
    window.once("stop_record", move |_| {
        let _ = sender.send(());
    });

    let filename = format!("{}", Local::now().format("%Y-%m-%d_%H:%M.mp4"));
    let mut vid_dir = get_rec_folder();
    vid_dir.push(PathBuf::from(&filename));

    let mut settings = RecorderSettings::new();
    settings.set_window(Window::new(
        "League of Legends (TM) Client",
        Some("RiotWindowClass".into()),
        Some("League of Legends.exe".into()),
    ));
    settings.set_output_resolution(Resolution::_1080p);
    settings.set_framerate(Framerate::new(30, 1));
    settings.set_cqp(Cqp::new(18)); // for amd/nvidia/software
    settings.set_icq(Icq::new(18)); // for intel quicksync
    settings.record_audio(true);
    settings.set_output_path(vid_dir.to_str().unwrap());

    if let Ok(mut recorder) = Recorder::get(settings) {
        if recorder.start_recording() {
            let _ = receiver.recv_timeout(Duration::from_secs(5000)); // ~83.3 minutes
            recorder.stop_recording();
        }
    }
    Ok(filename)
}
