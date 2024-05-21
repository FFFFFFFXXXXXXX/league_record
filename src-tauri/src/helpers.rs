use std::cmp::Ordering;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use anyhow::{bail, Result};
use log::LevelFilter;
use tauri::api::dialog;
use tauri::async_runtime;
use tauri::{AppHandle, CustomMenuItem, Manager, SystemTrayMenu, SystemTrayMenuItem, Window};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_log::LogTarget;

use crate::recorder::{process_data, Deferred, MetadataFile, NoData};
use crate::state::{CurrentlyRecording, SettingsFile, SettingsWrapper, WindowState};
use crate::{filewatcher, MAIN_WINDOW};

pub fn create_tray_menu() -> SystemTrayMenu {
    SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("rec", "Recording").disabled())
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("settings", "Settings"))
        .add_item(CustomMenuItem::new("open", "Open"))
        .add_item(CustomMenuItem::new("quit", "Quit"))
}

pub fn set_recording_tray_item(app_handle: &AppHandle, recording: bool) {
    let item = app_handle.tray_handle().get_item("rec");
    // set selected only updates the tray menu when open if the menu item is enabled
    _ = item.set_enabled(true);
    _ = item.set_selected(recording);
    _ = item.set_enabled(false);
}

pub fn check_for_update(app_handle: &AppHandle) {
    let app_handle = app_handle.clone();
    async_runtime::spawn(async move {
        let update_check = match app_handle.updater().check().await {
            Ok(update_check) => update_check,
            Err(e) => {
                log::error!("update check failed: {e}");
                return;
            }
        };

        if update_check.is_update_available() {
            let tray_menu = create_tray_menu()
                .add_native_item(SystemTrayMenuItem::Separator)
                .add_item(CustomMenuItem::new("update", "Update Available!"));

            if let Err(e) = app_handle.tray_handle().set_menu(tray_menu) {
                log::error!("failed to update tray with 'update available' button: {e}");
            }
        }
    });
}

pub fn update(app_handle: &AppHandle) {
    let update_check = match async_runtime::block_on(app_handle.updater().check()) {
        Ok(update_check) => update_check,
        Err(e) => {
            log::error!("update check failed: {e}");
            return;
        }
    };

    let parent_window = app_handle.get_window(MAIN_WINDOW);

    if !dialog::blocking::ask(
        parent_window.as_ref(),
        "LeagueRecord update available!",
        format!(
            "Version {} available!\n\n{}\n\nDo you want to update now?",
            update_check.latest_version(),
            update_check.body().map(String::as_str).unwrap_or_default()
        ),
    ) {
        return;
    }

    if let Err(e) = async_runtime::block_on(update_check.download_and_install()) {
        dialog::blocking::message(
            parent_window.as_ref(),
            "Update failed!",
            "Failed to download and install update!",
        );
        log::error!("failed to download and install the update: {e}");
    } else {
        // "On macOS and Linux you will need to restart the app manually"
        // https://tauri.app/v1/guides/distribution/updater/
        app_handle.restart();
    }
}

pub fn sync_autostart(app_handle: &AppHandle) {
    let settings = app_handle.state::<SettingsWrapper>();
    let autostart_manager = app_handle.autolaunch();

    match autostart_manager.is_enabled() {
        Ok(autostart_enabled) => {
            if settings.autostart() != autostart_enabled {
                let result = if settings.autostart() {
                    autostart_manager.enable()
                } else {
                    autostart_manager.disable()
                };

                if let Err(error) = result {
                    log::warn!("failed to set autostart to {}: {error:?}", settings.autostart());
                }
            }
        }
        Err(error) => {
            log::warn!("unable to get current autostart state: {error:?}");
        }
    }
}

pub fn add_log_plugin(app_handle: &AppHandle) -> Result<()> {
    let plugin = tauri_plugin_log::Builder::default()
        .targets([LogTarget::LogDir, LogTarget::Stdout])
        .log_name(format!("{}", chrono::Local::now().format("%Y-%m-%d_%H-%M")))
        .level(LevelFilter::Info)
        .format(|out, msg, record| {
            out.finish(format_args!(
                "[{}][{}]: {}",
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                msg
            ))
        })
        .build();

    Ok(app_handle.plugin(plugin)?)
}

pub fn remove_log_plugin(app_handle: &AppHandle) {
    // the name the tauri log plugin registers itself with is currently "log"
    // maybe this will change in the future?
    app_handle.remove_plugin("log");
}

pub fn get_recordings(app_handle: &AppHandle) -> Vec<PathBuf> {
    // get all mp4 files in ~/Videos/%folder-name%
    let mut recordings = Vec::<PathBuf>::new();
    let Ok(read_dir) = app_handle.state::<SettingsWrapper>().get_recordings_path().read_dir() else {
        return vec![];
    };

    let currently_recording = app_handle.state::<CurrentlyRecording>().get();

    for entry in read_dir.flatten() {
        let path = entry.path();

        if !path.is_file() || Some(&path) == currently_recording.as_ref() {
            continue;
        }

        if let Some(ext) = path.extension() {
            if ext == "mp4" {
                recordings.push(path);
            }
        }
    }
    recordings
}

