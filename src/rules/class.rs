//! Class definitions for OSE B/X 7 + Advanced Fantasy 15 = 22 classes.
//! Data from OSE Reference Booklet p12, p19 and Players Tomes.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use super::save::{SaveCategory, SaveCategoryId};
use super::spell::{SpellProgression, SpellListType};

// ── Canonical class names ────────────────────────────────────

/// All 22 canonical class names in alphabetical order.
pub const CANONICAL_CLASS_NAMES: [&str; 22] = [
    "Acrobat", "Assassin", "Barbarian", "Bard", "Cleric", "Drow",
    "Druid", "Duergar", "Dwarf", "Elf", "Fighter", "Gnome",
    "Half-Elf", "Halfling", "Half-Orc", "Illusionist", "Knight",
    "Magic-User", "Paladin", "Ranger", "Svirfneblin", "Thief",
];

/// Normalize a class name string to canonical form (case-insensitive,
/// accepts common variants like "MU", "magic user", "half-elf").
/// Returns `None` for unknown/homebrew class names.
pub fn normalize_class_name(s: &str) -> Option<&'static str> {
    match s.to_lowercase().replace(['-', '_', ' '], "").as_str() {
        "acrobat" => Some("Acrobat"),
        "assassin" => Some("Assassin"),
        "barbarian" => Some("Barbarian"),
        "bard" => Some("Bard"),
        "cleric" => Some("Cleric"),
        "drow" => Some("Drow"),
        "druid" => Some("Druid"),
        "duergar" => Some("Duergar"),
        "dwarf" => Some("Dwarf"),
        "elf" => Some("Elf"),
        "fighter" => Some("Fighter"),
        "gnome" => Some("Gnome"),
        "halfelf" => Some("Half-Elf"),
        "halfling" => Some("Halfling"),
        "halforc" => Some("Half-Orc"),
        "illusionist" => Some("Illusionist"),
        "knight" => Some("Knight"),
        "magicuser" | "mu" | "mage" | "wizard" => Some("Magic-User"),
        "paladin" => Some("Paladin"),
        "ranger" => Some("Ranger"),
        "svirfneblin" | "deepgnome" => Some("Svirfneblin"),
        "thief" => Some("Thief"),
        _ => None,
    }
}

/// Map a canonical class display name to its DSL enum variant name.
/// Most names are identical; only hyphenated/multi-word names differ.
pub fn canonical_to_dsl_variant(name: &str) -> &str {
    match name {
        "Half-Elf" => "HalfElf",
        "Half-Orc" => "HalfOrc",
        "Magic-User" => "MagicUser",
        _ => name,
    }
}

// ── ClassId: string-based class identifier ───────────────────

/// String-based class identifier for data-driven class lookups.
/// Wraps `Arc<str>` for O(1) clone. Canonical form is the display name
/// (e.g., "Fighter", "Magic-User", "Half-Elf").
/// Serializes transparently as a plain string for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClassId(Arc<str>);

impl ClassId {
    /// Create a ClassId, normalizing to canonical form.
    /// Known classes (the core 22) are normalized via `normalize_class_name()`.
    /// Unknown names (module/homebrew classes) are stored as-is.
    pub fn new(s: &str) -> Self {
        if let Some(canonical) = normalize_class_name(s) {
            ClassId(Arc::from(canonical))
        } else {
            ClassId(Arc::from(s))
        }
    }

    /// The canonical string form of this class identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Display name for this class (same as as_str, provided for API parity).
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ClassId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for ClassId {
    fn from(s: &str) -> Self {
        ClassId::new(s)
    }
}

/// Combat aptitude tier per OSE Reference Booklet p19.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatAptitude {
    Martial,
    SemiMartial,
    NonMartial,
}

/// Armour restrictions for a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmourPermission {
    Any,        // All armour + shield
    AnyNoShield, // All armour, no shield
    Leather,    // Leather only, no shield
    LeatherShield, // Leather + shield
    None,       // No armour, no shield
}

/// Ability score requirement: (ability_index, minimum_value).
/// Ability indices: 0=STR, 1=INT, 2=WIS, 3=DEX, 4=CON, 5=CHA.
pub type AbilityRequirement = (usize, i32);

/// Racial ability modifier: (ability_index, modifier).
pub type AbilityModifier = (usize, i32);

/// Static definition of a character class.
#[derive(Debug, Clone)]
pub struct ClassDef {
    pub class_id: ClassId,
    pub hit_die: u32,
    pub combat_aptitude: CombatAptitude,
    pub prime_requisites: &'static [usize], // ability indices
    pub requirements: &'static [AbilityRequirement],
    pub racial_modifiers: &'static [AbilityModifier],
    pub max_level: u32,
    pub armour: ArmourPermission,
    pub weapons_any: bool,       // true = any weapon, false = restricted
    pub weapons_blunt_only: bool, // if !weapons_any, only blunt weapons
    pub save_category: SaveCategoryId,
    pub spell_progression: SpellProgression,
    pub spell_list: SpellListType,
    pub starting_gold: &'static str, // dice notation for gold in gp
    pub is_demihuman: bool,
    // Capability tags (populated from DSL or native fallback)
    pub has_thief_skills: bool,
    pub can_backstab: bool,
    pub can_turn_undead: bool,
    pub bx_equivalent: Option<ClassId>,
}

