use serde::{Deserialize, Serialize};
use shaco::model::ingame::GameMode;

use super::{ChampionId, GameId, ParticipantId, SpellId};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub game_version: String,
    pub game_id: GameId,
    pub map_id: i64,
    pub game_mode: GameMode,
    pub queue_id: i64,
    pub game_duration: u64,
    pub participant_identities: Vec<ParticipantIdentity>,
    pub participants: Vec<Participant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantIdentity {
    pub participant_id: ParticipantId,
    pub player: Player,
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub game_name: String,
    pub tag_line: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summoner_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub participant_id: ParticipantId,
    pub champion_id: ChampionId,
    pub spell1_id: SpellId,
    pub spell2_id: SpellId,
    pub stats: Stats,
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub kills: u64,
    pub deaths: u64,
    pub assists: u64,
    pub largest_multi_kill: u64,
    pub neutral_minions_killed: u64,
    pub neutral_minions_killed_enemy_jungle: u64,
    pub neutral_minions_killed_team_jungle: u64,
    pub total_minions_killed: u64,
    pub vision_score: f64,
    pub vision_wards_bought_in_game: u64,
    pub wards_placed: u64,
    pub wards_killed: u64,
    pub win: bool,
}
