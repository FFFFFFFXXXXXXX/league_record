use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::lcu::{Player, Stats};

pub type QueueId = i64;
pub type GameId = i64;
pub type MapId = i64;
pub type ParticipantId = i64;
pub type SummonerId = i64;
pub type ChampionId = i64;
pub type Timestamp = i64;
pub type SpellId = i64;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameMetadata {
    #[serde(default)]
    pub game_id: GameId,
    pub ingame_time_rec_start_offset: f64,
    pub queue: Queue,
    pub player: Player,
    pub champion_name: String,
    pub stats: Stats,
    pub participant_id: ParticipantId,
    pub events: Vec<GameEvent>,
    #[serde(default)]
    pub favorite: bool,
}

#[derive(Debug, Clone, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub id: QueueId,
    pub name: String,
    pub is_ranked: bool,
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

// seperate struct for frontend compatability since Specta is a bit limited for now and doesn't support some of the
// tags on the 'deserialization struct'
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(from = "DeserializeGameEvent")]
pub struct GameEvent {
    pub event: Event,
    pub timestamp: Timestamp,
}

// seperate struct for frontend compatability since Specta is a bit limited for now and doesn't support some of the
// tags on the 'deserialization struct'
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
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
        building_type: BuildingType,
        assisting_participant_ids: Vec<ParticipantId>,
    },
    EliteMonsterKill {
        killer_id: ParticipantId,
        monster_type: MonsterType,
        assisting_participant_ids: Vec<ParticipantId>,
    },
    #[serde(untagged)]
    Unknown {},
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeserializeGameEvent {
    #[serde(flatten)]
    event: DeserializeEvent,
    timestamp: Timestamp,
}

impl From<DeserializeEvent> for Event {
    fn from(value: DeserializeEvent) -> Self {
        match value {
            DeserializeEvent::ChampionKill {
                victim_id,
                killer_id,
                assisting_participant_ids,
                position,
            } => Event::ChampionKill {
                victim_id,
                killer_id,
                assisting_participant_ids,
                position,
            },
            DeserializeEvent::BuildingKill {
                team_id,
                killer_id,
                building_type,
                assisting_participant_ids,
            } => Event::BuildingKill {
                team_id,
                killer_id,
                building_type,
                assisting_participant_ids,
            },
            DeserializeEvent::EliteMonsterKill {
                killer_id,
                monster_type,
                assisting_participant_ids,
            } => Event::EliteMonsterKill {
                killer_id,
                monster_type,
                assisting_participant_ids,
            },
            DeserializeEvent::Unknown {} => Event::Unknown {},
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE", rename_all_fields = "camelCase")]
enum DeserializeEvent {
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

impl From<DeserializeGameEvent> for GameEvent {
    fn from(value: DeserializeGameEvent) -> Self {
        GameEvent {
            event: value.event.into(),
            timestamp: value.timestamp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LaneType {
    TopLane,
    MidLane,
    BotLane,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TowerType {
    OuterTurret,
    InnerTurret,
    BaseTurret,
    NexusTurret,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "monsterType", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MonsterType {
    Horde,
    Riftherald,
    BaronNashor,
    Dragon {
        #[serde(rename = "monsterSubType")]
        dragon_type: DragonType,
    },
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize_repr, Deserialize_repr, specta::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u32)]
pub enum Team {
    Blue = 100,
    Red = 200,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
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
