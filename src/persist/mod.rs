use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::engine::retainer::Retainer;
use crate::model::{Party, CombatState};
use crate::pathutil::normalize_path;
use crate::rules::module::{CustomTable, ModuleRule, WanderingMonsterTable};
use crate::rules::monster::MonsterDef;
use crate::state::dungeon::DungeonState;
use crate::state::effect::ActiveEffect;
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
    /// Global effects not tied to a specific character or monster.
    #[serde(default)]
    pub effects: Vec<ActiveEffect>,
    /// Global monotonic counter for log entry sequencing.
    ///
    /// Subsystem log entries carry a `seq` from this counter so the
    /// companion TUI can merge them in chronological order.
    #[serde(default)]
    pub log_seq: u64,
    /// Module-specific monster definitions loaded alongside the active module.
    /// Keyed by lowercase monster name for case-insensitive lookup.
    #[serde(default)]
    pub module_monsters: HashMap<String, MonsterDef>,
    /// Module-specific wandering monster tables keyed by area/level.
    #[serde(default)]
    pub wandering_monster_tables: HashMap<String, WanderingMonsterTable>,
    /// Module-specific custom rules (defilement zones, special mechanics, etc.).
    #[serde(default)]
    pub module_rules: HashMap<String, ModuleRule>,
    /// Module-specific rollable tables (random events, treasure, etc.).
    #[serde(default)]
    pub module_tables: HashMap<String, CustomTable>,
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
            effects: Vec::new(),
            log_seq: 0,
            module_monsters: HashMap::new(),
            wandering_monster_tables: HashMap::new(),
            module_rules: HashMap::new(),
            module_tables: HashMap::new(),
        }
    }

    /// Single source of truth for the turn counter (delegates to TimeTracker).
    pub fn turn(&self) -> u32 {
        self.time.as_ref().map(|t| t.total_turns).unwrap_or(0)
    }

    /// Return the next monotonic sequence number for log entries.
    pub fn next_log_seq(&mut self) -> u64 {
        self.log_seq += 1;
        self.log_seq
    }

    /// Sync `log_seq` from all active subsystem counters.
    ///
    /// Each subsystem maintains its own `log_seq` counter.  Before
    /// handing the counter to a new subsystem (e.g. combat starting
    /// during exploration), we pull the maximum back to GameState.
    fn sync_log_seq_from_subsystems(&mut self) {
        if let Some(ref d) = self.dungeon { self.log_seq = self.log_seq.max(d.log_seq); }
        if let Some(ref t) = self.time { self.log_seq = self.log_seq.max(t.log_seq); }
        if let Some(ref w) = self.wilderness { self.log_seq = self.log_seq.max(w.log_seq); }
        if let Some(ref c) = self.combat { self.log_seq = self.log_seq.max(c.log_seq); }
    }

    // ── Mode transition methods ─────────────────────────────────────
    //
    // GameMode is **canonical**: it is the single source of truth for
    // which game phase is active. These methods enforce the invariant
    // that mode and associated sub-state (combat, dungeon, wilderness)
    // are always consistent.

    /// Transition into Combat mode.
    ///
    /// Saves the current mode so it can be restored when combat ends.
    /// Panics (debug) if combat state is already present.
    pub fn enter_combat(&mut self, mut combat: CombatState) {
        debug_assert!(self.combat.is_none(), "enter_combat called with combat already active");
        self.sync_log_seq_from_subsystems();
        self.pre_combat_mode = Some(self.mode);
        combat.log_seq = self.log_seq;
        self.combat = Some(combat);
        self.mode = GameMode::Combat;
    }

    /// Leave Combat mode, restoring the mode that was active before combat.
    ///
    /// Returns the `CombatState` so callers can extract results.
    /// Falls back to `Idle` if `pre_combat_mode` was not set.
    /// No-op when no combat is active — the current mode is preserved.
    pub fn exit_combat(&mut self) -> Option<CombatState> {
        let combat = self.combat.take();
        if let Some(ref c) = combat {
            self.log_seq = self.log_seq.max(c.log_seq);
            self.mode = self.pre_combat_mode.take().unwrap_or(GameMode::Idle);
        }
        combat
    }

    /// Transition into Exploration mode with a freshly-initialised dungeon.
    pub fn enter_exploration(&mut self, mut dungeon: DungeonState, level: u32) {
        debug_assert!(level > 0, "enter_exploration called with level 0");
        dungeon.log_seq = self.log_seq;
        self.dungeon = Some(dungeon);
        let mut time = TimeTracker::new();
        time.log_seq = self.log_seq;
        self.time = Some(time);
        self.dungeon_level = level;
        self.mode = GameMode::Exploration;
    }

    /// Transition into Wilderness mode.
    pub fn enter_wilderness(&mut self, mut wilderness: WildernessState) {
        wilderness.log_seq = self.log_seq;
        self.wilderness = Some(wilderness);
        self.mode = GameMode::Wilderness;
    }

    /// Leave Wilderness mode, returning to Idle.
    ///
    /// Clears the wilderness state. No-op if not in wilderness mode.
    pub fn exit_wilderness(&mut self) {
        if self.mode == GameMode::Wilderness {
            if let Some(ref ws) = self.wilderness {
                self.log_seq = self.log_seq.max(ws.log_seq);
            }
            self.wilderness = None;
            self.mode = GameMode::Idle;
        }
    }

    /// Assert that mode and associated sub-state are consistent.
    ///
    /// This is a debug-only check; it compiles to nothing in release builds.
    /// Call it at API boundaries (after load, after each command) to catch
    /// state corruption early.
    #[cfg(debug_assertions)]
    pub fn assert_mode_invariants(&self) {
        match self.mode {
            GameMode::Combat => {
                assert!(self.combat.is_some(), "mode is Combat but combat state is None");
            }
            GameMode::Exploration => {
                assert!(self.dungeon.is_some(), "mode is Exploration but dungeon state is None");
                assert!(self.dungeon_level > 0, "mode is Exploration but dungeon_level is 0");
            }
            GameMode::Wilderness => {
                assert!(self.wilderness.is_some(), "mode is Wilderness but wilderness state is None");
            }
            _ => {}
        }
    }

    /// No-op in release builds.
    #[cfg(not(debug_assertions))]
    pub fn assert_mode_invariants(&self) {}
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

