mod game;
mod metadata;
mod objects;
mod timeline;

pub use metadata::{process_data, GameMetadata};

pub type GameId = i64;
type ParticipantId = usize;
type ChampionId = usize;
type SpellId = usize;
/// time since start of game in milliseconds
type Timestamp = i64;
