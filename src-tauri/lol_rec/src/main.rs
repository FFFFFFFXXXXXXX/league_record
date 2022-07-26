mod config;

use std::{
    fs::File,
    io::stdin,
    path::PathBuf,
    process::exit,
    sync::mpsc::{channel, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use bytes::Bytes;
use libobs_recorder::{
    rate_control::{Cqp, Icq},
    Recorder, RecorderSettings, Window,
};
use reqwest::{blocking::Client, header::ACCEPT, StatusCode};
use serde_json::{json, Value};

use crate::config::Config;

const WINDOW_TITLE: &str = "League of Legends (TM) Client";
const WINDOW_CLASS: &str = "RiotWindowClass";
const WINDOW_PROCESS: &str = "League of Legends.exe";

const SLEEP_SECS: u64 = 1;

fn main() {
    let stdin = stdin();

    // read config from stdin
    let cfg = {
        let mut buffer = String::new();
        stdin.read_line(&mut buffer).expect("error reading stdin");

        match Config::init(&buffer) {
            Ok(cfg) => cfg,
            _ => exit(1),
        }
    };
    let debug_log: bool = cfg.debug_log();
    if debug_log {
        println!("lol_rec:");
        println!("config valid");
    }

    // init Recorder
    let libobs_data_path = Some(String::from("./libobs/data/libobs/"));
    let plugin_bin_path = Some(String::from("./libobs/obs-plugins/64bit/"));
    let plugin_data_path = Some(String::from("./libobs/data/obs-plugins/%module%/"));
    match Recorder::init(libobs_data_path, plugin_bin_path, plugin_data_path) {
        Ok(enc) => {
            if debug_log {
                println!("recorder init successful: {}", enc.id());
            }
        }
        Err(_) => exit(1),
    };

    // create recorder settings
    let filename = format!("{}", chrono::Local::now().format(cfg.filename_format()));
    if debug_log {
        println!("filename: {}", &filename);
    }
    let settings = create_recorder_settings(&cfg, &filename);
    let mut recorder = match Recorder::get(settings) {
        Ok(rec) => rec,
        Err(_) => exit(1),
    };
    if debug_log {
        println!("recorder created");
    }

    thread::sleep(Duration::from_secs(3));
    // start recording
    if !recorder.start_recording() {
        exit(1);
    }
    let instant = Instant::now();
    let stop_time = Duration::from_secs(5000);
    if debug_log {
        println!("recording started");
    }

    // poll game data thread
    let (sender, receiver) = channel::<_>();
    let thread = thread::spawn({
        let rec_folder = cfg.recordings_folder();
        let client = create_client();

        let mut init = true;
        let mut data_delay = 0.0;
        let mut game_data = Bytes::default();
        move || {
            loop {
                // update data if recording
                let data = get_league_data(&client).unwrap_or_default();
                if !data.is_empty() {
                    // store the delay between event time and recording time
                    // but only if recording delay is unset (<0.0)
                    if init {
                        if let Some(ts) = get_timestamp(&data, debug_log) {
                            data_delay = instant.elapsed().as_secs_f64() - ts;
                            init = false;
                        }
                    }
                    game_data = data;
                }

                // delay SLEEP_MS milliseconds waiting for stop event
                // break if stop event received or sender disconnected
                match receiver.recv_timeout(Duration::from_secs(SLEEP_SECS)) {
                    Ok(_) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }

            if !game_data.is_empty() {
                save_metadata(rec_folder, filename, data_delay, &game_data);
            }
        }
    });

    // wait for stdin: "stop" or timeout ~83min
    let mut buffer = String::new();
    while buffer != "stop" && instant.elapsed() < stop_time {
        if debug_log {
            println!("check buffer");
        }
        buffer.clear();
        stdin.read_line(&mut buffer).expect("error reading stdin");
    }

    recorder.stop_recording();
    let _ = sender.send(());
    let _ = thread.join();
    if debug_log {
        println!("stopped recording and exit lol_rec");
    }
}

fn create_recorder_settings(cfg: &Config, filename: &str) -> RecorderSettings {
    let mut settings = RecorderSettings::new();

    settings.set_window(Window::new(
        WINDOW_TITLE,
        Some(WINDOW_CLASS.into()),
        Some(WINDOW_PROCESS.into()),
    ));

    settings.set_input_size(cfg.window_size());

    settings.set_output_resolution(cfg.output_resolution());
    settings.set_framerate(cfg.framerate());
    settings.set_cqp(Cqp::new(cfg.encoding_quality())); // for amd/nvidia/software
    settings.set_icq(Icq::new(cfg.encoding_quality())); // for intel quicksync
    settings.record_audio(cfg.record_audio());

    let mut video_path = cfg.recordings_folder();
    video_path.push(PathBuf::from(filename));
    settings.set_output_path(
        video_path
            .to_str()
            .expect("error converting video_path to &str"),
    );

    settings
}

fn create_client() -> Client {
    let pem = include_bytes!("../riotgames.pem");
    let cert = reqwest::Certificate::from_pem(pem).expect("couldn't create certificate");

    Client::builder()
        .add_root_certificate(cert)
        .build()
        .expect("couldn't create http client")
}

fn get_league_data(client: &Client) -> Option<Bytes> {
    let result = client
        .get("https://127.0.0.1:2999/liveclientdata/allgamedata")
        .header(ACCEPT, "application/json")
        .timeout(Duration::from_secs(1))
        .send()
        .ok()?;

    match result.status() {
        StatusCode::OK => result.bytes().ok(),
        _ => None,
    }
}

fn get_timestamp(bytes: &Bytes, debug_log: bool) -> Option<f64> {
    let data: Value = match serde_json::from_slice(bytes) {
        Ok(data) => data,
        Err(_) => return None,
    };

    // make sure the game has started
    if debug_log {
        println!(
            "game started: {}",
            data["events"]["Events"][0]["EventName"] == "GameStart"
        );
    }
    if data["events"]["Events"][0]["EventName"] != "GameStart" {
        return None;
    }

    data["gameData"]["gameTime"].as_f64()
}

fn deserialize_game_data(bytes: &Bytes) -> Result<Value, ()> {
    let data: Value = match serde_json::from_slice(bytes) {
        Ok(data) => data,
        Err(_) => return Err(()),
    };

    let mut player_info = None;
    let player_array = data["allPlayers"].as_array().unwrap();
    for player in player_array {
        if player["summonerName"] == data["activePlayer"]["summonerName"] {
            player_info = Some(player);
            break;
        }
    }

    if let Some(player_info) = player_info {
        Ok(json!({
            "playerName": data["activePlayer"]["summonerName"],
            "championName": player_info["championName"],
            "stats": player_info["scores"],
            "events": data["events"]["Events"],
            "gameMode": data["gameData"]["gameMode"]
        }))
    } else {
        Err(())
    }
}

fn save_metadata(mut folder: PathBuf, filename: String, data_delay: f64, data: &Bytes) {
    let mut json = match deserialize_game_data(data) {
        Ok(j) => j,
        Err(_) => return,
    };

    let mut result = Value::Null;

    let player_name = json["playerName"].clone();
    let events = json
        .get_mut("events")
        .expect("invalid json metadata")
        .as_array()
        .expect("invalid json metadata");
    let new_events = events
        .iter()
        .filter_map(|event| match event["EventName"].as_str()? {
            "DragonKill" => {
                let mut dragon = String::from(
                    event["DragonType"]
                        .as_str()
                        .expect("error in allgamedata json: invalid DragonType"),
                );
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
            "GameEnd" => {
                result = event["Result"].clone();
                None
            }
            _ => None,
        });

    // replace old events with new events
    json["events"] = Value::Array(new_events.collect());
    json["dataDelay"] = Value::from(data_delay);
    json["result"] = result;

    folder.push(PathBuf::from(filename));
    folder.set_extension("json");
    if let Ok(file) = File::create(folder) {
        let _ = serde_json::to_writer(file, &json);
    }
}
