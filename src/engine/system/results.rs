use serde::Serialize;
use std::path::PathBuf;

/// Typed success payload for dice rolling.
#[derive(Debug, Clone, Serialize)]
pub struct RollDiceResult {
    pub rendered: String,
    pub total: i32,
}

/// Typed success payload for saving game state.
#[derive(Debug, Clone, Serialize)]
pub struct SaveGameResult {
    pub path: PathBuf,
}

/// Typed success payload for loading game state.
#[derive(Debug, Clone, Serialize)]
pub struct LoadGameResult {
    pub turn: u32,
    pub dungeon_level: u32,
    pub party_members: usize,
    pub combat_active: bool,
}

/// Typed success payload for ending a session.
#[derive(Debug, Clone, Serialize)]
pub struct QuitResult;

/// Typed success payload for help text rendering.
#[derive(Debug, Clone, Serialize)]
pub struct HelpResult {
    pub output: String,
}
