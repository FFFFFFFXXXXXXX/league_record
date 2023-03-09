use serde::{Deserialize, Serialize};

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameData {
    pub win: Option<bool>,
    pub game_info: GameInfo,
    pub stats: Stats,
    pub events: Vec<GameEvent>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub recording_delay: f64,
    pub game_mode: String,
    pub summoner_name: String,
    pub champion_name: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    #[serde(alias = "CHAMPIONS_KILLED")]
    pub kills: u64,
    #[serde(alias = "NUM_DEATHS")]
    pub deaths: u64,
    #[serde(alias = "ASSISTS")]
    pub assists: u64,
    #[serde(alias = "MINIONS_KILLED")]
    pub creep_score: u64,
    #[serde(alias = "VISION_SCORE")]
    pub ward_score: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEvent {
    pub name: &'static str,
    pub time: f64,
}
