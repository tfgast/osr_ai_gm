use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::rules::module::{load_module, ModuleDef, PlacedTreasure};
use crate::state::dungeon::{Door, DoorState, DungeonState, Room};
use crate::state::game::GameMode;
use crate::state::time::TimeTracker;
use std::collections::HashMap;

/// Load a prewritten adventure module from a JSON file.
pub struct LoadModuleCommand;

impl Command for LoadModuleCommand {
    fn name(&self) -> &str {
        "load_module"
    }

    fn help(&self) -> &str {
        "Load an adventure module (load_module <path>)"
    }

    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: load_module <path>");
        }

        let path = args.join(" ");
        let module = match load_module(&path) {
            Ok(m) => m,
            Err(e) => return CommandResult::error(e),
        };

        let dungeon = match module_to_dungeon(&module) {
            Ok(d) => d,
            Err(e) => return CommandResult::error(e),
        };

        let module_name = module.name.clone();
        let level_range = module.level_range;
        let room_count = dungeon.rooms.len();

        state.dungeon = Some(dungeon);
        state.time = Some(TimeTracker::new());
        state.dungeon_level = level_range.0;
        state.mode = GameMode::Exploration;

        CommandResult::ok(format!(
            "Loaded module: {} (levels {}-{}). {} rooms.\n\
             Use 'light torch <carrier>' or 'light lantern <carrier>' to light the way.\n\
             Use 'exploration_status' to see current position.",
            module_name, level_range.0, level_range.1, room_count
        ))
    }
}

/// Convert a ModuleDef to a DungeonState.
///
/// This assigns numeric IDs to string-keyed rooms, creates Room structs,
/// and creates Door structs from exits (deduplicating bidirectional doors).
pub fn module_to_dungeon(module: &ModuleDef) -> Result<DungeonState, String> {
    // Assign numeric IDs to rooms
    let mut room_id_map: HashMap<String, u32> = HashMap::new();
    let mut next_id: u32 = 0;

    // Ensure entry_room gets ID 0
    room_id_map.insert(module.entry_room.clone(), next_id);
    next_id += 1;

    // Assign IDs to remaining rooms
    for key in module.rooms.keys() {
        if !room_id_map.contains_key(key) {
            room_id_map.insert(key.clone(), next_id);
            next_id += 1;
        }
    }

    // Create dungeon with level from module
    let mut dungeon = DungeonState::new(module.level_range.0);

    // Create Room structs
    for (key, module_room) in &module.rooms {
        let id = *room_id_map.get(key).unwrap();
        let mut room = Room::new(id, &module_room.name);
        room.description = module_room.description.clone();
        room.trap = module_room.trap.clone();

        dungeon.add_room(room)?;
    }

    // Set entry room as current room
    let entry_id = *room_id_map.get(&module.entry_room).unwrap();
    dungeon.current_room = Some(entry_id);
    dungeon.explored.insert(entry_id);

    // Create doors from exits (deduplicate bidirectional)
    let mut door_id: u32 = 0;
    let mut created_doors: HashMap<(u32, u32), u32> = HashMap::new();

    for (key, module_room) in &module.rooms {
        let room_a = *room_id_map.get(key).unwrap();

        for exit in &module_room.exits {
            let room_b = *room_id_map.get(&exit.to).ok_or_else(|| {
                format!("room '{}' has exit to unknown room '{}'", key, exit.to)
            })?;

            // Normalize the room pair to avoid duplicate doors
            let pair = if room_a < room_b {
                (room_a, room_b)
            } else {
                (room_b, room_a)
            };

            // Only create door if we haven't seen this pair
            if !created_doors.contains_key(&pair) {
                let door = Door::new(door_id, pair.0, pair.1, exit.door)?;
                dungeon.add_door(door)?;
                created_doors.insert(pair, door_id);
                door_id += 1;
            }
        }
    }

    // Log module load
    dungeon.log(format!("Module '{}' loaded.", module.name));

    Ok(dungeon)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_module() -> ModuleDef {
        let json = r#"{
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
        }"#;
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn module_to_dungeon_creates_rooms() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();
        assert_eq!(dungeon.rooms.len(), 3);
    }

    #[test]
    fn module_to_dungeon_entry_room_is_current() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();

        // Entry room should be ID 0 and current
        assert_eq!(dungeon.current_room, Some(0));

        // Entry room should be explored
        assert!(dungeon.explored.contains(&0));

        // Entry room should be named "Crypt Entrance"
        let entry = dungeon.find_room(0).unwrap();
        assert_eq!(entry.name, "Crypt Entrance");
    }

    #[test]
    fn module_to_dungeon_creates_doors() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();

        // Should have 2 doors (entrance-guard, guard-vault)
        // Even though exits are defined bidirectionally, doors are deduplicated
        assert_eq!(dungeon.doors.len(), 2);
    }

    #[test]
    fn module_to_dungeon_door_states() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();

        // Find the locked door (guard-vault)
        let locked_door = dungeon.doors.iter()
            .find(|d| d.state == DoorState::Locked)
            .expect("should have a locked door");

        // Verify it connects the right rooms
        let room_ids: Vec<u32> = vec![locked_door.room_a, locked_door.room_b];
        let guard_id = dungeon.rooms.iter()
            .find(|r| r.name == "Guard Chamber")
            .map(|r| r.id)
            .unwrap();
        let vault_id = dungeon.rooms.iter()
            .find(|r| r.name == "Treasure Vault")
            .map(|r| r.id)
            .unwrap();

        assert!(room_ids.contains(&guard_id));
        assert!(room_ids.contains(&vault_id));
    }

    #[test]
    fn module_to_dungeon_room_descriptions() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();

        let vault = dungeon.rooms.iter()
            .find(|r| r.name == "Treasure Vault")
            .unwrap();
        assert_eq!(vault.description, "A dusty chest sits in the corner.");
    }

    #[test]
    fn module_to_dungeon_sets_level() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();
        assert_eq!(dungeon.level, 1);
    }

    #[test]
    fn module_to_dungeon_logs_load() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();
        assert!(dungeon.log.iter().any(|m| m.contains("Test Crypt")));
    }
}
