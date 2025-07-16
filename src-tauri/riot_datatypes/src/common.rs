use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub type QueueId = i64;
pub type GameId = i64;
pub type MapId = i64;
pub type ParticipantId = i64;
pub type SummonerId = i64;
pub type ChampionId = i64;
pub type Timestamp = i64;
pub type SpellId = i64;

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Champion {
    pub id: ChampionId,
    pub name: String,
}

impl PartialEq for Champion {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub id: QueueId,
    pub name: String,
    pub is_ranked: bool,
}

#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchId {
    pub game_id: GameId,
    pub platform_id: String,
}

impl Display for MatchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}_{}", self.platform_id, self.game_id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub events: Vec<GameEvent>,
    pub participant_frames: HashMap<ParticipantId, ParticipantFrame>,
    pub timestamp: Timestamp,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    #[serde(flatten)]
    pub event: Event,
    pub timestamp: Timestamp,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE", rename_all_fields = "camelCase")]
pub enum Event {
    ChampionKill {
        victim_id: ParticipantId,
        killer_id: ParticipantId,
        assisting_participant_ids: Vec<ParticipantId>,
        position: Position,
    },
    BuildingKill {
        team_id: Team,
        killer_id: ParticipantId,
        #[serde(flatten)]
        building_type: BuildingType,
        assisting_participant_ids: Vec<ParticipantId>,
    },
    EliteMonsterKill {
        killer_id: ParticipantId,
        #[serde(flatten)]
        monster_type: MonsterType,
        assisting_participant_ids: Vec<ParticipantId>,
    },
    #[serde(untagged)]
    Unknown {},
}

#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "buildingType",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase"
)]
pub enum BuildingType {
    InhibitorBuilding { lane_type: LaneType },
    TowerBuilding { lane_type: LaneType, tower_type: TowerType },
}

#[allow(clippy::enum_variant_names)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LaneType {
    TopLane,
    MidLane,
    BotLane,
}

#[allow(clippy::enum_variant_names)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TowerType {
    OuterTurret,
    InnerTurret,
    BaseTurret,
    NexusTurret,
}

#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "monsterType", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MonsterType {
    Horde,
    Riftherald,
    Atakhan,
    BaronNashor,
    Dragon {
        #[serde(rename = "monsterSubType")]
        dragon_type: DragonType,
    },
}

#[allow(clippy::enum_variant_names)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DragonType {
    FireDragon,
    EarthDragon,
    WaterDragon,
    AirDragon,
    HextechDragon,
    ChemtechDragon,
    ElderDragon,
}

#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u32)]
pub enum Team {
    Blue = 100,
    Red = 200,
}

#[cfg_attr(feature = "specta", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantFrame {
    pub participant_id: ParticipantId,
    pub level: i64,
    pub current_gold: i64,
    pub total_gold: i64,
    pub xp: i64,
    pub minions_killed: i64,
    pub jungle_minions_killed: i64,
    pub position: Position,
}
