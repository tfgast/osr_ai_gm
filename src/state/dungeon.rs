use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// State of a door in the dungeon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoorState {
    /// Standard door — must force open (2-in-6 base, modified by STR).
    #[serde(alias = "closed")]
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

impl Default for DoorState {
    fn default() -> Self {
        DoorState::Closed
    }
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
}

impl Door {
    pub fn new(id: u32, room_a: u32, room_b: u32, state: DoorState) -> Result<Self, String> {
        if room_a == room_b {
            return Err(format!("door {} connects room {} to itself", id, room_a));
        }
        let discovered = state != DoorState::Secret;
        Ok(Door { id, room_a, room_b, state, discovered })
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
}

impl PlacedMonsterInstance {
    pub fn new(name: &str, count: u32) -> Self {
        PlacedMonsterInstance {
            name: name.to_string(),
            count,
            spawned: false,
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
    /// Module room key for cross-referencing (e.g., "entrance", "guard").
    #[serde(default)]
    pub key: Option<String>,
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
            key: None,
            placed_monsters: Vec::new(),
            placed_treasure: Vec::new(),
            monsters_cleared: false,
            treasure_looted: false,
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
    pub log: Vec<String>,
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
    /// same ID already exists.
    pub fn add_door(&mut self, door: Door) -> Result<(), String> {
        if self.doors.iter().any(|d| d.id == door.id) {
            return Err(format!("duplicate door id {}", door.id));
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
    pub fn move_to(&mut self, room_id: u32) -> Result<(), String> {
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
        self.current_room = Some(room_id);
        self.explored.insert(room_id);
        Ok(())
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
        self.log.push(msg);
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
        let mut out = format!(
            "Dungeon Level: {}  Room: {} ({})\nExplored: {} rooms",
            self.level, room_id_str, room_name, self.explored.len()
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
                out.push_str(&format!("\n  Door {} → {} ({}) [{}]", d.id, dest, dest_name, state));
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
        assert!(room.placed_monsters.is_empty());
        assert!(room.placed_treasure.is_empty());
        assert!(!room.monsters_cleared);
        assert!(!room.treasure_looted);
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
}
