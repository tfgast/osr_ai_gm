use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::log_entry::LogEntry;
use crate::rules::module::ConnectionType;

/// How a trap is triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TrapTrigger {
    /// Trap rolls automatically when entering the room (1-2 on d6).
    #[serde(alias = "entry")]
    #[default]
    Entry,
    /// Trap triggers based on character action (e.g., interacting with an object).
    /// Does NOT auto-trigger on room entry.
    #[serde(alias = "action")]
    Action,
}

/// State of a door in the dungeon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DoorState {
    /// Standard door — must force open (2-in-6 base, modified by STR).
    #[serde(alias = "closed")]
    #[default]
    Closed,
    /// Door is open (forced or otherwise). Per OSE, doors close automatically.
    #[serde(alias = "open")]
    Open,
    /// Stuck shut — requires forcing.
    #[serde(alias = "stuck")]
    Stuck,
    /// Locked — requires a key or thief lockpicking.
    #[serde(alias = "locked")]
    Locked,
    /// Secret door — must be found by searching (1-in-6, elves 2-in-6).
    #[serde(alias = "secret")]
    Secret,
    /// Spiked open — door has been held with iron spikes, won't auto-close.
    #[serde(alias = "spiked")]
    Spiked,
}

impl fmt::Display for DoorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DoorState::Closed => "closed",
            DoorState::Open => "open",
            DoorState::Stuck => "stuck",
            DoorState::Locked => "locked",
            DoorState::Secret => "secret",
            DoorState::Spiked => "spiked",
        };
        f.write_str(name)
    }
}

impl FromStr for DoorState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(DoorState::Open),
            "closed" => Ok(DoorState::Closed),
            "stuck" => Ok(DoorState::Stuck),
            "locked" => Ok(DoorState::Locked),
            "secret" => Ok(DoorState::Secret),
            "spiked" => Ok(DoorState::Spiked),
            _ => Err(format!(
                "invalid door state '{}': must be open, closed, stuck, locked, secret, or spiked",
                s
            )),
        }
    }
}

/// A door in the dungeon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Door {
    pub id: u32,
    /// Connects room_a to room_b.
    pub room_a: u32,
    pub room_b: u32,
    pub state: DoorState,
    /// Whether the door has been discovered (relevant for secret doors).
    pub discovered: bool,
    /// Whether the door was defined as open by a module (permanent passage).
    /// Module-open doors represent archways, open gates, or collapsed walls
    /// and do not auto-close when passed through.
    #[serde(default)]
    pub module_open: bool,
    /// Type of physical connection (Door, Stairs, Pit, etc.).
    #[serde(default)]
    pub connection_type: ConnectionType,
}

impl Door {
    pub fn new(id: u32, room_a: u32, room_b: u32, state: DoorState) -> Result<Self, String> {
        if room_a == room_b {
            return Err(format!("door {} connects room {} to itself", id, room_a));
        }
        let discovered = state != DoorState::Secret;
        Ok(Door { id, room_a, room_b, state, discovered, module_open: false, connection_type: ConnectionType::default() })
    }

    /// Whether the party can attempt to pass through this door.
    pub fn is_passable(&self) -> bool {
        self.state == DoorState::Open || self.state == DoorState::Spiked
    }
}

/// A placed monster instance from a module, tracking spawn state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedMonsterInstance {
    pub name: String,
    pub count: u32,
    /// Whether this monster group has been spawned into combat.
    #[serde(default)]
    pub spawned: bool,
    /// Whether this monster is undead (for Turn Undead).
    /// Propagated from the module definition so the AI GM can pass it to SpawnEncounter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undead: Option<bool>,
}

impl PlacedMonsterInstance {
    pub fn new(name: &str, count: u32) -> Self {
        PlacedMonsterInstance {
            name: name.to_string(),
            count,
            spawned: false,
            undead: None,
        }
    }
}

/// A placed treasure instance from a module, tracking loot state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedTreasureInstance {
    pub description: String,
    pub gp_value: u64,
    /// Whether this treasure has been taken.
    #[serde(default)]
    pub taken: bool,
}

