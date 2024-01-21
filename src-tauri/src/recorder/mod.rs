use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    sync::mpsc::{channel, RecvTimeoutError},
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
use tokio::time::{interval, timeout};
use tokio_util::sync::CancellationToken;
use windows::Win32::UI::HiDpi::{
    GetAwarenessFromDpiAwarenessContext, GetDpiFromDpiAwarenessContext, GetThreadDpiAwarenessContext,
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
};
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

const WINDOW_TITLE: &str = "League of Legends (TM) Client";
const WINDOW_CLASS: &str = "RiotWindowClass";
const WINDOW_PROCESS: &str = "League of Legends.exe";

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
    aspect_ratios.sort_by(|(_, ratio1), (_, ratio2)| ratio1.partial_cmp(ratio2).unwrap_or(Ordering::Equal));
    aspect_ratios.first().unwrap().0
}

pub fn start(app_handle: &AppHandle) {
    let app_handle = app_handle.clone();

    // send stop to channel on "shutdown" event
    let (tx, rx) = channel::<_>();
    app_handle.once_global("shutdown_recorder", move |_| _ = tx.send(()));

    thread::spawn(move || {
        #[cfg(target_os = "windows")]
        unsafe {
            // Get correct window size from get_lol_window() / GetClientRect
            let result = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE);
            let dpi_awareness_context = GetThreadDpiAwarenessContext();
            log::info!(
                "SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE): {:?} | {:?} | {:?} | {:?}",
                result,
                dpi_awareness_context,
                GetAwarenessFromDpiAwarenessContext(dpi_awareness_context),
                GetDpiFromDpiAwarenessContext(dpi_awareness_context),
            )
        };

        enum State {
            Idle,
            Recording((JoinHandle<()>, CancellationToken)),
        }

        let mut state = State::Idle;

        'recorder: loop {
            match state {
                State::Idle => 'inner: {
                    // --- initialize recorder if LoL window is found ---
                    let Some(window_handle) = get_lol_window() else {
                        break 'inner;
                    };

                    log::info!("LoL Window found");

                    let Ok(window_size) = get_window_size(window_handle) else {
                        log::error!("unable to get window size of League of Legends.exe");
                        break 'inner;
                    };

                    let settings_state = app_handle.state::<Settings>();

                    // either get the explicitly set resolution or choose the default resolution for the LoL window aspect ratio
                    let output_resolution = settings_state
                        .get_output_resolution()
                        .unwrap_or_else(|| closest_resolution_to_size(&window_size));

                    log::info!("Using resolution ({output_resolution:?}) for window ({window_size:?})");

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

                    // if LeagueRecord gets launched by Windows Autostart the CWD is system32 instead of the installation folder
                    // get directory to current executable so we can locate extprocess_recorder.exe
                    let exe_dir = match std::env::current_exe()
                        .ok()
                        .and_then(|exe| exe.parent().map(|a| a.to_path_buf()))
                    {
                        Some(exe_dir) => {
                            log::info!("executable directory: {:?}", exe_dir);
                            exe_dir
                        }
                        None => {
                            log::warn!("unable to get executable directory - trying relative path instead");
                            PathBuf::from("./")
                        }
                    };
                    let mut recorder = match Recorder::new_with_paths(
                        Some(exe_dir.join(Path::new("libobs/extprocess_recorder.exe")).as_path()),
                        None,
                        None,
                        None,
                        settings_state.debug_log(),
                    ) {
                        Ok(rec) => rec,
                        Err(e) => {
                            log::error!("failed to create recorder: {e}");
                            break 'inner;
                        }
                    };

                    let configured = recorder.configure(&settings);
                    log::info!("recorder configured: {configured:?}");
                    log::info!("Available encoders: {:?}", recorder.available_encoders());
                    log::info!("Selected encoder: {:?}", recorder.selected_encoder());
                    if configured.is_err() {
                        break 'inner;
                    }

                    // --- ingame data collection ---
                    let cancel_token = CancellationToken::new();
                    let handle = async_runtime::spawn({
                        // preparation for task
                        let app_handle = app_handle.clone();
                        let cancel_subtoken = cancel_token.child_token();
                        let mut outfile = settings_state.get_recordings_path().join(filename_path);
                        outfile.set_extension("json");

                        // actual task
                        async move { collect_ingame_data(app_handle, cancel_subtoken, recorder, outfile).await }
                    });
                    log::info!("ingame task spawned: {handle:?}");

                    state = State::Recording((handle, cancel_token));
                }
                State::Recording((mut handle, cancel_token)) => 'inner: {
                    // don't stop while LoL window is open
                    if get_lol_window().is_some() {
                        state = State::Recording((handle, cancel_token));
                        break 'inner;
                    } else {
                        state = State::Idle;
                    }

                    // if task already finished -> done
                    if handle.inner().is_finished() {
                        break 'inner;
                    }

                    // spawn async thread to cleanup the gamedata task
                    async_runtime::spawn(async move {
                        // wait for 90s for EOG lobby before trying to cancel the task
                        if timeout(Duration::from_secs(90), &mut handle).await.is_ok() {
                            return;
                        } else {
                            cancel_token.cancel();
                        }

                        // abort task if the cancel_token didn't stop the it (after 5s)
                        if timeout(Duration::from_secs(5), &mut handle).await.is_err() {
                            handle.abort();
                        }
                    });
                }
            }

            // break if stop event received or sender disconnected
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(_) | Err(RecvTimeoutError::Disconnected) => {
                    // cleanup the gamedata task if it doesn't exit by itself
                    if let State::Recording((handle, cancel_token)) = state {
                        if !handle.inner().is_finished() {
                            cancel_token.cancel();

                            // give the task a little bit of time to complete a fs::write or sth
                            std::thread::sleep(Duration::from_millis(500));

                            // if it is still not finished -> abort
                            if !handle.inner().is_finished() {
                                handle.abort();
                            }
                        }
                    }

                    break 'recorder;
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }

        app_handle.trigger_global("recorder_shutdown", None);
        log::info!("recorder shutdown");
    });
}

