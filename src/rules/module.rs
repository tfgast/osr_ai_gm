/// Adventure module definitions loaded from JSON files.
/// Modules are prewritten dungeon adventures with rooms, monsters, treasure, and exits.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::state::dungeon::DoorState;

/// A complete adventure module definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDef {
    pub name: String,
    pub level_range: (u32, u32),
    pub entry_room: String,
    pub rooms: HashMap<String, ModuleRoom>,
}

/// A room within an adventure module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRoom {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub monsters: Vec<PlacedMonster>,
    #[serde(default)]
    pub treasure: Vec<PlacedTreasure>,
    #[serde(default)]
    pub trap: Option<String>,
    #[serde(default)]
    pub exits: Vec<ModuleExit>,
}

/// A monster placement within a module room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedMonster {
    pub name: String,
    #[serde(default = "default_count")]
    pub count: u32,
}

fn default_count() -> u32 {
    1
}

/// Treasure placed in a module room.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlacedTreasure {
    Coins { gp: u64 },
    Item { item: String },
}

/// An exit from a module room to another room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleExit {
    pub to: String,
    #[serde(default)]
    pub door: DoorState,
}

/// Load a module definition from a JSON file.
pub fn load_module(path: &str) -> Result<ModuleDef, String> {
    let path = Path::new(path);
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read module file {}: {}", path.display(), e))?;
    let module: ModuleDef = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse module file {}: {}", path.display(), e))?;

    // Validate the module
    validate_module(&module)?;

    Ok(module)
}