/// Ability index constants.
pub const STR: usize = 0;
pub const INT: usize = 1;
pub const WIS: usize = 2;
pub const DEX: usize = 3;
pub const CON: usize = 4;
pub const CHA: usize = 5;

/// Get the full class definition for a given class by ClassId.
pub fn class_def(id: &ClassId) -> ClassDef {
    let name = id.as_str();
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Class) {
        if let Some(def) = dsl_gate::dsl_class_def(name) {
            return def;
        }
    }
    native_class_def(name)
}

fn native_class_def(name: &str) -> ClassDef {
    match name {
        "Acrobat" => ClassDef {
            class_id: ClassId::new("Acrobat"), hit_die: 4,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[DEX],
            requirements: &[(DEX, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::None,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Thief),
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: true, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Thief")),
        },
        "Assassin" => ClassDef {
            class_id: ClassId::new("Assassin"), hit_die: 4,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[DEX],
            requirements: &[(DEX, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Leather,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Thief),
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: true, can_backstab: true, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Thief")),
        },
        "Barbarian" => ClassDef {
            class_id: ClassId::new("Barbarian"), hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(STR, 9), (DEX, 9), (CON, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Barbarian),
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Fighter")),
        },
        "Bard" => ClassDef {
            class_id: ClassId::new("Bard"), hit_die: 6,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[CHA],
            requirements: &[(DEX, 9), (CHA, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::LeatherShield,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Thief),
            spell_progression: SpellProgression::Bard,
            spell_list: SpellListType::Druid,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: true, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Thief")),
        },
        "Cleric" => ClassDef {
            class_id: ClassId::new("Cleric"), hit_die: 6,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[WIS],
            requirements: &[],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: false, weapons_blunt_only: true,
            save_category: SaveCategoryId::from_enum(SaveCategory::Cleric),
            spell_progression: SpellProgression::Cleric,
            spell_list: SpellListType::Cleric,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: true,
            bx_equivalent: None,
        },
        "Drow" => ClassDef {
            class_id: ClassId::new("Drow"), hit_die: 6,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR, INT],
            requirements: &[(INT, 9)],
            racial_modifiers: &[(DEX, 1), (CON, -1)],
            max_level: 10, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Drow),
            spell_progression: SpellProgression::Drow,
            spell_list: SpellListType::DrowArcaneAndDivine,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Elf")),
        },
        "Druid" => ClassDef {
            class_id: ClassId::new("Druid"), hit_die: 6,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[WIS],
            requirements: &[],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Leather,
            weapons_any: false, weapons_blunt_only: true,
            save_category: SaveCategoryId::from_enum(SaveCategory::Cleric),
            spell_progression: SpellProgression::Druid,
            spell_list: SpellListType::Druid,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Cleric")),
        },
        "Duergar" => ClassDef {
            class_id: ClassId::new("Duergar"), hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(INT, 9), (CON, 9)],
            racial_modifiers: &[(CON, 1), (CHA, -1)],
            max_level: 10, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Dwarf),
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Dwarf")),
        },
        "Dwarf" => ClassDef {
            class_id: ClassId::new("Dwarf"), hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(CON, 9)],
            racial_modifiers: &[(CON, 1), (CHA, -1)],
            max_level: 12, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Dwarf),
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: None,
        },
        "Elf" => ClassDef {
            class_id: ClassId::new("Elf"), hit_die: 6,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR, INT],
            requirements: &[(INT, 9)],
            racial_modifiers: &[(DEX, 1), (CON, -1)],
            max_level: 10, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Elf),
            spell_progression: SpellProgression::ArcaneFullCaster,
            spell_list: SpellListType::MagicUser,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: None,
        },
        "Fighter" => ClassDef {
            class_id: ClassId::new("Fighter"), hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Fighter),
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: None,
        },
        "Gnome" => ClassDef {
            class_id: ClassId::new("Gnome"), hit_die: 6,
            combat_aptitude: CombatAptitude::NonMartial,
            prime_requisites: &[DEX, INT],
            requirements: &[(INT, 9), (CON, 9)],
            racial_modifiers: &[],
            max_level: 8, armour: ArmourPermission::Leather,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Gnome),
            spell_progression: SpellProgression::ArcaneFullCaster,
            spell_list: SpellListType::Illusionist,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Halfling")),
        },
        "Half-Elf" => ClassDef {
            class_id: ClassId::new("Half-Elf"), hit_die: 6,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR, INT],
            requirements: &[(CON, 9), (CHA, 9)],
            racial_modifiers: &[],
            max_level: 12, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::HalfElf),
            spell_progression: SpellProgression::HalfElf,
            spell_list: SpellListType::MagicUser,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Elf")),
        },
        "Halfling" => ClassDef {
            class_id: ClassId::new("Halfling"), hit_die: 6,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR, DEX],
            requirements: &[(CON, 9), (DEX, 9)],
            racial_modifiers: &[(STR, -1), (DEX, 1)],
            max_level: 8, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Dwarf),
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: None,
        },
        "Half-Orc" => ClassDef {
            class_id: ClassId::new("Half-Orc"), hit_die: 6,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[STR, DEX],
            requirements: &[],
            racial_modifiers: &[(STR, 1), (CON, 1), (CHA, -2)],
            max_level: 8, armour: ArmourPermission::Leather,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::HalfOrc),
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: true, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Dwarf")),
        },
        "Illusionist" => ClassDef {
            class_id: ClassId::new("Illusionist"), hit_die: 4,
            combat_aptitude: CombatAptitude::NonMartial,
            prime_requisites: &[INT],
            requirements: &[(DEX, 9), (INT, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::None,
            weapons_any: false, weapons_blunt_only: false, // dagger only
            save_category: SaveCategoryId::from_enum(SaveCategory::MagicUser),
            spell_progression: SpellProgression::ArcaneFullCaster,
            spell_list: SpellListType::Illusionist,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Magic-User")),
        },
        "Knight" => ClassDef {
            class_id: ClassId::new("Knight"), hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(STR, 9), (CHA, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Fighter),
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Fighter")),
        },
        "Magic-User" => ClassDef {
            class_id: ClassId::new("Magic-User"), hit_die: 4,
            combat_aptitude: CombatAptitude::NonMartial,
            prime_requisites: &[INT],
            requirements: &[],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::None,
            weapons_any: false, weapons_blunt_only: false, // dagger only
            save_category: SaveCategoryId::from_enum(SaveCategory::MagicUser),
            spell_progression: SpellProgression::ArcaneFullCaster,
            spell_list: SpellListType::MagicUser,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: None,
        },
        "Paladin" => ClassDef {
            class_id: ClassId::new("Paladin"), hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(CHA, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Paladin),
            spell_progression: SpellProgression::Paladin,
            spell_list: SpellListType::Cleric,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: true,
            bx_equivalent: Some(ClassId::new("Fighter")),
        },
        "Ranger" => ClassDef {
            class_id: ClassId::new("Ranger"), hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(STR, 9), (WIS, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Fighter),
            spell_progression: SpellProgression::Ranger,
            spell_list: SpellListType::Druid,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Fighter")),
        },
        "Svirfneblin" => ClassDef {
            class_id: ClassId::new("Svirfneblin"), hit_die: 6,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[DEX, INT],
            requirements: &[(CON, 9)],
            racial_modifiers: &[],
            max_level: 8, armour: ArmourPermission::Leather,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Svirfneblin),
            spell_progression: SpellProgression::ArcaneFullCaster,
            spell_list: SpellListType::Illusionist,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(ClassId::new("Halfling")),
        },
        "Thief" => ClassDef {
            class_id: ClassId::new("Thief"), hit_die: 4,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[DEX],
            requirements: &[],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Leather,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategoryId::from_enum(SaveCategory::Thief),
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: true, can_backstab: true, can_turn_undead: false,
            bx_equivalent: None,
        },
        _ => panic!("unknown class: {}", name),
    }
}