impl PlacedTreasureInstance {
    pub fn new(description: &str, gp_value: u64) -> Self {
        PlacedTreasureInstance {
            description: description.to_string(),
            gp_value,
            taken: false,
        }
    }
}

/// A room or area in the dungeon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub searched: bool,
    pub trap: Option<String>,
    pub trap_triggered: bool,
    /// How the trap is triggered: automatically on entry, or by character action.
    #[serde(default)]
    pub trap_trigger: TrapTrigger,
    /// Module room key for cross-referencing (e.g., "entrance", "guard").
    #[serde(default)]
    pub key: Option<String>,
    /// Level key this room belongs to (for multi-level modules).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_key: Option<String>,
    /// Monsters placed by module definition.
    #[serde(default)]
    pub placed_monsters: Vec<PlacedMonsterInstance>,
    /// Treasure placed by module definition.
    #[serde(default)]
    pub placed_treasure: Vec<PlacedTreasureInstance>,
    /// Whether all placed monsters have been defeated.
    #[serde(default)]
    pub monsters_cleared: bool,
    /// Whether placed treasure has been looted.
    #[serde(default)]
    pub treasure_looted: bool,
    /// Notable features in the room (from module definition).
    #[serde(default)]
    pub features: Vec<RoomFeatureInstance>,
    /// Descriptive tags (e.g., "dark", "flooded", "sacred").
    #[serde(default)]
    pub tags: Vec<String>,
    /// Boxed text to read aloud when first entering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_aloud: Option<String>,
    /// Private GM notes about this room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gm_notes: Option<String>,
}

/// A room feature instance in the runtime dungeon state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomFeatureInstance {
    pub name: String,
    pub description: String,
    pub kind: String,
    /// What happens when players interact with this feature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction: Option<String>,
}

impl Room {
    pub fn new(id: u32, name: &str) -> Self {
        Room {
            id,
            name: name.to_string(),
            description: String::new(),
            searched: false,
            trap: None,
            trap_triggered: false,
            trap_trigger: TrapTrigger::Entry,
            key: None,
            level_key: None,
            placed_monsters: Vec::new(),
            placed_treasure: Vec::new(),
            monsters_cleared: false,
            treasure_looted: false,
            features: Vec::new(),
            tags: Vec::new(),
            read_aloud: None,
            gm_notes: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_trap(mut self, trap: &str) -> Self {
        self.trap = Some(trap.to_string());
        self
    }

    pub fn with_trap_trigger(mut self, trigger: TrapTrigger) -> Self {
        self.trap_trigger = trigger;
        self
    }

    pub fn with_key(mut self, key: &str) -> Self {
        self.key = Some(key.to_string());
        self
    }

    pub fn with_placed_monsters(mut self, monsters: Vec<PlacedMonsterInstance>) -> Self {
        self.placed_monsters = monsters;
        self
    }

    pub fn with_placed_treasure(mut self, treasure: Vec<PlacedTreasureInstance>) -> Self {
        self.placed_treasure = treasure;
        self
    }
}

/// Result of a cross-level movement (e.g., going down stairs to a deeper level).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelTransition {
    pub from_level: u32,
    pub to_level: u32,
}

/// Tracks the dungeon map: rooms, doors, exploration state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonState {
    pub level: u32,
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    /// IDs of rooms the party has visited.
    pub explored: HashSet<u32>,
    /// The room the party is currently in (None if no rooms exist yet).
    pub current_room: Option<u32>,
    /// Event log.
    pub log: Vec<LogEntry>,
    /// Monotonic sequence counter for log entry ordering.
    #[serde(default)]
    pub log_seq: u64,
    /// Current level key in a multi-level module (e.g., "surface", "depths").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_level_key: Option<String>,
    /// Maps level keys to dungeon level numbers for cross-level transitions.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub level_map: HashMap<String, u32>,
}

impl DungeonState {
    /// Maximum log entries retained before oldest are dropped.
    const MAX_LOG_ENTRIES: usize = 1000;

