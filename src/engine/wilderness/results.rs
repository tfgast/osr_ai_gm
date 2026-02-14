use serde::Serialize;

use crate::rules::encounter::EncounterEntry;
use crate::state::wilderness::Terrain;

#[derive(Debug, Clone, Serialize)]
pub struct EncounterSummary {
    pub name: String,
    pub number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hd: Option<String>,
}

impl From<EncounterEntry> for EncounterSummary {
    fn from(value: EncounterEntry) -> Self {
        Self {
            name: value.name,
            number: value.number,
            hd: value.hd,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterWildernessResult {
    pub message: String,
    pub terrain: Terrain,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddHexResult {
    pub message: String,
    pub x: i32,
    pub y: i32,
    pub terrain: Terrain,
}

#[derive(Debug, Clone, Serialize)]
pub struct TravelResult {
    pub message: String,
    pub messages: Vec<String>,
    pub lost: bool,
    pub has_encounter: bool,
    pub encounters: Vec<EncounterSummary>,
    pub foraged: Option<bool>,
    pub rations_consumed: u32,
    pub starving: bool,
    pub starvation_damage: u32,
    pub rations_remaining: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrientResult {
    pub message: String,
    pub success: bool,
    pub terrain: Terrain,
    pub lost: bool,
    pub travel_day: u32,
    pub rations_consumed: u32,
    pub starving: bool,
    pub starvation_damage: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForageResult {
    pub message: String,
    pub quantity: u32,
    pub success: bool,
    pub rations_remaining: u32,
    pub rations_consumed: u32,
    pub starving: bool,
    pub starvation_damage: u32,
    pub travel_day: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct HuntResult {
    pub message: String,
    pub quantity: u32,
    pub success: bool,
    pub rations_remaining: u32,
    pub rations_consumed: u32,
    pub starving: bool,
    pub starvation_damage: u32,
    pub travel_day: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct WildernessStatusResult {
    pub message: String,
    pub movement_rate: u32,
}
