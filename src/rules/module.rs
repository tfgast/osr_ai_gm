//! Adventure module definitions loaded from JSON files.
//! Modules are prewritten dungeon adventures with rooms, monsters, treasure, and exits.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::pathutil::normalize_path;
use crate::state::dungeon::{DoorState, TrapTrigger};

/// A complete adventure module definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDef {
    pub name: String,
    pub level_range: (u32, u32),
    pub entry_room: String,
    /// Non-room content: introduction, background, hooks, lore, epilogue, etc.
    /// Keys are section names, values are the text content.
    #[serde(default)]
    pub sections: HashMap<String, String>,
    pub rooms: HashMap<String, ModuleRoom>,
    /// Dungeon levels. Keys are level identifiers (e.g. "surface", "depths").
    /// When defined, every room must be assigned to exactly one level.
    #[serde(default)]
    pub levels: HashMap<String, ModuleLevel>,
}

/// A dungeon level within a multi-level module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleLevel {
    pub name: String,
    pub dungeon_level: u32,
    /// Optional reference to a wandering monster table for this level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wandering_table: Option<String>,
    pub rooms: Vec<String>,
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
    /// How the trap is triggered: "entry" (auto on room entry, default) or
    /// "action" (requires character interaction).
    #[serde(default)]
    pub trap_trigger: TrapTrigger,
    #[serde(default)]
    pub exits: Vec<ModuleExit>,
}

/// A monster placement within a module room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedMonster {
    pub name: String,
    #[serde(default = "default_count")]
    pub count: u32,
    /// Whether this monster is undead (for Turn Undead).
    /// If omitted, looked up from the core monster database at spawn time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undead: Option<bool>,
}

fn default_count() -> u32 {
    1
}

/// Treasure placed in a module room.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlacedTreasure {
    Coins { gp: u64 },
    Item {
        item: String,
        #[serde(default)]
        value_gp: u64,
    },
}

/// Type of physical connection between rooms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConnectionType {
    #[serde(alias = "door")]
    #[default]
    Door,
    #[serde(alias = "stairs")]
    Stairs,
    #[serde(alias = "pit")]
    Pit,
    #[serde(alias = "ladder")]
    Ladder,
    #[serde(alias = "teleporter")]
    Teleporter,
    #[serde(alias = "custom")]
    Custom,
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ConnectionType::Door => "door",
            ConnectionType::Stairs => "stairs",
            ConnectionType::Pit => "pit",
            ConnectionType::Ladder => "ladder",
            ConnectionType::Teleporter => "teleporter",
            ConnectionType::Custom => "custom",
        };
        f.write_str(name)
    }
}

/// An exit from a module room to another room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleExit {
    pub to: String,
    #[serde(default)]
    pub door: DoorState,
    /// Type of physical connection (Door, Stairs, Pit, etc.). Defaults to Door.
    #[serde(default)]
    pub connection_type: ConnectionType,
    /// Freeform description of the exit (e.g. "narrow spiral staircase").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Expand a leading `~` to the user's home directory.
fn expand_tilde(path: &str) -> String {
    if path == "~" {
        std::env::var("HOME").unwrap_or_else(|_| path.to_string())
    } else if let Some(rest) = path.strip_prefix("~/") {
        match std::env::var("HOME") {
            Ok(home) => format!("{}/{}", home, rest),
            Err(_) => path.to_string(),
        }
    } else {
        path.to_string()
    }
}

/// Default directory for module files, relative to the working directory.
pub const DEFAULT_MODULES_DIR: &str = "data/modules";

