use serde::Serialize;

/// Result payload for create character flow.
#[derive(Debug, Clone, Serialize)]
pub struct CreateCharacterResult {
    pub name: String,
    pub class: String,
    pub alignment: String,
    pub used_provided_abilities: bool,
    pub base_abilities: [i32; 6],
    pub abilities: [i32; 6],
    pub applied_racial_modifiers: bool,
    pub created: bool,
    pub eligible_classes: Vec<String>,
    pub character_sheet: Option<String>,
}

/// Party member summary for query responses.
#[derive(Debug, Clone, Serialize)]
pub struct PartyMemberSummary {
    pub name: String,
    pub class: String,
    pub level: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub thac0: u32,
    pub xp: u64,
    pub alive: bool,
    pub alignment: String,
    pub movement_rate: u32,
    pub next_level_xp: Option<u64>,
    pub ready_to_train: bool,
}

/// Result payload for querying party state.
#[derive(Debug, Clone, Serialize)]
pub struct QueryPartyResult {
    pub members: Vec<PartyMemberSummary>,
    pub days_without_food: u32,
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
    pub eligible: Vec<String>,
}
