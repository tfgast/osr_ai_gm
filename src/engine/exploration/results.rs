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
    /// When an encounter is present, lists the commands available to resolve it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub next_steps: Vec<String>,
    pub placed_monsters: Option<Vec<PlacedMonsterInstance>>,
    pub placed_treasure: Option<Vec<PlacedTreasureInstance>>,
}

/// Build next-step guidance for a wandering monster encounter.
fn encounter_next_steps(enc: &EncounterResult) -> Vec<String> {
    vec![
        format!(
            "SpawnMonster {{name: \"{}\", count: <roll {}>, distance: 2d6×10}} to start combat",
            enc.name, enc.number
        ),
        "RollReaction {character: \"<name>\"} to check monster attitude".to_string(),
        "Evade {monster_count: <n>, monster_movement: <mv>} to attempt escape".to_string(),
    ]
}

/// Build next-step guidance for placed module monsters.
fn placed_monster_next_steps(monsters: &[PlacedMonsterInstance]) -> Vec<String> {
    use crate::rules::monster as monster_db;

    let mut steps = Vec::new();
    for m in monsters {
        if monster_db::find_monster(&m.name).is_some() {
            steps.push(format!(
                "SpawnMonster {{name: \"{}\", count: {}, distance: 10}} to start combat",
                m.name, m.count
            ));
        } else {
            steps.push(format!(
                "SpawnEncounter {{name: \"{}\", count: {}, hit_dice: <HD>, ac: <AC>, hp: <HP>, \
                damage: \"<dmg>\", morale: <M>, distance: 10}} — \
                not in core DB, provide stats from module",
                m.name, m.count
            ));
        }
    }
    steps
}

impl From<super::ExplorationResult> for ExplorationActionResult {
    fn from(value: super::ExplorationResult) -> Self {
        // Build base message before moving fields out of value.
        let base_message = value.to_string();

        let encounter = value.encounter.map(|entry| EncounterResult {
            name: entry.name,
            number: entry.number,
            hd: entry.hd,
        });
        let has_encounter = encounter.is_some();

        let (mut next_steps, guidance_msg) = match &encounter {
            Some(enc) => {
                let guidance = "ENCOUNTER RESOLUTION REQUIRED: Use SpawnMonster to fight, \
                     RollReaction to parley, or Evade to flee.".to_string();
                (encounter_next_steps(enc), Some(guidance))
            }
            None => (Vec::new(), None),
        };

        // Add guidance for placed module monsters
        let placed_guidance = if let Some(ref placed) = value.placed_monsters {
            if !placed.is_empty() {
                next_steps.extend(placed_monster_next_steps(placed));
                Some("PLACED MONSTERS: Use SpawnMonster to fight, or RollReaction to parley.".to_string())
            } else {
                None
            }
        } else {
            None
        };

        let mut messages = value.messages;
        let message = if let Some(guidance) = &guidance_msg {
            messages.push(guidance.clone());
            format!("{}{}\n", base_message, guidance)
        } else if let Some(placed_msg) = &placed_guidance {
            messages.push(placed_msg.clone());
            format!("{}{}\n", base_message, placed_msg)
        } else {
            base_message
        };

        Self {
            message,
            messages,
            has_encounter,
            encounter,
            next_steps,
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

/// Typed success payload for `look` (describe current room).
#[derive(Debug, Clone, Serialize)]
pub struct LookResult {
    pub message: String,
    pub room_id: u32,
    pub room_name: String,
    pub description: String,
}
