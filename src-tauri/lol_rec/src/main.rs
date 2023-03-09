extern crate core;

use std::{
    fs::{self, File},
    io::stdin,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use anyhow::anyhow;
use futures::stream::StreamExt;
use libobs_recorder::{RateControl, Recorder, RecorderSettings, Window};
use shaco::{
    ingame::{EventStream, IngameClient},
    model::ingame::{DragonType, GameEvent, GameResult, Killer},
    model::ws::LcuSubscriptionType::JsonApiEvent,
    ws::LcuWebsocketClient,
};
use tokio::{io::AsyncBufReadExt, io::BufReader, time};
use tokio_util::sync::CancellationToken;

use config::Config;

use crate::data::GameData;

mod config;
mod data;

const WINDOW_TITLE: &str = "League of Legends (TM) Client";
const WINDOW_CLASS: &str = "RiotWindowClass";
const WINDOW_PROCESS: &str = "League of Legends.exe";

fn main() -> anyhow::Result<()> {
    // read config from stdin
    let cfg = {
        let mut buffer = String::new();
        stdin().read_line(&mut buffer)?;
        Config::init(&buffer)?
    };
    let debug_log: bool = cfg.debug_log();
    if debug_log {
        println!("lol_rec:");
        println!("config valid");
    }

    // async runtime for since Shaco is an async LoL API client
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    // init Recorder
    let libobs_data_path = Some(String::from("./libobs/data/libobs/"));
    let plugin_bin_path = Some(String::from("./libobs/obs-plugins/64bit/"));
    let plugin_data_path = Some(String::from("./libobs/data/obs-plugins/%module%/"));
    match Recorder::init(libobs_data_path, plugin_bin_path, plugin_data_path) {
        Ok(enc) => {
            if debug_log {
                println!("recorder init successful");
                println!("available encoders: {:?}", enc);
            }
        }
        Err(e) => return Err(anyhow!("{e}")),
    };

    // create recorder settings and libobs_recorder::Recorder
    let filename = format!("{}", chrono::Local::now().format(cfg.filename_format()));
    if debug_log {
        println!("filename: {}", &filename);
    }
    let settings = {
        let cfg = &cfg;
        let filename: &str = &filename;
        let mut settings = RecorderSettings::new();

        settings.set_window(Window::new(
            WINDOW_TITLE,
            Some(WINDOW_CLASS.into()),
            Some(WINDOW_PROCESS.into()),
        ));

        settings.set_input_size(cfg.window_size());
        settings.set_output_resolution(cfg.output_resolution());
        settings.set_framerate(cfg.framerate());
        settings.set_rate_control(RateControl::CQP(cfg.encoding_quality()));
        settings.record_audio(cfg.record_audio());

        let mut video_path = cfg.recordings_folder();
        video_path.push(PathBuf::from(filename));
        settings.set_output_path(video_path.to_str().expect("error converting video_path to &str"));

        settings
    };
    let mut recorder = match Recorder::get(settings) {
        Ok(rec) => rec,
        Err(e) => return Err(anyhow!("{e}")),
    };
    if debug_log {
        println!("recorder created");
    }

    // wait for recorder to initialize, hook into the game, ...
    // if we don't do this we start the recording with a few seconds of black screen
    thread::sleep(Duration::from_secs(3));

    if !recorder.start_recording() {
        return Err(anyhow!("Error starting recording"));
    }
    if debug_log {
        println!("recording started");
    }

    let recording_start = Instant::now();

    let cancel_token = CancellationToken::new();
    let cancel_subtoken1 = cancel_token.child_token();
    let cancel_subtoken2 = cancel_token.child_token();

    // wait for premature stop by parent process (league_record.exe)
    let handle = runtime.spawn(async move {
        let mut buffer = String::new();
        let mut reader = BufReader::new(tokio::io::stdin());

        loop {
            let read_line = reader.read_line(&mut buffer);
            if time::timeout(Duration::from_secs(5000), read_line).await.is_err() || buffer == "stop" {
                // if there was a "stop" in stdin or we are running longer than 83 minutes (5000 minutes)
                // cancel all other tasks and exit
                cancel_token.cancel();
                return;
            }
            buffer.clear();
        }
    });

    // wait for game to start and collect initial game infos
    let ingame_client = IngameClient::new()?;
    let game_data = runtime.block_on(async move {
        let mut timer = time::interval(Duration::from_millis(500));
        while !ingame_client.active_game().await {
            // busy wait for game to start
            // "sleep" by selecting either the next timer tick or the token cancel
            tokio::select! {
                _ = cancel_subtoken1.cancelled() => return None,
                _ = timer.tick() => {}
            }
        }

        // don't record spectator games
        if let Ok(true) = ingame_client.is_spectator_mode().await {
            return None;
        };

        let mut game_data = GameData::default();
        if let Ok(data) = ingame_client.all_game_data(None).await {
            // delay from recording start to ingame start (00:00:00)
            game_data.game_info.recording_delay = recording_start.elapsed().as_secs_f64() - data.game_data.game_time;
            game_data.game_info.game_mode = data.game_data.game_mode.to_string();
            // unwrap because active player always exists in livegame which we check for above
            game_data.game_info.summoner_name = data.active_player.unwrap().summoner_name;
            game_data.game_info.champion_name = data
                .all_players
                .into_iter()
                .find_map(|p| {
                    if p.summoner_name == game_data.game_info.summoner_name {
                        Some(p.champion_name)
                    } else {
                        None
                    }
                })
                .unwrap();
        }

        // pass back the data and the ingame_client to reuse
        Some((game_data, ingame_client))
    });

    // If some error occurred stop and delete the recording
    let Some((mut game_data, ingame_client)) = game_data else {
        recorder.stop_recording();
        let outfile = cfg.recordings_folder().join(filename);
        let _ = fs::remove_file(outfile);
        return Ok(());
    };

    let game_data = runtime.block_on(async move {
        // wait for ingame events and filter them (or stop recording on cancel)
        let mut ingame_events = EventStream::from_ingame_client(ingame_client, None);
        loop {
            tokio::select! {
                _ = cancel_subtoken2.cancelled() => {
                    recorder.stop_recording();
                    return game_data;
                }
                Some(event) = ingame_events.next() => {
                    let time = event.get_event_time();
                    let event_name = match event {
                        GameEvent::BaronKill(_) => Some("Baron"),
                        GameEvent::ChampionKill(e) => {
                            let summoner_name = &game_data.game_info.summoner_name;

                            let mut result = None;
                            if let Killer::Summoner(ref killer_name) = e.killer_name {
                                if killer_name == summoner_name {
                                    result = Some("Kill");
                                }
                            } else if e.assisters.contains(summoner_name) {
                                result = Some("Assist");
                            } else if &e.victim_name == summoner_name {
                                result = Some("Death");
                            }

                            result
                        }
                        GameEvent::DragonKill(e) => {
                            let dragon = match e.dragon_type {
                                DragonType::Infernal => "Infernal-Dragon",
                                DragonType::Ocean => "Ocean-Dragon",
                                DragonType::Mountain => "Mountain-Dragon",
                                DragonType::Cloud => "Cloud-Dragon",
                                DragonType::Hextech => "Hextech-Dragon",
                                DragonType::Chemtech => "Chemtech-Dragon",
                                DragonType::Elder => "Elder-Dragon",
                            };
                            Some(dragon)
                        }
                        GameEvent::GameEnd(e) => {
                            game_data.win = match e.result {
                                GameResult::Win => Some(true),
                                GameResult::Lose => Some(false),
                            };
                            None
                        }
                        GameEvent::HeraldKill(_) => Some("Herald"),
                        GameEvent::InhibKilled(_) => Some("Inhibitor"),
                        GameEvent::TurretKilled(_) => Some("Turret"),
                        _ => None,
                    };

                    if let Some(name) = event_name {
                        game_data.events.push(data::GameEvent { name, time })
                    }
                }
                else => {
                    recorder.stop_recording();
                    break;
                }
            }
        }

        // after the game client closes wait for LCU websocket End Of Game event
        let Ok(mut ws_client) = LcuWebsocketClient::connect().await else { return game_data };
        let subscription = ws_client
            .subscribe(JsonApiEvent("lol-end-of-game/v1/eog-stats-block".to_string()))
            .await;
        if subscription.is_err() {
            return game_data;
        }

        tokio::select! {
            _ = cancel_subtoken2.cancelled() => (),
            Ok(Some(event)) = time::timeout(Duration::from_secs(15), ws_client.next()) => {
                if let Ok(stats) = serde_json::from_value(event.data) {
                    game_data.stats = stats;
                } else if debug_log {
                    println!("Error deserializing end of game stats");
                }
            }
            else => ()
        }
        game_data
    });

    // stop the future listening for "stop" on stdin since we now always complete
    handle.abort();

    // write metadata file for recording
    let mut outfile = cfg.recordings_folder().join(filename);
    outfile.set_extension("json");
    if let Ok(file) = File::create(&outfile) {
        let _ = serde_json::to_writer(file, &game_data);
    }

    // Recorder::shutdown(); // somehow hangs here - dont know why yet

    if debug_log {
        println!("stopped recording and exit lol_rec");
    }
    Ok(())
}
