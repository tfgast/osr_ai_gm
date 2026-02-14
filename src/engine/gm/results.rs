use serde::Serialize;

use crate::rules::class::Class;

/// Result payload for awarding XP.
#[derive(Debug, Clone, Serialize)]
pub struct AwardXpResult {
    pub character: String,
    pub base_xp: u64,
    pub adjusted_xp: u64,
    pub modifier_pct: i32,
    pub total_xp: u64,
    pub next_level_xp: Option<u64>,
    pub ready_to_train: bool,
}

/// Result payload for recording a ruling.
#[derive(Debug, Clone, Serialize)]
pub struct RulingResult {
    pub text: String,
    pub note: String,
}

/// One note entry with 1-based index.
#[derive(Debug, Clone, Serialize)]
pub struct NoteEntry {
    pub index: usize,
    pub text: String,
}

/// Result payload for listing session notes.
#[derive(Debug, Clone, Serialize)]
pub struct ListNotesResult {
    pub notes: Vec<NoteEntry>,
}

/// Result payload for deleting one note.
#[derive(Debug, Clone, Serialize)]
pub struct DeleteNoteResult {
    pub index: usize,
    pub deleted: String,
}

/// One retainer entry.
#[derive(Debug, Clone, Serialize)]
pub struct RetainerSummary {
    pub name: String,
    pub class: Class,
    pub level: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub loyalty: u32,
    pub wage_gp: u32,
    pub alive: bool,
}

/// Result payload for listing retainers.
#[derive(Debug, Clone, Serialize)]
pub struct ListRetainersResult {
    pub retainers: Vec<RetainerSummary>,
}

/// Result payload for dismissing a retainer.
#[derive(Debug, Clone, Serialize)]
pub struct DismissRetainerResult {
    pub name: String,
    pub class: Class,
}

/// Result payload for healing a character.
#[derive(Debug, Clone, Serialize)]
pub struct HealResult {
    pub character: String,
    pub healed: i32,
    pub old_hp: i32,
    pub hp: i32,
    pub max_hp: i32,
}

/// Result payload for damaging a character.
#[derive(Debug, Clone, Serialize)]
pub struct DamageResult {
    pub character: String,
    pub damage: i32,
    pub old_hp: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub alive: bool,
    pub status: String,
}

/// Result payload for directly setting HP.
#[derive(Debug, Clone, Serialize)]
pub struct SetHpResult {
    pub character: String,
    pub old_hp: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub alive: bool,
    pub status: String,
}

/// Result payload for setting party rations.
#[derive(Debug, Clone, Serialize)]
pub struct SetRationsResult {
    pub old_rations: u32,
    pub rations: u32,
}

/// Result payload for adding party rations.
#[derive(Debug, Clone, Serialize)]
pub struct AddRationsResult {
    pub added: u32,
    pub rations: u32,
}

/// Result payload for a thief skill check.
#[derive(Debug, Clone, Serialize)]
pub struct ThiefSkillCheckResult {
    #[serde(skip_serializing)]
    pub message: String,
    pub character: String,
    pub skill: String,
    pub level: u32,
    pub target: u32,
    pub roll: u32,
    pub die_type: String,
    pub success: bool,
}
