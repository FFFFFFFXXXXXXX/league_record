use std::{
    fs::File,
    path::PathBuf,
    sync::mpsc::{channel, TryRecvError},
    time::Duration,
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
use tauri::{Manager, Runtime};

use crate::helpers::{create_client, get_recordings_folder};

const SLEEP_SECS: u64 = 5;

fn save_metadata(filename: String, mut json: Value) {
    let player_name = json["playerName"].clone();
    let events = if let Some(e) = json.get_mut("events") {
        e
    } else {
        return;
    };
    let events_array = if let Some(arr) = events.as_array() {
        arr
    } else {
        return;
    };

    let new_events: Vec<Value> = events_array
        .iter()
        .filter_map(|event| {
            if let Some(event_name) = event["EventName"].as_str() {
                match event_name {
                    "DragonKill" => {
                        let mut dragon = String::from(event["DragonType"].as_str().unwrap());
                        dragon.push_str(" Dragon");
                        Some(json!({
                            "eventName": dragon,
                            "eventTime": event["EventTime"]
                        }))
                    }
                    "HeraldKill" => Some(json!({
                        "eventName": "Herald",
                        "eventTime": event["EventTime"]
                    })),
                    "BaronKill" => Some(json!({
                        "eventName": "Baron",
                        "eventTime": event["EventTime"]
                    })),
                    "ChampionKill" => {
                        let assisters = if let Some(arr) = event["Assisters"].as_array() {
                            arr
                        } else {
                            return None;
                        };
                        if event["VictimName"] == player_name {
                            Some(json!({
                                "eventName": "Death",
                                "eventTime": event["EventTime"]
                            }))
                        } else if event["KillerName"] == player_name
                            || assisters.contains(&player_name)
                        {
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
                }
            } else {
                None
            }
        })
        .collect();

    // replace old events with new events
    *events = Value::Array(new_events);

    let mut filepath = get_recordings_folder();
    filepath.push(PathBuf::from(filename));
    filepath.set_extension("json");
    if let Ok(file) = File::create(filepath) {
        let _ = serde_json::to_writer(file, &json);
    }
}

fn poll_league_data() -> Option<Value> {
    let client = create_client();

    let result = client
        .get("https://127.0.0.1:2999/liveclientdata/allgamedata")
        .header(ACCEPT, "application/json")
        .timeout(Duration::from_secs(1))
        .send();
    let data = if let Ok(result) = result {
        if result.status() == StatusCode::OK {
            if let Ok(res) = result.json::<Value>() {
                res
            } else {
                return None;
            }
        } else {
            return None;
        }
    } else {
        return None;
    };

    if data["events"]["Events"][0]["EventName"] != "GameStart" {
        return None;
    }

    let mut player_info: Option<&Value> = None;
    let player_array = if let Some(arr) = data["allPlayers"].as_array() {
        arr
    } else {
        return None;
    };
    for player in player_array {
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
        None
    }
}

fn get_recorder_settings(video_path: &PathBuf) -> RecorderSettings {
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
    if let Some(path) = video_path.to_str() {
        settings.set_output_path(path);
    }
    return settings;
}

pub fn start_polling<R: Runtime>(app: tauri::AppHandle<R>) {
    let (sender, receiver) = channel::<_>();

    // send stop to channel on "shutdown" event
    app.once_global("shutdown", move |_| {
        let _ = sender.send(());
    });

    {
        // stuff to persist over loops
        // there is a scope around these so recorder gets dropped before Recorder::shutdown() is called
        let mut recorder = None;
        let mut filename = String::new();
        let mut league_data = Value::Null;
        let mut recording = false;

        loop {
            // if we are not recording and we get data from the API => start recording
            if let Some(data) = poll_league_data() {
                if !recording {
                    // create new unique filename from current time
                    let new_filename = format!("{}", Local::now().format("%Y-%m-%d_%H-%M.mp4"));
                    let mut video_path = get_recordings_folder();
                    video_path.push(PathBuf::from(&new_filename));

                    // create and set recorder if successful
                    let settings = get_recorder_settings(&video_path);
                    recorder = if let Ok(mut rec) = Recorder::get(settings) {
                        recording = rec.start_recording();
                        filename = new_filename;
                        Some(rec)
                    } else {
                        None
                    }
                }
                // update data to newest received version
                league_data = data;

            // if we are recording and we cant get any data from the API anymore => stop recording
            } else if recording {
                if let Some(mut rec) = recorder {
                    rec.stop_recording();
                };
                recording = false;
                // drop recorder
                recorder = None;

                // tell frontend to update video list
                let _ = app.emit_all("new_recording", &filename);

                // pass output_path and league data to save_metadata() and replace with placeholders
                save_metadata(filename, league_data);
                filename = String::new();
                league_data = Value::Null;
            }

            // if value received or disconnected => break
            // checks for sender disconnect
            match receiver.try_recv() {
                Err(TryRecvError::Empty) => {}
                _ => break,
            }
            // delay SLEEP_MS milliseconds waiting for stop event
            // break if stop event received
            // recv_timeout can't differentiate between timeout and disconnect
            match receiver.recv_timeout(Duration::from_secs(SLEEP_SECS)) {
                Ok(_) => break,
                _ => {}
            }
        }
    }

    Recorder::shutdown();
    app.exit(0);
}
