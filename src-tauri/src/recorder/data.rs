use serde::{de::Visitor, Deserialize, Serialize};

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameData {
    pub win: Option<bool>,
    pub game_info: GameInfo,
    pub stats: Stats,
    pub events: Vec<GameEvent>,
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub game_mode: String,
    pub summoner_name: String,
    pub champion_name: String,
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    #[serde(alias = "CHAMPIONS_KILLED")]
    pub kills: u32,
    #[serde(alias = "NUM_DEATHS")]
    pub deaths: u32,
    #[serde(alias = "ASSISTS")]
    pub assists: u32,
    /// lane minons killed
    #[serde(default)]
    #[serde(alias = "MINIONS_KILLED")]
    pub minions_killed: u32,
    /// neutral objectives killed
    #[serde(default)]
    #[serde(alias = "NEUTRAL_MINIONS_KILLED")]
    pub neutral_minions_killed: u32,
    // add default value fallback since there is no ward score in some game modes like ARAM
    #[serde(default)]
    #[serde(alias = "VISION_SCORE")]
    pub ward_score: f32,
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEvent {
    pub name: EventName,
    pub time: f32,
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, Serialize)]
pub enum EventName {
    Kill,
    Death,
    Assist,
    Voidgrub,
    Herald,
    Baron,
    Inhibitor,
    Turret,
    InfernalDragon,
    OceanDragon,
    MountainDragon,
    CloudDragon,
    HextechDragon,
    ChemtechDragon,
    ElderDragon,
}

// custom (de-)serializers because #[serde(rename|alias("..."))] are not yet supported by specta
// https://github.com/oscartbeaumont/specta/issues/190
// and we need to be able to deserialize old {videoName}.json files
impl<'de> Deserialize<'de> for EventName {
    fn deserialize<D>(des: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct EventNameVisitor;
        impl<'de> Visitor<'de> for EventNameVisitor {
            type Value = EventName;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct MarkerFlags")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "Kill" => Ok(EventName::Kill),
                    "Death" => Ok(EventName::Death),
                    "Assist" => Ok(EventName::Assist),
                    "Voidgrub" => Ok(EventName::Voidgrub),
                    "Herald" => Ok(EventName::Herald),
                    "Baron" => Ok(EventName::Baron),
                    "Inhibitor" => Ok(EventName::Inhibitor),
                    "Turret" => Ok(EventName::Turret),
                    "Infernal-Dragon" | "InfernalDragon" => Ok(EventName::InfernalDragon),
                    "Ocean-Dragon" | "OceanDragon" => Ok(EventName::OceanDragon),
                    "Mountain-Dragon" | "MountainDragon" => Ok(EventName::MountainDragon),
                    "Cloud-Dragon" | "CloudDragon" => Ok(EventName::CloudDragon),
                    "Hextech-Dragon" | "HextechDragon" => Ok(EventName::HextechDragon),
                    "Chemtech-Dragon" | "ChemtechDragon" => Ok(EventName::ChemtechDragon),
                    "Elder-Dragon" | "ElderDragon" => Ok(EventName::ElderDragon),
                    _ => Err(E::unknown_variant(
                        v,
                        &[
                            "Kill",
                            "Death",
                            "Assist",
                            "Turret",
                            "Inhibitor",
                            "Voidgrub",
                            "Herald",
                            "Baron",
                            "Infernal-Dragon",
                            "InfernalDragon",
                            "Ocean-Dragon",
                            "OceanDragon",
                            "Mountain-Dragon",
                            "MountainDragon",
                            "Cloud-Dragon",
                            "CloudDragon",
                            "Hextech-Dragon",
                            "HextechDragon",
                            "Chemtech-Dragon",
                            "ChemtechDragon",
                            "Elder-Dragon",
                            "ElderDragon",
                        ],
                    )),
                }
            }
        }

        des.deserialize_str(EventNameVisitor)
    }
}
