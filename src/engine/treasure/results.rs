use serde::Serialize;

/// Typed response payload for a rolled treasure item.
#[derive(Debug, Clone, Serialize)]
pub struct RollTreasureItemData {
    #[serde(rename = "type")]
    pub item_type: String,
    pub quantity: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gp_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_gp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restriction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Structured data payload for `RollTreasure`.
#[derive(Debug, Clone, Serialize)]
pub struct RollTreasureData {
    pub letter: String,
    pub category: String,
    pub items: Vec<RollTreasureItemData>,
    pub total_gp: f64,
}

/// Unified roll result used by CLI and API adapters.
#[derive(Debug, Clone)]
pub struct RollTreasureResult {
    pub message: String,
    pub cli_output: String,
    pub data: RollTreasureData,
}

/// Typed response payload for a treasure type entry.
#[derive(Debug, Clone, Serialize)]
pub struct TreasureTypeEntryData {
    pub chance: u32,
    pub quantity: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub restriction: Option<String>,
    pub note: Option<String>,
}

/// Structured data payload for `LookupTreasureType`.
#[derive(Debug, Clone, Serialize)]
pub struct LookupTreasureTypeData {
    pub letter: String,
    pub category: String,
    pub average_gp: f64,
    pub entries: Vec<TreasureTypeEntryData>,
}

/// Unified lookup result for API adapters.
#[derive(Debug, Clone)]
pub struct LookupTreasureTypeResult {
    pub message: String,
    pub data: LookupTreasureTypeData,
}

/// Unified result for CLI `treasure list`.
#[derive(Debug, Clone)]
pub struct TreasureListResult {
    pub output: String,
}
