use serde::Serialize;

use crate::state::dungeon::{DoorState, PlacedMonsterInstance, PlacedTreasureInstance};

/// Minimal encounter payload for exploration actions.
#[derive(Debug, Clone, Serialize)]
pub struct EncounterResult {
    pub name: String,
    pub number: String,
    pub hd: Option<String>,
}

/// Typed success payload for `move_room` / `action_move_through_door`.
#[derive(Debug, Clone, Serialize)]
pub struct ExplorationActionResult {
    pub message: String,
    pub messages: Vec<String>,
    pub has_encounter: bool,
    pub encounter: Option<EncounterResult>,
    pub placed_monsters: Option<Vec<PlacedMonsterInstance>>,
    pub placed_treasure: Option<Vec<PlacedTreasureInstance>>,
}

impl From<super::ExplorationResult> for ExplorationActionResult {
    fn from(value: super::ExplorationResult) -> Self {
        let message = value.to_string();
        let encounter = value.encounter.map(|entry| EncounterResult {
            name: entry.name,
            number: entry.number,
            hd: entry.hd,
        });
        let has_encounter = encounter.is_some();

        Self {
            message,
            messages: value.messages,
            has_encounter,
            encounter,
            placed_monsters: value.placed_monsters,
            placed_treasure: value.placed_treasure,
        }
    }
}

pub type MoveThroughDoorResult = ExplorationActionResult;
pub type AdvanceDungeonTurnResult = ExplorationActionResult;
pub type SearchRoomResult = ExplorationActionResult;
pub type ListenAtDoorResult = ExplorationActionResult;

/// Typed success payload for `force_door`.
#[derive(Debug, Clone, Serialize)]
pub struct ForceDoorResult {
    pub message: String,
    pub door_id: u32,
    pub character: String,
    pub forced_open: bool,
}

/// Typed success payload for `enter_dungeon`.
#[derive(Debug, Clone, Serialize)]
pub struct EnterDungeonResult {
    pub message: String,
    pub level: u32,
    pub room_name: String,
}

/// Typed success payload for `light`.
#[derive(Debug, Clone, Serialize)]
pub struct LightResult {
    pub message: String,
    pub source: String,
    pub carrier: String,
    pub duration_turns: u32,
}

/// Typed success payload for `rest`.
#[derive(Debug, Clone, Serialize)]
pub struct RestResult {
    pub message: String,
    pub total_turns: u32,
}

/// Typed success payload for `add_room`.
#[derive(Debug, Clone, Serialize)]
pub struct AddRoomResult {
    pub message: String,
    pub room_id: u32,
    pub name: String,
}

/// Typed success payload for `add_door`.
#[derive(Debug, Clone, Serialize)]
pub struct AddDoorResult {
    pub message: String,
    pub door_id: u32,
    pub room_a: u32,
    pub room_b: u32,
    pub door_state: DoorState,
}

/// Typed success payload for `open_door`.
#[derive(Debug, Clone, Serialize)]
pub struct OpenDoorResult {
    pub message: String,
    pub door_id: u32,
    pub steps: Vec<String>,
    pub forced: bool,
    pub moved: bool,
}

/// Typed success payload for `pick_lock`.
#[derive(Debug, Clone, Serialize)]
pub struct PickLockResult {
    pub message: String,
    pub door_id: u32,
    pub character: String,
    pub success: bool,
}

/// Typed success payload for `exploration_status`.
#[derive(Debug, Clone, Serialize)]
pub struct ExplorationStatusResult {
    pub message: String,
}
