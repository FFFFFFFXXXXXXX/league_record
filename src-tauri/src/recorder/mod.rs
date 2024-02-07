use std::{
    cmp::Ordering,
    path::PathBuf,
    sync::mpsc::{channel, RecvTimeoutError},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
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

use crate::{
    helpers::set_recording_tray_item,
    recorder::data::{GameInfo, Stats},
    state::SettingsWrapper,
    CurrentlyRecording,
};

use self::data::GameData;

pub mod data;

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

    std::thread::spawn(move || {
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

        '_loop: loop {
            match state {
                State::Idle => '_match: {
                    // --- initialize recorder if LoL window is found ---
                    let Some(window_handle) = get_lol_window() else {
                        break '_match;
                    };

                    log::info!("LoL Window found");

                    let (recorder, filename_path) = match setup_recorder(window_handle, &app_handle) {
                        Ok(value) => value,
                        Err(e) => {
                            log::error!("error creating recorder: {e}");
                            break '_match;
                        }
                    };

                    // make filewatcher ignore the video / metadata files while the recording is going on
                    app_handle
                        .state::<CurrentlyRecording>()
                        .set(Some(filename_path.clone()));

                    // --- ingame data collection ---
                    let cancel_token = CancellationToken::new();
                    let handle = async_runtime::spawn({
                        // preparation for task
                        let app_handle = app_handle.clone();
                        let cancel_subtoken = cancel_token.child_token();
                        let mut metadata_path = filename_path;
                        metadata_path.set_extension("json");

                        // actual task
                        async move {
                            collect_ingame_data(app_handle.clone(), cancel_subtoken, recorder, metadata_path).await;
                            // recording is done - tell filewatcher not to ignore these files
                            // do this here instead of in collect_ingame_data(...) since that function has mutliple return point
                            app_handle.state::<CurrentlyRecording>().set(None);
                        }
                    });

                    log::info!("ingame task spawned");

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

                    // spawn async thread to cleanup the gamedata task no matter what after a certain delay
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

                    break '_loop;
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }

        log::info!("recorder shutdown");
        app_handle.exit(0);
    });
}

fn setup_recorder(window_handle: HWND, app_handle: &AppHandle) -> Result<(Recorder, PathBuf)> {
    let settings_state = app_handle.state::<SettingsWrapper>();

    let window_size =
        get_window_size(window_handle).map_err(|_| anyhow!("unable to get window size of League of Legends.exe"))?;

    // either get the explicitly set resolution or choose the default resolution for the LoL window aspect ratio
    let output_resolution = settings_state
        .get_output_resolution()
        .unwrap_or_else(|| closest_resolution_to_size(&window_size));

    log::info!("Using resolution ({output_resolution:?}) for window ({window_size:?})");

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

    let mut filename = settings_state.get_filename_format();
    if !filename.ends_with(".mp4") {
        filename.push_str(".mp4");
    }
    let filename_path = settings_state
        .get_recordings_path()
        .join(format!("{}", chrono::Local::now().format(&filename)));
    settings.set_output_path(
        filename_path
            .to_str()
            .context("filename_path is not a valid UTF-8 string")?,
    );

    let mut recorder = Recorder::new_with_paths(
        app_handle
            .path_resolver()
            .resolve_resource("libobs/extprocess_recorder.exe"),
        None,
        None,
        None,
    )?;

    recorder.configure(&settings)?;
    log::info!("recorder configured");
    log::info!("Available encoders: {:?}", recorder.available_encoders());
    log::info!("Selected encoder: {:?}", recorder.selected_encoder());

    Ok((recorder, filename_path))
}

