use std::{
    fs::{create_dir_all, File},
    io::BufReader,
    path::PathBuf,
    sync::Mutex,
};

use serde::{Deserialize, Serialize};

use serde_json::{json, Value};
use tauri::api::path::video_dir;

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
        let port =
            port_check::free_local_port_in_range(1024, 65535).expect("no free port available");
        AssetPort(port)
    }
    pub fn get(&self) -> u16 {
        self.0
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_recordings_folder")]
    #[serde(deserialize_with = "deserialize_recordings_folder")]
    recordings_folder: PathBuf,
    #[serde(default = "default_filename_format")]
    filename_format: String,
    #[serde(default = "default_encoding_quality")]
    encoding_quality: u32,
    #[serde(default = "default_output_resolution")]
    output_resolution: String,
    #[serde(default = "default_framerate")]
    framerate: (u32, u32),
    #[serde(default = "default_true")]
    record_audio: bool,
    #[serde(skip_serializing)]
    marker_flags: MarkerFlags,
    #[serde(skip_serializing)]
    #[serde(default = "default_true")]
    check_for_updates: bool,
    debug_log: bool,
    // for passing to lol_rec
    #[serde(skip_deserializing)]
    window_size: (u32, u32),
}

impl Settings {
    pub fn init() -> Self {
        let mut exe_dir = std::path::PathBuf::from("./");
        if let Ok(p) = std::env::current_exe() {
            if let Ok(mut path) = p.canonicalize() {
                path.pop();
                exe_dir = path;
            }
        }
        exe_dir.push("settings");
        exe_dir.push("settings.json");
        if let Ok(file) = File::open(&exe_dir) {
            let reader = BufReader::new(file);
            if let Ok(settings) = serde_json::from_reader::<_, Settings>(reader) {
                return settings;
            }
        }

        Self {
            recordings_folder: default_recordings_folder(),
            filename_format: default_filename_format(),
            encoding_quality: default_encoding_quality(),
            output_resolution: default_output_resolution(),
            framerate: default_framerate(),
            record_audio: true,
            check_for_updates: true,
            marker_flags: MarkerFlags::default(),
            debug_log: false,
            window_size: (0, 0),
        }
    }

    pub fn recordings_folder(&self) -> PathBuf {
        self.recordings_folder.clone()
    }
    pub fn recordings_folder_as_string(&self) -> Result<String, ()> {
        self.recordings_folder
            .clone()
            .into_os_string()
            .into_string()
            .map_err(|_| ())
    }
    pub fn check_for_updates(&self) -> bool {
        self.check_for_updates
    }
    pub fn marker_flags(&self) -> Value {
        self.marker_flags.to_json_value()
    }
    pub fn debug_log(&self) -> bool {
        self.debug_log
    }

    pub fn create_lol_rec_cfg(&mut self, window_size: (u32, u32)) -> Result<String, ()> {
        self.window_size = window_size;
        let mut cfg = serde_json::to_string(&self).map_err(|_| ())?;
        cfg.push('\n'); // add newline as something like a termination sequence
        Ok(cfg)
    }
}

// (DE-)SERIALIZERS //
fn deserialize_recordings_folder<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<PathBuf, D::Error> {
    let folder_name: String = Deserialize::deserialize(deserializer)?;
    let mut recordings_folder = video_dir().expect("video_dir doesn't exist");
    recordings_folder.push(PathBuf::from(folder_name));
    if !recordings_folder.exists() {
        let _ = create_dir_all(recordings_folder.as_path());
    }
    Ok(recordings_folder)
}

// DEFAULTS //
fn default_recordings_folder() -> PathBuf {
    let mut recordings_folder = video_dir().expect("video_dir doesn't exist");
    recordings_folder.push(PathBuf::from("league_recordings"));
    if !recordings_folder.exists() {
        let _ = create_dir_all(recordings_folder.as_path());
    }
    recordings_folder
}
fn default_filename_format() -> String {
    String::from("%Y-%m-%d_%H-%M.mp4")
}
fn default_encoding_quality() -> u32 {
    20
}
fn default_output_resolution() -> String {
    String::from("1080p")
}
fn default_framerate() -> (u32, u32) {
    (30, 1)
}
fn default_true() -> bool {
    true
}
