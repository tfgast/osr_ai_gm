use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::engine::retainer::Retainer;
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
    /// Mode before combat started, restored when combat ends.
    #[serde(default)]
    pub pre_combat_mode: Option<GameMode>,
    /// Hired retainers (NPC followers).
    #[serde(default)]
    pub retainers: Vec<Retainer>,
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
            pre_combat_mode: None,
            retainers: Vec::new(),
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

/// Return the saves directory (`~/.osr_data/saves/`).
pub fn saves_dir() -> io::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;
    Ok(PathBuf::from(home).join(".osr_data").join("saves"))
}

/// Resolve a user-provided filename to a safe path inside the saves directory.
///
/// Rejects path separators and `..` components to prevent path traversal.
/// Appends `.json` if the filename has no extension.
pub fn safe_save_path(filename: &str) -> io::Result<PathBuf> {
    let filename = filename.trim();
    if filename.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "filename must not be empty",
        ));
    }
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "filename must be a simple name, not a path",
        ));
    }
    let mut name = filename.to_string();
    if !name.ends_with(".json") {
        name.push_str(".json");
    }
    let dir = saves_dir()?;
    Ok(dir.join(name))
}

/// Save game state to a JSON file.
/// Uses atomic write (write-to-temp-then-rename) to prevent corruption.
pub fn save(state: &GameState, path: &Path) -> io::Result<()> {
    let json = serde_json::to_string_pretty(state)
        .map_err(io::Error::other)?;

    // Write to a temporary file in the same directory, then rename.
    // This ensures the save is atomic — either the old file remains or
    // the new one replaces it; a crash mid-write won't corrupt data.
    let parent = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let tmp_path = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("save")
    ));
    fs::write(&tmp_path, &json)?;
    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }
    Ok(())
}

/// Return the live-state export path (`~/.osr_data/live_state.json`).
///
/// The companion TUI watches this file for real-time state updates.
pub fn live_state_path() -> io::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;
    Ok(PathBuf::from(home).join(".osr_data").join("live_state.json"))
}

/// Export the current game state to the live-state file.
///
/// Uses the same atomic write (write-to-temp-then-rename) as [`save`] to
/// prevent the companion TUI from reading a partially-written file.
pub fn export_live_state(state: &GameState) -> io::Result<()> {
    let path = live_state_path()?;
    save(state, &path)
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

    #[test]
    fn safe_save_path_simple_name() {
        let path = safe_save_path("mycamp").unwrap();
        assert!(path.ends_with("mycamp.json"));
        assert!(path.to_str().unwrap().contains(".osr_data/saves/"));
    }

    #[test]
    fn safe_save_path_already_has_json() {
        let path = safe_save_path("mycamp.json").unwrap();
        assert!(path.ends_with("mycamp.json"));
        // Should not double-append .json
        assert!(!path.to_str().unwrap().ends_with(".json.json"));
    }

    #[test]
    fn safe_save_path_rejects_slash() {
        let result = safe_save_path("/etc/shadow");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("simple name"));
    }

    #[test]
    fn safe_save_path_rejects_dotdot() {
        let result = safe_save_path("../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn safe_save_path_rejects_backslash() {
        let result = safe_save_path("..\\windows\\system32");
        assert!(result.is_err());
    }

    #[test]
    fn safe_save_path_rejects_empty() {
        let result = safe_save_path("");
        assert!(result.is_err());
    }

    #[test]
    fn safe_save_path_rejects_whitespace_only() {
        let result = safe_save_path("   ");
        assert!(result.is_err());
    }

    #[test]
    fn export_live_state_roundtrip() {
        // Override HOME so we write to a temp directory, not the real home.
        let dir = std::env::temp_dir().join("osr_live_state_test");
        let _ = fs::remove_dir_all(&dir);
        let osr_data = dir.join(".osr_data");
        fs::create_dir_all(&osr_data).unwrap();

        // Temporarily set HOME for this test.
        let orig_home = std::env::var("HOME").unwrap();
        std::env::set_var("HOME", &dir);

        let mut state = GameState::new();
        state.party.add_member(Character::new("Tharos", Class::MagicUser));
        state.dungeon_level = 5;
        state.notes.push("Found the amulet.".to_string());

        export_live_state(&state).unwrap();

        let live_path = live_state_path().unwrap();
        assert!(live_path.exists(), "live_state.json should exist after export");

        let loaded = load(&live_path).unwrap();
        assert_eq!(loaded.party.members.len(), 1);
        assert_eq!(loaded.party.members[0].name, "Tharos");
        assert_eq!(loaded.dungeon_level, 5);
        assert_eq!(loaded.notes[0], "Found the amulet.");

        // Restore HOME and clean up.
        std::env::set_var("HOME", orig_home);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_cleans_up_temp_on_rename_failure() {
        let dir = std::env::temp_dir().join("osr_persist_test_rename_fail");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Create a directory at the target path so rename(file -> dir) fails.
        let target = dir.join("save.json");
        fs::create_dir(&target).unwrap();
        let tmp_path = dir.join(".save.json.tmp");

        let state = GameState::new();
        let result = save(&state, &target);
        assert!(result.is_err(), "rename of file over directory should fail");
        assert!(!tmp_path.exists(), "temp file should be cleaned up after rename failure");

        let _ = fs::remove_dir_all(&dir);
    }
}
