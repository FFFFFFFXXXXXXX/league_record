pub use libobs_recorder::*;
use serde::{Deserialize, Serialize};

use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub recordings_folder: PathBuf,
    pub filename_format: String,
    pub window_size: Size,
    pub encoding_quality: u32,
    pub output_resolution: Resolution,
    pub framerate: Framerate,
    pub record_audio: AudioSource,
}