async fn collect_ingame_data(
    app_handle: AppHandle,
    cancel_subtoken: CancellationToken,
    mut recorder: Recorder,
    outfile: PathBuf,
) {
    let ingame_client = IngameClient::new();

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
        log::info!("game started");
    }

    let Ok(all_game_data) = ingame_client.all_game_data(None).await else {
        log::error!("unable to collect initial game infos - aborting");
        let shutdown = recorder.shutdown();
        log::info!("recorder shutdown: {shutdown:?}");
        return;
    };
    let game_info = {
        let game_mode = all_game_data.game_data.game_mode.to_string();

        // the summoner name in ActivePlayer now has the format <name>#<tag>
        // but the summoner_name in the all-player list is just the <name> part
        let summoner_name = all_game_data
            .active_player
            .map(|active_player| {
                active_player
                    .summoner_name
                    .rsplit_once('#')
                    .unwrap_or_default()
                    .0
                    .to_owned()
            })
            .unwrap_or_default();

        let champion_name = all_game_data.all_players.into_iter().find_map(|p| {
            if p.summoner_name == summoner_name {
                Some(p.champion_name)
            } else {
                None
            }
        });
        if champion_name.is_none() {
            log::error!("no champion for player with name {} found", summoner_name);
        }

        GameInfo {
            game_mode,
            summoner_name,
            champion_name: champion_name.unwrap_or_default(),
        }
    };

    log::info!("initial data parsed: {:?}", game_info);

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

    // if the window still exists after the ingame API has stopped responding
    // assume the ingame API is buggy and continue recording
    let mut prev_game_data = None;
    let mut game_data = loop {
        // if cancelled via the token break out of loop immediately
        let (game_data, cancelled) = process_ingame_events(&game_info, recording_start, &cancel_subtoken).await;

        // just to be sure wait for a short amout after the API has stopped responding
        // before checking if the LoL window still exists
        if !cancelled {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        if cancelled || get_lol_window().is_none() {
            // if we somehow get less information in the retry than in the previous attempt
            // return previous data instead of the new data
            if prev_game_data
                .as_ref()
                .is_some_and(|prev: &GameData| prev.events.len() > game_data.events.len())
            {
                break prev_game_data.unwrap(); // unwrap is ok since we check in .is_some_and
            } else {
                break game_data;
            }
        }

        prev_game_data = Some(game_data);
    };

    let stopped = recorder.stop_recording();
    log::info!("recorder stopped: {stopped:?}");
    let shutdown = recorder.shutdown();
    log::info!("recorder shutdown: {shutdown:?}");
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
                    log::info!("EOG stats: {:#?}", event.data);

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
                        Err(e) => log::warn!("error deserializing end of game stats: {e:?}"),
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
            let result = serde_json::to_writer(&file, &game_data);
            log::info!("metadata saved: {result:?}");

            _ = app_handle.emit_all("recordings_changed", ());
        }
    });
}

async fn process_ingame_events(
    game_data: &GameInfo,
    recording_start: Instant,
    cancel_subtoken: &CancellationToken,
) -> (GameData, bool) {
    let mut game_data = GameData {
        win: None,
        game_info: game_data.clone(),
        stats: Stats::default(),
        events: Vec::new(),
    };

    log::info!("Starting EventStream - listening to ingame events");
    let mut ingame_events = EventStream::new();
    while let Some(event) = tokio::select! {
        event = ingame_events.next() => event,
        _ = cancel_subtoken.cancelled() => {
            log::info!("task cancelled while listening for ingame API events");
            return (game_data, true);
        }
    } {
        use data::EventName;

        let time = recording_start.elapsed().as_secs_f32();
        log::info!("[{}]: {:?}", time, event);

        let event_name = match event {
            GameEvent::BaronKill(_) => Some(EventName::Baron),
            GameEvent::ChampionKill(e) => {
                let summoner_name = &game_data.game_info.summoner_name;
                match e {
                    ChampionKill {
                        killer_name: Killer::Summoner(ref killer_name),
                        ..
                    } if killer_name == summoner_name => Some(EventName::Kill),
                    ChampionKill { ref victim_name, .. } if victim_name == summoner_name => Some(EventName::Death),
                    ChampionKill { assisters, .. } if assisters.contains(summoner_name) => Some(EventName::Assist),
                    _ => None,
                }
            }
            GameEvent::DragonKill(e) => Some(match e.dragon_type {
                DragonType::Infernal => EventName::InfernalDragon,
                DragonType::Ocean => EventName::OceanDragon,
                DragonType::Mountain => EventName::MountainDragon,
                DragonType::Cloud => EventName::CloudDragon,
                DragonType::Hextech => EventName::HextechDragon,
                DragonType::Chemtech => EventName::ChemtechDragon,
                DragonType::Elder => EventName::ElderDragon,
            }),
            GameEvent::GameEnd(e) => {
                game_data.win = match e.result {
                    GameResult::Win => Some(true),
                    GameResult::Lose => Some(false),
                };
                None
            }
            GameEvent::HordeKill(_) => Some(EventName::Voidgrub),
            GameEvent::HeraldKill(_) => Some(EventName::Herald),
            GameEvent::InhibKilled(_) => Some(EventName::Inhibitor),
            GameEvent::TurretKilled(_) => Some(EventName::Turret),
            _ => None,
        };

        if let Some(name) = event_name {
            game_data.events.push(data::GameEvent { name, time })
        }
    }

    log::info!("Ingame API connection stopped");

    (game_data, false)
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