async fn collect_ingame_data(
    app_handle: AppHandle,
    cancel_subtoken: CancellationToken,
    mut recorder: Recorder,
    outfile: PathBuf,
) {
    // IngameClient::new() never actually returns Err()
    let ingame_client = IngameClient::new().unwrap();

    log::info!("waiting for game to start");

    // wait for game start
    let mut timer = interval(Duration::from_millis(500));
    while !ingame_client.active_game().await {
        // "sleep" by selecting either the next timer tick or the token cancel
        tokio::select! {
            _ = cancel_subtoken.cancelled() => {
                let shutdown = recorder.shutdown();
                log::info!("recorder shutdown: {shutdown:?}");
                return;
            }
            _ = timer.tick() => {}
        }
    }

    // don't record spectator games
    if let Ok(true) = ingame_client.is_spectator_mode().await {
        log::info!("spectator game detected - aborting");
        let shutdown = recorder.shutdown();
        log::info!("recorder shutdown: {shutdown:?}");
        return;
    } else {
        log::info!("game started")
    }

    let mut game_data = data::GameData::default();
    if let Ok(data) = ingame_client.all_game_data(None).await {
        game_data.game_info.game_mode = data.game_data.game_mode.to_string();

        if let Some(active_player) = data.active_player {
            // the summoner name in ActivePlayer now has the format <name>#<tag>
            // but the summoner_name in the all-player list is just the <name> part
            game_data.game_info.summoner_name = active_player
                .summoner_name
                .rsplit_once('#')
                .unwrap_or_default()
                .0
                .to_owned();
        }

        let champion_name = data.all_players.into_iter().find_map(|p| {
            if p.summoner_name == game_data.game_info.summoner_name {
                Some(p.champion_name)
            } else {
                None
            }
        });
        if let Some(champion_name) = champion_name {
            game_data.game_info.champion_name = champion_name;
        } else {
            log::error!(
                "no champion for player with name {} found",
                game_data.game_info.summoner_name
            );
        }
    }

    log::info!("initial data parsed: {game_data:?}");

    // if initial game_data is successful => start recording
    let start_recording = recorder.start_recording();
    log::info!("start recording: {start_recording:?}");

    if start_recording.is_err() {
        // if recording start failed stop recording just in case and retry next 'recorder loop
        let stop_recording = recorder.stop_recording();
        let shutdown = recorder.shutdown();
        log::error!("recording start failed - stop recording: {stop_recording:?}");
        log::info!("recorder shutdown: {shutdown:?}");
        set_recording_tray_item(&app_handle, false);
        return;
    }

    let recording_start = Instant::now();
    set_recording_tray_item(&app_handle, true);

    let mut ws_client = subscribe_to_postgame_stats().await;

    log::info!("Starting EventStream - listening to ingame events");

    let mut ingame_events = EventStream::from_ingame_client(ingame_client, None);
    while let Some(event) =
        tokio::select! { event = ingame_events.next() => event, _ = cancel_subtoken.cancelled() => None }
    {
        let time = recording_start.elapsed().as_secs_f64();
        log::info!("[{}]: {:?}", time, event);

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
            GameEvent::HordeKill(_) => Some("Voidgrub"),
            GameEvent::HeraldKill(_) => Some("Herald"),
            GameEvent::InhibKilled(_) => Some("Inhibitor"),
            GameEvent::TurretKilled(_) => Some("Turret"),
            _ => None,
        };

        if let Some(name) = event_name {
            game_data.events.push(data::GameEvent { name, time })
        }
    }

    log::info!("Ingame window has closed");

    let stopped = recorder.stop_recording();
    let shutdown = recorder.shutdown();
    log::info!("recorder shutdown: {shutdown:?}");
    log::info!("recorder stopped: {stopped:?}");
    set_recording_tray_item(&app_handle, false);

    log::info!("waiting for post game stats");

    // after the game has ended retry connecting to LeagueClient for 10s
    // (if not already connected) in case the LeagueClient gets closed during the game
    for _ in 0..10 {
        if ws_client.is_some() {
            break;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
        ws_client = subscribe_to_postgame_stats().await;
    }

    if let Some(mut ws_client) = ws_client {
        tokio::select! {
            _ = cancel_subtoken.cancelled() => log::info!("canceled waiting for post game stats"),
            event = ws_client.next() => {
                if let Some(mut event) = event {
                    log::info!("EOG stats: {:?}", event.data);

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
                            game_data.stats = stats;
                            log::info!("collected post game stats successfully");
                        }
                        Err(e) => log::warn!("Error deserializing end of game stats: {e:?}"),
                    }
                } else {
                    log::warn!("LCU event listener timed out");
                }
            }
        }
    }

    async_runtime::spawn_blocking(move || {
        log::info!("writing game metadata to file: {outfile:?}");

        // serde_json requires a std::fs::File
        if let Ok(file) = std::fs::File::create(&outfile) {
            let result = serde_json::to_writer(file, &game_data);
            log::info!("metadata saved: {result:?}");
        }
    });
}

async fn subscribe_to_postgame_stats() -> Option<LcuWebsocketClient> {
    // prepare LcuWebsocketClient subscription for post game stats
    // if we do this after the ingame window closes we could technically miss the event
    match LcuWebsocketClient::connect().await {
        Ok(mut ws_client) => {
            let subscription_result = ws_client
                .subscribe(JsonApiEvent("lol-end-of-game/v1/eog-stats-block".to_string()))
                .await;
            if let Err(e) = subscription_result {
                log::warn!("unable to subscribe to LoL client post game stats ({e:?})");
            }
            Some(ws_client)
        }
        Err(e) => {
            log::warn!("unable to connect to LoL client ({e:?})");
            None
        }
    }
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
