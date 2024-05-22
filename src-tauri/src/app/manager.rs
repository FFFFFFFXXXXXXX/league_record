use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, Result};
use log::LevelFilter;
use tauri::api::path::app_config_dir;
use tauri::api::{dialog, version};
use tauri::{async_runtime, AppHandle, Manager};
use tauri_plugin_log::LogTarget;

use super::{RecordingManager, SystemTrayManager, WindowManager};
use crate::constants::{window, APP_NAME, CURRENT_VERSION};
use crate::state::{SettingsFile, SettingsWrapper};
use crate::{filewatcher, recorder::LeagueRecorder};

pub trait AppManager {
    const SETTINGS_FILE: &'static str;

    fn setup(&self) -> Result<()>;

    fn initialize_settings(&self, config_folder: &Path) -> Result<tauri::State<SettingsWrapper>>;

    fn check_for_update(&self, callback: impl FnOnce(AppHandle) + Send + 'static);
    fn update(&self);

    fn add_log_plugin(&self) -> Result<()>;
    fn remove_log_plugin(&self);

    fn sync_autostart(&self);
}

impl AppManager for AppHandle {
    const SETTINGS_FILE: &'static str = "settings.json";

    fn setup(&self) -> Result<()> {
        let config_folder = app_config_dir(&self.config()).context("Error getting app directory")?;

        let settings = self.initialize_settings(&config_folder)?;

        let debug_log = settings.debug_log();
        if debug_log {
            self.add_log_plugin()?;
        }

        log::info!("{APP_NAME} v{CURRENT_VERSION}");
        log::info!("{}", chrono::Local::now().format("%d-%m-%Y %H:%M"));
        log::info!("debug_log: {}", if debug_log { "enabled" } else { "disabled" });
        log::info!("Settings: {}", settings.inner());

        // set default tray menu
        self.set_system_tray(false);

        if settings.check_for_updates_enabled() {
            self.check_for_update(|app_handle| app_handle.set_system_tray(true));
        }

        // check if app was updated
        let version_file = config_folder.join(".version");
        match fs::read_to_string(&version_file) {
            Ok(version) => {
                if version::is_greater(&version, CURRENT_VERSION).is_ok_and(|yes| yes) {
                    dialog::message(
                        self.get_window(window::MAIN).as_ref(),
                        format!("{APP_NAME} update successful!"),
                        format!("Successfully installed {APP_NAME} v{CURRENT_VERSION}"),
                    );
                    _ = fs::write(&version_file, CURRENT_VERSION);
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    _ = fs::write(&version_file, CURRENT_VERSION);
                }
            }
        }

        // make sure the system autostart setting for the app matches what is set in the settings
        self.sync_autostart();

        // don't show window on startup and set initial window state
        if let Some(window) = self.get_window(window::MAIN) {
            self.save_window_state(&window);
            _ = window.close();
        }

        // start watching recordings folder for changes
        let recordings_path = settings.get_recordings_path();
        log::info!("recordings folder: {:?}", recordings_path);
        filewatcher::replace(self, &recordings_path);

        // start checking for LoL games to record
        self.manage(LeagueRecorder::new(self.clone()));

        // cleanup recordings if they are too old or the total size of the recordings gets too big
        // this only happens if 'maxRecordingAge' or 'maxRecordingsSize' is configured in the settings
        tauri::async_runtime::spawn_blocking({
            let app_handle = self.clone();
            move || app_handle.cleanup_recordings()
        });

        Ok(())
    }

    fn initialize_settings(&self, config_folder: &Path) -> Result<tauri::State<SettingsWrapper>> {
        let settings_file = config_folder.join(Self::SETTINGS_FILE);
        // create settings.json file if missing
        SettingsWrapper::ensure_settings_exist(&settings_file);

        let settings = SettingsWrapper::new_from_file(&settings_file)?;
        settings.load_from_file(&settings_file);

        self.manage::<SettingsWrapper>(settings);
        self.manage::<SettingsFile>(SettingsFile::new(settings_file));

        Ok(self.state::<SettingsWrapper>())
    }

    fn check_for_update(&self, callback: impl FnOnce(AppHandle) + Send + 'static) {
        let app_handle = self.clone();
        async_runtime::spawn(async move {
            let update_available = match app_handle.updater().check().await {
                Ok(update_check) => update_check.is_update_available(),
                Err(e) => {
                    log::error!("update check failed: {e}");
                    false
                }
            };

            if update_available {
                callback(app_handle);
            }
        });
    }

    fn update(&self) {
        let update_check = match async_runtime::block_on(self.updater().check()) {
            Ok(update_check) => update_check,
            Err(e) => {
                log::error!("update check failed: {e}");
                return;
            }
        };

        let parent_window = self.get_window(window::MAIN);

        if !dialog::blocking::ask(
            parent_window.as_ref(),
            format!("{APP_NAME} update available!"),
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
            self.restart();
        }
    }

    fn add_log_plugin(&self) -> Result<()> {
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

        Ok(self.plugin(plugin)?)
    }

    fn remove_log_plugin(&self) {
        // the name the tauri log plugin registers itself with is currently "log"
        // maybe this will change in the future?
        self.remove_plugin("log");
    }

    fn sync_autostart(&self) {
        use tauri_plugin_autostart::ManagerExt;

        let settings = self.state::<SettingsWrapper>();
        let autostart_manager = self.autolaunch();

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
}
