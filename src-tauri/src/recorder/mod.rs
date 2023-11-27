use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    sync::{
        mpsc::{channel, RecvTimeoutError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use libobs_recorder::{
    settings::{RateControl, Resolution, Size, Window},
    Recorder, RecorderSettings,
};
use shaco::{
    ingame::{EventStream, IngameClient},
    model::{
        ingame::{ChampionKill, DragonType, GameEvent, GameResult, Killer},
        ws::LcuSubscriptionType::JsonApiEvent,
    },
    ws::LcuWebsocketClient,
};
use tauri::{
    async_runtime::{self, JoinHandle},
    AppHandle, Manager,
};
use tokio_util::sync::CancellationToken;
#[cfg(target_os = "windows")]
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{HWND, RECT},
        UI::WindowsAndMessaging::{FindWindowA, GetClientRect},
    },
};

use crate::{helpers::set_recording_tray_item, state::Settings};

mod data;

const WINDOW_TITLE: &'static str = "League of Legends (TM) Client";
const WINDOW_CLASS: &'static str = "RiotWindowClass";
const WINDOW_PROCESS: &'static str = "League of Legends.exe";

const DEFAULT_RESOLUTIONS_FOR_ASPECT_RATIOS: [(Resolution, f64); 9] = [
    (Resolution::_1600x1200p, 4.0 / 3.0),
    (Resolution::_1280x1024p, 5.0 / 4.0),
    (Resolution::_1920x1080p, 16.0 / 9.0),
    (Resolution::_1920x1200p, 16.0 / 10.0),
    (Resolution::_2560x1080p, 21.0 / 9.0),
    (Resolution::_2580x1080p, 43.0 / 18.0),
    (Resolution::_3840x1600p, 24.0 / 10.0),
    (Resolution::_3840x1080p, 32.0 / 9.0),
    (Resolution::_3840x1200p, 32.0 / 10.0),
];

fn closest_resolution_to_size(window_size: &Size) -> Resolution {
    let aspect_ratio = f64::from(window_size.width()) / f64::from(window_size.height());
    // sort difference of aspect_ratio to comparison by absolute values => most similar aspect ratio is at index 0
    let mut aspect_ratios =
        DEFAULT_RESOLUTIONS_FOR_ASPECT_RATIOS.map(|(res, ratio)| (res, f64::abs(ratio - aspect_ratio)));
    aspect_ratios.sort_by(|(_, ratio1), (_, ratio2)| ratio1.partial_cmp(&ratio2).or(Some(Ordering::Equal)).unwrap());
    aspect_ratios.first().unwrap().0
}

