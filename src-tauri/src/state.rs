use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};
use std::{fmt, fs};

use libobs_recorder::settings::{AudioSource, Framerate, StdResolution};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use tauri::api::path::video_dir;

pub struct WindowState {
    pub size: Mutex<(f64, f64)>,
    pub position: Mutex<(f64, f64)>,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            size: Mutex::from((1200.0, 650.0)),
            position: Mutex::from((-1.0, -1.0)),
        }
    }
}

#[derive(Debug)]
pub struct SettingsFile(PathBuf);

impl SettingsFile {
    pub fn new(pathbuf: PathBuf) -> Self {
        Self(pathbuf)
    }

    pub fn get(&self) -> &Path {
        self.0.as_path()
    }
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct MarkerFlags {
    kill: bool,
    death: bool,
    assist: bool,
    turret: bool,
    inhibitor: bool,
    dragon: bool,
    herald: bool,
    baron: bool,
}

// Infallible
// custom deserializer that uses default values on deserialization errors instead of failing
impl<'de> Deserialize<'de> for MarkerFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MarkerFlagsVisitor;
        impl<'de> Visitor<'de> for MarkerFlagsVisitor {
            type Value = MarkerFlags;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct MarkerFlags")
            }

            fn visit_map<V>(self, mut map: V) -> Result<MarkerFlags, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut marker_flags = MarkerFlags::default();

                while let Some(key) = map.next_key()? {
                    match key {
                        "kill" => {
                            marker_flags.kill = map.next_value().unwrap_or(true);
                        }
                        "death" => {
                            marker_flags.death = map.next_value().unwrap_or(true);
                        }
                        "assist" => {
                            marker_flags.assist = map.next_value().unwrap_or(true);
                        }
                        "turret" => {
                            marker_flags.turret = map.next_value().unwrap_or(true);
                        }
                        "inhibitor" => {
                            marker_flags.inhibitor = map.next_value().unwrap_or(true);
                        }
                        "dragon" => {
                            marker_flags.dragon = map.next_value().unwrap_or(true);
                        }
                        "herald" => {
                            marker_flags.herald = map.next_value().unwrap_or(true);
                        }
                        "baron" => {
                            marker_flags.baron = map.next_value().unwrap_or(true);
                        }
                        _ => { /* ignored */ }
                    }
                }

                Ok(marker_flags)
            }
        }

        deserializer.deserialize_map(MarkerFlagsVisitor)
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
pub struct SettingsWrapper(RwLock<Settings>);

impl SettingsWrapper {
    pub fn load_from_file(&self, settings_path: &Path) {
        let Ok(json) = fs::read_to_string(settings_path) else {
            return;
        };
        let mut settings = serde_json::from_str::<Settings>(json.as_str()).unwrap_or_default();

        // if recordings_folder is absolute the whole path gets replaced by the absolute path
        // if recordings_folder is relative the path gets appened to the system video directory
        settings.recordings_folder = video_dir()
            .expect("video_dir doesn't exist")
            .join(settings.recordings_folder);
        if fs::create_dir_all(settings.recordings_folder.as_path()).is_err() {
            log::error!("unable to create recordings_folder");
        }

        *self.0.write().unwrap() = settings;
        // write parsed settings back to file so the internal settings and the content of the file stay in sync
        // to avoid confusing the user when editing the file
        self.write_to_file(settings_path);
    }

    pub fn write_to_file(&self, settings_path: &Path) {
        let json = serde_json::to_string_pretty(&*self.0.read().unwrap()).unwrap();
        _ = fs::write(settings_path, json);
    }

    pub fn get_recordings_path(&self) -> PathBuf {
        self.0.read().unwrap().recordings_folder.clone()
    }

    pub fn get_filename_format(&self) -> String {
        self.0.read().unwrap().filename_format.clone()
    }

    pub fn get_encoding_quality(&self) -> u32 {
        self.0.read().unwrap().encoding_quality
    }

    pub fn get_output_resolution(&self) -> Option<StdResolution> {
        self.0.read().unwrap().output_resolution
    }

    pub fn get_framerate(&self) -> Framerate {
        self.0.read().unwrap().framerate
    }