    pub fn new(level: u32) -> Self {
        DungeonState {
            level,
            rooms: Vec::new(),
            doors: Vec::new(),
            explored: HashSet::new(),
            current_room: None,
            log: Vec::new(),
            log_seq: 0,
            current_level_key: None,
            level_map: HashMap::new(),
        }
    }

    /// Add a room to the dungeon. Returns an error if a room with the same
    /// ID already exists.
    pub fn add_room(&mut self, room: Room) -> Result<(), String> {
        if self.rooms.iter().any(|r| r.id == room.id) {
            return Err(format!("duplicate room id {}", room.id));
        }
        if self.current_room.is_none() {
            self.current_room = Some(room.id);
        }
        self.rooms.push(room);
        Ok(())
    }

    /// Add a door between two rooms. Returns an error if a door with the
    /// same ID already exists or if either room does not exist.
    pub fn add_door(&mut self, door: Door) -> Result<(), String> {
        if self.doors.iter().any(|d| d.id == door.id) {
            return Err(format!("duplicate door id {}", door.id));
        }
        if self.find_room(door.room_a).is_none() {
            return Err(format!("door {} references non-existent room_a {}", door.id, door.room_a));
        }
        if self.find_room(door.room_b).is_none() {
            return Err(format!("door {} references non-existent room_b {}", door.id, door.room_b));
        }
        self.doors.push(door);
        Ok(())
    }

    /// Mark the current room as explored.
    pub fn explore_current(&mut self) {
        if let Some(room) = self.current_room {
            self.explored.insert(room);
        }
    }

