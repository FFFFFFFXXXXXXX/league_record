use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::{ParticipantId, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub events: Vec<Event>,
    pub participant_frames: HashMap<ParticipantId, ParticipantFrame>,
    pub timestamp: Timestamp,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE", rename_all_fields = "camelCase")]
pub enum Event {
    ChampionKill {
        timestamp: Timestamp,
        victim_id: ParticipantId,
        killer_id: ParticipantId,
        assisting_participant_ids: Vec<ParticipantId>,
        position: Position,
    },
    BuildingKill {
        timestamp: Timestamp,
        team_id: Team,
        killer_id: ParticipantId,
        #[serde(flatten)]
        building_type: BuildingType,
        assisting_participant_ids: Vec<ParticipantId>,
    },
    EliteMonsterKill {
        timestamp: Timestamp,
        killer_id: ParticipantId,
        #[serde(flatten)]
        monster_type: MonsterType,
        assisting_participant_ids: Vec<ParticipantId>,
    },
}

// seperate struct for frontend compatability since Specta is a bit limited for now and doesn't support some of the
// tags on the 'deserialization struct'
#[allow(clippy::enum_variant_names)]
#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    ChampionKill {
        timestamp: Timestamp,
        victim_id: ParticipantId,
        killer_id: ParticipantId,
        assisting_participant_ids: Vec<ParticipantId>,
        position: Position,
    },
    BuildingKill {
        timestamp: Timestamp,
        team_id: Team,
        killer_id: ParticipantId,
        building_type: BuildingType,
        assisting_participant_ids: Vec<ParticipantId>,
    },
    EliteMonsterKill {
        timestamp: Timestamp,
        killer_id: ParticipantId,
        monster_type: MonsterType,
        assisting_participant_ids: Vec<ParticipantId>,
    },
}

impl From<Event> for GameEvent {
    fn from(value: Event) -> Self {
        match value {
            Event::ChampionKill {
                timestamp,
                victim_id,
                killer_id,
                assisting_participant_ids,
                position,
            } => GameEvent::ChampionKill {
                timestamp,
                victim_id,
                killer_id,
                assisting_participant_ids,
                position,
            },
            Event::BuildingKill {
                timestamp,
                team_id,
                killer_id,
                building_type,
                assisting_participant_ids,
            } => GameEvent::BuildingKill {
                timestamp,
                team_id,
                killer_id,
                building_type,
                assisting_participant_ids,
            },
            Event::EliteMonsterKill {
                timestamp,
                killer_id,
                monster_type,
                assisting_participant_ids,
            } => GameEvent::EliteMonsterKill {
                timestamp,
                killer_id,
                monster_type,
                assisting_participant_ids,
            },
        }
    }
}

#[cfg_attr(test, derive(specta::Type))]
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
#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LaneType {
    TopLane,
    MidLane,
    BotLane,
}

#[allow(clippy::enum_variant_names)]
#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TowerType {
    OuterTurret,
    InnerTurret,
    BaseTurret,
    NexusTurret,
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[cfg_attr(test, derive(specta::Type))]
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

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u32)]
pub enum Team {
    Blue = 100,
    Red = 200,
}

#[cfg_attr(test, derive(specta::Type))]
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
    pub level: u64,
    pub current_gold: u64,
    pub total_gold: u64,
    pub xp: u64,
    pub minions_killed: u64,
    pub jungle_minions_killed: u64,
    pub position: Position,
}
