use std::{
    fmt, fs,
    path::PathBuf,
    sync::{Mutex, RwLock},
};

use port_check::free_local_port_in_range;
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Serialize,
};
use tauri::api::path::video_dir;

use libobs_recorder::settings::{AudioSource, Framerate, Resolution};

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

// impl PartialEq for MarkerFlags {
//     fn eq(&self, other: &Self) -> bool {
//         self.kill == other.kill
//             && self.death == other.death
//             && self.assist == other.assist
//             && self.turret == other.turret
//             && self.inhibitor == other.inhibitor
//             && self.dragon == other.dragon
//             && self.herald == other.herald
//             && self.baron == other.baron
//     }
// }

#[derive(Debug)]
pub struct Settings(RwLock<SettingsInner>);

impl Settings {
    pub fn load_from_file(&self, settings_path: &PathBuf) {
        let Ok(json) = fs::read_to_string(settings_path) else {
            return;
        };
        let mut settings = serde_json::from_str::<SettingsInner>(json.as_str()).unwrap_or_default();

        // if recordings_folder is absolute the whole path gets replaced by the absolute path
        // if recordings_folder is relative the path gets appened to the system video directory
        settings.recordings_folder = video_dir()
            .expect("video_dir doesn't exist")
            .join(settings.recordings_folder);
        if fs::create_dir_all(settings.recordings_folder.as_path()).is_err() && settings.debug_log {
            log::error!("Unable to create recordings_folder");
        }

        *self.0.write().unwrap() = settings;
        // write parsed settings back to file so the internal settings and the content of the file stay in sync
        // to avoid confusing the user when editing the file
        self.write_to_file(settings_path);
    }

    pub fn write_to_file(&self, settings_path: &PathBuf) {
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

    pub fn get_output_resolution(&self) -> Option<Resolution> {
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

    pub fn autostart(&self) -> bool {
        self.0.read().unwrap().autostart
    }

    pub fn debug_log(&self) -> bool {
        self.0.read().unwrap().debug_log || std::env::args().any(|e| e == "-d" || e == "--debug")
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self(RwLock::from(SettingsInner::default()))
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInner {
    // only used in the tauri application
    marker_flags: MarkerFlags,
    check_for_updates: bool,
    debug_log: bool,
    // these get passed to lol_rec
    recordings_folder: PathBuf,
    filename_format: String,
    encoding_quality: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_resolution: Option<Resolution>,
    framerate: Framerate,
    record_audio: AudioSource,
    autostart: bool,
}

const DEFAULT_UPDATE_CHECK: bool = true;
const DEFAULT_DEBUG_LOG: bool = false;
const DEFAULT_ENCODING_QUALITY: u32 = 25;
const DEFAULT_RECORD_AUDIO: AudioSource = AudioSource::APPLICATION;
const DEFAULT_AUTOSTART: bool = false;

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

impl Default for SettingsInner {
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
            autostart: false,
        }
    }
}

// custom deserializer that uses default values on deserialization errors instead of failing
impl<'de> Deserialize<'de> for SettingsInner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SettingsVisitor;
        impl<'de> Visitor<'de> for SettingsVisitor {
            type Value = SettingsInner;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct SettingsInner")
            }

            fn visit_map<V>(self, mut map: V) -> Result<SettingsInner, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut settings = SettingsInner::default();

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
                        "autostart" => {
                            settings.autostart = map.next_value().unwrap_or(DEFAULT_AUTOSTART);
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

#[derive(Debug, Default)]
pub struct FileWatcher(Mutex<Option<notify::RecommendedWatcher>>);

impl FileWatcher {
    pub fn set(&self, watcher: notify::RecommendedWatcher) {
        *self.0.lock().unwrap() = Some(watcher);
    }

    pub fn drop(&self) {
        self.0.lock().unwrap().take();
    }
}