    /// Move to a room by ID. Requires a passable door connecting the
    /// current room to the destination (prevents teleportation).
    /// If the destination room is on a different level, updates
    /// `current_level_key` and `level` accordingly.
    pub fn move_to(&mut self, room_id: u32) -> Result<Option<LevelTransition>, String> {
        let current = self.current_room
            .ok_or_else(|| "no current room set".to_string())?;
        if !self.rooms.iter().any(|r| r.id == room_id) {
            return Err(format!("room {} does not exist", room_id));
        }
        // Check that a passable door connects current room to destination
        let connected = self.doors.iter().any(|d| {
            d.is_passable()
                && ((d.room_a == current && d.room_b == room_id)
                    || (d.room_b == current && d.room_a == room_id))
        });
        if !connected {
            return Err(format!(
                "no open door between room {} and room {}",
                current, room_id
            ));
        }

        // Check for cross-level transition
        let dest_level_key = self.rooms.iter()
            .find(|r| r.id == room_id)
            .and_then(|r| r.level_key.clone());
        let transition = if let Some(ref dlk) = dest_level_key {
            if self.current_level_key.as_ref() != Some(dlk) {
                if let Some(&new_dl) = self.level_map.get(dlk) {
                    let old_level = self.level;
                    self.level = new_dl;
                    self.current_level_key = Some(dlk.clone());
                    Some(LevelTransition { from_level: old_level, to_level: new_dl })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        self.current_room = Some(room_id);
        self.explored.insert(room_id);
        Ok(transition)
    }

    /// Find a room by ID.
    pub fn find_room(&self, id: u32) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id == id)
    }

    /// Find a room by ID, mutably.
    pub fn find_room_mut(&mut self, id: u32) -> Option<&mut Room> {
        self.rooms.iter_mut().find(|r| r.id == id)
    }

    /// Find a door by ID.
    pub fn find_door_mut(&mut self, id: u32) -> Option<&mut Door> {
        self.doors.iter_mut().find(|d| d.id == id)
    }

    /// Get all doors connected to the current room.
    pub fn doors_from_current(&self) -> Vec<&Door> {
        let current = match self.current_room {
            Some(id) => id,
            None => return Vec::new(),
        };
        self.doors
            .iter()
            .filter(|d| d.room_a == current || d.room_b == current)
            .filter(|d| d.discovered)
            .collect()
    }

    /// Append a message to the log, capping at MAX_LOG_ENTRIES.
    pub fn log(&mut self, msg: String) {
        if self.log.len() >= Self::MAX_LOG_ENTRIES {
            let drain = self.log.len() - Self::MAX_LOG_ENTRIES / 2;
            self.log.drain(..drain);
        }
        self.log_seq += 1;
        self.log.push(LogEntry::new(self.log_seq, msg));
    }

    /// Status display of the current position.
    pub fn status(&self) -> String {
        let (room_id_str, room_name) = match self.current_room {
            Some(id) => {
                let name = self.find_room(id)
                    .map(|r| r.name.as_str())
                    .unwrap_or("unknown");
                (id.to_string(), name)
            }
            None => ("none".to_string(), "no rooms"),
        };
        let level_label = match &self.current_level_key {
            Some(key) => format!("Dungeon Level: {} ({})", self.level, key),
            None => format!("Dungeon Level: {}", self.level),
        };
        let mut out = format!(
            "{}  Room: {} ({})\nExplored: {} rooms",
            level_label, room_id_str, room_name, self.explored.len()
        );
        let doors = self.doors_from_current();
        if !doors.is_empty() {
            let current = self.current_room.unwrap();
            out.push_str("\nExits:");
            for d in &doors {
                let dest = if d.room_a == current { d.room_b } else { d.room_a };
                let dest_name = self.find_room(dest)
                    .map(|r| r.name.as_str())
                    .unwrap_or("?");
                let state = match d.state {
                    DoorState::Open => "open",
                    DoorState::Closed => "closed",
                    DoorState::Stuck => "stuck",
                    DoorState::Locked => "locked",
                    DoorState::Secret => "secret",
                    DoorState::Spiked => "spiked open",
                };
                let conn_label = match d.connection_type {
                    ConnectionType::Door => "Door",
                    ConnectionType::Stairs => "Stairs",
                    ConnectionType::Pit => "Pit",
                    ConnectionType::Ladder => "Ladder",
                    ConnectionType::Teleporter => "Teleporter",
                    ConnectionType::Custom => "Passage",
                };
                out.push_str(&format!("\n  {} {} → {} ({}) [{}]", conn_label, d.id, dest, dest_name, state));
            }
        }
        out
    }
}

impl Default for DungeonState {
    fn default() -> Self {
        Self::new(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dungeon() -> DungeonState {
        let mut ds = DungeonState::new(1);
        ds.add_room(Room::new(0, "Entrance")).unwrap();
        ds.add_room(Room::new(1, "Guard Room")).unwrap();
        ds.add_room(Room::new(2, "Hidden Chamber")).unwrap();
        ds.add_door(Door::new(0, 0, 1, DoorState::Closed).unwrap()).unwrap();
        ds.add_door(Door::new(1, 1, 2, DoorState::Secret).unwrap()).unwrap();
        ds
    }

    #[test]
    fn move_to_room_through_open_door() {
        let mut ds = sample_dungeon();
        assert_eq!(ds.current_room, Some(0));
        // Open the door first
        ds.find_door_mut(0).unwrap().state = DoorState::Open;
        ds.move_to(1).unwrap();
        assert_eq!(ds.current_room, Some(1));
        assert!(ds.explored.contains(&1));
    }

    #[test]
    fn move_to_nonexistent_fails() {
        let mut ds = sample_dungeon();
        assert!(ds.move_to(99).is_err());
    }

    #[test]
    fn move_to_without_open_door_fails() {
        let mut ds = sample_dungeon();
        // Door 0 is closed — can't move through
        assert!(ds.move_to(1).is_err());
    }

    #[test]
    fn move_to_nonadjacent_fails() {
        let mut ds = sample_dungeon();
        // No door from room 0 to room 2
        assert!(ds.move_to(2).is_err());
    }

    #[test]
    fn doors_from_current_excludes_secret() {
        let ds = sample_dungeon();
        // From room 0, only door 0 (to room 1) is visible
        let doors = ds.doors_from_current();
        assert_eq!(doors.len(), 1);
        assert_eq!(doors[0].id, 0);
    }

    #[test]
    fn secret_door_discovered() {
        let mut ds = sample_dungeon();
        // Open door 0 so we can move to room 1
        ds.find_door_mut(0).unwrap().state = DoorState::Open;
        ds.move_to(1).unwrap();
        // Secret door to room 2 is not discovered
        let doors = ds.doors_from_current();
        assert_eq!(doors.len(), 1); // only door 0 back to entrance
        // Discover the secret door
        ds.find_door_mut(1).unwrap().discovered = true;
        let doors = ds.doors_from_current();
        assert_eq!(doors.len(), 2);
    }

    #[test]
    fn door_passable_when_open() {
        let door = Door::new(0, 0, 1, DoorState::Open).unwrap();
        assert!(door.is_passable());

        let door = Door::new(0, 0, 1, DoorState::Closed).unwrap();
        assert!(!door.is_passable());

        let door = Door::new(0, 0, 1, DoorState::Locked).unwrap();
        assert!(!door.is_passable());
    }

    #[test]
    fn door_self_connection_rejected() {
        assert!(Door::new(0, 1, 1, DoorState::Closed).is_err());
    }

    #[test]
    fn duplicate_room_id_rejected() {
        let mut ds = DungeonState::new(1);
        ds.add_room(Room::new(0, "First")).unwrap();
        assert!(ds.add_room(Room::new(0, "Duplicate")).is_err());
    }

    #[test]
    fn duplicate_door_id_rejected() {
        let mut ds = DungeonState::new(1);
        ds.add_room(Room::new(0, "A")).unwrap();
        ds.add_room(Room::new(1, "B")).unwrap();
        ds.add_room(Room::new(2, "C")).unwrap();
        ds.add_door(Door::new(0, 0, 1, DoorState::Closed).unwrap()).unwrap();
        assert!(ds.add_door(Door::new(0, 1, 2, DoorState::Closed).unwrap()).is_err());
    }

    #[test]
    fn empty_dungeon_has_no_current_room() {
        let ds = DungeonState::new(1);
        assert!(ds.current_room.is_none());
    }

    #[test]
    fn first_room_becomes_current() {
        let mut ds = DungeonState::new(1);
        ds.add_room(Room::new(5, "Start")).unwrap();
        assert_eq!(ds.current_room, Some(5));
    }

    #[test]
    fn room_trap() {
        let room = Room::new(0, "Trap Room").with_trap("Pit trap");
        assert!(room.trap.is_some());
        assert!(!room.trap_triggered);
    }

    #[test]
    fn explore_marks_room() {
        let mut ds = sample_dungeon();
        assert!(ds.explored.is_empty());
        ds.explore_current();
        assert!(ds.explored.contains(&0));
    }

    #[test]
    fn status_display() {
        let ds = sample_dungeon();
        let s = ds.status();
        assert!(s.contains("Entrance"));
        assert!(s.contains("Level: 1"));
    }

    #[test]
    fn room_with_module_fields() {
        let monsters = vec![
            PlacedMonsterInstance::new("skeleton", 3),
            PlacedMonsterInstance::new("zombie", 2),
        ];
        let treasure = vec![
            PlacedTreasureInstance::new("Gold coins", 500),
            PlacedTreasureInstance::new("Potion of Healing", 50),
        ];
        let room = Room::new(0, "Guard Chamber")
            .with_key("guard")
            .with_placed_monsters(monsters)
            .with_placed_treasure(treasure);

        assert_eq!(room.key, Some("guard".to_string()));
        assert_eq!(room.placed_monsters.len(), 2);
        assert_eq!(room.placed_monsters[0].name, "skeleton");
        assert_eq!(room.placed_monsters[0].count, 3);
        assert!(!room.placed_monsters[0].spawned);
        assert_eq!(room.placed_treasure.len(), 2);
        assert_eq!(room.placed_treasure[0].gp_value, 500);
        assert!(!room.placed_treasure[0].taken);
        assert!(!room.monsters_cleared);
        assert!(!room.treasure_looted);
    }

    #[test]
    fn old_room_json_loads_with_defaults() {
        // Simulate loading an old save without the new fields
        let old_json = r#"{
            "id": 1,
            "name": "Old Room",
            "description": "A dusty chamber",
            "searched": false,
            "trap": null,
            "trap_triggered": false
        }"#;
        let room: Room = serde_json::from_str(old_json).unwrap();
        assert_eq!(room.id, 1);
        assert_eq!(room.name, "Old Room");
        assert!(room.key.is_none());
        assert!(room.level_key.is_none());
        assert!(room.placed_monsters.is_empty());
        assert!(room.placed_treasure.is_empty());
        assert!(!room.monsters_cleared);
        assert!(!room.treasure_looted);
        // trap_trigger defaults to Entry for backward compatibility
        assert_eq!(room.trap_trigger, TrapTrigger::Entry);
    }

    #[test]
    fn room_with_action_trap() {
        let room = Room::new(0, "Mirror Room")
            .with_trap("Save vs paralysis or be frozen")
            .with_trap_trigger(TrapTrigger::Action);
        assert_eq!(room.trap, Some("Save vs paralysis or be frozen".to_string()));
        assert_eq!(room.trap_trigger, TrapTrigger::Action);
        assert!(!room.trap_triggered);
    }

    #[test]
    fn trap_trigger_serializes_round_trip() {
        let room = Room::new(0, "Test")
            .with_trap("Freezing mirror")
            .with_trap_trigger(TrapTrigger::Action);
        let json = serde_json::to_string(&room).unwrap();
        let loaded: Room = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.trap_trigger, TrapTrigger::Action);
    }