/// Return the base data directory.
///
/// Checks `OSR_DATA_DIR` first; falls back to `~/.osr_data/`.
/// This allows per-instance isolation when multiple processes run concurrently.
pub fn data_dir() -> io::Result<PathBuf> {
    if let Ok(dir) = std::env::var("OSR_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var("HOME")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;
    Ok(PathBuf::from(home).join(".osr_data"))
}

/// Return the saves directory (`<data_dir>/saves/`).
pub fn saves_dir() -> io::Result<PathBuf> {
    Ok(data_dir()?.join("saves"))
}

/// Resolve a user-provided filename to a safe path inside the saves directory.
///
/// Rejects path separators and `..` components to prevent path traversal.
/// Appends `.json` if the filename has no extension.
///
/// Defense-in-depth: after joining the filename to the saves directory, the
/// resolved path is normalized and verified to still reside within the saves
/// directory. This catches any unexpected path resolution behavior that the
/// character-level checks might miss.
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
    if filename.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "filename must not contain null bytes",
        ));
    }
    let mut name = filename.to_string();
    if !name.ends_with(".json") {
        name.push_str(".json");
    }
    let dir = saves_dir()?;
    let path = dir.join(&name);

    // Defense-in-depth: verify the resolved path stays within the saves directory.
    let normalized = normalize_path(&path);
    let base_normalized = normalize_path(&dir);
    if !normalized.starts_with(&base_normalized) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "resolved save path escapes the saves directory",
        ));
    }

    Ok(path)
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
    fs::create_dir_all(parent).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("cannot create saves directory '{}': {}. Set OSR_DATA_DIR to override.", parent.display(), e),
        )
    })?;
    let tmp_path = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("save"),
        std::process::id()
    ));
    fs::write(&tmp_path, &json).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("cannot write save file '{}': {}", tmp_path.display(), e),
        )
    })?;
    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(io::Error::new(
            e.kind(),
            format!("cannot finalize save file '{}': {}", path.display(), e),
        ));
    }
    Ok(())
}