/// Check if a set of ability scores meets the requirements for a class.
/// Abilities array: [STR, INT, WIS, DEX, CON, CHA] (after racial modifiers).
pub fn meets_requirements(id: &ClassId, abilities: &[i32; 6]) -> bool {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Class) {
        if let Some(val) = dsl_gate::dsl_meets_requirements(id.as_str(), abilities) {
            return val;
        }
    }
    let def = class_def(id);
    for &(idx, min) in def.requirements {
        if abilities[idx] < min {
            return false;
        }
    }
    true
}

/// Apply racial ability modifiers for a demihuman class.
/// Modifies abilities in place and clamps to 3..=18.
pub fn apply_racial_modifiers(id: &ClassId, abilities: &mut [i32; 6]) {
    let def = class_def(id);
    for &(idx, modifier) in def.racial_modifiers {
        abilities[idx] = (abilities[idx] + modifier).clamp(3, 18);
    }
}

/// Get all classes that a given set of abilities qualifies for.
/// For demihuman classes, racial modifiers are applied before checking requirements.
pub fn eligible_classes(abilities: &[i32; 6]) -> Vec<ClassId> {
    CANONICAL_CLASS_NAMES.iter()
        .filter(|&&name| {
            let id = ClassId::new(name);
            let def = class_def(&id);
            if def.racial_modifiers.is_empty() {
                meets_requirements(&id, abilities)
            } else {
                let mut modified = *abilities;
                apply_racial_modifiers(&id, &mut modified);
                meets_requirements(&id, &modified)
            }
        })
        .map(|&name| ClassId::new(name))
        .collect()
}

// ── DSL gate helpers ──────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
mod dsl_gate {
    use std::collections::BTreeMap;

    use ttrpg_ast::Name;
    use ttrpg_interp::effect::{Effect, EffectHandler, Response};
    use ttrpg_interp::state::{ActiveCondition, EntityRef, StateProvider};
    use ttrpg_interp::value::Value;

    use crate::backend;
    use super::*;

    struct NullState;