pub fn start(app_handle: &AppHandle) {
    let app_handle = app_handle.clone();

    thread::spawn(move || {
        // send stop to channel on "shutdown" event
        let (tx, rx) = channel::<_>();
        app_handle.once_global("shutdown_recorder", move |_| _ = tx.send(()));

        // get owned copy of settings so we can change window_size
        let settings_state = app_handle.state::<Settings>();
        let debug_log = settings_state.debug_log();

        enum State {
            Idle,
            Recording,
        }

        let mut state = State::Idle;

        // use Options to 'store' values between loops
        let recorder: Arc<Mutex<Option<Recorder>>> = Arc::new(Mutex::new(None));
        let mut game_data_thread: Option<(JoinHandle<()>, CancellationToken)> = None;

        'recorder: loop {
            match state {
                State::Idle => 'inner: {
                    // --- initialize recorder if LoL window is found ---
                    let Some(window_handle) = get_lol_window() else {
                        break 'inner;
                    };

                    if debug_log {
                        println!("LoL Window found");
                    }

                    let Ok(window_size) = get_window_size(window_handle) else {
                        if debug_log {
                            println!("unable to get window size of League of Legends.exe");
                        }
                        break 'inner;
                    };

                    // either get the explicitly set resolution or choose the default resolution for the LoL window aspect ratio
                    let output_resolution = settings_state
                        .get_output_resolution()
                        .unwrap_or_else(|| closest_resolution_to_size(&window_size));

                    if debug_log {
                        println!("Using resolution ({output_resolution:?}) for window ({window_size:?})");
                    }

                    let mut filename_path = settings_state.get_recordings_path();
                    filename_path.push(format!(
                        "{}",
                        chrono::Local::now().format(&settings_state.get_filename_format())
                    ));

                    let mut settings = RecorderSettings::new();
                    settings.set_window(Window::new(
                        WINDOW_TITLE,
                        Some(WINDOW_CLASS.into()),
                        Some(WINDOW_PROCESS.into()),
                    ));
                    settings.set_input_resolution(window_size);
                    settings.set_output_resolution(output_resolution);
                    settings.set_framerate(settings_state.get_framerate());
                    settings.set_rate_control(RateControl::CQP(settings_state.get_encoding_quality()));
                    settings.record_audio(settings_state.get_audio_source());
                    settings.set_output_path(filename_path.to_str().expect("error converting filename path to &str"));

                    let mut rec = match Recorder::new_with_paths(
                        Some(Path::new("./libobs/extprocess_recorder.exe")),
                        None,
                        None,
                        None,
                        debug_log,
                    ) {
                        Ok(rec) => rec,
                        Err(e) => {
                            if debug_log {
                                println!("failed to create recorder: {e}");
                            }
                            break 'inner;
                        }
                    };

                    let configured = rec.configure(&settings);
                    if debug_log {
                        println!("recorder configured: {configured:?}");
                        println!("Available encoders: {:?}", rec.available_encoders());
                        println!("Selected encoder: {:?}", rec.selected_encoder());
                    }
                    if configured.is_err() {
                        break 'inner;
                    }

                    *recorder.lock().unwrap() = Some(rec);

                    // --- ingame data collection ---
                    let cancel_token = CancellationToken::new();
                    let app_handle = app_handle.clone();
                    let handle = async_runtime::spawn({
                        let cancel_subtoken = cancel_token.child_token();
                        let recorder = Arc::clone(&recorder);

                        let mut outfile = settings_state.get_recordings_path().join(filename_path);
                        outfile.set_extension("json");

                        async move {
                            collect_ingame_data(app_handle, cancel_subtoken, recorder, outfile, debug_log).await;
                        }
                    });
                    if debug_log {
                        println!("ingame task spawned: {handle:?}");
                    }

                    game_data_thread = Some((handle, cancel_token));
                    state = State::Recording;
                }
                State::Recording => 'inner: {
                    // stop if LoL window closed
                    if get_lol_window().is_some() {
                        break 'inner;
                    }

                    // stop recorder
                    if let Some(mut rec) = recorder.lock().unwrap().take() {
                        let stopped = rec.stop_recording();
                        let shutdown = rec.shutdown();

                        if debug_log {
                            println!("recorder stopped: {stopped:?}");
                            println!("recorder shutdown: {shutdown:?}");
                        }

                        set_recording_tray_item(&app_handle, false);
                    };

                    // spawn async thread to cleanup the game_data_thread if it doesn't exit by itself
                    if let Some((mut handle, cancel_token)) = game_data_thread.take() {
                        if handle.inner().is_finished() {
                            break 'inner;
                        }

                        async_runtime::spawn(async move {
                            // wait for 90s for EOG lobby before trying to cancel the task
                            match tokio::time::timeout(Duration::from_secs(90), &mut handle).await {
                                Ok(_) => return,
                                Err(_) => cancel_token.cancel(),
                            }
                            // abort task if the cancel_token didn't stop the it (after 5s)
                            if tokio::time::timeout(Duration::from_secs(5), &mut handle).await.is_err() {
                                handle.abort();
                            }
                        });
                    }

                    state = State::Idle;
                }
            }

            // break if stop event received or sender disconnected
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(_) | Err(RecvTimeoutError::Disconnected) => {
                    // stop recorder if running
                    if let Some(mut rec) = recorder.lock().unwrap().take() {
                        let stopped = rec.stop_recording();
                        let shutdown = rec.shutdown();

                        if debug_log {
                            println!("recorder stopped: {stopped:?}|{shutdown:?}");
                        }

                        set_recording_tray_item(&app_handle, false);
                    };

                    // cleanup the game_data_thread if it doesn't exit by itself
                    if let Some((handle, cancel_token)) = game_data_thread.take() {
                        cancel_token.cancel();
                        // give the task a little bit of time to complete a fs::write or sth
                        std::thread::sleep(Duration::from_millis(250));
                        handle.abort();
                    }

                    break 'recorder;
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }

        app_handle.trigger_global("recorder_shutdown", None);

        if debug_log {
            println!("recorder shutdown");
        }
    });
}