/// Validate a module definition for consistency.
fn validate_module(module: &ModuleDef) -> Result<(), String> {
    // Check that entry_room exists
    if !module.rooms.contains_key(&module.entry_room) {
        return Err(format!(
            "Module '{}': entry_room '{}' not found in rooms",
            module.name, module.entry_room
        ));
    }

    // Check that all exit destinations exist
    for (room_key, room) in &module.rooms {
        for exit in &room.exits {
            if !module.rooms.contains_key(&exit.to) {
                return Err(format!(
                    "Module '{}': room '{}' has exit to non-existent room '{}'",
                    module.name, room_key, exit.to
                ));
            }
        }
    }

    // Check for mismatched bidirectional door states
    for (room_key, room) in &module.rooms {
        for exit in &room.exits {
            if let Some(other_room) = module.rooms.get(&exit.to) {
                if let Some(reverse_exit) = other_room.exits.iter().find(|e| e.to == *room_key) {
                    if exit.door != reverse_exit.door {
                        return Err(format!(
                            "Module '{}': conflicting door states between '{}' and '{}' \
                             ({} vs {})",
                            module.name, room_key, exit.to, exit.door, reverse_exit.door
                        ));
                    }
                }
            }
        }
    }

    // Check level_range is valid
    if module.level_range.0 > module.level_range.1 {
        return Err(format!(
            "Module '{}': invalid level_range ({}, {})",
            module.name, module.level_range.0, module.level_range.1
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_module_json() -> &'static str {
        r#"{
            "name": "Test Crypt",
            "level_range": [1, 3],
            "entry_room": "entrance",
            "rooms": {
                "entrance": {
                    "name": "Crypt Entrance",
                    "description": "Stone steps descend into darkness.",
                    "exits": [{"to": "guard", "door": "closed"}]
                },
                "guard": {
                    "name": "Guard Chamber",
                    "description": "Bones litter the floor.",
                    "monsters": [{"name": "skeleton", "count": 3}],
                    "exits": [
                        {"to": "entrance", "door": "closed"},
                        {"to": "vault", "door": "locked"}
                    ]
                },
                "vault": {
                    "name": "Treasure Vault",
                    "description": "A dusty chest sits in the corner.",
                    "treasure": [{"gp": 500}, {"item": "Potion of Healing"}],
                    "exits": [{"to": "guard", "door": "locked"}]
                }
            }
        }"#
    }

    #[test]
    fn parse_module_json() {
        let module: ModuleDef = serde_json::from_str(sample_module_json()).unwrap();
        assert_eq!(module.name, "Test Crypt");
        assert_eq!(module.level_range, (1, 3));
        assert_eq!(module.entry_room, "entrance");
        assert_eq!(module.rooms.len(), 3);
    }

    #[test]
    fn parse_room_with_monsters() {
        let module: ModuleDef = serde_json::from_str(sample_module_json()).unwrap();
        let guard = module.rooms.get("guard").unwrap();
        assert_eq!(guard.name, "Guard Chamber");
        assert_eq!(guard.monsters.len(), 1);
        assert_eq!(guard.monsters[0].name, "skeleton");
        assert_eq!(guard.monsters[0].count, 3);
    }

    #[test]
    fn parse_room_with_treasure() {
        let module: ModuleDef = serde_json::from_str(sample_module_json()).unwrap();
        let vault = module.rooms.get("vault").unwrap();
        assert_eq!(vault.treasure.len(), 2);
        match &vault.treasure[0] {
            PlacedTreasure::Coins { gp } => assert_eq!(*gp, 500),
            _ => panic!("Expected coins"),
        }
        match &vault.treasure[1] {
            PlacedTreasure::Item { item } => assert_eq!(item, "Potion of Healing"),
            _ => panic!("Expected item"),
        }
    }

    #[test]
    fn parse_exits_with_door_state() {
        let module: ModuleDef = serde_json::from_str(sample_module_json()).unwrap();
        let guard = module.rooms.get("guard").unwrap();
        assert_eq!(guard.exits.len(), 2);
        assert_eq!(guard.exits[0].to, "entrance");
        assert_eq!(guard.exits[0].door, DoorState::Closed);
        assert_eq!(guard.exits[1].to, "vault");
        assert_eq!(guard.exits[1].door, DoorState::Locked);
    }

    #[test]
    fn default_door_state_is_closed() {
        let json = r#"{"to": "somewhere"}"#;
        let exit: ModuleExit = serde_json::from_str(json).unwrap();
        assert_eq!(exit.door, DoorState::Closed);
    }

    #[test]
    fn default_monster_count_is_one() {
        let json = r#"{"name": "goblin"}"#;
        let monster: PlacedMonster = serde_json::from_str(json).unwrap();
        assert_eq!(monster.count, 1);
    }

    #[test]
    fn validate_missing_entry_room() {
        let module = ModuleDef {
            name: "Bad Module".to_string(),
            level_range: (1, 2),
            entry_room: "nonexistent".to_string(),
            rooms: HashMap::new(),
        };
        assert!(validate_module(&module).is_err());
    }

    #[test]
    fn validate_bad_exit_reference() {
        let mut rooms = HashMap::new();
        rooms.insert("start".to_string(), ModuleRoom {
            name: "Start".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            exits: vec![ModuleExit {
                to: "nowhere".to_string(),
                door: DoorState::Closed,
            }],
        });
        let module = ModuleDef {
            name: "Bad Exit".to_string(),
            level_range: (1, 2),
            entry_room: "start".to_string(),
            rooms,
        };
        assert!(validate_module(&module).is_err());
    }

    #[test]
    fn validate_bad_level_range() {
        let mut rooms = HashMap::new();
        rooms.insert("start".to_string(), ModuleRoom {
            name: "Start".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            exits: Vec::new(),
        });
        let module = ModuleDef {
            name: "Bad Range".to_string(),
            level_range: (5, 2), // Invalid: min > max
            entry_room: "start".to_string(),
            rooms,
        };
        assert!(validate_module(&module).is_err());
    }

    #[test]
    fn validate_good_module() {
        let module: ModuleDef = serde_json::from_str(sample_module_json()).unwrap();
        assert!(validate_module(&module).is_ok());
    }

    #[test]
    fn validate_conflicting_bidirectional_door_states() {
        let mut rooms = HashMap::new();
        rooms.insert("room_a".to_string(), ModuleRoom {
            name: "Room A".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            exits: vec![ModuleExit {
                to: "room_b".to_string(),
                door: DoorState::Locked,
            }],
        });
        rooms.insert("room_b".to_string(), ModuleRoom {
            name: "Room B".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            exits: vec![ModuleExit {
                to: "room_a".to_string(),
                door: DoorState::Closed,
            }],
        });
        let module = ModuleDef {
            name: "Conflict".to_string(),
            level_range: (1, 2),
            entry_room: "room_a".to_string(),
            rooms,
        };
        let err = validate_module(&module).unwrap_err();
        assert!(err.contains("conflicting door states"), "expected conflict error, got: {}", err);
    }

    #[test]
    fn validate_matching_bidirectional_door_states() {
        let mut rooms = HashMap::new();
        rooms.insert("room_a".to_string(), ModuleRoom {
            name: "Room A".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            exits: vec![ModuleExit {
                to: "room_b".to_string(),
                door: DoorState::Locked,
            }],
        });
        rooms.insert("room_b".to_string(), ModuleRoom {
            name: "Room B".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            exits: vec![ModuleExit {
                to: "room_a".to_string(),
                door: DoorState::Locked,
            }],
        });
        let module = ModuleDef {
            name: "Match".to_string(),
            level_range: (1, 2),
            entry_room: "room_a".to_string(),
            rooms,
        };
        assert!(validate_module(&module).is_ok());
    }

    #[test]
    fn room_with_trap() {
        let json = r#"{
            "name": "Trap Room",
            "description": "Watch your step.",
            "trap": "Pit trap (1d6 damage)",
            "exits": []
        }"#;
        let room: ModuleRoom = serde_json::from_str(json).unwrap();
        assert_eq!(room.trap, Some("Pit trap (1d6 damage)".to_string()));
    }

    #[test]
    fn room_defaults() {
        let json = r#"{"name": "Empty Room"}"#;
        let room: ModuleRoom = serde_json::from_str(json).unwrap();
        assert_eq!(room.description, "");
        assert!(room.monsters.is_empty());
        assert!(room.treasure.is_empty());
        assert!(room.trap.is_none());
        assert!(room.exits.is_empty());
    }
}