    impl StateProvider for NullState {
        fn read_field(&self, _: &EntityRef, _: &str) -> Option<Value> { None }
        fn read_conditions(&self, _: &EntityRef) -> Option<Vec<ActiveCondition>> { None }
        fn read_turn_budget(&self, _: &EntityRef) -> Option<BTreeMap<Name, Value>> { None }
        fn read_enabled_options(&self) -> Vec<Name> { Vec::new() }
        fn position_eq(&self, _: &Value, _: &Value) -> bool { false }
        fn distance(&self, _: &Value, _: &Value) -> Option<i64> { None }
    }

    struct NullHandler;

    impl EffectHandler for NullHandler {
        fn handle(&mut self, _: Effect) -> Response { Response::Acknowledged }
    }

    fn class_to_dsl(name: &str) -> Value {
        Value::EnumVariant {
            enum_name: "Class".into(),
            variant: Name::from(canonical_to_dsl_variant(name)),
            fields: BTreeMap::new(),
        }
    }

    fn dsl_int(fields: &BTreeMap<Name, Value>, key: &str) -> Option<i64> {
        match fields.get(&Name::from(key))? {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }

    fn dsl_bool(fields: &BTreeMap<Name, Value>, key: &str) -> Option<bool> {
        match fields.get(&Name::from(key))? {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }

    fn dsl_variant<'a>(fields: &'a BTreeMap<Name, Value>, key: &str) -> Option<&'a str> {
        match fields.get(&Name::from(key))? {
            Value::EnumVariant { variant, .. } => Some(variant.as_str()),
            _ => None,
        }
    }

    fn parse_combat_aptitude(s: &str) -> Option<CombatAptitude> {
        match s {
            "martial" => Some(CombatAptitude::Martial),
            "semi_martial" => Some(CombatAptitude::SemiMartial),
            "non_martial" => Some(CombatAptitude::NonMartial),
            _ => None,
        }
    }

    fn parse_armour_permission(s: &str) -> Option<ArmourPermission> {
        match s {
            "any_armour" => Some(ArmourPermission::Any),
            "any_no_shield" => Some(ArmourPermission::AnyNoShield),
            "leather_only" => Some(ArmourPermission::Leather),
            "leather_shield" => Some(ArmourPermission::LeatherShield),
            "no_armour" => Some(ArmourPermission::None),
            _ => None,
        }
    }

    fn parse_save_category(s: &str) -> Option<SaveCategoryId> {
        SaveCategory::parse(s).map(SaveCategoryId::from_enum)
    }

    fn parse_spell_progression(s: &str) -> Option<SpellProgression> {
        match s {
            "prog_bard" => Some(SpellProgression::Bard),
            "prog_cleric" => Some(SpellProgression::Cleric),
            "prog_drow" => Some(SpellProgression::Drow),
            "prog_druid" => Some(SpellProgression::Druid),
            "prog_arcane_full" => Some(SpellProgression::ArcaneFullCaster),
            "prog_half_elf" => Some(SpellProgression::HalfElf),
            "prog_paladin" => Some(SpellProgression::Paladin),
            "prog_ranger" => Some(SpellProgression::Ranger),
            "non_caster" => Some(SpellProgression::NonCaster),
            _ => None,
        }
    }

    fn parse_spell_list(s: &str) -> Option<SpellListType> {
        match s {
            "no_list" => Some(SpellListType::None),
            "list_cleric" => Some(SpellListType::Cleric),
            "list_druid" => Some(SpellListType::Druid),
            "list_illusionist" => Some(SpellListType::Illusionist),
            "list_magic_user" => Some(SpellListType::MagicUser),
            "list_drow_arcane_divine" => Some(SpellListType::DrowArcaneAndDivine),
            _ => None,
        }
    }

    pub fn dsl_class_def(name: &str) -> Option<ClassDef> {
        let rt = backend::dsl()?;
        let args = vec![class_to_dsl(name)];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "class_def", args).ok()?;

        let fields = match &result {
            Value::Struct { fields, .. } => fields,
            _ => return None,
        };

        // Extract DSL fields
        let hit_die = dsl_int(fields, "hit_die")? as u32;
        let combat_aptitude = parse_combat_aptitude(dsl_variant(fields, "combat_aptitude")?)?;
        let max_level = dsl_int(fields, "max_level")? as u32;
        let armour = parse_armour_permission(dsl_variant(fields, "armour")?)?;
        let weapons_any = dsl_bool(fields, "weapons_any")?;
        let weapons_blunt_only = dsl_bool(fields, "weapons_blunt_only")?;
        let save_category = parse_save_category(dsl_variant(fields, "save_category")?)?;
        let spell_progression = parse_spell_progression(dsl_variant(fields, "spell_progression")?)?;
        let spell_list = parse_spell_list(dsl_variant(fields, "spell_list")?)?;
        let is_demihuman = dsl_bool(fields, "is_demihuman")?;

        // Capability tags
        let has_thief_skills = dsl_bool(fields, "has_thief_skills")?;
        let can_backstab = dsl_bool(fields, "can_backstab")?;
        let can_turn_undead = dsl_bool(fields, "can_turn_undead")?;
        let bx_equivalent = match fields.get(&Name::from("bx_equivalent")) {
            Some(Value::EnumVariant { variant, .. }) => {
                // DSL uses bare Class (not option<Class>). Self-mapping means
                // "this IS a B/X class" → None. Different class → Some(equivalent).
                match normalize_class_name(variant.as_str()) {
                    Some(equiv_name) if equiv_name == name => None,
                    Some(equiv_name) => Some(ClassId::new(equiv_name)),
                    None => None,
                }
            }
            _ => None,
        };

