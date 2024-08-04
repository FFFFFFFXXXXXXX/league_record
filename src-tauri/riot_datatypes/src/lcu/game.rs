use serde::{Deserialize, Serialize};

use crate::{ChampionId, GameId, MapId, ParticipantId, QueueId, SpellId, SummonerId, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub game_version: String,
    pub game_id: GameId,
    pub map_id: MapId,
    pub queue_id: QueueId,
    pub game_duration: Timestamp,
    pub participant_identities: Vec<ParticipantIdentity>,
    pub participants: Vec<Participant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantIdentity {
    pub participant_id: ParticipantId,
    pub player: Player,
}

#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub game_name: String,
    pub tag_line: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summoner_id: Option<SummonerId>,
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.game_name == other.game_name && self.tag_line == other.tag_line
    }
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

#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub largest_multi_kill: i64,
    pub neutral_minions_killed: i64,
    pub neutral_minions_killed_enemy_jungle: i64,
    pub neutral_minions_killed_team_jungle: i64,
    pub total_minions_killed: i64,
    pub vision_score: f64,
    pub vision_wards_bought_in_game: i64,
    pub wards_placed: i64,
    pub wards_killed: i64,
    /// remake
    /// if this field is true `win` has to be ignored because the team that had to remake counts as the loser of the game
    /// surrenders pre minute 20 count as a normal surrender (field `game_ended_in_surrender`)
    pub game_ended_in_early_surrender: bool,
    pub game_ended_in_surrender: bool,
    pub win: bool,
}
