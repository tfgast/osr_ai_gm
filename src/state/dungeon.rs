use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// State of a door in the dungeon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoorState {
    /// Standard door — must force open (2-in-6 base, modified by STR).
    Closed,
    /// Door is open (forced or otherwise).
    Open,
    /// Stuck shut — requires forcing.
    Stuck,
    /// Locked — requires a key or thief lockpicking.
    Locked,
    /// Secret door — must be found by searching (1-in-6, elves 2-in-6).
    Secret,
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
    pub fn new(id: u32, room_a: u32, room_b: u32, state: DoorState) -> Self {
        let discovered = state != DoorState::Secret;
        Door { id, room_a, room_b, state, discovered }
    }

    /// Whether the party can attempt to pass through this door.
    pub fn is_passable(&self) -> bool {
        self.state == DoorState::Open
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
}

/// Tracks the dungeon map: rooms, doors, exploration state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonState {
    pub level: u32,
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    /// IDs of rooms the party has visited.
    pub explored: HashSet<u32>,
    /// The room the party is currently in.
    pub current_room: u32,
    /// Event log.
    pub log: Vec<String>,
}

impl DungeonState {
    pub fn new(level: u32) -> Self {
        DungeonState {
            level,
            rooms: Vec::new(),
            doors: Vec::new(),
            explored: HashSet::new(),
            current_room: 0,
            log: Vec::new(),
        }
    }

    /// Add a room to the dungeon.
    pub fn add_room(&mut self, room: Room) {
        self.rooms.push(room);
    }

    /// Add a door between two rooms.
    pub fn add_door(&mut self, door: Door) {
        self.doors.push(door);
    }

    /// Mark the current room as explored.
    pub fn explore_current(&mut self) {
        self.explored.insert(self.current_room);
    }

    /// Move to a room by ID.
    pub fn move_to(&mut self, room_id: u32) -> Result<(), String> {
        if self.rooms.iter().any(|r| r.id == room_id) {
            self.current_room = room_id;
            self.explored.insert(room_id);
            Ok(())
        } else {
            Err(format!("room {} does not exist", room_id))
        }
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
        self.doors
            .iter()
            .filter(|d| d.room_a == self.current_room || d.room_b == self.current_room)
            .filter(|d| d.discovered)
            .collect()
    }

    /// Status display of the current position.
    pub fn status(&self) -> String {
        let room_name = self.find_room(self.current_room)
            .map(|r| r.name.as_str())
            .unwrap_or("unknown");
        let mut out = format!(
            "Dungeon Level: {}  Room: {} ({})\nExplored: {} rooms",
            self.level, self.current_room, room_name, self.explored.len()
        );
        let doors = self.doors_from_current();
        if !doors.is_empty() {
            out.push_str("\nExits:");
            for d in &doors {
                let dest = if d.room_a == self.current_room { d.room_b } else { d.room_a };
                let dest_name = self.find_room(dest)
                    .map(|r| r.name.as_str())
                    .unwrap_or("?");
                let state = match d.state {
                    DoorState::Open => "open",
                    DoorState::Closed => "closed",
                    DoorState::Stuck => "stuck",
                    DoorState::Locked => "locked",
                    DoorState::Secret => "secret",
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
        ds.add_room(Room::new(0, "Entrance"));
        ds.add_room(Room::new(1, "Guard Room"));
        ds.add_room(Room::new(2, "Hidden Chamber"));
        ds.add_door(Door::new(0, 0, 1, DoorState::Closed));
        ds.add_door(Door::new(1, 1, 2, DoorState::Secret));
        ds
    }

    #[test]
    fn move_to_room() {
        let mut ds = sample_dungeon();
        assert_eq!(ds.current_room, 0);
        ds.move_to(1).unwrap();
        assert_eq!(ds.current_room, 1);
        assert!(ds.explored.contains(&1));
    }

    #[test]
    fn move_to_nonexistent_fails() {
        let mut ds = sample_dungeon();
        assert!(ds.move_to(99).is_err());
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
        let door = Door::new(0, 0, 1, DoorState::Open);
        assert!(door.is_passable());

        let door = Door::new(0, 0, 1, DoorState::Closed);
        assert!(!door.is_passable());

        let door = Door::new(0, 0, 1, DoorState::Locked);
        assert!(!door.is_passable());
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
}
