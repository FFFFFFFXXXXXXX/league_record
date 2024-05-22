pub const APP_NAME: &str = "LeagueRecord";
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod exit {
    pub const SUCCESS: i32 = 0;
    pub const ERROR: i32 = 1;
}

pub mod window {
    pub const MAIN: &str = "main";
}

pub mod menu_item {
    pub const RECORDING: &str = "recording";
    pub const SETTINGS: &str = "settings";
    pub const OPEN: &str = "open";
    pub const QUIT: &str = "quit";
    pub const UPDATE: &str = "update";
}
