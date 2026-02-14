use serde::Serialize;

/// Typed success payload for `buy` / `action_buy`.
#[derive(Debug, Clone, Serialize)]
pub struct BuyResult {
    pub message: String,
    pub character: String,
    pub item: String,
    pub cost_gp: u32,
    pub gold_remaining: u32,
}

/// Typed success payload for `drop` / `action_drop`.
#[derive(Debug, Clone, Serialize)]
pub struct DropResult {
    pub message: String,
    pub character: String,
    pub item: String,
}

/// Typed success payload for `equip` / `action_equip`.
#[derive(Debug, Clone, Serialize)]
pub struct EquipResult {
    pub message: String,
    pub character: String,
    pub item: String,
    pub action: String,
    pub ac: i32,
}

/// Typed success payload for `loot` / `action_loot`.
#[derive(Debug, Clone, Serialize)]
pub struct LootResult {
    pub message: String,
    pub character: String,
    pub item: String,
    pub value_gp: u32,
}

/// A single equipment item summary for listing.
#[derive(Debug, Clone, Serialize)]
pub struct EquipmentItemSummary {
    pub name: String,
    pub cost_gp: u32,
    pub category: String,
}

/// Typed success payload for `list_equipment` / `action_list_equipment`.
#[derive(Debug, Clone, Serialize)]
pub struct ListEquipmentResult {
    pub weapons: Vec<EquipmentItemSummary>,
    pub armour: Vec<EquipmentItemSummary>,
    pub gear: Vec<EquipmentItemSummary>,
    pub ammunition: Vec<EquipmentItemSummary>,
}
