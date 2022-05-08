use std::{
    fs::{create_dir_all, File},
    io::BufReader,
    path::PathBuf,
};

use libobs_recorder::{Framerate, Resolution};
use serde::Deserialize;

use serde_json::{json, Value};
use tauri::api::path::video_dir;

pub struct AssetPort(u16);
impl AssetPort {
    pub fn init() -> Self {
        let port = port_check::free_local_port_in_range(1024, 65535).unwrap();
        AssetPort(port)
    }
    pub fn get(&self) -> u16 {
        self.0
    }
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "recordings_folder")]
    #[serde(deserialize_with = "deserialize_recordings_folder")]
    recordings_folder: PathBuf,
    #[serde(default = "default_filename_format")]
    filename_format: String,
    #[serde(default = "default_recording_quality")]
    recording_quality: u32,
    #[serde(deserialize_with = "deserialize_resolution")]
    #[serde(default = "default_output_resolution")]
    output_resolution: Resolution,
    #[serde(deserialize_with = "deserialize_framerate")]
    #[serde(default = "default_output_framerate")]
    framerate: Framerate,
    #[serde(default = "default_true")]
    record_audio: bool,
    marker_flags: MarkerFlags,
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
        exe_dir.push("settings.json");
        if let Ok(file) = File::open(&exe_dir) {
            let reader = BufReader::new(file);
            if let Ok(settings) = serde_json::from_reader::<_, Settings>(reader) {
                return settings;
            }
        }

        // return defaults if parsing error
        let mut recordings_folder = video_dir().unwrap();
        recordings_folder.push(PathBuf::from("league_recordings"));
        if !recordings_folder.exists() {
            let _ = create_dir_all(recordings_folder.as_path());
        }
        Self {
            recordings_folder,
            filename_format: String::from("%Y-%m-%d_%H-%M.mp4"),
            recording_quality: 20,
            output_resolution: Resolution::_1080p,
            framerate: Framerate::new(30, 1),
            record_audio: true,
            marker_flags: MarkerFlags::default(),
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
    pub fn filename_format(&self) -> &str {
        &self.filename_format
    }
    pub fn recording_quality(&self) -> u32 {
        self.recording_quality
    }
    pub fn output_resolution(&self) -> Resolution {
        self.output_resolution
    }
    pub fn framerate(&self) -> Framerate {
        self.framerate
    }
    pub fn record_audio(&self) -> bool {
        self.record_audio
    }
    pub fn marker_flags(&self) -> Value {
        json!({
            "kill": self.marker_flags.kill,
            "death": self.marker_flags.death,
            "assist": self.marker_flags.assist,
            "turret": self.marker_flags.turret,
            "inhibitor": self.marker_flags.inhibitor,
            "dragon": self.marker_flags.dragon,
            "herald": self.marker_flags.herald,
            "baron": self.marker_flags.baron
        })
    }
}

// DESERIALIZERS //
fn deserialize_recordings_folder<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<PathBuf, D::Error> {
    let folder_name: String = Deserialize::deserialize(deserializer)?;
    let mut recordings_folder = video_dir().unwrap();
    recordings_folder.push(PathBuf::from(folder_name));
    if !recordings_folder.exists() {
        let _ = create_dir_all(recordings_folder.as_path());
    }
    Ok(recordings_folder)
}
fn deserialize_resolution<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Resolution, D::Error> {
    let res: String = Deserialize::deserialize(deserializer)?;
    Ok(match res.as_str() {
        "480p" | "_480p" => Resolution::_480p,
        "720p" | "_720p" => Resolution::_720p,
        "1080p" | "_1080p" => Resolution::_1080p,
        "1440p" | "_1440p" => Resolution::_1440p,
        "2160p" | "_2160p" => Resolution::_2160p,
        "4320p" | "_4320p" => Resolution::_4320p,
        _ => Resolution::_1080p,
    })
}
fn deserialize_framerate<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Framerate, D::Error> {
    let fr: (u32, u32) = Deserialize::deserialize(deserializer)?;
    Ok(Framerate::new(fr.0, fr.1))
}

// DEFAULTS //
fn recordings_folder() -> PathBuf {
    let mut recordings_folder = video_dir().unwrap();
    recordings_folder.push(PathBuf::from("league_recordings"));
    if !recordings_folder.exists() {
        let _ = create_dir_all(recordings_folder.as_path());
    }
    return recordings_folder;
}

fn default_filename_format() -> String {
    String::from("%Y-%m-%d_%H-%M.mp4")
}

fn default_recording_quality() -> u32 {
    20
}

fn default_output_resolution() -> Resolution {
    Resolution::_1080p
}

fn default_output_framerate() -> Framerate {
    Framerate::new(30, 1)
}

fn default_true() -> bool {
    true
}
