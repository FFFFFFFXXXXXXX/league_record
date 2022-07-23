use std::path::PathBuf;

use libobs_recorder::{Framerate, Resolution, Size};
use serde::Deserialize;
use serde_json::error::Result as SerdeResult;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    recordings_folder: PathBuf,
    filename_format: String,
    window_size: (u32, u32),
    encoding_quality: u32,
    #[serde(deserialize_with = "deserialize_resolution")]
    output_resolution: Resolution,
    #[serde(deserialize_with = "deserialize_framerate")]
    framerate: Framerate,
    record_audio: bool,
    debug_log: bool,
}

impl Config {
    pub fn init(json: &str) -> SerdeResult<Self> {
        serde_json::from_str::<Self>(json)
    }

    pub fn recordings_folder(&self) -> PathBuf {
        self.recordings_folder.clone()
    }
    pub fn filename_format(&self) -> &str {
        &self.filename_format
    }
    pub fn window_size(&self) -> Size {
        Size::new(self.window_size.0, self.window_size.1)
    }
    pub fn encoding_quality(&self) -> u32 {
        self.encoding_quality
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
    pub fn debug_log(&self) -> bool {
        self.debug_log
    }
}

// DESERIALIZERS //
fn deserialize_resolution<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Resolution, D::Error> {
    let res: String = Deserialize::deserialize(deserializer)?;
    Ok(match res.as_str() {
        "480p" => Resolution::_480p,
        "720p" => Resolution::_720p,
        "1080p" => Resolution::_1080p,
        "1440p" => Resolution::_1440p,
        "2160p" => Resolution::_2160p,
        "4320p" => Resolution::_4320p,
        _ => Resolution::_1080p,
    })
}
fn deserialize_framerate<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Framerate, D::Error> {
    let fr: (u32, u32) = Deserialize::deserialize(deserializer)?;
    Ok(Framerate::new(fr.0, fr.1))
}
