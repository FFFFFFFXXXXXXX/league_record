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
use tauri::{AppHandle, Manager, Runtime};

use crate::{helpers::create_client, state::RecordingsFolder};

const FILENAME_FORMAT: &str = "%Y-%m-%d_%H-%M.mp4";
const SLEEP_SECS: u64 = 5;

fn save_metadata<R: Runtime>(
    filename: String,
    recording_delay: Value,
    mut json: Value,
    app_handle: &AppHandle<R>,
) {
    let player_name = json["playerName"].clone();
    let events = json.get_mut("events").unwrap().as_array().unwrap();
    let new_events = events
        .iter()
        .filter_map(|event| match event["EventName"].as_str()? {
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
                let assisters = event["Assisters"].as_array()?;
                if event["VictimName"] == player_name {
                    Some(json!({
                        "eventName": "Death",
                        "eventTime": event["EventTime"]
                    }))
                } else if event["KillerName"] == player_name {
                    Some(json!({
                        "eventName": "Kill",
                        "eventTime": event["EventTime"]
                    }))
                } else if assisters.contains(&player_name) {
                    Some(json!({
                        "eventName": "Assist",
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
        });

    // replace old events with new events
    json["events"] = Value::Array(new_events.collect());
    json["recordingDelay"] = recording_delay;

    let mut filepath = app_handle.state::<RecordingsFolder>().get();
    filepath.push(PathBuf::from(filename));
    filepath.set_extension("json");
    if let Ok(file) = File::create(filepath) {
        let _ = serde_json::to_writer(file, &json);
    }
}

fn set_recording_tray_item<R: Runtime>(app_handle: &AppHandle<R>, recording: bool) {
    let item = app_handle.tray_handle().get_item("rec");
    // set selected only updates the tray menu when open if the menu item is enabled
    let _ = item.set_enabled(true);
    let _ = item.set_selected(recording);
    let _ = item.set_enabled(false);
}

fn poll_league_data() -> Option<Value> {
    let client = create_client();

    let result = client
        .get("https://127.0.0.1:2999/liveclientdata/allgamedata")
        .header(ACCEPT, "application/json")
        .timeout(Duration::from_secs(1))
        .send()
        .ok()?;

    let data = if result.status() == StatusCode::OK {
        result.json::<Value>().ok()?
    } else {
        return None;
    };

    if data["events"]["Events"][0]["EventName"] != "GameStart" {
        return None;
    }

    let mut player_info: Option<&Value> = None;
    let player_array = data["allPlayers"].as_array()?;
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
            "gameMode": data["gameData"]["gameMode"],
            "recordingDelay": data["gameData"]["gameTime"]
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

pub fn start_polling<R: Runtime>(app_handle: AppHandle<R>) {
    let (sender, receiver) = channel::<_>();

    // send stop to channel on "shutdown" event
    app_handle.once_global("shutdown", move |_| {
        let _ = sender.send(());
    });

    {
        // stuff to persist over loops
        // there is a scope around these so recorder gets dropped before Recorder::shutdown() is called
        let mut recorder = None;
        let mut recording = false;
        let mut league_data = Value::Null;
        let mut recording_delay = Value::Null;
        let mut filename = String::new();

        loop {
            // if we are not recording and we get data from the API => start recording
            if let Some(data) = poll_league_data() {
                if !recording {
                    // create new unique filename from current time
                    let new_filename = format!("{}", Local::now().format(FILENAME_FORMAT));
                    let mut video_path = app_handle.state::<RecordingsFolder>().get();
                    video_path.push(PathBuf::from(&new_filename));

                    // create and set recorder if successful
                    let settings = get_recorder_settings(&video_path);
                    recorder = if let Ok(mut rec) = Recorder::get(settings) {
                        recording = rec.start_recording();
                        if recording {
                            set_recording_tray_item(&app_handle, true);
                            // store the delay between ingame time and first poll time
                            recording_delay = data["recordingDelay"].clone();
                            filename = new_filename;
                            Some(rec)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                // update data to newest received version
                league_data = data;

            // if we are recording and we cant get any data from the API anymore => stop recording
            } else if recording {
                if let Some(mut rec) = recorder {
                    recording = rec.stop_recording();
                };
                set_recording_tray_item(&app_handle, false);
                // drop recorder
                recorder = None;

                // tell frontend to update video list
                let _ = app_handle.emit_all("new_recording", &filename);

                // pass output_path and league data to save_metadata() and replace with placeholders
                save_metadata(filename, recording_delay, league_data, &app_handle);
                league_data = Value::Null;
                recording_delay = Value::Null;
                filename = String::new();
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
    app_handle.exit(0);
}
