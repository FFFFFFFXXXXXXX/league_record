pub use libobs_recorder::*;
use serde::{Deserialize, Serialize};

use std::{
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_recordings_folder")]
    pub recordings_folder: PathBuf,
    #[serde(default = "default_filename_format")]
    pub filename_format: String,
    #[serde(default = "default_window_size")]
    pub window_size: Size,
    #[serde(default = "default_encoding_quality")]
    pub encoding_quality: u32,
    #[serde(default = "default_output_resolution")]
    pub output_resolution: Resolution,
    #[serde(default = "default_framerate")]
    pub framerate: Framerate,
    #[serde(default = "default_record_audio")]
    pub record_audio: AudioSource,
    #[serde(default)]
    pub debug_log: bool,
}

impl Config {
    pub fn new(settings_path: &PathBuf, mut video_dir: PathBuf) -> Option<Self> {
        if let Ok(file) = File::open(&settings_path) {
            let reader = BufReader::new(file);
            if let Ok(mut config) = serde_json::from_reader::<_, Self>(reader) {
                video_dir.push(config.recordings_folder);
                config.recordings_folder = video_dir;

                if !config.recordings_folder.exists() {
                    let _ = fs::create_dir_all(config.recordings_folder.as_path());
                }
                return Some(config);
            }
        }
        None
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            recordings_folder: default_recordings_folder(),
            filename_format: default_filename_format(),
            window_size: Size::new(0, 0),
            encoding_quality: default_encoding_quality(),
            output_resolution: default_output_resolution(),
            framerate: default_framerate(),
            record_audio: AudioSource::APPLICATION,
            debug_log: false,
        }
    }
}

// DEFAULTS //
fn default_recordings_folder() -> PathBuf {
    PathBuf::from("league_recordings")
}
fn default_filename_format() -> String {
    String::from("%Y-%m-%d_%H-%M.mp4")
}
fn default_window_size() -> Size {
    Size::new(1920, 1080)
}
fn default_encoding_quality() -> u32 {
    30
}
fn default_output_resolution() -> Resolution {
    Resolution::_1080p
}
fn default_framerate() -> Framerate {
    Framerate::new(30, 1)
}
fn default_record_audio() -> AudioSource {
    AudioSource::APPLICATION
}

#[cfg(test)]
mod test {
    use serde::Deserialize;

    #[test]
    fn test() {
        #[allow(dead_code)]
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Test {
            check_for_updates: bool,
            // these get passed to lol_rec
            #[serde(flatten)]
            pub config: crate::Config,
        }

        let cfg = serde_json::from_str::<Test>(include_str!("../../default-settings.json"));
        println!("{cfg:?}");
        if let Ok(cfg) = cfg {
            println!("{cfg:?}")
        }
    }
}