/// Validate that a user-provided module path resolves within the allowed
/// modules directory. Returns the canonicalized path on success.
///
/// This prevents path traversal attacks where a user could read arbitrary
/// files by passing paths like `../../../../etc/passwd`.
fn validate_module_path(user_path: &str, modules_dir: &str) -> Result<PathBuf, String> {
    if user_path.contains('\0') {
        return Err("Module path must not contain null bytes.".to_string());
    }
    let expanded = expand_tilde(user_path);
    let path = Path::new(&expanded);

    // Resolve to absolute path
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| "Failed to resolve module path.".to_string())?
            .join(path)
    };

    // Normalize the path (resolve `.` and `..`) without hitting the filesystem,
    // so we can reject traversal even when the target file doesn't exist.
    let normalized = normalize_path(&absolute);

    // Normalize the base directory the same way, then canonicalize it
    let base_abs = if Path::new(modules_dir).is_absolute() {
        PathBuf::from(modules_dir)
    } else {
        std::env::current_dir()
            .map_err(|_| "Failed to resolve module path.".to_string())?
            .join(modules_dir)
    };
    let base_normalized = normalize_path(&base_abs);

    // Check that the normalized path is within the modules directory.
    // This catches traversal even if the file doesn't exist.
    if !normalized.starts_with(&base_normalized) {
        return Err("Module path must be within the modules directory.".to_string());
    }

    // Now verify the file actually exists and resolve symlinks
    let base_canonical = base_abs
        .canonicalize()
        .map_err(|_| "Modules directory not found.".to_string())?;

    let canonical = absolute
        .canonicalize()
        .map_err(|_| "Module file not found.".to_string())?;

    // Final check after symlink resolution
    if !canonical.starts_with(&base_canonical) {
        return Err("Module path must be within the modules directory.".to_string());
    }

    Ok(canonical)
}


/// Resolve and validate a module path, returning the canonical filesystem path.
///
/// Wraps the internal `validate_module_path` for use by callers that need
/// the resolved path (e.g. to locate companion files like `monsters.json`).
pub fn resolve_module_path(path: &str, modules_dir: &str) -> Result<PathBuf, String> {
    validate_module_path(path, modules_dir)
}

/// Load a module definition from a JSON file.
///
/// The path is validated to resolve within `modules_dir` to prevent
/// path traversal attacks. Pass [`DEFAULT_MODULES_DIR`] for the standard
/// location.
pub fn load_module(path: &str, modules_dir: &str) -> Result<ModuleDef, String> {
    let safe_path = validate_module_path(path, modules_dir)?;
    load_module_from_path(&safe_path)
}

