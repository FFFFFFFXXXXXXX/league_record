mod game;
mod metadata;
mod objects;
mod timeline;

pub use metadata::{process_data, process_data_with_retry, GameMetadata};

pub type QueueId = i64;
pub type GameId = i64;
pub type MapId = i64;
pub type SummonerId = i64;
type ParticipantId = usize;
type ChampionId = usize;
type SpellId = usize;
/// time since start of game in milliseconds
type Timestamp = i64;
