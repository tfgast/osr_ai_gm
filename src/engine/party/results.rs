use serde::Serialize;

use crate::rules::alignment::AlignmentId;
use crate::rules::class::ClassId;
use crate::rules::encumbrance::EncumbranceLevel;

/// Result payload for create character flow.
#[derive(Debug, Clone, Serialize)]
pub struct CreateCharacterResult {
    pub name: String,
    pub class: ClassId,
    pub alignment: AlignmentId,
    pub used_provided_abilities: bool,
    pub base_abilities: [i32; 6],
    pub abilities: [i32; 6],
    pub applied_racial_modifiers: bool,
    pub created: bool,
    pub eligible_classes: Vec<ClassId>,
    pub character_sheet: Option<String>,
}

/// Inventory summary for a single party member.
#[derive(Debug, Clone, Serialize)]
pub struct MemberInventorySummary {
    pub total_weight_cn: u32,
    pub encumbrance_level: EncumbranceLevel,
    pub item_count: u32,
    pub equipped_items: Vec<String>,
}

/// Party member summary for query responses.
#[derive(Debug, Clone, Serialize)]
pub struct PartyMemberSummary {
    pub name: String,
    pub class: ClassId,
    pub level: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub thac0: u32,
    pub xp: u64,
    pub alive: bool,
    pub alignment: AlignmentId,
    pub movement_rate: u32,
    pub next_level_xp: Option<u64>,
    pub ready_to_train: bool,
    pub inventory: MemberInventorySummary,
}

/// Result payload for querying party state.
#[derive(Debug, Clone, Serialize)]
pub struct QueryPartyResult {
    pub members: Vec<PartyMemberSummary>,
    pub days_without_food: u32,
    pub rations: u32,
    pub party_gold: u64,
}

/// One class requirement entry.
#[derive(Debug, Clone, Serialize)]
pub struct AbilityRequirement {
    pub ability: String,
    pub minimum: i32,
}

/// One class summary entry.
#[derive(Debug, Clone, Serialize)]
pub struct ClassSummary {
    pub name: String,
    pub hit_die: u32,
    pub requirements: Vec<AbilityRequirement>,
    pub is_demihuman: bool,
}

/// Result payload for class listing.
#[derive(Debug, Clone, Serialize)]
pub struct ListClassesResult {
    pub classes: Vec<ClassSummary>,
}

/// Result payload for class eligibility checks.
#[derive(Debug, Clone, Serialize)]
pub struct EligibleClassesResult {
    pub abilities: [i32; 6],
    pub eligible: Vec<ClassId>,
}
