use port_check::free_local_port_in_range;
use serde::{Deserialize, Serialize};
use tauri::api::path::video_dir;

use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, RwLock},
};

pub struct WindowState {
    pub size: Mutex<(f64, f64)>,
    pub position: Mutex<(f64, f64)>,
}
impl WindowState {
    pub fn init() -> Self {
        Self {
            size: Mutex::from((1200.0, 650.0)),
            position: Mutex::from((-1.0, -1.0)),
        }
    }
}

pub struct AssetPort(u16);
impl AssetPort {
    pub fn init() -> Self {
        // dont accidentally block port 2999 which the LoL ingame API uses
        // use a "ephemeral"/"dynamic" port for temporary applications
        Self(free_local_port_in_range(49152, 65535).expect("no free port available"))
    }
    pub fn get(&self) -> u16 {
        self.0
    }
}

#[derive(Default, Debug)]
pub struct SettingsFile(RwLock<Option<PathBuf>>);

impl SettingsFile {
    pub fn get(&self) -> PathBuf {
        self.0.read().unwrap().to_owned().unwrap()
    }

    pub fn set(&self, folder: PathBuf) {
        *self.0.write().unwrap() = Some(folder);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarkerFlags {
    #[serde(default = "default_true")]
    kill: bool,
    #[serde(default = "default_true")]
    death: bool,
    #[serde(default = "default_true")]
    assist: bool,
    #[serde(default = "default_true")]
    turret: bool,
    #[serde(default = "default_true")]
    inhibitor: bool,
    #[serde(default = "default_true")]
    dragon: bool,
    #[serde(default = "default_true")]
    herald: bool,
    #[serde(default = "default_true")]
    baron: bool,
}

impl Default for MarkerFlags {
    fn default() -> Self {
        MarkerFlags {
            kill: true,
            death: true,
            assist: true,
            turret: true,
            inhibitor: true,
            dragon: true,
            herald: true,
            baron: true,
        }
    }
}

#[derive(Debug)]
pub struct Settings(RwLock<SettingsInner>);

impl Settings {
    pub fn load_from_file(&self, settings_path: &PathBuf) {
        let Ok(json) = fs::read_to_string(settings_path) else { return };
        let mut settings = serde_json::from_str::<SettingsInner>(json.as_str())
            .expect("Deserialization to 'Config' should always succeed since every field has a 'default'");

        let mut video_dir = video_dir().expect("video_dir doesn't exist");
        // if recordings_folder is absolute the whole path gets replaced by the absolute path
        // if recordings_folder is relative the path gets appened to the system video directory
        video_dir.push(settings.config.recordings_folder);
        settings.config.recordings_folder = video_dir;
        if fs::create_dir_all(settings.config.recordings_folder.as_path()).is_err() && settings.debug_log {
            println!("Unable to create recordings_folder");
        }

        *self.0.write().unwrap() = settings;
    }

    pub fn write_to_file(&self, settings_path: &PathBuf) {
        let json = serde_json::to_string_pretty(&*self.0.read().unwrap()).unwrap();
        let _ = fs::write(settings_path, json);
    }

    pub fn get_recordings_path(&self) -> PathBuf {
        self.0.read().unwrap().config.recordings_folder.clone()
    }

    pub fn check_for_updates(&self) -> bool {
        self.0.read().unwrap().check_for_updates
    }

    pub fn get_marker_flags(&self) -> MarkerFlags {
        self.0.read().unwrap().marker_flags.clone()
    }

    pub fn set_marker_flags(&self, marker_flags: MarkerFlags) {
        self.0.write().unwrap().marker_flags = marker_flags;
    }

    pub fn debug_log(&self) -> bool {
        let debug = std::env::args().find(|e| e == "-d" || e == "--debug");
        self.0.read().unwrap().debug_log || debug.is_some()
    }

    pub fn create_lol_rec_cfg(&self, window_size: (u32, u32)) -> String {
        let mut s = self.0.write().unwrap();
        s.config.window_size = common::Size::new(window_size.0, window_size.1);
        let mut cfg = serde_json::to_string(&s.config).expect("error serializing lol_rec config");
        cfg.push('\n'); // add newline as something like a termination sequence
        cfg
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self(RwLock::from(SettingsInner::default()))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInner {
    // only used in the tauri application
    #[serde(default)]
    marker_flags: MarkerFlags,
    #[serde(default = "default_true")]
    check_for_updates: bool,
    #[serde(default)]
    pub debug_log: bool,
    // these get passed to lol_rec
    #[serde(flatten)]
    #[serde(default)]
    pub config: common::Config,
}

impl Default for SettingsInner {
    fn default() -> Self {
        Self {
            check_for_updates: true,
            marker_flags: MarkerFlags::default(),
            debug_log: false,
            config: common::Config::default(),
        }
    }
}

fn default_true() -> bool {
    true
}