        // Fields not in DSL — backfill from native
        let native = native_class_def(name);

        Some(ClassDef {
            class_id: ClassId::new(name),
            hit_die,
            combat_aptitude,
            prime_requisites: native.prime_requisites,
            requirements: native.requirements,
            racial_modifiers: native.racial_modifiers,
            max_level,
            armour,
            weapons_any,
            weapons_blunt_only,
            save_category,
            spell_progression,
            spell_list,
            starting_gold: native.starting_gold,
            is_demihuman,
            has_thief_skills,
            can_backstab,
            can_turn_undead,
            bx_equivalent,
        })
    }

    pub fn dsl_meets_requirements(name: &str, abilities: &[i32; 6]) -> Option<bool> {
        let rt = backend::dsl()?;

        // Build DSL map<Ability, int> from ability array
        let ability_names = ["STR", "INT", "WIS", "DEX", "CON", "CHA"];
        let mut ability_map = BTreeMap::new();
        for (i, &aname) in ability_names.iter().enumerate() {
            let key = Value::EnumVariant {
                enum_name: "Ability".into(),
                variant: Name::from(aname),
                fields: BTreeMap::new(),
            };
            ability_map.insert(key, Value::Int(abilities[i] as i64));
        }

        let args = vec![
            class_to_dsl(name),
            Value::Map(ability_map),
        ];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "meets_requirements", args).ok()?;
        match result {
            Value::Bool(v) => Some(v),
            _ => None,
        }
    }
}

// ── ClassRegistry: DSL-populated class data cache ────────────

/// Registry of all class definitions, populated from DSL evaluation at startup.
/// Falls back to native definitions if DSL is unavailable.
pub struct ClassRegistry {
    classes: Vec<ClassDef>,
    by_name: std::collections::HashMap<String, usize>,
}

impl ClassRegistry {
    /// Build the registry by evaluating class_def for each canonical class name.
    fn build() -> Self {
        let mut classes = Vec::with_capacity(CANONICAL_CLASS_NAMES.len());
        let mut by_name = std::collections::HashMap::with_capacity(CANONICAL_CLASS_NAMES.len());

        for &name in &CANONICAL_CLASS_NAMES {
            let id = ClassId::new(name);
            let def = class_def(&id);
            by_name.insert(name.to_string(), classes.len());
            classes.push(def);
        }

        ClassRegistry { classes, by_name }
    }

    /// Look up a class definition by name (case-sensitive, display name).
    pub fn get_by_name(&self, name: &str) -> Option<&ClassDef> {
        self.by_name.get(name).map(|&idx| &self.classes[idx])
    }

    /// Look up a class definition by ClassId.
    pub fn get_by_id(&self, id: &ClassId) -> Option<&ClassDef> {
        self.get_by_name(id.as_str())
    }

    /// Iterate over all registered class definitions.
    pub fn all(&self) -> &[ClassDef] {
        &self.classes
    }
}

static CLASS_REGISTRY: std::sync::OnceLock<ClassRegistry> = std::sync::OnceLock::new();