pub fn path_to_string(path: &Path) -> String {
    path.to_owned().into_os_string().into_string().expect("invalid path")
}

pub fn compare_time(a: &Path, b: &Path) -> io::Result<Ordering> {
    let a_time = a.metadata()?.created()?;
    let b_time = b.metadata()?.created()?;
    Ok(a_time.cmp(&b_time).reverse())
}

pub fn show_window(window: &Window) {
    _ = window.unminimize();
    _ = window.set_focus();
}

pub fn create_window(app_handle: &AppHandle) {
    if let Some(main) = app_handle.windows().get(MAIN_WINDOW) {
        show_window(main);
    } else {
        let window_state = app_handle.state::<WindowState>();

        let builder = Window::builder(app_handle, MAIN_WINDOW, tauri::WindowUrl::default());

        let size = window_state.get_size();
        let position = window_state.get_position();
        let window = builder
            .title("LeagueRecord")
            .inner_size(size.0, size.1)
            .position(position.0, position.1)
            .min_inner_size(800.0, 450.0)
            .visible(false);

        if let Err(e) = window.build() {
            log::error!("error creating window: {e}");
        }
    }
}

pub fn save_window_state(app_handle: &AppHandle, window: &Window) {
    let scale_factor = match window.scale_factor() {
        Ok(scale_factor) => scale_factor,
        Err(e) => {
            log::error!("Error getting window scale factor: {e}");
            return;
        }
    };

    let window_state = app_handle.state::<WindowState>();
    if let Ok(size) = window.inner_size() {
        let size = ((size.width as f64) / scale_factor, (size.height as f64) / scale_factor);
        window_state.set_size(size);
    }
    if let Ok(position) = window.outer_position() {
        let position = ((position.x as f64) / scale_factor, (position.y as f64) / scale_factor);
        window_state.set_position(position);
    }
}

pub fn ensure_settings_exist(settings_file: &Path) -> bool {
    if !settings_file.is_file() {
        // get directory of settings file
        let Some(parent) = settings_file.parent() else {
            return false;
        };
        // create the whole settings_file to the directory
        let Ok(_) = fs::create_dir_all(parent) else {
            return false;
        };
        // create the settings file with the default settings json
        let Ok(_) = fs::write(settings_file, include_str!("../default-settings.json")) else {
            return false;
        };
    }
    true
}

pub fn let_user_edit_settings(app_handle: &AppHandle) {
    let app_handle = app_handle.clone();

    // spawn a separate thread to avoid blocking the main thread with Command::status()
    async_runtime::spawn_blocking(move || {
        let settings_file = app_handle.state::<SettingsFile>();
        let settings_file = settings_file.get();

        if ensure_settings_exist(settings_file) {
            let settings = app_handle.state::<SettingsWrapper>();
            let old_marker_flags = settings.get_marker_flags();
            let old_recordings_path = settings.get_recordings_path();
            let old_log = settings.debug_log();

            // hardcode 'notepad' since league_record currently only works on windows anyways
            if let Err(e) = Command::new("notepad").arg(settings_file).status() {
                log::error!("failed to start text editor: {e}");
                return;
            }

            // reload settings from settings.json
            settings.load_from_file(settings_file);
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

            // check if UI window needs to be updated
            let recordings_path = settings.get_recordings_path();
            if recordings_path != old_recordings_path {
                filewatcher::replace(&app_handle, &recordings_path);
                if let Err(e) = app_handle.emit_all("recordings_changed", ()) {
                    log::error!("failed to emit 'recordings_changed' event: {e}");
                }
            }

            let marker_flags = settings.get_marker_flags();
            if marker_flags != old_marker_flags {
                if let Err(e) = app_handle.emit_all("markerflags_changed", ()) {
                    log::error!("failed to emit 'markerflags_changed' event: {e}");
                }
            }

            cleanup_recordings(&app_handle);
        }
    });
}

pub fn cleanup_recordings(app_handle: &AppHandle) {
    cleanup_recordings_by_age(app_handle);
    cleanup_recordings_by_size(app_handle);
}