/// Load a module definition from an already-validated filesystem path.
pub fn load_module_from_path(safe_path: &Path) -> Result<ModuleDef, String> {
    let content = fs::read_to_string(safe_path)
        .map_err(|_| "Failed to read module file.".to_string())?;
    let module: ModuleDef = serde_json::from_str(&content)
        .map_err(|_| "Module file contains invalid JSON.".to_string())?;

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

    // Validate levels if defined
    if !module.levels.is_empty() {
        let mut assigned_rooms: HashSet<&str> = HashSet::new();
        for (level_key, level) in &module.levels {
            for room_key in &level.rooms {
                if !module.rooms.contains_key(room_key) {
                    return Err(format!(
                        "Module '{}': level '{}' references non-existent room '{}'",
                        module.name, level_key, room_key
                    ));
                }
                if !assigned_rooms.insert(room_key.as_str()) {
                    return Err(format!(
                        "Module '{}': room '{}' assigned to multiple levels",
                        module.name, room_key
                    ));
                }
            }
        }
        // All rooms must be assigned to exactly one level
        for room_key in module.rooms.keys() {
            if !assigned_rooms.contains(room_key.as_str()) {
                return Err(format!(
                    "Module '{}': room '{}' not assigned to any level",
                    module.name, room_key
                ));
            }
        }
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
            PlacedTreasure::Item { item, value_gp } => {
                assert_eq!(item, "Potion of Healing");
                assert_eq!(*value_gp, 0);
            }
            _ => panic!("Expected item"),
        }
    }

    #[test]
    fn parse_item_treasure_with_value_gp() {
        let json = r#"{"item": "Pearl necklace (500gp)", "value_gp": 500}"#;
        let treasure: PlacedTreasure = serde_json::from_str(json).unwrap();
        match &treasure {
            PlacedTreasure::Item { item, value_gp } => {
                assert_eq!(item, "Pearl necklace (500gp)");
                assert_eq!(*value_gp, 500);
            }
            _ => panic!("Expected item with value_gp"),
        }
    }

    #[test]
    fn parse_item_treasure_without_value_gp_defaults_zero() {
        let json = r#"{"item": "Potion of Healing"}"#;
        let treasure: PlacedTreasure = serde_json::from_str(json).unwrap();
        match &treasure {
            PlacedTreasure::Item { item, value_gp } => {
                assert_eq!(item, "Potion of Healing");
                assert_eq!(*value_gp, 0);
            }
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
    fn default_connection_type_is_door() {
        let json = r#"{"to": "somewhere"}"#;
        let exit: ModuleExit = serde_json::from_str(json).unwrap();
        assert_eq!(exit.connection_type, ConnectionType::Door);
        assert!(exit.description.is_none());
    }

    #[test]
    fn parse_exit_with_connection_type_and_description() {
        let json = r#"{"to": "cellar", "door": "closed", "connection_type": "Stairs", "description": "narrow spiral staircase"}"#;
        let exit: ModuleExit = serde_json::from_str(json).unwrap();
        assert_eq!(exit.to, "cellar");
        assert_eq!(exit.door, DoorState::Closed);
        assert_eq!(exit.connection_type, ConnectionType::Stairs);
        assert_eq!(exit.description, Some("narrow spiral staircase".to_string()));
    }

    #[test]
    fn parse_all_connection_types() {
        for (json_val, expected) in [
            ("Door", ConnectionType::Door),
            ("door", ConnectionType::Door),
            ("Stairs", ConnectionType::Stairs),
            ("stairs", ConnectionType::Stairs),
            ("Pit", ConnectionType::Pit),
            ("pit", ConnectionType::Pit),
            ("Ladder", ConnectionType::Ladder),
            ("ladder", ConnectionType::Ladder),
            ("Teleporter", ConnectionType::Teleporter),
            ("teleporter", ConnectionType::Teleporter),
            ("Custom", ConnectionType::Custom),
            ("custom", ConnectionType::Custom),
        ] {
            let json = format!(r#"{{"to": "x", "connection_type": "{}"}}"#, json_val);
            let exit: ModuleExit = serde_json::from_str(&json).unwrap();
            assert_eq!(exit.connection_type, expected, "failed for {}", json_val);
        }
    }

    #[test]
    fn connection_type_display() {
        assert_eq!(format!("{}", ConnectionType::Door), "door");
        assert_eq!(format!("{}", ConnectionType::Stairs), "stairs");
        assert_eq!(format!("{}", ConnectionType::Pit), "pit");
        assert_eq!(format!("{}", ConnectionType::Ladder), "ladder");
        assert_eq!(format!("{}", ConnectionType::Teleporter), "teleporter");
        assert_eq!(format!("{}", ConnectionType::Custom), "custom");
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
            sections: HashMap::new(),
            rooms: HashMap::new(),
            levels: HashMap::new(),
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
            trap_trigger: TrapTrigger::Entry,
            exits: vec![ModuleExit {
                to: "nowhere".to_string(),
                door: DoorState::Closed,
                connection_type: ConnectionType::default(),
                description: None,
            }],
        });
        let module = ModuleDef {
            name: "Bad Exit".to_string(),
            level_range: (1, 2),
            entry_room: "start".to_string(),
            sections: HashMap::new(),
            rooms,
            levels: HashMap::new(),
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
            trap_trigger: TrapTrigger::Entry,
            exits: Vec::new(),
        });
        let module = ModuleDef {
            name: "Bad Range".to_string(),
            level_range: (5, 2), // Invalid: min > max
            entry_room: "start".to_string(),
            sections: HashMap::new(),
            rooms,
            levels: HashMap::new(),
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
            trap_trigger: TrapTrigger::Entry,
            exits: vec![ModuleExit {
                to: "room_b".to_string(),
                door: DoorState::Locked,
                connection_type: ConnectionType::default(),
                description: None,
            }],
        });
        rooms.insert("room_b".to_string(), ModuleRoom {
            name: "Room B".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            trap_trigger: TrapTrigger::Entry,
            exits: vec![ModuleExit {
                to: "room_a".to_string(),
                door: DoorState::Closed,
                connection_type: ConnectionType::default(),
                description: None,
            }],
        });
        let module = ModuleDef {
            name: "Conflict".to_string(),
            level_range: (1, 2),
            entry_room: "room_a".to_string(),
            sections: HashMap::new(),
            rooms,
            levels: HashMap::new(),
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
            trap_trigger: TrapTrigger::Entry,
            exits: vec![ModuleExit {
                to: "room_b".to_string(),
                door: DoorState::Locked,
                connection_type: ConnectionType::default(),
                description: None,
            }],
        });
        rooms.insert("room_b".to_string(), ModuleRoom {
            name: "Room B".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            trap_trigger: TrapTrigger::Entry,
            exits: vec![ModuleExit {
                to: "room_a".to_string(),
                door: DoorState::Locked,
                connection_type: ConnectionType::default(),
                description: None,
            }],
        });
        let module = ModuleDef {
            name: "Match".to_string(),
            level_range: (1, 2),
            entry_room: "room_a".to_string(),
            sections: HashMap::new(),
            rooms,
            levels: HashMap::new(),
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
        assert_eq!(room.trap_trigger, TrapTrigger::Entry);
        assert!(room.exits.is_empty());
    }

    #[test]
    fn room_with_action_trap_trigger() {
        let json = r#"{
            "name": "Freezing Mirror",
            "description": "A full-length mirror of dark glass dominates the room.",
            "trap": "Save vs paralysis or be frozen still",
            "trap_trigger": "Action",
            "exits": []
        }"#;
        let room: ModuleRoom = serde_json::from_str(json).unwrap();
        assert_eq!(room.trap, Some("Save vs paralysis or be frozen still".to_string()));
        assert_eq!(room.trap_trigger, TrapTrigger::Action);
    }

    #[test]
    fn room_trap_trigger_defaults_to_entry() {
        let json = r#"{
            "name": "Trap Room",
            "trap": "Pit trap (1d6 damage)",
            "exits": []
        }"#;
        let room: ModuleRoom = serde_json::from_str(json).unwrap();
        assert_eq!(room.trap_trigger, TrapTrigger::Entry);
    }

    #[test]
    fn parse_module_with_sections() {
        let json = r#"{
            "name": "Test Module",
            "level_range": [1, 3],
            "entry_room": "start",
            "sections": {
                "introduction": "A short adventure for levels 1-3.",
                "background": "Long ago, a knight fell in love with a fairy princess.",
                "hooks": "A recurring dream leads the PCs to a burial mound."
            },
            "rooms": {
                "start": {
                    "name": "Entrance",
                    "exits": []
                }
            }
        }"#;
        let module: ModuleDef = serde_json::from_str(json).unwrap();
        assert_eq!(module.sections.len(), 3);
        assert!(module.sections["introduction"].contains("short adventure"));
        assert!(module.sections.contains_key("background"));
        assert!(module.sections.contains_key("hooks"));
    }

    #[test]
    fn sections_default_to_empty() {
        let module: ModuleDef = serde_json::from_str(sample_module_json()).unwrap();
        assert!(module.sections.is_empty());
    }

    #[test]
    fn expand_tilde_with_subpath() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_tilde("~/foo/bar.json"), format!("{}/foo/bar.json", home));
    }

    #[test]
    fn expand_tilde_alone() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_tilde("~"), home);
    }

    #[test]
    fn expand_tilde_no_tilde() {
        assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
        assert_eq!(expand_tilde("relative/path"), "relative/path");
    }

    #[test]
    fn validate_path_within_modules_dir() {
        // Valid path within data/modules should succeed
        let result = validate_module_path(
            "data/modules/sample_crypt/module.json",
            "data/modules",
        );
        assert!(result.is_ok(), "valid module path should succeed: {:?}", result);
    }

    #[test]
    fn validate_path_traversal_rejected() {
        // Path traversal should be rejected
        let result = validate_module_path(
            "data/modules/../../Cargo.toml",
            "data/modules",
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("must be within the modules directory"),
            "expected modules directory error, got: {}",
            err
        );
    }

    #[test]
    fn validate_absolute_path_outside_modules_rejected() {
        // Absolute path outside modules dir should be rejected
        let result = validate_module_path("/etc/passwd", "data/modules");
        assert!(result.is_err());
    }

    #[test]
    fn validate_nonexistent_file_rejected() {
        // Nonexistent file should fail at canonicalization
        let result = validate_module_path(
            "data/modules/nonexistent/module.json",
            "data/modules",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn load_module_valid_path() {
        let result = load_module(
            "data/modules/sample_crypt/module.json",
            "data/modules",
        );
        assert!(result.is_ok(), "should load valid module: {:?}", result);
        assert_eq!(result.unwrap().name, "Sample Crypt");
    }

    #[test]
    fn load_module_traversal_blocked() {
        let result = load_module(
            "data/modules/../../Cargo.toml",
            "data/modules",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be within the modules directory"));
    }

    #[test]
    fn validate_null_byte_rejected() {
        let result = validate_module_path(
            "data/modules/sample_crypt\0/../../etc/passwd",
            "data/modules",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("null bytes"));
    }

    #[test]
    fn validate_tilde_traversal_rejected() {
        // ~ expands to HOME but should still be checked against modules dir
        let result = validate_module_path("~/../../etc/passwd", "data/modules");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be within the modules directory"));
    }

    #[test]
    fn validate_deeply_nested_traversal_rejected() {
        let result = validate_module_path(
            "data/modules/a/b/c/../../../../../etc/shadow",
            "data/modules",
        );
        assert!(result.is_err());
    }

    #[test]
    fn validate_dot_only_path_rejected() {
        // "." resolves to cwd which is not within data/modules
        let result = validate_module_path(".", "data/modules");
        assert!(result.is_err());
    }

    #[test]
    fn validate_empty_path_rejected() {
        // Empty string resolves to cwd
        let result = validate_module_path("", "data/modules");
        assert!(result.is_err());
    }

    #[test]
    fn load_module_from_absolute_dir() {
        // Simulates loading from data_dir()/modules/ by using a temp directory
        let tmp = std::env::temp_dir().join("osr_test_modules_abs");
        let sub = tmp.join("test_mod");
        std::fs::create_dir_all(&sub).unwrap();
        let module_path = sub.join("module.json");
        std::fs::write(&module_path, sample_module_json()).unwrap();

        let dir_str = tmp.to_string_lossy().to_string();
        let file_path = format!("{}/test_mod/module.json", dir_str);
        let result = load_module(&file_path, &dir_str);
        assert!(result.is_ok(), "should load from absolute dir: {:?}", result);
        assert_eq!(result.unwrap().name, "Test Crypt");

        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn validate_path_traversal_from_absolute_dir_rejected() {
        // Even with an absolute modules dir, path traversal should be rejected
        let tmp = std::env::temp_dir().join("osr_test_modules_trav");
        std::fs::create_dir_all(&tmp).unwrap();

        let dir_str = tmp.to_string_lossy().to_string();
        let result = validate_module_path(
            &format!("{}/../etc/passwd", dir_str),
            &dir_str,
        );
        assert!(result.is_err());

        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn parse_module_with_levels() {
        let json = r#"{
            "name": "Multi-Level Dungeon",
            "level_range": [1, 3],
            "entry_room": "entrance",
            "levels": {
                "surface": {
                    "name": "Surface Level",
                    "dungeon_level": 1,
                    "rooms": ["entrance", "guard"]
                },
                "depths": {
                    "name": "The Depths",
                    "dungeon_level": 2,
                    "wandering_table": "depths_wandering",
                    "rooms": ["vault"]
                }
            },
            "rooms": {
                "entrance": {
                    "name": "Cave Mouth",
                    "exits": [{"to": "guard", "door": "open"}]
                },
                "guard": {
                    "name": "Guard Post",
                    "exits": [
                        {"to": "entrance", "door": "open"},
                        {"to": "vault", "door": "closed", "connection_type": "Stairs"}
                    ]
                },
                "vault": {
                    "name": "Deep Vault",
                    "exits": [{"to": "guard", "door": "closed", "connection_type": "Stairs"}]
                }
            }
        }"#;
        let module: ModuleDef = serde_json::from_str(json).unwrap();
        assert_eq!(module.levels.len(), 2);
        let surface = &module.levels["surface"];
        assert_eq!(surface.name, "Surface Level");
        assert_eq!(surface.dungeon_level, 1);
        assert_eq!(surface.rooms, vec!["entrance", "guard"]);
        assert!(surface.wandering_table.is_none());
        let depths = &module.levels["depths"];
        assert_eq!(depths.name, "The Depths");
        assert_eq!(depths.dungeon_level, 2);
        assert_eq!(depths.wandering_table, Some("depths_wandering".to_string()));
        assert_eq!(depths.rooms, vec!["vault"]);
    }

    #[test]
    fn levels_default_to_empty() {
        let module: ModuleDef = serde_json::from_str(sample_module_json()).unwrap();
        assert!(module.levels.is_empty());
    }

    #[test]
    fn validate_level_references_nonexistent_room() {
        let mut rooms = HashMap::new();
        rooms.insert("start".to_string(), ModuleRoom {
            name: "Start".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            trap_trigger: TrapTrigger::Entry,
            exits: Vec::new(),
        });
        let mut levels = HashMap::new();
        levels.insert("floor1".to_string(), ModuleLevel {
            name: "Floor 1".to_string(),
            dungeon_level: 1,
            wandering_table: None,
            rooms: vec!["start".to_string(), "nonexistent".to_string()],
        });
        let module = ModuleDef {
            name: "Bad Level Ref".to_string(),
            level_range: (1, 2),
            entry_room: "start".to_string(),
            sections: HashMap::new(),
            rooms,
            levels,
        };
        let err = validate_module(&module).unwrap_err();
        assert!(err.contains("non-existent room"), "expected non-existent room error, got: {}", err);
    }

    #[test]
    fn validate_room_in_multiple_levels() {
        let mut rooms = HashMap::new();
        rooms.insert("shared".to_string(), ModuleRoom {
            name: "Shared Room".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            trap_trigger: TrapTrigger::Entry,
            exits: Vec::new(),
        });
        let mut levels = HashMap::new();
        levels.insert("floor1".to_string(), ModuleLevel {
            name: "Floor 1".to_string(),
            dungeon_level: 1,
            wandering_table: None,
            rooms: vec!["shared".to_string()],
        });
        levels.insert("floor2".to_string(), ModuleLevel {
            name: "Floor 2".to_string(),
            dungeon_level: 2,
            wandering_table: None,
            rooms: vec!["shared".to_string()],
        });
        let module = ModuleDef {
            name: "Multi Assign".to_string(),
            level_range: (1, 2),
            entry_room: "shared".to_string(),
            sections: HashMap::new(),
            rooms,
            levels,
        };
        let err = validate_module(&module).unwrap_err();
        assert!(err.contains("multiple levels"), "expected multiple levels error, got: {}", err);
    }

    #[test]
    fn validate_room_not_assigned_to_level() {
        let mut rooms = HashMap::new();
        rooms.insert("assigned".to_string(), ModuleRoom {
            name: "Assigned".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            trap_trigger: TrapTrigger::Entry,
            exits: Vec::new(),
        });
        rooms.insert("orphan".to_string(), ModuleRoom {
            name: "Orphan".to_string(),
            description: String::new(),
            monsters: Vec::new(),
            treasure: Vec::new(),
            trap: None,
            trap_trigger: TrapTrigger::Entry,
            exits: Vec::new(),
        });
        let mut levels = HashMap::new();
        levels.insert("floor1".to_string(), ModuleLevel {
            name: "Floor 1".to_string(),
            dungeon_level: 1,
            wandering_table: None,
            rooms: vec!["assigned".to_string()],
        });
        let module = ModuleDef {
            name: "Missing Room".to_string(),
            level_range: (1, 2),
            entry_room: "assigned".to_string(),
            sections: HashMap::new(),
            rooms,
            levels,
        };
        let err = validate_module(&module).unwrap_err();
        assert!(err.contains("not assigned to any level"), "expected unassigned error, got: {}", err);
    }

    #[test]
    fn validate_module_with_valid_levels() {
        let json = r#"{
            "name": "Valid Multi-Level",
            "level_range": [1, 3],
            "entry_room": "entrance",
            "levels": {
                "surface": {
                    "name": "Surface",
                    "dungeon_level": 1,
                    "rooms": ["entrance"]
                },
                "depths": {
                    "name": "Depths",
                    "dungeon_level": 2,
                    "rooms": ["vault"]
                }
            },
            "rooms": {
                "entrance": {
                    "name": "Entrance",
                    "exits": [{"to": "vault", "door": "closed", "connection_type": "Stairs"}]
                },
                "vault": {
                    "name": "Vault",
                    "exits": [{"to": "entrance", "door": "closed", "connection_type": "Stairs"}]
                }
            }
        }"#;
        let module: ModuleDef = serde_json::from_str(json).unwrap();
        assert!(validate_module(&module).is_ok());
    }
}