/// Get the global class registry (lazily built from DSL or native definitions).
pub fn class_registry() -> &'static ClassRegistry {
    CLASS_REGISTRY.get_or_init(ClassRegistry::build)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_class_count() {
        assert_eq!(CANONICAL_CLASS_NAMES.len(), 22);
    }

    #[test]
    fn normalize_class_names() {
        assert_eq!(normalize_class_name("fighter"), Some("Fighter"));
        assert_eq!(normalize_class_name("Magic-User"), Some("Magic-User"));
        assert_eq!(normalize_class_name("magic user"), Some("Magic-User"));
        assert_eq!(normalize_class_name("MU"), Some("Magic-User"));
        assert_eq!(normalize_class_name("half-elf"), Some("Half-Elf"));
        assert_eq!(normalize_class_name("halfelf"), Some("Half-Elf"));
        assert_eq!(normalize_class_name("svirfneblin"), Some("Svirfneblin"));
        assert_eq!(normalize_class_name("deep gnome"), Some("Svirfneblin"));
        assert_eq!(normalize_class_name("nonsense"), None);
    }

    #[test]
    fn fighter_def() {
        let def = class_def(&ClassId::new("Fighter"));
        assert_eq!(def.hit_die, 8);
        assert_eq!(def.combat_aptitude, CombatAptitude::Martial);
        assert_eq!(def.max_level, 14);
        assert!(def.weapons_any);
        assert_eq!(def.armour, ArmourPermission::Any);
        assert!(!def.is_demihuman);
    }

    #[test]
    fn magic_user_def() {
        let def = class_def(&ClassId::new("Magic-User"));
        assert_eq!(def.hit_die, 4);
        assert_eq!(def.combat_aptitude, CombatAptitude::NonMartial);
        assert_eq!(def.armour, ArmourPermission::None);
        assert!(!def.weapons_any);
        assert_eq!(def.spell_list, SpellListType::MagicUser);
    }

    #[test]
    fn dwarf_def() {
        let def = class_def(&ClassId::new("Dwarf"));
        assert_eq!(def.hit_die, 8);
        assert!(def.is_demihuman);
        assert_eq!(def.max_level, 12);
        assert_eq!(def.requirements, &[(CON, 9)]);
        assert_eq!(def.racial_modifiers, &[(CON, 1), (CHA, -1)]);
    }

    #[test]
    fn combat_aptitude_groups() {
        use CombatAptitude::*;
        // Per OSE Reference Booklet p19
        let martial = [
            "Barbarian", "Drow", "Duergar", "Dwarf",
            "Elf", "Half-Elf", "Halfling", "Knight",
            "Paladin", "Ranger", "Svirfneblin",
        ];
        let semi = [
            "Acrobat", "Assassin", "Bard",
            "Cleric", "Druid", "Half-Orc", "Thief",
        ];
        let non = ["Gnome", "Illusionist", "Magic-User"];
        // Also Fighter is martial
        assert_eq!(class_def(&ClassId::new("Fighter")).combat_aptitude, Martial);

        for name in martial { assert_eq!(class_def(&ClassId::new(name)).combat_aptitude, Martial, "{}", name); }
        for name in semi { assert_eq!(class_def(&ClassId::new(name)).combat_aptitude, SemiMartial, "{}", name); }
        for name in non { assert_eq!(class_def(&ClassId::new(name)).combat_aptitude, NonMartial, "{}", name); }
    }

    #[test]
    fn meets_fighter_requirements() {
        let abilities = [10, 10, 10, 10, 10, 10];
        assert!(meets_requirements(&ClassId::new("Fighter"), &abilities));
    }

    #[test]
    fn meets_barbarian_requirements() {
        // Barbarian needs STR 9, DEX 9, CON 9
        assert!(meets_requirements(&ClassId::new("Barbarian"), &[9, 10, 10, 9, 9, 10]));
        assert!(!meets_requirements(&ClassId::new("Barbarian"), &[8, 10, 10, 9, 9, 10]));
        assert!(!meets_requirements(&ClassId::new("Barbarian"), &[9, 10, 10, 8, 9, 10]));
    }

    #[test]
    fn meets_dwarf_requirements() {
        assert!(meets_requirements(&ClassId::new("Dwarf"), &[10, 10, 10, 10, 9, 10]));
        assert!(!meets_requirements(&ClassId::new("Dwarf"), &[10, 10, 10, 10, 8, 10]));
    }

    #[test]
    fn apply_dwarf_modifiers() {
        let mut abilities = [10, 10, 10, 10, 10, 10];
        apply_racial_modifiers(&ClassId::new("Dwarf"), &mut abilities);
        assert_eq!(abilities, [10, 10, 10, 10, 11, 9]); // CON+1, CHA-1
    }

    #[test]
    fn apply_halfling_modifiers() {
        let mut abilities = [10, 10, 10, 10, 10, 10];
        apply_racial_modifiers(&ClassId::new("Halfling"), &mut abilities);
        assert_eq!(abilities, [9, 10, 10, 11, 10, 10]); // STR-1, DEX+1
    }

    #[test]
    fn apply_modifier_clamping() {
        let mut abilities = [3, 10, 10, 10, 10, 10];
        apply_racial_modifiers(&ClassId::new("Halfling"), &mut abilities);
        assert_eq!(abilities[0], 3); // STR can't go below 3
    }

    #[test]
    fn eligible_classes_average() {
        let abilities = [10, 10, 10, 10, 10, 10];
        let eligible = eligible_classes(&abilities);
        // Should include Fighter, Cleric, MU, Thief, etc.
        assert!(eligible.contains(&ClassId::new("Fighter")));
        assert!(eligible.contains(&ClassId::new("Cleric")));
        assert!(eligible.contains(&ClassId::new("Magic-User")));
        assert!(eligible.contains(&ClassId::new("Thief")));
        // Should not include classes with CON 9 requirement if CON is 10
        assert!(eligible.contains(&ClassId::new("Dwarf")));
    }

    #[test]
    fn eligible_classes_low_stats() {
        let abilities = [3, 3, 3, 3, 3, 3];
        let eligible = eligible_classes(&abilities);
        // Only classes with no requirements
        assert!(eligible.contains(&ClassId::new("Fighter")));
        assert!(eligible.contains(&ClassId::new("Cleric")));
        assert!(eligible.contains(&ClassId::new("Magic-User")));
        assert!(eligible.contains(&ClassId::new("Thief")));
        assert!(!eligible.contains(&ClassId::new("Dwarf"))); // needs CON 9
        assert!(!eligible.contains(&ClassId::new("Barbarian"))); // needs STR/DEX/CON 9
    }

    // ── Capability tag tests ──────────────────────────────────

    #[test]
    fn has_thief_skills_tags() {
        let thief_classes = ["Thief", "Acrobat", "Assassin", "Half-Orc", "Bard"];
        for &name in &CANONICAL_CLASS_NAMES {
            let def = class_def(&ClassId::new(name));
            if thief_classes.contains(&name) {
                assert!(def.has_thief_skills, "{} should have thief skills", name);
            } else {
                assert!(!def.has_thief_skills, "{} should NOT have thief skills", name);
            }
        }
    }

    #[test]
    fn can_backstab_tags() {
        let backstab_classes = ["Thief", "Assassin"];
        for &name in &CANONICAL_CLASS_NAMES {
            let def = class_def(&ClassId::new(name));
            if backstab_classes.contains(&name) {
                assert!(def.can_backstab, "{} should be able to backstab", name);
            } else {
                assert!(!def.can_backstab, "{} should NOT be able to backstab", name);
            }
        }
    }

    #[test]
    fn can_turn_undead_tags() {
        let turn_classes = ["Cleric", "Paladin"];
        for &name in &CANONICAL_CLASS_NAMES {
            let def = class_def(&ClassId::new(name));
            if turn_classes.contains(&name) {
                assert!(def.can_turn_undead, "{} should be able to turn undead", name);
            } else {
                assert!(!def.can_turn_undead, "{} should NOT be able to turn undead", name);
            }
        }
    }

    #[test]
    fn bx_equivalent_tags() {
        // B/X classes map to None (they ARE the base class)
        let bx_classes = ["Fighter", "Cleric", "Magic-User", "Thief", "Elf", "Dwarf", "Halfling"];
        for &name in &CANONICAL_CLASS_NAMES {
            let def = class_def(&ClassId::new(name));
            if bx_classes.contains(&name) {
                assert_eq!(def.bx_equivalent, None, "{} is a B/X class, bx_equivalent should be None", name);
            } else {
                assert!(def.bx_equivalent.is_some(), "{} is AF, bx_equivalent should be Some", name);
            }
        }
        // Spot-check specific mappings
        assert_eq!(class_def(&ClassId::new("Barbarian")).bx_equivalent, Some(ClassId::new("Fighter")));
        assert_eq!(class_def(&ClassId::new("Acrobat")).bx_equivalent, Some(ClassId::new("Thief")));
        assert_eq!(class_def(&ClassId::new("Druid")).bx_equivalent, Some(ClassId::new("Cleric")));
        assert_eq!(class_def(&ClassId::new("Illusionist")).bx_equivalent, Some(ClassId::new("Magic-User")));
        assert_eq!(class_def(&ClassId::new("Drow")).bx_equivalent, Some(ClassId::new("Elf")));
        assert_eq!(class_def(&ClassId::new("Duergar")).bx_equivalent, Some(ClassId::new("Dwarf")));
        assert_eq!(class_def(&ClassId::new("Gnome")).bx_equivalent, Some(ClassId::new("Halfling")));
    }

    // ── ClassId tests ─────────────────────────────────────────

    #[test]
    fn class_id_new_normalizes() {
        // Case-insensitive, variant-tolerant normalization
        assert_eq!(ClassId::new("fighter").as_str(), "Fighter");
        assert_eq!(ClassId::new("FIGHTER").as_str(), "Fighter");
        assert_eq!(ClassId::new("magic-user").as_str(), "Magic-User");
        assert_eq!(ClassId::new("magicuser").as_str(), "Magic-User");
        assert_eq!(ClassId::new("MU").as_str(), "Magic-User");
        assert_eq!(ClassId::new("half-elf").as_str(), "Half-Elf");
        assert_eq!(ClassId::new("halfelf").as_str(), "Half-Elf");
    }

    #[test]
    fn class_id_unknown_class_preserved() {
        let id = ClassId::new("Necromancer");
        assert_eq!(id.as_str(), "Necromancer");
    }

    #[test]
    fn class_id_from_trait() {
        let id: ClassId = "thief".into();
        assert_eq!(id.as_str(), "Thief");
    }

    #[test]
    fn class_id_display() {
        assert_eq!(format!("{}", ClassId::new("Magic-User")), "Magic-User");
        assert_eq!(format!("{}", ClassId::new("Necromancer")), "Necromancer");
    }

    #[test]
    fn class_id_equality() {
        // Same canonical form
        assert_eq!(ClassId::new("fighter"), ClassId::new("Fighter"));
        assert_eq!(ClassId::new("MU"), ClassId::new("Magic-User"));
        // Different classes
        assert_ne!(ClassId::new("Fighter"), ClassId::new("Thief"));
    }

    #[test]
    fn class_id_serde_roundtrip() {
        let id = ClassId::new("Magic-User");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"Magic-User\"");

        let deserialized: ClassId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn class_id_serde_unknown_class() {
        let json = "\"Necromancer\"";
        let id: ClassId = serde_json::from_str(json).unwrap();
        assert_eq!(id.as_str(), "Necromancer");
    }

    #[test]
    fn class_id_hash_consistent() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ClassId::new("Fighter"));
        assert!(set.contains(&ClassId::new("fighter")));
        assert!(!set.contains(&ClassId::new("Thief")));
    }

    // ── ClassRegistry tests ───────────────────────────────────

    #[test]
    fn registry_covers_all_classes() {
        let reg = class_registry();
        assert_eq!(reg.all().len(), 22);
        for &name in &CANONICAL_CLASS_NAMES {
            let def = reg.get_by_name(name).unwrap();
            assert_eq!(def.class_id.as_str(), name, "registry entry class mismatch for {}", name);
        }
    }

    #[test]
    fn registry_lookup_by_name() {
        let reg = class_registry();
        assert_eq!(reg.get_by_name("Fighter").unwrap().class_id.as_str(), "Fighter");
        assert_eq!(reg.get_by_name("Magic-User").unwrap().class_id.as_str(), "Magic-User");
        assert_eq!(reg.get_by_name("Half-Elf").unwrap().class_id.as_str(), "Half-Elf");
        assert!(reg.get_by_name("Nonexistent").is_none());
    }

    #[test]
    fn registry_lookup_by_class_id() {
        let reg = class_registry();
        let id = ClassId::new("Fighter");
        assert_eq!(reg.get_by_id(&id).unwrap().class_id.as_str(), "Fighter");

        let id = ClassId::new("magic-user");
        assert_eq!(reg.get_by_id(&id).unwrap().class_id.as_str(), "Magic-User");

        let id = ClassId::new("Necromancer");
        assert!(reg.get_by_id(&id).is_none());
    }

    #[test]
    fn registry_parity_with_native() {
        let reg = class_registry();
        for &name in &CANONICAL_CLASS_NAMES {
            let reg_def = reg.get_by_name(name).unwrap();
            let native = native_class_def(name);
            assert_eq!(reg_def.hit_die, native.hit_die, "{} hit_die", name);
            assert_eq!(reg_def.combat_aptitude, native.combat_aptitude, "{} combat_aptitude", name);
            assert_eq!(reg_def.max_level, native.max_level, "{} max_level", name);
            assert_eq!(reg_def.armour, native.armour, "{} armour", name);
            assert_eq!(reg_def.weapons_any, native.weapons_any, "{} weapons_any", name);
            assert_eq!(reg_def.weapons_blunt_only, native.weapons_blunt_only, "{} weapons_blunt_only", name);
            assert_eq!(reg_def.save_category, native.save_category, "{} save_category", name);
            assert_eq!(reg_def.spell_progression, native.spell_progression, "{} spell_progression", name);
            assert_eq!(reg_def.spell_list, native.spell_list, "{} spell_list", name);
            assert_eq!(reg_def.is_demihuman, native.is_demihuman, "{} is_demihuman", name);
            assert_eq!(reg_def.has_thief_skills, native.has_thief_skills, "{} has_thief_skills", name);
            assert_eq!(reg_def.can_backstab, native.can_backstab, "{} can_backstab", name);
            assert_eq!(reg_def.can_turn_undead, native.can_turn_undead, "{} can_turn_undead", name);
            assert_eq!(reg_def.bx_equivalent, native.bx_equivalent, "{} bx_equivalent", name);
        }
    }
}

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_parity_tests {
    use super::*;

    #[test]
    fn dsl_class_def_parity_all_22() {
        for &name in &CANONICAL_CLASS_NAMES {
            let dsl = dsl_gate::dsl_class_def(name);
            let dsl = match dsl {
                Some(d) => d,
                None => panic!("DSL class_def returned None for {}", name),
            };
            let native = native_class_def(name);

            assert_eq!(dsl.hit_die, native.hit_die, "{} hit_die", name);
            assert_eq!(dsl.combat_aptitude, native.combat_aptitude, "{} combat_aptitude", name);
            assert_eq!(dsl.max_level, native.max_level, "{} max_level", name);
            assert_eq!(dsl.armour, native.armour, "{} armour", name);
            assert_eq!(dsl.weapons_any, native.weapons_any, "{} weapons_any", name);
            assert_eq!(dsl.weapons_blunt_only, native.weapons_blunt_only, "{} weapons_blunt_only", name);
            assert_eq!(dsl.save_category, native.save_category, "{} save_category", name);
            assert_eq!(dsl.spell_progression, native.spell_progression, "{} spell_progression", name);
            assert_eq!(dsl.spell_list, native.spell_list, "{} spell_list", name);
            assert_eq!(dsl.is_demihuman, native.is_demihuman, "{} is_demihuman", name);
            assert_eq!(dsl.has_thief_skills, native.has_thief_skills, "{} has_thief_skills", name);
            assert_eq!(dsl.can_backstab, native.can_backstab, "{} can_backstab", name);
            assert_eq!(dsl.can_turn_undead, native.can_turn_undead, "{} can_turn_undead", name);
            assert_eq!(dsl.bx_equivalent, native.bx_equivalent, "{} bx_equivalent", name);
        }
    }

    #[test]
    fn dsl_enum_variants_lists_all_classes() {
        use crate::backend;
        let rt = backend::dsl().expect("DSL should load in test");
        let variants = rt.enum_variants("Class").expect("Class enum should exist");
        assert_eq!(variants.len(), 22, "should have 22 Class variants");
        // Verify all expected variants are present
        for &name in &CANONICAL_CLASS_NAMES {
            let variant = canonical_to_dsl_variant(name);
            assert!(variants.contains(&variant.to_string()), "missing variant: {}", variant);
        }
    }
}