fn cleanup_recordings_by_size(app_handle: &AppHandle) {
    let Some(max_gb) = app_handle.state::<SettingsWrapper>().max_recordings_size() else { return };
    let max_size = max_gb * 1_000_000_000; // convert to bytes

    let mut recordings = get_recordings(app_handle);
    recordings.sort_by(|a, b| compare_time(a, b).unwrap_or(Ordering::Equal));

    let mut total_size = 0;

    // add size from video thats currently being recorded to the total (in case there is one)
    // so the total size of all videos stays below the threshhold set in settings
    if let Some(currently_recording_metadata) = app_handle
        .state::<CurrentlyRecording>()
        .get()
        .and_then(|pb| pb.metadata().ok())
    {
        total_size += currently_recording_metadata.len();
    }

    // split recordings into 'favorites' and 'others' by json metadata 'favorite' value
    // in case reading the metadata fails put the recording into favorites so it doesn't get deleted
    let (favorites, others): (Vec<_>, Vec<_>) = recordings.into_iter().partition(|recording| {
        get_metadata(recording, false)
            .map(|metadata_file| metadata_file.is_favorite())
            .unwrap_or(true)
    });

    // get sum of sizes of recordings marked as favorites
    for recording in favorites {
        match recording.metadata() {
            Ok(metadata) => total_size += metadata.len(),
            Err(e) => log::warn!(
                "Failed to get size of recording (favorite) {}: {e}",
                recording.display(),
            ),
        }
    }

    for recording in others {
        match recording.metadata() {
            Ok(metadata) => total_size += metadata.len(),
            Err(e) => log::warn!("Failed to get size of recording {}: {e}", recording.display(),),
        }

        if total_size > max_size {
            if let Err(e) = delete_recording(recording) {
                log::error!("deleting file due to size limit failed: {e}");
            }
        }
    }
}

fn cleanup_recordings_by_age(app_handle: &AppHandle) {
    fn too_old(file: &Path, max_age: Duration, now: SystemTime) -> Result<bool> {
        let creation_time = file.metadata()?.created()?;
        let time_passed = now.duration_since(creation_time)?;
        Ok(time_passed > max_age)
    }

    fn is_favorite(file: &Path) -> Result<bool> {
        get_metadata(file, false).map(|metadata_file| metadata_file.is_favorite())
    }

    let Some(max_days) = app_handle.state::<SettingsWrapper>().max_recording_age() else { return };
    let max_age = Duration::from_secs(max_days * 24 * 60 * 60);
    let now = SystemTime::now();
    for recording in get_recordings(app_handle) {
        // in case checking 'too_old(...)' or 'is_favorite(...)' fails default to not deleting the file
        if too_old(&recording, max_age, now).unwrap_or(false) && !is_favorite(&recording).unwrap_or(true) {
            if let Err(e) = delete_recording(recording) {
                log::error!("deleting file due to age limit failed: {e}");
            }
        }
    }
}

pub fn delete_recording(recording: PathBuf) -> Result<()> {
    fs::remove_file(&recording)?;

    let mut metadata_file = recording;
    metadata_file.set_extension("json");
    fs::remove_file(metadata_file)?;

    Ok(())
}

pub fn get_metadata(video_path: &Path, fetch: bool) -> Result<MetadataFile> {
    let mut video_path = video_path.to_owned();
    video_path.set_extension("json");

    let filedata = if video_path.exists() && fs::metadata(&video_path)?.is_file() {
        let reader = BufReader::new(File::open(&video_path)?);
        serde_json::from_reader::<_, MetadataFile>(reader)?
    } else {
        let metadata_file = MetadataFile::NoData(NoData { favorite: false });
        save_metadata(&video_path, &metadata_file)?;
        metadata_file
    };

    match filedata {
        MetadataFile::Deferred(Deferred {
            match_id,
            ingame_time_rec_start_offset,
            favorite,
        }) => {
            if !fetch {
                bail!("deferred, no metadata");
            }

            let mut metadata = async_runtime::block_on(process_data(ingame_time_rec_start_offset, match_id))?;
            metadata.favorite = favorite;
            let metadata_file = MetadataFile::Metadata(metadata);
            if let Err(e) = save_metadata(&video_path, &metadata_file) {
                log::error!("failed to save re-processed game metadata: {e}");
            }
            Ok(metadata_file)
        }
        metadata_file => Ok(metadata_file),
    }
}

pub fn save_metadata(path: &Path, metadata_file: &MetadataFile) -> Result<()> {
    let mut path = path.to_owned();
    path.set_extension("json");

    let writer = BufWriter::new(File::create(path)?);
    Ok(serde_json::to_writer(writer, &metadata_file)?)
}

#[macro_export]
macro_rules! cancellable {
    ($function:expr, $cancel_token:expr, Option) => {
        select! {
            option = $function => option,
            _ = $cancel_token.cancelled() => None
        }
    };
    ($function:expr, $cancel_token:expr, Result) => {
        select! {
            result = $function => result.map_err(|e| anyhow::anyhow!("{e}")),
            _ = $cancel_token.cancelled() => Err(anyhow::anyhow!("cancelled"))
        }
    };
    ($function:expr, $cancel_token:expr, ()) => {
        select! {
            _ = $function => false,
            _ = $cancel_token.cancelled() => true
        }
    };
}