/// Return the live-state export path (`<data_dir>/live_state.json`).
///
/// The companion TUI watches this file for real-time state updates.
pub fn live_state_path() -> io::Result<PathBuf> {
    Ok(data_dir()?.join("live_state.json"))
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
    if state.version > SAVE_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "save version {} is newer than supported version {}",
                state.version, SAVE_VERSION
            ),
        ));
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    
    use crate::test_util::lock_env;
    use std::path::PathBuf;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("osr_ai_gm_test_save.json");

        let mut state = GameState::new();
        state.party.add_member(Character::new("Aldric", "Fighter"));
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
    fn load_rejects_future_save_version() {
        let dir = std::env::temp_dir();
        let path = dir.join("osr_ai_gm_test_future_version.json");
        let state = GameState::new();
        save(&state, &path).unwrap();
        // Patch version to a future value
        let mut data: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        data["version"] = serde_json::json!(999);
        fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
        let result = load(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("999"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn safe_save_path_simple_name() {
        let _env = lock_env();
        let orig = std::env::var("OSR_DATA_DIR").ok();
        unsafe { std::env::remove_var("OSR_DATA_DIR") };

        let path = safe_save_path("mycamp").unwrap();
        assert!(path.ends_with("mycamp.json"));
        assert!(path.to_str().unwrap().contains(".osr_data/saves/"));

        if let Some(v) = orig {
            unsafe { std::env::set_var("OSR_DATA_DIR", v) };
        }
    }

    #[test]
    fn safe_save_path_already_has_json() {
        let _env = lock_env();
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
    fn safe_save_path_rejects_null_bytes() {
        let result = safe_save_path("save\0evil");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("null bytes"));
    }

    #[test]
    fn safe_save_path_rejects_embedded_dotdot() {
        // ".." anywhere in the string is rejected
        let result = safe_save_path("foo..bar");
        assert!(result.is_err());
    }

    #[test]
    fn safe_save_path_rejects_absolute_unix() {
        let result = safe_save_path("/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn safe_save_path_rejects_absolute_with_dotdot() {
        let result = safe_save_path("/../../../etc/shadow");
        assert!(result.is_err());
    }

    #[test]
    fn safe_save_path_trims_and_validates() {
        let _env = lock_env();
        let path = safe_save_path("  mycamp  ").unwrap();
        assert!(path.ends_with("mycamp.json"));
    }

    #[test]
    fn safe_save_path_allows_dashes_and_underscores() {
        let _env = lock_env();
        let path = safe_save_path("my-save_file").unwrap();
        assert!(path.ends_with("my-save_file.json"));
    }

    #[test]
    fn safe_save_path_allows_unicode_names() {
        let _env = lock_env();
        let path = safe_save_path("campagne_épée").unwrap();
        assert!(path.ends_with("campagne_épée.json"));
    }

    #[test]
    fn safe_save_path_result_inside_saves_dir() {
        let _env = lock_env();
        let path = safe_save_path("test_save").unwrap();
        let saves = saves_dir().unwrap();
        assert!(
            path.starts_with(&saves),
            "save path {:?} must be inside saves dir {:?}",
            path, saves
        );
    }

    #[test]
    fn export_live_state_roundtrip() {
        let _env = lock_env();
        // Use OSR_DATA_DIR to isolate this test from the real home directory.
        let dir = std::env::temp_dir().join("osr_live_state_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let orig = std::env::var("OSR_DATA_DIR").ok();
        unsafe { std::env::set_var("OSR_DATA_DIR", &dir) };

        let mut state = GameState::new();
        state.party.add_member(Character::new("Tharos", "Magic-User"));
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

        // Restore and clean up.
        match orig {
            Some(v) => unsafe { std::env::set_var("OSR_DATA_DIR", v) },
            None => unsafe { std::env::remove_var("OSR_DATA_DIR") },
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_creates_missing_directory() {
        let dir = std::env::temp_dir().join("osr_persist_test_missing_dir/saves");
        let _ = fs::remove_dir_all(dir.parent().unwrap());
        assert!(!dir.exists(), "directory should not exist before save");

        let target = dir.join("test.json");
        let state = GameState::new();
        save(&state, &target).unwrap();
        assert!(target.exists(), "save should create directory and file");

        let loaded = load(&target).unwrap();
        assert_eq!(loaded.version, SAVE_VERSION);

        let _ = fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn save_cleans_up_temp_on_rename_failure() {
        let dir = std::env::temp_dir().join("osr_persist_test_rename_fail");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Create a directory at the target path so rename(file -> dir) fails.
        let target = dir.join("save.json");
        fs::create_dir(&target).unwrap();
        let tmp_path = dir.join(format!(".save.json.{}.tmp", std::process::id()));

        let state = GameState::new();
        let result = save(&state, &target);
        assert!(result.is_err(), "rename of file over directory should fail");
        assert!(!tmp_path.exists(), "temp file should be cleaned up after rename failure");

        let _ = fs::remove_dir_all(&dir);
    }

    // ── Mode transition tests ───────────────────────────────────

    #[test]
    fn enter_combat_sets_mode_and_saves_pre_combat() {
        let mut state = GameState::new();
        assert_eq!(state.mode, GameMode::Idle);
        let combat = CombatState::new(vec![], 60);
        state.enter_combat(combat);
        assert_eq!(state.mode, GameMode::Combat);
        assert_eq!(state.pre_combat_mode, Some(GameMode::Idle));
        assert!(state.combat.is_some());
    }

    #[test]
    fn exit_combat_restores_pre_combat_mode() {
        let mut state = GameState::new();
        let dungeon = DungeonState::new(1);
        state.enter_exploration(dungeon, 1);
        assert_eq!(state.mode, GameMode::Exploration);

        state.enter_combat(CombatState::new(vec![], 30));
        assert_eq!(state.mode, GameMode::Combat);

        let combat = state.exit_combat();
        assert!(combat.is_some());
        assert_eq!(state.mode, GameMode::Exploration);
        assert!(state.combat.is_none());
        assert!(state.pre_combat_mode.is_none());
    }

    #[test]
    fn exit_combat_falls_back_to_idle() {
        let mut state = GameState::new();
        state.combat = Some(CombatState::new(vec![], 30));
        state.mode = GameMode::Combat;
        // pre_combat_mode not set
        let _ = state.exit_combat();
        assert_eq!(state.mode, GameMode::Idle);
    }

    /// Regression: exit_combat with no active combat must not change the mode.
    /// Bug oag-uee9i: calling exit_combat when already in Exploration (no combat)
    /// would reset the mode to Idle via unwrap_or(GameMode::Idle).
    #[test]
    fn exit_combat_no_combat_preserves_mode() {
        let mut state = GameState::new();
        let dungeon = DungeonState::new(1);
        state.enter_exploration(dungeon, 1);
        assert_eq!(state.mode, GameMode::Exploration);

        // exit_combat with no active combat should be a no-op on mode
        let combat = state.exit_combat();
        assert!(combat.is_none());
        assert_eq!(state.mode, GameMode::Exploration,
            "exit_combat with no combat must not change mode");
    }

    #[test]
    fn enter_exploration_sets_all_state() {
        let mut state = GameState::new();
        let dungeon = DungeonState::new(3);
        state.enter_exploration(dungeon, 3);
        assert_eq!(state.mode, GameMode::Exploration);
        assert_eq!(state.dungeon_level, 3);
        assert!(state.dungeon.is_some());
        assert!(state.time.is_some());
    }

    #[test]
    fn enter_wilderness_sets_state() {
        let mut state = GameState::new();
        let ws = WildernessState::new();
        state.enter_wilderness(ws);
        assert_eq!(state.mode, GameMode::Wilderness);
        assert!(state.wilderness.is_some());
    }

    #[test]
    fn exit_wilderness_clears_state() {
        let mut state = GameState::new();
        state.enter_wilderness(WildernessState::new());
        assert_eq!(state.mode, GameMode::Wilderness);
        state.exit_wilderness();
        assert_eq!(state.mode, GameMode::Idle);
        assert!(state.wilderness.is_none());
    }

    #[test]
    fn exit_wilderness_noop_when_not_in_wilderness() {
        let mut state = GameState::new();
        let dungeon = DungeonState::new(1);
        state.enter_exploration(dungeon, 1);
        assert_eq!(state.mode, GameMode::Exploration);
        state.exit_wilderness();
        assert_eq!(state.mode, GameMode::Exploration, "exit_wilderness should be a no-op when not in wilderness");
    }

    #[test]
    fn assert_mode_invariants_passes_for_idle() {
        let state = GameState::new();
        state.assert_mode_invariants(); // should not panic
    }

    #[test]
    fn assert_mode_invariants_passes_for_exploration() {
        let mut state = GameState::new();
        state.enter_exploration(DungeonState::new(1), 1);
        state.assert_mode_invariants();
    }

    #[test]
    fn assert_mode_invariants_passes_for_combat() {
        let mut state = GameState::new();
        state.enter_combat(CombatState::new(vec![], 30));
        state.assert_mode_invariants();
    }

    #[test]
    fn assert_mode_invariants_passes_for_wilderness() {
        let mut state = GameState::new();
        state.enter_wilderness(WildernessState::new());
        state.assert_mode_invariants();
    }

    #[test]
    #[should_panic(expected = "mode is Combat but combat state is None")]
    fn assert_mode_invariants_catches_combat_without_state() {
        let mut state = GameState::new();
        state.mode = GameMode::Combat;
        state.assert_mode_invariants();
    }

    #[test]
    #[should_panic(expected = "mode is Exploration but dungeon state is None")]
    fn assert_mode_invariants_catches_exploration_without_dungeon() {
        let mut state = GameState::new();
        state.mode = GameMode::Exploration;
        state.dungeon_level = 1;
        state.assert_mode_invariants();
    }

    #[test]
    #[should_panic(expected = "mode is Wilderness but wilderness state is None")]
    fn assert_mode_invariants_catches_wilderness_without_state() {
        let mut state = GameState::new();
        state.mode = GameMode::Wilderness;
        state.assert_mode_invariants();
    }

    // ── data_dir / OSR_DATA_DIR tests ───────────────────────────

    #[test]
    fn data_dir_uses_osr_data_dir_when_set() {
        let _env = lock_env();
        let orig = std::env::var("OSR_DATA_DIR").ok();
        unsafe { std::env::set_var("OSR_DATA_DIR", "/tmp/custom_osr") };

        let dir = data_dir().unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/custom_osr"));

        match orig {
            Some(v) => unsafe { std::env::set_var("OSR_DATA_DIR", v) },
            None => unsafe { std::env::remove_var("OSR_DATA_DIR") },
        }
    }

    #[test]
    fn data_dir_falls_back_to_home() {
        let _env = lock_env();
        let orig = std::env::var("OSR_DATA_DIR").ok();
        unsafe { std::env::remove_var("OSR_DATA_DIR") };

        let dir = data_dir().unwrap();
        let home = std::env::var("HOME").unwrap();
        assert_eq!(dir, PathBuf::from(home).join(".osr_data"));

        if let Some(v) = orig {
            unsafe { std::env::set_var("OSR_DATA_DIR", v) };
        }
    }

    #[test]
    fn saves_dir_respects_osr_data_dir() {
        let _env = lock_env();
        let orig = std::env::var("OSR_DATA_DIR").ok();
        unsafe { std::env::set_var("OSR_DATA_DIR", "/tmp/custom_osr") };

        let dir = saves_dir().unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/custom_osr/saves"));

        match orig {
            Some(v) => unsafe { std::env::set_var("OSR_DATA_DIR", v) },
            None => unsafe { std::env::remove_var("OSR_DATA_DIR") },
        }
    }

    #[test]
    fn live_state_path_respects_osr_data_dir() {
        let _env = lock_env();
        let orig = std::env::var("OSR_DATA_DIR").ok();
        unsafe { std::env::set_var("OSR_DATA_DIR", "/tmp/custom_osr") };

        let path = live_state_path().unwrap();
        assert_eq!(path, PathBuf::from("/tmp/custom_osr/live_state.json"));

        match orig {
            Some(v) => unsafe { std::env::set_var("OSR_DATA_DIR", v) },
            None => unsafe { std::env::remove_var("OSR_DATA_DIR") },
        }
    }

    // ── Log sequence ordering tests ─────────────────────────────

    #[test]
    fn log_seq_propagates_through_combat_lifecycle() {
        let mut state = GameState::new();
        let dungeon = DungeonState::new(1);
        state.enter_exploration(dungeon, 1);

        // Log a dungeon event
        state.dungeon.as_mut().unwrap().log("Entered room 1".into());
        state.dungeon.as_mut().unwrap().log("Found a chest".into());

        // Enter combat — seq should continue from dungeon's
        let dungeon_seq = state.dungeon.as_ref().unwrap().log_seq;
        state.enter_combat(CombatState::new(vec![], 30));

        let combat = state.combat.as_ref().unwrap();
        assert_eq!(combat.log_seq, state.log_seq,
            "combat should inherit GameState's log_seq");

        // Add combat events
        state.combat.as_mut().unwrap().log_event("Initiative rolled".into());
        state.combat.as_mut().unwrap().log_event("Fighter attacks goblin".into());
        let combat_last_seq = state.combat.as_ref().unwrap().log_seq;
        assert!(combat_last_seq > dungeon_seq,
            "combat seqs should be after dungeon seqs");

        // Exit combat — GameState.log_seq should be updated
        let _ = state.exit_combat();
        assert!(state.log_seq >= combat_last_seq,
            "GameState.log_seq should reflect combat's final seq");

        // Log another dungeon event — seq should be after combat
        state.dungeon.as_mut().unwrap().log("Moved to room 2".into());
        // Dungeon's local counter may not reflect combat's updates, but
        // when merged by collect_logs, the seq ordering is:
        //   dungeon entries (1, 2) < combat entries (3, 4) since combat
        //   inherited the initial seq and continued from there.
    }

    #[test]
    fn log_entries_carry_sequence_numbers() {
        let mut state = GameState::new();
        state.enter_combat(CombatState::new(vec![], 60));
        let combat = state.combat.as_mut().unwrap();
        combat.log_event("Event A".into());
        combat.log_event("Event B".into());
        combat.log_event("Event C".into());

        assert_eq!(combat.log.len(), 3);
        assert!(combat.log[0].seq < combat.log[1].seq);
        assert!(combat.log[1].seq < combat.log[2].seq);
        assert!(combat.log[0].contains("Event A"));
        assert!(combat.log[2].contains("Event C"));
    }

    #[test]
    fn log_entry_backward_compat_deserialize() {
        // Old save format: log entries are plain strings
        let json = r#"{
            "party": {"members": [], "gold": 0, "marching_order": []},
            "dungeon_level": 0,
            "notes": [],
            "combat": {
                "round": 1,
                "monsters": [],
                "party_initiative": 3,
                "monster_initiative": 4,
                "distance": 30,
                "log": ["Round 1 begins", "Fighter attacks"]
            }
        }"#;
        let state: GameState = serde_json::from_str(json).unwrap();
        let combat = state.combat.unwrap();
        assert_eq!(combat.log.len(), 2);
        assert_eq!(combat.log[0].seq, 0, "old entries should have seq 0");
        assert!(combat.log[0].contains("Round 1"));
        assert!(combat.log[1].contains("Fighter attacks"));
    }
}
