use riot_datatypes::*;
use serde::{Deserialize, Serialize};

// allow large difference in enum Variant size because the big variant is the more common one
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub enum MetadataFile {
    Metadata(GameMetadata),
    Deferred(Deferred),
    NoData(NoData),
}

impl MetadataFile {
    pub fn is_favorite(&self) -> bool {
        match self {
            MetadataFile::Metadata(metadata) => metadata.favorite,
            MetadataFile::Deferred(deferred) => deferred.favorite,
            MetadataFile::NoData(no_data) => no_data.favorite,
        }
    }

    pub fn set_favorite(&mut self, favorite: bool) {
        match self {
            MetadataFile::Metadata(metadata) => metadata.favorite = favorite,
            MetadataFile::Deferred(deferred) => deferred.favorite = favorite,
            MetadataFile::NoData(no_data) => no_data.favorite = favorite,
        };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameMetadata {
    pub match_id: MatchId,
    pub ingame_time_rec_start_offset: f64,
    pub queue: Queue,
    pub player: lcu::Player,
    pub champion_name: String,
    pub stats: lcu::Stats,
    pub participant_id: ParticipantId,
    pub events: Vec<GameEvent>,
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Deferred {
    pub match_id: MatchId,
    pub ingame_time_rec_start_offset: f64,
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NoData {
    pub favorite: bool,
}

// seperate struct for frontend compatability since Specta is a bit limited for now and doesn't support some of the
// tags on the 'deserialization struct'
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GameEvent {
    #[serde(flatten)]
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
}

#[derive(Debug, Clone)]
pub struct UnknownEvent(riot_datatypes::Event);

impl std::error::Error for UnknownEvent {}

impl std::fmt::Display for UnknownEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:#?}", self.0))
    }
}

impl TryFrom<riot_datatypes::Event> for Event {
    type Error = UnknownEvent;

    fn try_from(value: riot_datatypes::Event) -> Result<Self, Self::Error> {
        Ok(match value {
            riot_datatypes::Event::ChampionKill {
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
            riot_datatypes::Event::BuildingKill {
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
            riot_datatypes::Event::EliteMonsterKill {
                killer_id,
                monster_type,
                assisting_participant_ids,
            } => Event::EliteMonsterKill {
                killer_id,
                monster_type,
                assisting_participant_ids,
            },
            event => return Err(UnknownEvent(event)),
        })
    }
}

impl TryFrom<riot_datatypes::GameEvent> for GameEvent {
    type Error = UnknownEvent;

    fn try_from(value: riot_datatypes::GameEvent) -> Result<Self, Self::Error> {
        Ok(GameEvent {
            event: value.event.try_into()?,
            timestamp: value.timestamp,
        })
    }
}
