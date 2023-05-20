mod data;

use anyhow::anyhow;
use futures::stream::StreamExt;
use libobs_recorder::{RateControl, Recorder, RecorderSettings, Window};
use shaco::{
    ingame::{EventStream, IngameClient},
    model::ingame::{DragonType, GameEvent, GameResult, Killer},
    model::{ws::LcuSubscriptionType::JsonApiEvent, ingame::ChampionKill},
    ws::LcuWebsocketClient,
};
use tokio::{io::AsyncBufReadExt, io::BufReader, time};
use tokio_util::sync::CancellationToken;

use std::{
    fs::{self, File},
    io::stdin,
    path::PathBuf,
    process::exit,
    thread,
    time::{Duration, Instant},
};

use crate::data::GameData;

const WINDOW_TITLE: &str = "League of Legends (TM) Client";
const WINDOW_CLASS: &str = "RiotWindowClass";
const WINDOW_PROCESS: &str = "League of Legends.exe";

fn main() -> anyhow::Result<()> {
    // read config from stdin
    let cfg = {
        let mut buffer = String::new();
        stdin().read_line(&mut buffer)?;
        serde_json::from_str::<common::Config>(&buffer)?
    };
    if cfg.debug_log {
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
            if cfg.debug_log {
                println!("recorder init successful");
                println!("available encoders: {:?}", enc);
            }
        }
        Err(e) => return Err(anyhow!("{e}")),
    };

    // create recorder settings and libobs_recorder::Recorder
    let filename = format!("{}", chrono::Local::now().format(&cfg.filename_format));
    if cfg.debug_log {
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

        settings.set_input_size(cfg.window_size);
        settings.set_output_resolution(cfg.output_resolution);
        settings.set_framerate(cfg.framerate);
        settings.set_rate_control(RateControl::CQP(cfg.encoding_quality));
        settings.record_audio(cfg.record_audio);

        let mut video_path = cfg.recordings_folder.clone();
        video_path.push(PathBuf::from(filename));
        settings.set_output_path(video_path.to_str().expect("error converting video_path to &str"));

        settings
    };
    let mut recorder = match Recorder::get(settings) {
        Ok(rec) => rec,
        Err(e) => return Err(anyhow!("{e}")),
    };
    if cfg.debug_log {
        println!("recorder created");
    }

    // wait for recorder to initialize, hook into the game, ...
    // if we don't do this we start the recording with a few seconds of black screen
    thread::sleep(Duration::from_millis(2500));

    if !recorder.start_recording() {
        return Err(anyhow!("Error starting recording"));
    }
    let recording_start = Instant::now();
    if cfg.debug_log {
        println!("recording started");
    }

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
                if cfg.debug_log {
                    println!("\"stop\" signal received or timeout");
                }
                return;
            }
            buffer.clear();
        }
    });

    if cfg.debug_log {
        println!("stop listener spawned");
    }

    // wait for API to be available and collect initial game infos
    let ingame_client = IngameClient::new()?;
    let game_data = runtime.block_on(async move {
        let mut timer = time::interval(Duration::from_millis(500));
        while !ingame_client.active_game().await {
            // busy wait for API
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

    if cfg.debug_log {
        println!("Game started / initial data collected");
    }

    // If some error occurred stop and delete the recording
    let Some((mut game_data, ingame_client)) = game_data else {
        recorder.stop_recording();
        let outfile = cfg.recordings_folder.join(filename);
        let _ = fs::remove_file(outfile);

        if cfg.debug_log {
            println!("Error getting initial data - stopping and deleting recording");
        }

        Recorder::shutdown();
    
        // explicitly call exit() instead of return normally since the process doesn't stop if we don't
        // my best guess is there are some libobs background tasks or threads still running that prevent full termination
        exit(1);
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
                event = ingame_events.next() => {
                    let Some(event) = event else {
                        if cfg.debug_log {
                            println!("None event received - stopping recording");
                        }
                        recorder.stop_recording();
                        break;
                    };

                    let time = recording_start.elapsed().as_secs_f64();
                    if cfg.debug_log {
                        println!("[{}] new ingame event: {:?}", time, event);
                    }

                    let event_name = match event {
                        GameEvent::BaronKill(_) => Some("Baron"),
                        GameEvent::ChampionKill(e) => {
                            let summoner_name = &game_data.game_info.summoner_name;
                            match e {
                                ChampionKill { killer_name: Killer::Summoner(ref killer_name), .. } if killer_name == summoner_name => {
                                    Some("Kill")
                                }
                                ChampionKill { ref victim_name, .. } if victim_name == summoner_name => {
                                    Some("Death")
                                }
                                ChampionKill { assisters, .. } if assisters.contains(summoner_name) => {
                                    Some("Assist")
                                }
                                _ => None,
                            }
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
            event = time::timeout(Duration::from_secs(30), ws_client.next()) => {
                if let Ok(Some(mut event)) = event {
                    let json_stats = event.data["localPlayer"]["stats"].take();

                    if cfg.debug_log {
                        println!("EOG stats: {:?}", json_stats);
                    }

                    if game_data.win.is_none() {
                        // on win the data contains a "WIN" key with a value of '1'
                        // on lose the data contains a "LOSE" key with a value of '1'
                        // So if json_stats["WIN"] is not null => WIN
                        // and if json_stats["LOSE"] is not null => LOSE
                        if !json_stats["WIN"].is_null() {
                            game_data.win = Some(true);
                        } else if !json_stats["LOSE"].is_null() {
                            game_data.win = Some(false);
                        }
                    }

                    match serde_json::from_value(json_stats) {
                        Ok(stats) => game_data.stats = stats,
                        Err(e) if cfg.debug_log => println!("Error deserializing end of game stats: {:?}", e),
                        _ => {}
                    }
                } else if cfg.debug_log {
                    println!("LCU event listener timed out");
                }
            }
        }
        game_data
    });

    // stop the future listening for "stop" on stdin since we now always complete
    handle.abort();

    // write metadata file for recording
    let mut outfile = cfg.recordings_folder.join(filename);
    outfile.set_extension("json");
    if let Ok(file) = File::create(&outfile) {
        let _ = serde_json::to_writer(file, &game_data);

        if cfg.debug_log {
            println!("metadata saved"); 
        }
    }

    if cfg.debug_log {
        println!("shutting down Recorder");
    }

    Recorder::shutdown();

    if cfg.debug_log {
        println!("stopped recording and exit lol_rec");
    }

    // explicitly call exit() instead of return normally since the process doesn't stop if we don't
    // my best guess is there are some libobs background tasks or threads still running that prevent full termination
    exit(0);
}
