mod game_listener;
mod league_recorder;
mod metadata;
mod recording_task;
#[cfg(target_os = "windows")]
mod window;

pub use league_recorder::LeagueRecorder;
pub use metadata::process_data;

const RECORDINGS_CHANGED_EVENT: &str = "recordings_changed";

/// LCU API paths required for the [`crate::recorder`] module
mod api {
    pub const PLATFORM_ID: &str = "/lol-platform-config/v1/namespaces/LoginDataPacket/platformId";
    pub const GAMEFLOW_SESSION: &str = "/lol-gameflow/v1/session";
    pub const EOG_STATS_BLOCK: &str = "/lol-end-of-game/v1/eog-stats-block";
}

// allow large difference in enum Variant size because the big variant is the more common one
#[allow(clippy::large_enum_variant)]
#[derive(serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(untagged)]
pub enum MetadataFile {
    Metadata(riot_datatypes::GameMetadata),
    Deferred((riot_datatypes::MatchId, f64)),
}
