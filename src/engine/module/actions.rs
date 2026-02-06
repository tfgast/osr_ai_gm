use crate::engine::result::EngineError;
use crate::persist::GameState;
use crate::rules::module::{self as module_rules, ModuleDef, PlacedTreasure};
use crate::state::dungeon::{
    Door, DungeonState, PlacedMonsterInstance, PlacedTreasureInstance, Room,
};
use crate::state::game::GameMode;
use crate::state::time::TimeTracker;
use std::collections::HashMap;

use super::results::LoadModuleResult;

/// Convert a ModuleDef to a DungeonState.
///
/// This assigns numeric IDs to string-keyed rooms, creates Room structs,
/// and creates Door structs from exits (deduplicating bidirectional doors).
pub fn module_to_dungeon(module: &ModuleDef) -> Result<DungeonState, String> {
    let mut room_id_map: HashMap<String, u32> = HashMap::new();
    let mut next_id: u32 = 0;

    room_id_map.insert(module.entry_room.clone(), next_id);
    next_id += 1;

    let mut sorted_keys: Vec<&String> = module.rooms.keys().collect();
    sorted_keys.sort();
    for key in &sorted_keys {
        if !room_id_map.contains_key(*key) {
            room_id_map.insert((*key).clone(), next_id);
            next_id += 1;
        }
    }

    let mut dungeon = DungeonState::new(module.level_range.0);

    for key in &sorted_keys {
        let module_room = &module.rooms[*key];
        let id = *room_id_map.get(*key).unwrap();
        let mut room = Room::new(id, &module_room.name);
        room.description = module_room.description.clone();
        room.trap = module_room.trap.clone();
        room.key = Some((*key).clone());

        for placed_monster in &module_room.monsters {
            room.placed_monsters.push(PlacedMonsterInstance::new(
                &placed_monster.name,
                placed_monster.count,
            ));
        }

        for placed_treasure in &module_room.treasure {
            let (description, gp_value) = match placed_treasure {
                PlacedTreasure::Coins { gp } => (format!("{} gold pieces", gp), *gp),
                PlacedTreasure::Item { item } => (item.clone(), 0),
            };
            room.placed_treasure
                .push(PlacedTreasureInstance::new(&description, gp_value));
        }

        dungeon.add_room(room)?;
    }

    let entry_id = *room_id_map.get(&module.entry_room).unwrap();
    dungeon.current_room = Some(entry_id);
    dungeon.explored.insert(entry_id);

    let mut door_id: u32 = 0;
    let mut created_doors: HashMap<(u32, u32), u32> = HashMap::new();

    for key in &sorted_keys {
        let module_room = &module.rooms[*key];
        let room_a = *room_id_map.get(*key).unwrap();

        for exit in &module_room.exits {
            let room_b = *room_id_map
                .get(&exit.to)
                .ok_or_else(|| format!("room '{}' has exit to unknown room '{}'", key, exit.to))?;

            let pair = if room_a < room_b {
                (room_a, room_b)
            } else {
                (room_b, room_a)
            };

            if let std::collections::hash_map::Entry::Vacant(entry) = created_doors.entry(pair) {
                let door = Door::new(door_id, pair.0, pair.1, exit.door)?;
                dungeon.add_door(door)?;
                entry.insert(door_id);
                door_id += 1;
            }
        }
    }

    dungeon.log(format!("Module '{}' loaded.", module.name));

    Ok(dungeon)
}

pub fn action_load_module(
    state: &mut GameState,
    path: &str,
) -> Result<LoadModuleResult, EngineError> {
    let module_def = module_rules::load_module(path, module_rules::DEFAULT_MODULES_DIR)
        .map_err(EngineError::InvalidInput)?;
    let dungeon = module_to_dungeon(&module_def).map_err(EngineError::InvalidInput)?;

    let module_name = module_def.name.clone();
    let level_range = module_def.level_range;
    let room_count = dungeon.rooms.len();

    state.dungeon = Some(dungeon);
    state.time = Some(TimeTracker::new());
    state.dungeon_level = level_range.0;
    state.mode = GameMode::Exploration;

    Ok(LoadModuleResult {
        message: format!(
            "loaded module: {} (levels {}-{}). {} rooms.",
            module_name, level_range.0, level_range.1, room_count
        ),
        module_name,
        level_range,
        room_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::dungeon::DoorState;

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

        assert_eq!(dungeon.current_room, Some(0));
        assert!(dungeon.explored.contains(&0));

        let entry = dungeon.find_room(0).unwrap();
        assert_eq!(entry.name, "Crypt Entrance");
    }

    #[test]
    fn module_to_dungeon_creates_doors() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();
        assert_eq!(dungeon.doors.len(), 2);
    }

    #[test]
    fn module_to_dungeon_door_states() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();

        let locked_door = dungeon
            .doors
            .iter()
            .find(|d| d.state == DoorState::Locked)
            .expect("should have a locked door");

        let room_ids: Vec<u32> = vec![locked_door.room_a, locked_door.room_b];
        let guard_id = dungeon
            .rooms
            .iter()
            .find(|r| r.name == "Guard Chamber")
            .map(|r| r.id)
            .unwrap();
        let vault_id = dungeon
            .rooms
            .iter()
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

        let vault = dungeon.rooms.iter().find(|r| r.name == "Treasure Vault").unwrap();
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

    #[test]
    fn module_to_dungeon_loads_monsters() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();

        let guard = dungeon
            .rooms
            .iter()
            .find(|r| r.name == "Guard Chamber")
            .expect("should have Guard Chamber");

        assert_eq!(guard.placed_monsters.len(), 1);
        assert_eq!(guard.placed_monsters[0].name, "skeleton");
        assert_eq!(guard.placed_monsters[0].count, 3);
        assert!(!guard.placed_monsters[0].spawned);
        assert!(!guard.monsters_cleared);
    }

    #[test]
    fn module_to_dungeon_loads_treasure() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();

        let vault = dungeon
            .rooms
            .iter()
            .find(|r| r.name == "Treasure Vault")
            .expect("should have Treasure Vault");

        assert_eq!(vault.placed_treasure.len(), 2);
        assert!(vault.placed_treasure[0].description.contains("500"));
        assert_eq!(vault.placed_treasure[0].gp_value, 500);
        assert_eq!(vault.placed_treasure[1].description, "Potion of Healing");
        assert_eq!(vault.placed_treasure[1].gp_value, 0);
        assert!(!vault.treasure_looted);
    }

    #[test]
    fn module_to_dungeon_deterministic_room_ids() {
        let module = sample_module();
        let results: Vec<_> = (0..5)
            .map(|_| {
                let dungeon = module_to_dungeon(&module).unwrap();
                dungeon
                    .rooms
                    .iter()
                    .map(|r| (r.id, r.name.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        for result in &results[1..] {
            assert_eq!(
                &results[0], result,
                "room IDs should be deterministic across runs"
            );
        }
    }

    #[test]
    fn module_to_dungeon_sets_room_keys() {
        let module = sample_module();
        let dungeon = module_to_dungeon(&module).unwrap();

        let entry = dungeon.find_room(0).unwrap();
        assert_eq!(entry.key, Some("entrance".to_string()));

        let guard = dungeon
            .rooms
            .iter()
            .find(|r| r.name == "Guard Chamber")
            .expect("should have Guard Chamber");
        assert_eq!(guard.key, Some("guard".to_string()));
    }
}