    pub fn get_audio_source(&self) -> AudioSource {
        self.0.read().unwrap().record_audio
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

    pub fn only_record_ranked(&self) -> bool {
        self.0.read().unwrap().only_record_ranked
    }

    pub fn autostart(&self) -> bool {
        self.0.read().unwrap().autostart
    }

    pub fn max_recording_age(&self) -> Option<u64> {
        self.0.read().unwrap().max_recording_age
    }

    pub fn max_recordings_size(&self) -> Option<u64> {
        self.0.read().unwrap().max_recordings_size
    }

    pub fn debug_log(&self) -> bool {
        self.0.read().unwrap().debug_log || std::env::args().any(|e| e == "-d" || e == "--debug")
    }
}

impl Default for SettingsWrapper {
    fn default() -> Self {
        Self(RwLock::from(Settings::default()))
    }
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    // only used in the tauri application
    marker_flags: MarkerFlags,
    check_for_updates: bool,
    debug_log: bool,
    // these get passed to lol_rec
    recordings_folder: PathBuf,
    filename_format: String,
    encoding_quality: u32,
    output_resolution: Option<StdResolution>,
    framerate: Framerate,
    record_audio: AudioSource,
    only_record_ranked: bool,
    autostart: bool,
    max_recording_age: Option<u64>,
    max_recordings_size: Option<u64>,
}

const DEFAULT_UPDATE_CHECK: bool = true;
const DEFAULT_DEBUG_LOG: bool = false;
const DEFAULT_ENCODING_QUALITY: u32 = 25;
const DEFAULT_RECORD_AUDIO: AudioSource = AudioSource::APPLICATION;
const DEFAULT_ONLY_RECORD_RANKED: bool = false;
const DEFAULT_AUTOSTART: bool = false;
const DEFAULT_MAX_RECORDING_AGE: Option<u64> = None;
const DEFAULT_MAX_RECORDINGS_SIZE: Option<u64> = None;

#[inline]
fn default_recordings_folder() -> PathBuf {
    PathBuf::from("league_recordings")
}

#[inline]
fn default_filename_format() -> String {
    String::from("%Y-%m-%d_%H-%M.mp4")
}

#[inline]
fn default_framerate() -> Framerate {
    Framerate::new(30, 1)
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            check_for_updates: DEFAULT_UPDATE_CHECK,
            marker_flags: MarkerFlags::default(),
            debug_log: DEFAULT_DEBUG_LOG,
            recordings_folder: default_recordings_folder(),
            filename_format: default_filename_format(),
            encoding_quality: DEFAULT_ENCODING_QUALITY,
            output_resolution: None,
            framerate: default_framerate(),
            record_audio: DEFAULT_RECORD_AUDIO,
            only_record_ranked: DEFAULT_ONLY_RECORD_RANKED,
            autostart: false,
            max_recording_age: None,
            max_recordings_size: None,
        }
    }
}

// custom deserializer that uses default values on deserialization errors instead of failing
impl<'de> Deserialize<'de> for Settings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SettingsVisitor;
        impl<'de> Visitor<'de> for SettingsVisitor {
            type Value = Settings;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Settings")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Settings, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut settings = Settings::default();

                while let Some(key) = map.next_key()? {
                    match key {
                        "checkForUpdates" => {
                            settings.check_for_updates = map.next_value().unwrap_or(DEFAULT_UPDATE_CHECK);
                        }
                        "markerFlags" => {
                            settings.marker_flags = map
                                .next_value()
                                .expect("MarkerFlags deserialization should be Infallible");
                        }
                        "debugLog" => settings.debug_log = map.next_value().unwrap_or(DEFAULT_DEBUG_LOG),
                        "recordingsFolder" => {
                            settings.recordings_folder =
                                map.next_value().unwrap_or_else(|_| default_recordings_folder());
                        }
                        "filenameFormat" => {
                            settings.filename_format = map.next_value().unwrap_or_else(|_| default_filename_format());
                        }
                        "encodingQuality" => {
                            settings.encoding_quality = map.next_value().unwrap_or(DEFAULT_ENCODING_QUALITY);
                        }
                        "outputResolution" => {
                            settings.output_resolution = map.next_value().ok();
                        }
                        "framerate" => {
                            settings.framerate = map.next_value().unwrap_or_else(|_| default_framerate());
                        }
                        "recordAudio" => {
                            settings.record_audio = map.next_value().unwrap_or(DEFAULT_RECORD_AUDIO);
                        }
                        "onlyRecordRanked" => {
                            settings.only_record_ranked = map.next_value().unwrap_or(DEFAULT_ONLY_RECORD_RANKED);
                        }
                        "autostart" => {
                            settings.autostart = map.next_value().unwrap_or(DEFAULT_AUTOSTART);
                        }
                        "maxRecordingAge" => {
                            settings.max_recording_age = map.next_value().unwrap_or(DEFAULT_MAX_RECORDING_AGE);
                        }
                        "maxRecordingsSize" => {
                            settings.max_recordings_size = map.next_value().unwrap_or(DEFAULT_MAX_RECORDINGS_SIZE);
                        }
                        _ => { /* ignored */ }
                    }
                }

                Ok(settings)
            }
        }

        deserializer.deserialize_map(SettingsVisitor)
    }
}

#[derive(Debug)]
pub struct FileWatcher(Mutex<notify::RecommendedWatcher>);

impl FileWatcher {
    pub fn new(watcher: notify::RecommendedWatcher) -> Self {
        FileWatcher(Mutex::new(watcher))
    }

    pub fn set(&self, watcher: notify::RecommendedWatcher) {
        // dropping the previous filewatcher stops it
        drop(std::mem::replace(&mut *self.0.lock().unwrap(), watcher));
    }
}

#[derive(Debug, Default)]
pub struct CurrentlyRecording(Mutex<Option<PathBuf>>);

impl CurrentlyRecording {
    pub fn set(&self, path: Option<PathBuf>) {
        *self.0.lock().unwrap() = path;
    }

    pub fn get(&self) -> Option<PathBuf> {
        self.0.lock().unwrap().clone()
    }
}