    #[test]
    fn new_room_fields_serialize() {
        let room = Room::new(0, "Test")
            .with_key("test_key")
            .with_placed_monsters(vec![PlacedMonsterInstance::new("goblin", 4)]);

        let json = serde_json::to_string(&room).unwrap();
        assert!(json.contains("test_key"));
        assert!(json.contains("goblin"));

        // Round-trip
        let loaded: Room = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.key, Some("test_key".to_string()));
        assert_eq!(loaded.placed_monsters.len(), 1);
        assert_eq!(loaded.placed_monsters[0].name, "goblin");
        assert_eq!(loaded.placed_monsters[0].count, 4);
    }

    #[test]
    fn placed_monster_instance_defaults() {
        let monster = PlacedMonsterInstance::new("orc", 5);
        assert_eq!(monster.name, "orc");
        assert_eq!(monster.count, 5);
        assert!(!monster.spawned);
    }

    #[test]
    fn placed_treasure_instance_defaults() {
        let treasure = PlacedTreasureInstance::new("Ancient sword", 1000);
        assert_eq!(treasure.description, "Ancient sword");
        assert_eq!(treasure.gp_value, 1000);
        assert!(!treasure.taken);
    }

    #[test]
    fn add_door_rejects_nonexistent_rooms() {
        let mut ds = DungeonState::new(1);
        ds.add_room(Room::new(0, "Only Room")).unwrap();
        let err = ds.add_door(Door::new(1, 0, 99, DoorState::Open).unwrap()).unwrap_err();
        assert!(err.contains("room_b"), "should mention room_b: {}", err);
        let err = ds.add_door(Door::new(2, 99, 0, DoorState::Open).unwrap()).unwrap_err();
        assert!(err.contains("room_a"), "should mention room_a: {}", err);
    }

