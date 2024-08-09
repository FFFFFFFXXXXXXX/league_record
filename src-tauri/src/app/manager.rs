use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, Result};
use log::LevelFilter;
use semver::Version;
use tauri::{async_runtime, AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_updater::UpdaterExt;

use super::{RecordingManager, SystemTrayManager};
use crate::constants::{APP_NAME, CURRENT_VERSION};
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
        let config_folder = self.path().app_config_dir().context("Error getting app directory")?;

        let settings = self.initialize_settings(&config_folder)?;

        let debug_log = settings.debug_log();
        if debug_log {
            self.add_log_plugin()?;
        }

        log::info!("{APP_NAME} v{CURRENT_VERSION}");
        log::info!("{}", chrono::Local::now().format("%d-%m-%Y %H:%M"));
        log::info!("debug_log: {}", if debug_log { "enabled" } else { "disabled" });
        log::info!("Settings: {}", settings.inner());

        // create system tray-icon
        self.init_tray_menu();
        if settings.check_for_updates_enabled() {
            self.check_for_update(|app_handle| app_handle.set_tray_menu_update_available(true));
        }

        // check if app was updated
        let version_file = config_folder.join(".version");
        match fs::read_to_string(&version_file).map(|v| Version::parse(&v)) {
            // if we successfully read and parsed the version, we compare it to the version of this binary
            // if the version is smaller the app was just updated => show dialog
            Ok(Ok(version)) => {
                if version < Version::parse(CURRENT_VERSION)? {
                    self.dialog()
                        .message(format!("Successfully installed {APP_NAME} v{CURRENT_VERSION}"))
                        .title(format!("{APP_NAME} update successful!"))
                        .blocking_show();
                    _ = fs::write(&version_file, CURRENT_VERSION);
                }
            }
            // if the version can't be parsed overwrite it with a valid version
            Ok(Err(_)) => _ = fs::write(&version_file, CURRENT_VERSION),
            // if the version file is missing create one
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    _ = fs::write(&version_file, CURRENT_VERSION);
                }
            }
        }

        // make sure the system autostart setting for the app matches what is set in the settings
        self.sync_autostart();

        // start watching recordings folder for changes
        let recordings_path = settings.get_recordings_path();
        log::info!("recordings folder: {:?}", recordings_path);
        filewatcher::replace(self, &recordings_path);

        // start checking for LoL games to record
        self.manage(LeagueRecorder::new(self.clone()));

        // cleanup recordings if they are too old or the total size of the recordings gets too big
        // this only happens if 'maxRecordingAge' or 'maxRecordingsSize' is configured in the settings
        async_runtime::spawn_blocking({
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
        settings.load_from_file(&settings_file, self);

        self.manage::<SettingsWrapper>(settings);
        self.manage::<SettingsFile>(SettingsFile::new(settings_file));

        Ok(self.state::<SettingsWrapper>())
    }

    fn check_for_update(&self, callback: impl FnOnce(AppHandle) + Send + 'static) {
        let app_handle = self.clone();
        async_runtime::spawn(async move {
            let update_available = match app_handle.updater().unwrap().check().await {
                Ok(Some(_)) => true,
                Ok(None) => false,
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
        let update_check = match async_runtime::block_on(self.updater().unwrap().check()) {
            Ok(Some(update_check)) => update_check,
            Ok(None) => {
                log::warn!("no update available");
                return;
            }
            Err(e) => {
                log::error!("update check failed: {e}");
                return;
            }
        };

        if !self
            .dialog()
            .message(format!(
                "Version {} available!\n\n{}\n\nDo you want to update now?",
                &update_check.version,
                update_check.body.as_deref().unwrap_or_default()
            ))
            .title(format!("{APP_NAME} update available!"))
            .ok_button_label("Yes")
            .cancel_button_label("No")
            .blocking_show()
        {
            return;
        }

        if let Err(e) = async_runtime::block_on(update_check.download_and_install(|_, _| {}, || {})) {
            self.dialog()
                .message("Failed to download and install update, please try again later!")
                .title("Update failed!")
                .blocking_show();
            log::error!("failed to download and install the update: {e}");
        } else {
            // "On macOS and Linux you will need to restart the app manually"
            // https://tauri.app/v1/guides/distribution/updater/
            self.restart();
        }
    }

    fn add_log_plugin(&self) -> Result<()> {
        let file_name = Some(format!("{}", chrono::Local::now().format("%Y-%m-%d_%H-%M")));
        let plugin = tauri_plugin_log::Builder::default()
            .targets([
                Target::new(TargetKind::LogDir { file_name }),
                Target::new(TargetKind::Stdout),
            ])
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
