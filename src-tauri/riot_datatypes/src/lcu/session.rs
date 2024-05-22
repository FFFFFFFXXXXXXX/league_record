use serde::{Deserialize, Serialize};

use crate::{GameId, Queue};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SubscriptionResponse {
    Session(SessionEventData),
    EogStatsBlock {}, // curly braces are important - else deserialization fails if the data doesn't match SessionEventData
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEventData {
    pub game_data: GameData,
    pub phase: GamePhase,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GamePhase {
    None,
    Lobby,
    Matchmaking,
    CheckedIntoTournament,
    ReadyCheck,
    ChampSelect,
    GameStart,
    FailedToLaunch,
    InProgress,
    Reconnect,
    WaitingForStats,
    PreEndOfGame,
    EndOfGame,
    TerminatedInError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameData {
    /// this queue does not have a valid 'name' field for some reason
    pub queue: Queue,
    pub game_id: GameId,
}
