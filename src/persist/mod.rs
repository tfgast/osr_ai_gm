use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

use crate::model::{Party, CombatState};
use crate::state::dungeon::DungeonState;
use crate::state::time::TimeTracker;
use crate::state::wilderness::WildernessState;

/// The full game state that gets persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub party: Party,
    pub turn: u64,
    pub dungeon_level: u32,
    pub notes: Vec<String>,
    #[serde(default)]
    pub combat: Option<CombatState>,
    #[serde(default)]
    pub time: Option<TimeTracker>,
    #[serde(default)]
    pub dungeon: Option<DungeonState>,
    #[serde(default)]
    pub wilderness: Option<WildernessState>,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            party: Party::new(),
            turn: 0,
            dungeon_level: 0,
            notes: Vec::new(),
            combat: None,
            time: None,
            dungeon: None,
            wilderness: None,
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

/// Save game state to a JSON file.
pub fn save(state: &GameState, path: &Path) -> io::Result<()> {
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(path, json)
}

/// Load game state from a JSON file.
pub fn load(path: &Path) -> io::Result<GameState> {
    let data = fs::read_to_string(path)?;
    let state: GameState = serde_json::from_str(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    use std::path::PathBuf;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("osr_ai_gm_test_save.json");

        let mut state = GameState::new();
        state.party.add_member(Character::new("Aldric", "Fighter"));
        state.turn = 42;
        state.dungeon_level = 3;
        state.notes.push("Entered the crypt.".to_string());

        save(&state, &path).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(loaded.turn, 42);
        assert_eq!(loaded.dungeon_level, 3);
        assert_eq!(loaded.party.members.len(), 1);
        assert_eq!(loaded.party.members[0].name, "Aldric");
        assert_eq!(loaded.notes[0], "Entered the crypt.");

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file() {
        let result = load(&PathBuf::from("/nonexistent/save.json"));
        assert!(result.is_err());
    }
}
