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
    pub game_mode: String,
    pub summoner_name: String,
    pub champion_name: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    #[serde(alias = "CHAMPIONS_KILLED")]
    pub kills: u64,
    #[serde(alias = "NUM_DEATHS")]
    pub deaths: u64,
    #[serde(alias = "ASSISTS")]
    pub assists: u64,
    /// lane minons killed
    #[serde(default)]
    #[serde(alias = "MINIONS_KILLED")]
    pub minions_killed: u64,
    /// neutral objectives killed
    #[serde(default)]
    #[serde(alias = "NEUTRAL_MINIONS_KILLED")]
    pub neutral_minions_killed: u64,
    // add default value fallback since there is no ward score in some game modes like ARAM
    #[serde(default)]
    #[serde(alias = "VISION_SCORE")]
    pub ward_score: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEvent {
    pub name: &'static str,
    pub time: f64,
}
