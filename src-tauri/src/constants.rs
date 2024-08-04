pub const APP_NAME: &str = "LeagueRecord";
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// see tauri.conf.json for tray_id
pub const TRAY_ID: &str = "mainTray";

pub const EXIT_SUCCESS: i32 = 0;

pub mod menu_item {
    pub const RECORDING: &str = "recording";
    pub const SETTINGS: &str = "settings";
    pub const OPEN: &str = "open";
    pub const QUIT: &str = "quit";
    pub const UPDATE: &str = "update";
}
