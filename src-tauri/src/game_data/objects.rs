use serde::{Deserialize, Serialize};

use super::ChampionId;

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

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub id: i64,
    pub name: String,
    pub description: String,
}
