use std::{error::Error, process::Command, thread, time::Duration};

use tauri::{
    api::{
        path::{app_config_dir, video_dir},
        shell,
    },
    App, AppHandle, Manager, RunEvent, SystemTray, SystemTrayEvent, WindowEvent, Wry,
};

use crate::{
    fileserver, filewatcher,
    helpers::{
        add_log_plugin, check_updates, create_tray_menu, create_window, ensure_settings_exist, remove_log_plugin,
        save_window_state, sync_autostart,
    },
    recorder,
    state::{FileWatcher, Settings, SettingsFile},
    AssetPort,
};

pub fn create_system_tray() -> SystemTray {
    SystemTray::new().with_menu(create_tray_menu())
}

pub fn system_tray_event_handler(app_handle: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::DoubleClick { .. } => {
            create_window(app_handle);
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "settings" => {
                // spawn a separate thread to avoid blocking the main thread with .status()
                thread::spawn({
                    let app_handle = app_handle.clone();
                    move || {
                        let path = app_handle.state::<SettingsFile>().get();

                        if ensure_settings_exist(&path) {
                            let settings = app_handle.state::<Settings>();
                            let old_recordings_path = settings.get_recordings_path();
                            let old_marker_flags = settings.get_marker_flags();
                            let old_log = settings.debug_log();

                            // hardcode 'notepad' since league_record currently only works on windows anyways
                            Command::new("notepad")
                                .arg(&path)
                                .status()
                                .expect("failed to start text editor");

                            // reload settings from settings.json
                            settings.load_from_file(&path);
                            log::info!("Settings updated: {:?}", settings.inner());

                            // check and update autostart if necessary
                            sync_autostart(&app_handle);

                            // add / remove logs plugin if needed
                            if old_log != settings.debug_log() {
                                if settings.debug_log() {
                                    if add_log_plugin(&app_handle).is_err() {
                                        // retry
                                        remove_log_plugin(&app_handle);
                                        _ = add_log_plugin(&app_handle);
                                    }
                                } else {
                                    remove_log_plugin(&app_handle);
                                }
                            }

                            // check if fileserver needs to be restarted
                            let marker_flags = settings.get_marker_flags();
                            let recordings_path = settings.get_recordings_path();
                            let marker_flags_changed = marker_flags != old_marker_flags;
                            let recordings_path_changed = recordings_path != old_recordings_path;

                            if recordings_path_changed {
                                // send stop fileserver signal
                                app_handle.trigger_global("shutdown_fileserver", None);
                                // when fileserver stopped restart with new folder on the same port as before
                                app_handle.once_global("fileserver_shutdown", {
                                    let app_handle = app_handle.clone();
                                    move |_| {
                                        let port = app_handle.state::<AssetPort>().get();
                                        fileserver::start(&app_handle, recordings_path, port);
                                    }
                                });
                            }

                            // check if UI window needs to be updated
                            if marker_flags_changed || recordings_path_changed {
                                _ = app_handle.emit_all("reload_ui", ());
                            }
                        }
                    }
                });
            }
            "open" => create_window(app_handle),
            "quit" => {
                // close UI window
                if let Some(main) = app_handle.windows().get("main") {
                    _ = main.close();
                }

                // stop filewatcher
                app_handle.state::<FileWatcher>().drop();

                // setup event listeners
                let (tx1, rx1) = tokio::sync::oneshot::channel::<()>();
                let (tx2, rx2) = tokio::sync::oneshot::channel::<()>();
                app_handle.once_global("fileserver_shutdown", |_| _ = tx1.send(()));
                app_handle.once_global("recorder_shutdown", |_| _ = tx2.send(()));

                // await shutdown of fileserver and recorder modules or timeout
                tauri::async_runtime::spawn({
                    let app_handle = app_handle.clone();
                    async move {
                        let timeout = Duration::from_secs(3);
                        let (a, b) =
                            tokio::join!(tokio::time::timeout(timeout, rx1), tokio::time::timeout(timeout, rx2));
                        if a.is_err() || b.is_err() {
                            log::warn!("forcing app shutdown");
                        }
                        app_handle.exit(0);
                    }
                });

                // trigger shutdown
                app_handle.trigger_global("shutdown_fileserver", None);
                app_handle.trigger_global("shutdown_recorder", None);
            }
            "update" => {
                _ = shell::open(
                    &app_handle.shell_scope(),
                    "https://github.com/FFFFFFFXXXXXXX/league_record/releases/latest",
                    None,
                );
            }
            _ => {}
        },
        _ => {}
    }
}

pub fn setup_handler(app: &mut App<Wry>) -> Result<(), Box<dyn Error>> {
    let app_handle = app.app_handle();

    // get path to config directory
    let config_path = app_config_dir(app_handle.config().as_ref()).expect("Error getting app directory");

    let settings_path = config_path.join("settings.json");
    let settings = app_handle.state::<Settings>();
    // create settings.json file if missing
    ensure_settings_exist(&settings_path);
    // load settings and set state
    settings.load_from_file(&settings_path);
    app_handle.state::<SettingsFile>().set(settings_path);

    let debug_log = settings.debug_log();

    if debug_log {
        add_log_plugin(&app_handle)?;
    }

    log::info!("LeagueRecord v{}", env!("CARGO_PKG_VERSION"));
    log::info!("{}", chrono::Local::now().format("%d-%m-%Y %H:%M"));
    log::info!("debug_log: {}", if debug_log { "enabled" } else { "disabled" });

    if settings.check_for_updates() {
        check_updates(&app_handle);
    }

    log::info!("Settings: {:?}", settings.inner());

    sync_autostart(&app_handle);

    // only start app if video directory exists
    if video_dir().is_none() {
        log::error!("Error: No video folder available");
        app_handle.exit(-1);
    }

    // don't show window on startup and set initial window state
    if let Some(window) = app_handle.get_window("main") {
        save_window_state(&app_handle, &window);
        _ = window.close();
    }

    let recordings_path = settings.get_recordings_path();
    let port = app_handle.state::<AssetPort>().get();
    log::info!("video folder: {:?}", recordings_path);
    log::info!("fileserver port: {}", port);

    filewatcher::replace_filewatcher(&app_handle, &recordings_path);
    // launch static-file-server as a replacement for the broken asset protocol
    fileserver::start(&app_handle, recordings_path, port);
    recorder::start(&app_handle);
    Ok(())
}

pub fn run_handler(app_handle: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::WindowEvent {
            label: _,
            event: WindowEvent::CloseRequested { api: _, .. },
            ..
        } => {
            // triggered on window close (X Button)
            if let Some(window) = app_handle.get_window("main") {
                save_window_state(app_handle, &window);
            }
        }
        RunEvent::ExitRequested { api, .. } => {
            // triggered when no windows remain
            // prevent complete shutdown of program so that just the tray icon stays
            api.prevent_exit();
        }
        _ => {}
    }
}
