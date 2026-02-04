use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

use crate::model::{Party, CombatState};
use crate::state::dungeon::DungeonState;
use crate::state::game::GameMode;
use crate::state::time::TimeTracker;
use crate::state::wilderness::WildernessState;

/// Current save file format version.
pub const SAVE_VERSION: u32 = 1;

/// The full game state that gets persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Save file format version. Used for forward-compatible loading.
    #[serde(default = "default_version")]
    pub version: u32,
    pub party: Party,
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
    #[serde(default)]
    pub mode: GameMode,
}

fn default_version() -> u32 { 0 }

impl GameState {
    pub fn new() -> Self {
        GameState {
            version: SAVE_VERSION,
            party: Party::new(),
            dungeon_level: 0,
            notes: Vec::new(),
            combat: None,
            time: None,
            dungeon: None,
            wilderness: None,
            mode: GameMode::default(),
        }
    }

    /// Single source of truth for the turn counter (delegates to TimeTracker).
    pub fn turn(&self) -> u32 {
        self.time.as_ref().map(|t| t.total_turns).unwrap_or(0)
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

/// Save game state to a JSON file.
/// Uses atomic write (write-to-temp-then-rename) to prevent corruption.
pub fn save(state: &GameState, path: &Path) -> io::Result<()> {
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Write to a temporary file in the same directory, then rename.
    // This ensures the save is atomic — either the old file remains or
    // the new one replaces it; a crash mid-write won't corrupt data.
    let parent = path.parent().unwrap_or(Path::new("."));
    let tmp_path = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("save")
    ));
    fs::write(&tmp_path, &json)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
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
    use crate::rules::class::Class;
    use std::path::PathBuf;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("osr_ai_gm_test_save.json");

        let mut state = GameState::new();
        state.party.add_member(Character::new("Aldric", Class::Fighter));
        state.time = Some(TimeTracker::new());
        for _ in 0..42 {
            state.time.as_mut().unwrap().advance_turn();
        }
        state.dungeon_level = 3;
        state.notes.push("Entered the crypt.".to_string());

        save(&state, &path).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(loaded.turn(), 42);
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
