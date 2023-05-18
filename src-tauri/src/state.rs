use port_check::free_local_port_in_range;
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::api::path::video_dir;

use std::{
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

pub struct MarkerFlagsState(pub Mutex<Option<MarkerFlags>>);
impl MarkerFlagsState {
    pub fn init() -> MarkerFlagsState {
        Self(Mutex::new(None))
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

#[derive(Deserialize, Debug, Clone)]
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
impl MarkerFlags {
    pub fn to_json_value(&self) -> Value {
        json!({
            "kill": self.kill,
            "death": self.death,
            "assist": self.assist,
            "turret": self.turret,
            "inhibitor": self.inhibitor,
            "dragon": self.dragon,
            "herald": self.herald,
            "baron": self.baron
        })
    }
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
    pub fn load_settings_file(&self, settings_path: &PathBuf) {
        if let Some(config) = common::Config::new(settings_path, video_dir().expect("video_dir doesn't exist")) {
            self.0.write().unwrap().config = config;
        }
    }

    pub fn recordings_folder(&self) -> PathBuf {
        self.0.read().unwrap().config.recordings_folder.clone()
    }
    pub fn check_for_updates(&self) -> bool {
        self.0.read().unwrap().check_for_updates
    }
    pub fn marker_flags(&self) -> Value {
        self.0.read().unwrap().marker_flags.to_json_value()
    }
    pub fn debug_log(&self) -> bool {
        self.0.read().unwrap().config.debug_log
    }

    pub fn create_lol_rec_cfg(&self, window_size: (u32, u32)) -> Result<String, ()> {
        let mut s = self.0.write().unwrap();
        s.config.window_size = common::Size::new(window_size.0, window_size.1);
        let mut cfg = serde_json::to_string(&s.config).map_err(|_| ())?;
        cfg.push('\n'); // add newline as something like a termination sequence
        Ok(cfg)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self(RwLock::from(SettingsInner::default()))
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInner {
    // only used in the tauri application
    marker_flags: MarkerFlags,
    #[serde(default = "default_true")]
    check_for_updates: bool,
    // these get passed to lol_rec
    #[serde(flatten)]
    pub config: common::Config,
}

impl Default for SettingsInner {
    fn default() -> Self {
        Self {
            marker_flags: MarkerFlags::default(),
            check_for_updates: true,
            config: common::Config::default(),
        }
    }
}

fn default_true() -> bool {
    true
}