async fn collect_ingame_data(
    app_handle: AppHandle,
    cancel_subtoken: CancellationToken,
    recorder: Arc<Mutex<Option<Recorder>>>,
    outfile: PathBuf,
    debug_log: bool,
) {
    // IngameClient::new() never actually returns Err()
    let ingame_client = IngameClient::new().unwrap();

    if debug_log {
        println!("waiting for game to start");
    }

    let mut timer = tokio::time::interval(Duration::from_millis(500));
    while !ingame_client.active_game().await {
        // busy wait for API
        // "sleep" by selecting either the next timer tick or the token cancel
        tokio::select! {
            _ = cancel_subtoken.cancelled() => return,
            _ = timer.tick() => {}
        }
    }

    // don't record spectator games
    if let Ok(true) = ingame_client.is_spectator_mode().await {
        if debug_log {
            println!("spectator game detected - aborting");
        }
        return;
    } else if debug_log {
        println!("game started")
    }

    let mut game_data = data::GameData::default();
    if let Ok(data) = ingame_client.all_game_data(None).await {
        if debug_log {
            println!("initial data recieved: {data:?}");
        }

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

    if debug_log {
        println!("initial data parsed: {game_data:?}");
    }

    // if initial game_data is successful => start recording
    if let Some(rec) = recorder.lock().unwrap().as_mut() {
        let start_recording = rec.start_recording();

        if debug_log {
            println!("start recording: {start_recording:?}");
        }

        if start_recording.is_err() {
            // if recording start failed stop recording just in case and retry next loop
            let stop_recording = rec.stop_recording();
            if debug_log {
                println!("start failed - stop recording: {stop_recording:?}");
            }
            return;
        }
    } else {
        if debug_log {
            println!("error getting recorder from Mutex - aborting");
        }
        return;
    }

    // recording started
    let recording_start = Some(Instant::now());
    set_recording_tray_item(&app_handle, true);

    // get values from Options that are always Some
    let mut ingame_events = EventStream::from_ingame_client(ingame_client, None);
    let recording_start = recording_start.as_ref().unwrap();

    if debug_log {
        println!("Started EventStream - listening to ingame events");
    }

    while let Some(event) =
        tokio::select! { event = ingame_events.next() => event, _ = cancel_subtoken.cancelled() => None }
    {
        let time = recording_start.elapsed().as_secs_f64();
        if debug_log {
            println!("[{}]: {:?}", time, event);
        }

        let event_name = match event {
            GameEvent::BaronKill(_) => Some("Baron"),
            GameEvent::ChampionKill(e) => {
                let summoner_name = &game_data.game_info.summoner_name;
                match e {
                    ChampionKill {
                        killer_name: Killer::Summoner(ref killer_name),
                        ..
                    } if killer_name == summoner_name => Some("Kill"),
                    ChampionKill { ref victim_name, .. } if victim_name == summoner_name => Some("Death"),
                    ChampionKill { assisters, .. } if assisters.contains(summoner_name) => Some("Assist"),
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

    if debug_log {
        println!("Ingame window has closed");
    }

    // after the game client closes wait for LCU websocket End Of Game event
    let mut ws_client = match LcuWebsocketClient::connect().await {
        Ok(ws_client) => ws_client,
        Err(e) => {
            if debug_log {
                println!("unable to connect to LoL client ({e:?}) - aborting");
            }
            return;
        }
    };
    let subscription_result = ws_client
        .subscribe(JsonApiEvent("lol-end-of-game/v1/eog-stats-block".to_string()))
        .await;
    if let Err(e) = subscription_result {
        if debug_log {
            println!("unable to subscribe to LoL client post game stats ({e:?}) - aborting");
        }
        return;
    }

    if debug_log {
        println!("waiting for post game stats");
    }

    tokio::select! {
        _ = cancel_subtoken.cancelled() => {
            if debug_log {
                println!("canceled waiting for post game stats");
            }
        }
        event = ws_client.next() => {
            if let Some(mut event) = event {
                if debug_log {
                    println!("EOG stats: {:?}", event.data);
                }

                let json_stats = event.data["localPlayer"]["stats"].take();

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
                    Ok(stats) => {
                        if debug_log {
                            println!("collected post game stats successfully");
                        }
                        game_data.stats = stats;
                    }
                    Err(e) => {
                        if debug_log {
                            println!("Error deserializing end of game stats: {:?}", e)
                        }
                    }
                }
            } else if debug_log {
                println!("LCU event listener timed out");
            }
        }
    }

    async_runtime::spawn_blocking(move || {
        if debug_log {
            println!("writing game metadata to file: {outfile:?}");
        }

        // serde_json requires a std::fs::File
        if let Ok(file) = std::fs::File::create(&outfile) {
            let result = serde_json::to_writer(file, &game_data);
            if debug_log {
                println!("metadata saved: {result:?}");
            }
        }
    });
}

#[cfg(target_os = "windows")]
fn get_lol_window() -> Option<HWND> {
    let mut window_title = WINDOW_TITLE.to_owned();
    window_title.push('\0'); // null terminate
    let mut window_class = WINDOW_CLASS.to_owned();
    window_class.push('\0'); // null terminate

    let title = PCSTR(window_title.as_ptr());
    let class = PCSTR(window_class.as_ptr());

    let hwnd = unsafe { FindWindowA(class, title) };
    if hwnd.0 == 0 {
        None
    } else {
        Some(hwnd)
    }
}

#[cfg(target_os = "windows")]
fn get_window_size(hwnd: HWND) -> Result<Size, ()> {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect as _) }.map_err(|_| ())?;
    if rect.right > 0 && rect.bottom > 0 {
        Ok(Size::new(rect.right as u32, rect.bottom as u32))
    } else {
        Err(())
    }
}