    #[test]
    fn door_default_connection_type_is_door() {
        let door = Door::new(0, 0, 1, DoorState::Closed).unwrap();
        assert_eq!(door.connection_type, ConnectionType::Door);
    }

    #[test]
    fn door_connection_type_serializes_round_trip() {
        let mut door = Door::new(0, 0, 1, DoorState::Closed).unwrap();
        door.connection_type = ConnectionType::Stairs;
        let json = serde_json::to_string(&door).unwrap();
        assert!(json.contains("Stairs"));
        let loaded: Door = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.connection_type, ConnectionType::Stairs);
    }

    #[test]
    fn old_door_json_loads_with_default_connection_type() {
        let old_json = r#"{
            "id": 1,
            "room_a": 0,
            "room_b": 1,
            "state": "Closed",
            "discovered": true,
            "module_open": false
        }"#;
        let door: Door = serde_json::from_str(old_json).unwrap();
        assert_eq!(door.connection_type, ConnectionType::Door);
    }

    #[test]
    fn status_shows_connection_type_label() {
        let mut ds = DungeonState::new(1);
        ds.add_room(Room::new(0, "Hall")).unwrap();
        ds.add_room(Room::new(1, "Cellar")).unwrap();
        let mut door = Door::new(0, 0, 1, DoorState::Open).unwrap();
        door.connection_type = ConnectionType::Stairs;
        ds.add_door(door).unwrap();
        let s = ds.status();
        assert!(s.contains("Stairs"), "status should show Stairs, got: {}", s);
    }

    #[test]
    fn move_to_cross_level_returns_transition() {
        let mut ds = DungeonState::new(1);
        let mut r0 = Room::new(0, "Surface");
        r0.level_key = Some("surface".to_string());
        ds.add_room(r0).unwrap();
        let mut r1 = Room::new(1, "Deep");
        r1.level_key = Some("depths".to_string());
        ds.add_room(r1).unwrap();
        ds.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();
        ds.level_map.insert("surface".to_string(), 1);
        ds.level_map.insert("depths".to_string(), 2);
        ds.current_level_key = Some("surface".to_string());

        let transition = ds.move_to(1).unwrap();
        assert_eq!(transition, Some(LevelTransition { from_level: 1, to_level: 2 }));
        assert_eq!(ds.level, 2);
        assert_eq!(ds.current_level_key, Some("depths".to_string()));
    }

    #[test]
    fn move_to_same_level_returns_none() {
        let mut ds = DungeonState::new(1);
        let mut r0 = Room::new(0, "Hall A");
        r0.level_key = Some("surface".to_string());
        ds.add_room(r0).unwrap();
        let mut r1 = Room::new(1, "Hall B");
        r1.level_key = Some("surface".to_string());
        ds.add_room(r1).unwrap();
        ds.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();
        ds.level_map.insert("surface".to_string(), 1);
        ds.current_level_key = Some("surface".to_string());

        let transition = ds.move_to(1).unwrap();
        assert_eq!(transition, None);
        assert_eq!(ds.level, 1);
    }

    #[test]
    fn move_to_without_level_keys_returns_none() {
        let mut ds = sample_dungeon();
        ds.find_door_mut(0).unwrap().state = DoorState::Open;
        let transition = ds.move_to(1).unwrap();
        assert_eq!(transition, None);
    }

    #[test]
    fn old_dungeon_json_loads_without_level_fields() {
        let old_json = r#"{
            "level": 1,
            "rooms": [],
            "doors": [],
            "explored": [],
            "current_room": null,
            "log": [],
            "log_seq": 0
        }"#;
        let ds: DungeonState = serde_json::from_str(old_json).unwrap();
        assert!(ds.current_level_key.is_none());
        assert!(ds.level_map.is_empty());
    }

    #[test]
    fn dungeon_level_fields_round_trip() {
        let mut ds = DungeonState::new(1);
        ds.current_level_key = Some("depths".to_string());
        ds.level_map.insert("surface".to_string(), 1);
        ds.level_map.insert("depths".to_string(), 2);

        let json = serde_json::to_string(&ds).unwrap();
        let loaded: DungeonState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.current_level_key, Some("depths".to_string()));
        assert_eq!(loaded.level_map["surface"], 1);
        assert_eq!(loaded.level_map["depths"], 2);
    }

    #[test]
    fn status_shows_level_key() {
        let mut ds = DungeonState::new(1);
        ds.current_level_key = Some("surface".to_string());
        ds.add_room(Room::new(0, "Hall")).unwrap();
        let s = ds.status();
        assert!(s.contains("Level: 1 (surface)"), "status should show level key, got: {}", s);
    }

    #[test]
    fn status_without_level_key() {
        let mut ds = DungeonState::new(1);
        ds.add_room(Room::new(0, "Hall")).unwrap();
        let s = ds.status();
        assert!(s.contains("Dungeon Level: 1  Room:"), "status should show level without key: {}", s);
        assert!(!s.contains("Level: 1 ("), "should not have level key parenthetical: {}", s);
    }
}
