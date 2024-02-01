use serde::{de::Visitor, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameData {
    pub win: Option<bool>,
    pub game_info: GameInfo,
    pub stats: Stats,
    pub events: Vec<GameEvent>,
}

#[derive(Debug, Serialize, Deserialize, Default, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub game_mode: String,
    pub summoner_name: String,
    pub champion_name: String,
}

#[derive(Debug, Serialize, Deserialize, Default, specta::Type)]
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

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEvent {
    pub name: EventName,
    pub time: f32,
}

#[derive(Debug, specta::Type)]
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

// custom (de-)serializers because #[serde(rename("..."))] is not yet supported by specta
// https://github.com/oscartbeaumont/specta/issues/190
impl Serialize for EventName {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            EventName::Kill => ser.serialize_unit_variant("EventName", 0, ""),
            EventName::Death => ser.serialize_unit_variant("EventName", 1, "Death"),
            EventName::Assist => ser.serialize_unit_variant("EventName", 2, "Assist"),
            EventName::Voidgrub => ser.serialize_unit_variant("EventName", 3, "Voidgrub"),
            EventName::Herald => ser.serialize_unit_variant("EventName", 4, "Herald"),
            EventName::Baron => ser.serialize_unit_variant("EventName", 5, "Baron"),
            EventName::Inhibitor => ser.serialize_unit_variant("EventName", 6, "Inhibitor"),
            EventName::Turret => ser.serialize_unit_variant("EventName", 7, "Turret"),
            EventName::InfernalDragon => ser.serialize_unit_variant("EventName", 8, "Infernal-Dragon"),
            EventName::OceanDragon => ser.serialize_unit_variant("EventName", 9, "Ocean-Dragon"),
            EventName::MountainDragon => ser.serialize_unit_variant("EventName", 10, "Mountain-Dragon"),
            EventName::CloudDragon => ser.serialize_unit_variant("EventName", 11, "Cloud-Dragon"),
            EventName::HextechDragon => ser.serialize_unit_variant("EventName", 12, "Hextech-Dragon"),
            EventName::ChemtechDragon => ser.serialize_unit_variant("EventName", 13, "Chemtech-Dragon"),
            EventName::ElderDragon => ser.serialize_unit_variant("EventName", 14, "Elder-Dragon"),
        }
    }
}

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
                    "Infernal-Dragon" => Ok(EventName::InfernalDragon),
                    "Ocean-Dragon" => Ok(EventName::OceanDragon),
                    "Mountain-Dragon" => Ok(EventName::MountainDragon),
                    "Cloud-Dragon" => Ok(EventName::CloudDragon),
                    "Hextech-Dragon" => Ok(EventName::HextechDragon),
                    "Chemtech-Dragon" => Ok(EventName::ChemtechDragon),
                    "Elder-Dragon" => Ok(EventName::ElderDragon),
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
                            "Ocean-Dragon",
                            "Mountain-Dragon",
                            "Cloud-Dragon",
                            "Hextech-Dragon",
                            "Chemtech-Dragon",
                            "Elder-Dragon",
                        ],
                    )),
                }
            }
        }

        des.deserialize_str(EventNameVisitor)
    }
}
