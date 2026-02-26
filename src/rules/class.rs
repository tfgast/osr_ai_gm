//! Class definitions for OSE B/X 7 + Advanced Fantasy 15 = 22 classes.
//! Data from OSE Reference Booklet p12, p19 and Players Tomes.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use super::save::SaveCategory;
use super::spell::{SpellProgression, SpellListType};

/// All 22 character classes (7 B/X + 15 Advanced Fantasy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Class {
    Acrobat,
    Assassin,
    Barbarian,
    Bard,
    Cleric,
    Drow,
    Druid,
    Duergar,
    Dwarf,
    Elf,
    Fighter,
    Gnome,
    #[serde(rename = "Half-Elf", alias = "HalfElf")]
    HalfElf,
    Halfling,
    #[serde(rename = "Half-Orc", alias = "HalfOrc")]
    HalfOrc,
    Illusionist,
    Knight,
    #[serde(rename = "Magic-User", alias = "MagicUser")]
    MagicUser,
    Paladin,
    Ranger,
    Svirfneblin,
    Thief,
}

impl Class {
    /// All classes in alphabetical order.
    pub const ALL: [Class; 22] = [
        Class::Acrobat, Class::Assassin, Class::Barbarian, Class::Bard,
        Class::Cleric, Class::Drow, Class::Druid, Class::Duergar,
        Class::Dwarf, Class::Elf, Class::Fighter, Class::Gnome,
        Class::HalfElf, Class::Halfling, Class::HalfOrc, Class::Illusionist,
        Class::Knight, Class::MagicUser, Class::Paladin, Class::Ranger,
        Class::Svirfneblin, Class::Thief,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Class::Acrobat => "Acrobat",
            Class::Assassin => "Assassin",
            Class::Barbarian => "Barbarian",
            Class::Bard => "Bard",
            Class::Cleric => "Cleric",
            Class::Drow => "Drow",
            Class::Druid => "Druid",
            Class::Duergar => "Duergar",
            Class::Dwarf => "Dwarf",
            Class::Elf => "Elf",
            Class::Fighter => "Fighter",
            Class::Gnome => "Gnome",
            Class::HalfElf => "Half-Elf",
            Class::Halfling => "Halfling",
            Class::HalfOrc => "Half-Orc",
            Class::Illusionist => "Illusionist",
            Class::Knight => "Knight",
            Class::MagicUser => "Magic-User",
            Class::Paladin => "Paladin",
            Class::Ranger => "Ranger",
            Class::Svirfneblin => "Svirfneblin",
            Class::Thief => "Thief",
        }
    }

    /// Parse class name (case-insensitive, accepts common variants).
    pub fn parse(s: &str) -> Option<Class> {
        match s.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "acrobat" => Some(Class::Acrobat),
            "assassin" => Some(Class::Assassin),
            "barbarian" => Some(Class::Barbarian),
            "bard" => Some(Class::Bard),
            "cleric" => Some(Class::Cleric),
            "drow" => Some(Class::Drow),
            "druid" => Some(Class::Druid),
            "duergar" => Some(Class::Duergar),
            "dwarf" => Some(Class::Dwarf),
            "elf" => Some(Class::Elf),
            "fighter" => Some(Class::Fighter),
            "gnome" => Some(Class::Gnome),
            "halfelf" => Some(Class::HalfElf),
            "halfling" => Some(Class::Halfling),
            "halforc" => Some(Class::HalfOrc),
            "illusionist" => Some(Class::Illusionist),
            "knight" => Some(Class::Knight),
            "magicuser" | "mu" | "mage" | "wizard" => Some(Class::MagicUser),
            "paladin" => Some(Class::Paladin),
            "ranger" => Some(Class::Ranger),
            "svirfneblin" | "deepgnome" => Some(Class::Svirfneblin),
            "thief" => Some(Class::Thief),
            _ => None,
        }
    }
}

impl std::fmt::Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
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
    /// Known classes (the core 22) are normalized via `Class::parse()`.
    /// Unknown names (module/homebrew classes) are stored as-is.
    pub fn new(s: &str) -> Self {
        if let Some(class) = Class::parse(s) {
            ClassId(Arc::from(class.name()))
        } else {
            ClassId(Arc::from(s))
        }
    }

    /// Create a ClassId from a `Class` enum variant.
    pub fn from_enum(class: Class) -> Self {
        ClassId(Arc::from(class.name()))
    }

    /// Try to resolve this ClassId back to a `Class` enum variant.
    /// Returns `None` for module/homebrew classes not in the core 22.
    pub fn to_enum(&self) -> Option<Class> {
        Class::parse(&self.0)
    }

    /// The canonical string form of this class identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ClassId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<Class> for ClassId {
    fn from(class: Class) -> Self {
        ClassId::from_enum(class)
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
    pub class: Class,
    pub hit_die: u32,
    pub combat_aptitude: CombatAptitude,
    pub prime_requisites: &'static [usize], // ability indices
    pub requirements: &'static [AbilityRequirement],
    pub racial_modifiers: &'static [AbilityModifier],
    pub max_level: u32,
    pub armour: ArmourPermission,
    pub weapons_any: bool,       // true = any weapon, false = restricted
    pub weapons_blunt_only: bool, // if !weapons_any, only blunt weapons
    pub save_category: SaveCategory,
    pub spell_progression: SpellProgression,
    pub spell_list: SpellListType,
    pub starting_gold: &'static str, // dice notation for gold in gp
    pub is_demihuman: bool,
    // Capability tags (populated from DSL or native fallback)
    pub has_thief_skills: bool,
    pub can_backstab: bool,
    pub can_turn_undead: bool,
    pub bx_equivalent: Option<Class>,
}

/// Ability index constants.
pub const STR: usize = 0;
pub const INT: usize = 1;
pub const WIS: usize = 2;
pub const DEX: usize = 3;
pub const CON: usize = 4;
pub const CHA: usize = 5;

/// Get the full class definition for a given class.
pub fn class_def(class: Class) -> ClassDef {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Class) {
        if let Some(def) = dsl_gate::dsl_class_def(class) {
            return def;
        }
    }
    native_class_def(class)
}

fn native_class_def(class: Class) -> ClassDef {
    match class {
        Class::Acrobat => ClassDef {
            class, hit_die: 4,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[DEX],
            requirements: &[(DEX, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::None,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Thief,
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: true, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Thief),
        },
        Class::Assassin => ClassDef {
            class, hit_die: 4,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[DEX],
            requirements: &[(DEX, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Leather,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Thief,
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: true, can_backstab: true, can_turn_undead: false,
            bx_equivalent: Some(Class::Thief),
        },
        Class::Barbarian => ClassDef {
            class, hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(STR, 9), (DEX, 9), (CON, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Barbarian,
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Fighter),
        },
        Class::Bard => ClassDef {
            class, hit_die: 6,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[CHA],
            requirements: &[(DEX, 9), (CHA, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::LeatherShield,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Thief,
            spell_progression: SpellProgression::Bard,
            spell_list: SpellListType::Druid,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: true, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Thief),
        },
        Class::Cleric => ClassDef {
            class, hit_die: 6,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[WIS],
            requirements: &[],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: false, weapons_blunt_only: true,
            save_category: SaveCategory::Cleric,
            spell_progression: SpellProgression::Cleric,
            spell_list: SpellListType::Cleric,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: true,
            bx_equivalent: None,
        },
        Class::Drow => ClassDef {
            class, hit_die: 6,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR, INT],
            requirements: &[(INT, 9)],
            racial_modifiers: &[(DEX, 1), (CON, -1)],
            max_level: 10, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Drow,
            spell_progression: SpellProgression::Drow,
            spell_list: SpellListType::DrowArcaneAndDivine,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Elf),
        },
        Class::Druid => ClassDef {
            class, hit_die: 6,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[WIS],
            requirements: &[],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Leather,
            weapons_any: false, weapons_blunt_only: true,
            save_category: SaveCategory::Cleric,
            spell_progression: SpellProgression::Druid,
            spell_list: SpellListType::Druid,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Cleric),
        },
        Class::Duergar => ClassDef {
            class, hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(INT, 9), (CON, 9)],
            racial_modifiers: &[(CON, 1), (CHA, -1)],
            max_level: 10, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Dwarf,
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Dwarf),
        },
        Class::Dwarf => ClassDef {
            class, hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(CON, 9)],
            racial_modifiers: &[(CON, 1), (CHA, -1)],
            max_level: 12, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Dwarf,
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: None,
        },
        Class::Elf => ClassDef {
            class, hit_die: 6,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR, INT],
            requirements: &[(INT, 9)],
            racial_modifiers: &[(DEX, 1), (CON, -1)],
            max_level: 10, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Elf,
            spell_progression: SpellProgression::ArcaneFullCaster,
            spell_list: SpellListType::MagicUser,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: None,
        },
        Class::Fighter => ClassDef {
            class, hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Fighter,
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: None,
        },
        Class::Gnome => ClassDef {
            class, hit_die: 6,
            combat_aptitude: CombatAptitude::NonMartial,
            prime_requisites: &[DEX, INT],
            requirements: &[(INT, 9), (CON, 9)],
            racial_modifiers: &[],
            max_level: 8, armour: ArmourPermission::Leather,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Gnome,
            spell_progression: SpellProgression::ArcaneFullCaster,
            spell_list: SpellListType::Illusionist,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Halfling),
        },
        Class::HalfElf => ClassDef {
            class, hit_die: 6,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR, INT],
            requirements: &[(CON, 9), (CHA, 9)],
            racial_modifiers: &[],
            max_level: 12, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::HalfElf,
            spell_progression: SpellProgression::HalfElf,
            spell_list: SpellListType::MagicUser,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Elf),
        },
        Class::Halfling => ClassDef {
            class, hit_die: 6,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR, DEX],
            requirements: &[(CON, 9), (DEX, 9)],
            racial_modifiers: &[(STR, -1), (DEX, 1)],
            max_level: 8, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Dwarf,
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: None,
        },
        Class::HalfOrc => ClassDef {
            class, hit_die: 6,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[STR, DEX],
            requirements: &[],
            racial_modifiers: &[(STR, 1), (CON, 1), (CHA, -2)],
            max_level: 8, armour: ArmourPermission::Leather,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::HalfOrc,
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: true, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Dwarf),
        },
        Class::Illusionist => ClassDef {
            class, hit_die: 4,
            combat_aptitude: CombatAptitude::NonMartial,
            prime_requisites: &[INT],
            requirements: &[(DEX, 9), (INT, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::None,
            weapons_any: false, weapons_blunt_only: false, // dagger only
            save_category: SaveCategory::MagicUser,
            spell_progression: SpellProgression::ArcaneFullCaster,
            spell_list: SpellListType::Illusionist,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::MagicUser),
        },
        Class::Knight => ClassDef {
            class, hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(STR, 9), (CHA, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Fighter,
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Fighter),
        },
        Class::MagicUser => ClassDef {
            class, hit_die: 4,
            combat_aptitude: CombatAptitude::NonMartial,
            prime_requisites: &[INT],
            requirements: &[],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::None,
            weapons_any: false, weapons_blunt_only: false, // dagger only
            save_category: SaveCategory::MagicUser,
            spell_progression: SpellProgression::ArcaneFullCaster,
            spell_list: SpellListType::MagicUser,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: None,
        },
        Class::Paladin => ClassDef {
            class, hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(CHA, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Paladin,
            spell_progression: SpellProgression::Paladin,
            spell_list: SpellListType::Cleric,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: true,
            bx_equivalent: Some(Class::Fighter),
        },
        Class::Ranger => ClassDef {
            class, hit_die: 8,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[STR],
            requirements: &[(STR, 9), (WIS, 9)],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Any,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Fighter,
            spell_progression: SpellProgression::Ranger,
            spell_list: SpellListType::Druid,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Fighter),
        },
        Class::Svirfneblin => ClassDef {
            class, hit_die: 6,
            combat_aptitude: CombatAptitude::Martial,
            prime_requisites: &[DEX, INT],
            requirements: &[(CON, 9)],
            racial_modifiers: &[],
            max_level: 8, armour: ArmourPermission::Leather,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Svirfneblin,
            spell_progression: SpellProgression::ArcaneFullCaster,
            spell_list: SpellListType::Illusionist,
            starting_gold: "3d6x10", is_demihuman: true,
            has_thief_skills: false, can_backstab: false, can_turn_undead: false,
            bx_equivalent: Some(Class::Halfling),
        },
        Class::Thief => ClassDef {
            class, hit_die: 4,
            combat_aptitude: CombatAptitude::SemiMartial,
            prime_requisites: &[DEX],
            requirements: &[],
            racial_modifiers: &[],
            max_level: 14, armour: ArmourPermission::Leather,
            weapons_any: true, weapons_blunt_only: false,
            save_category: SaveCategory::Thief,
            spell_progression: SpellProgression::NonCaster,
            spell_list: SpellListType::None,
            starting_gold: "3d6x10", is_demihuman: false,
            has_thief_skills: true, can_backstab: true, can_turn_undead: false,
            bx_equivalent: None,
        },
    }
}

/// Check if a set of ability scores meets the requirements for a class.
/// Abilities array: [STR, INT, WIS, DEX, CON, CHA] (after racial modifiers).
pub fn meets_requirements(class: Class, abilities: &[i32; 6]) -> bool {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Class) {
        if let Some(val) = dsl_gate::dsl_meets_requirements(class, abilities) {
            return val;
        }
    }
    let def = class_def(class);
    for &(idx, min) in def.requirements {
        if abilities[idx] < min {
            return false;
        }
    }
    true
}

/// Apply racial ability modifiers for a demihuman class.
/// Modifies abilities in place and clamps to 3..=18.
pub fn apply_racial_modifiers(class: Class, abilities: &mut [i32; 6]) {
    let def = class_def(class);
    for &(idx, modifier) in def.racial_modifiers {
        abilities[idx] = (abilities[idx] + modifier).clamp(3, 18);
    }
}

/// Get all classes that a given set of abilities qualifies for.
/// For demihuman classes, racial modifiers are applied before checking requirements.
pub fn eligible_classes(abilities: &[i32; 6]) -> Vec<Class> {
    Class::ALL.iter()
        .copied()
        .filter(|&c| {
            let def = class_def(c);
            if def.racial_modifiers.is_empty() {
                meets_requirements(c, abilities)
            } else {
                let mut modified = *abilities;
                apply_racial_modifiers(c, &mut modified);
                meets_requirements(c, &modified)
            }
        })
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

    fn class_to_dsl(class: Class) -> Value {
        Value::EnumVariant {
            enum_name: "Class".into(),
            variant: Name::from(format!("{:?}", class)),
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

    fn parse_save_category(s: &str) -> Option<SaveCategory> {
        match s {
            "save_thief" => Some(SaveCategory::Thief),
            "save_barbarian" => Some(SaveCategory::Barbarian),
            "save_cleric" => Some(SaveCategory::Cleric),
            "save_drow" => Some(SaveCategory::Drow),
            "save_dwarf" => Some(SaveCategory::Dwarf),
            "save_elf" => Some(SaveCategory::Elf),
            "save_fighter" => Some(SaveCategory::Fighter),
            "save_gnome" => Some(SaveCategory::Gnome),
            "save_half_elf" => Some(SaveCategory::HalfElf),
            "save_half_orc" => Some(SaveCategory::HalfOrc),
            "save_magic_user" => Some(SaveCategory::MagicUser),
            "save_paladin" => Some(SaveCategory::Paladin),
            "save_svirfneblin" => Some(SaveCategory::Svirfneblin),
            _ => None,
        }
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

    pub fn dsl_class_def(class: Class) -> Option<ClassDef> {
        let rt = backend::dsl()?;
        let args = vec![class_to_dsl(class)];
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

        // Capability tags (new fields from extended ClassDef)
        let has_thief_skills = dsl_bool(fields, "has_thief_skills")?;
        let can_backstab = dsl_bool(fields, "can_backstab")?;
        let can_turn_undead = dsl_bool(fields, "can_turn_undead")?;
        let bx_equivalent = match fields.get(&Name::from("bx_equivalent")) {
            Some(Value::EnumVariant { variant, .. }) => {
                // DSL uses bare Class (not option<Class>). Self-mapping means
                // "this IS a B/X class" → None. Different class → Some(equivalent).
                match Class::parse(variant.as_str()) {
                    Some(equiv) if equiv == class => None,
                    other => other,
                }
            }
            _ => None,
        };

        // Fields not in DSL — backfill from native
        let native = native_class_def(class);

        Some(ClassDef {
            class,
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

    pub fn dsl_meets_requirements(class: Class, abilities: &[i32; 6]) -> Option<bool> {
        let rt = backend::dsl()?;

        // Build DSL map<Ability, int> from ability array
        let ability_names = ["STR", "INT", "WIS", "DEX", "CON", "CHA"];
        let mut ability_map = BTreeMap::new();
        for (i, &name) in ability_names.iter().enumerate() {
            let key = Value::EnumVariant {
                enum_name: "Ability".into(),
                variant: Name::from(name),
                fields: BTreeMap::new(),
            };
            ability_map.insert(key, Value::Int(abilities[i] as i64));
        }

        let args = vec![
            class_to_dsl(class),
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
    /// Build the registry by evaluating DSL class_def for each Class variant.
    /// Falls back to native_class_def if DSL is unavailable or evaluation fails.
    fn build() -> Self {
        let mut classes = Vec::with_capacity(Class::ALL.len());
        let mut by_name = std::collections::HashMap::with_capacity(Class::ALL.len());

        for &class in &Class::ALL {
            let def = class_def(class);
            by_name.insert(class.name().to_string(), classes.len());
            classes.push(def);
        }

        ClassRegistry { classes, by_name }
    }

    /// Look up a class definition by Class enum variant.
    pub fn get(&self, class: Class) -> &ClassDef {
        // Safe: registry is built from Class::ALL which covers all variants
        let idx = self.by_name[class.name()];
        &self.classes[idx]
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
    fn class_count() {
        assert_eq!(Class::ALL.len(), 22);
    }

    #[test]
    fn parse_classes() {
        assert_eq!(Class::parse("fighter"), Some(Class::Fighter));
        assert_eq!(Class::parse("Magic-User"), Some(Class::MagicUser));
        assert_eq!(Class::parse("magic user"), Some(Class::MagicUser));
        assert_eq!(Class::parse("MU"), Some(Class::MagicUser));
        assert_eq!(Class::parse("half-elf"), Some(Class::HalfElf));
        assert_eq!(Class::parse("halfelf"), Some(Class::HalfElf));
        assert_eq!(Class::parse("svirfneblin"), Some(Class::Svirfneblin));
        assert_eq!(Class::parse("deep gnome"), Some(Class::Svirfneblin));
        assert_eq!(Class::parse("nonsense"), None);
    }

    #[test]
    fn fighter_def() {
        let def = class_def(Class::Fighter);
        assert_eq!(def.hit_die, 8);
        assert_eq!(def.combat_aptitude, CombatAptitude::Martial);
        assert_eq!(def.max_level, 14);
        assert!(def.weapons_any);
        assert_eq!(def.armour, ArmourPermission::Any);
        assert!(!def.is_demihuman);
    }

    #[test]
    fn magic_user_def() {
        let def = class_def(Class::MagicUser);
        assert_eq!(def.hit_die, 4);
        assert_eq!(def.combat_aptitude, CombatAptitude::NonMartial);
        assert_eq!(def.armour, ArmourPermission::None);
        assert!(!def.weapons_any);
        assert_eq!(def.spell_list, SpellListType::MagicUser);
    }

    #[test]
    fn dwarf_def() {
        let def = class_def(Class::Dwarf);
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
            Class::Barbarian, Class::Drow, Class::Duergar, Class::Dwarf,
            Class::Elf, Class::HalfElf, Class::Halfling, Class::Knight,
            Class::Paladin, Class::Ranger, Class::Svirfneblin,
        ];
        let semi = [
            Class::Acrobat, Class::Assassin, Class::Bard,
            Class::Cleric, Class::Druid, Class::HalfOrc, Class::Thief,
        ];
        let non = [Class::Gnome, Class::Illusionist, Class::MagicUser];
        // Also Fighter is martial
        assert_eq!(class_def(Class::Fighter).combat_aptitude, Martial);

        for c in martial { assert_eq!(class_def(c).combat_aptitude, Martial, "{:?}", c); }
        for c in semi { assert_eq!(class_def(c).combat_aptitude, SemiMartial, "{:?}", c); }
        for c in non { assert_eq!(class_def(c).combat_aptitude, NonMartial, "{:?}", c); }
    }

    #[test]
    fn meets_fighter_requirements() {
        let abilities = [10, 10, 10, 10, 10, 10];
        assert!(meets_requirements(Class::Fighter, &abilities));
    }

    #[test]
    fn meets_barbarian_requirements() {
        // Barbarian needs STR 9, DEX 9, CON 9
        assert!(meets_requirements(Class::Barbarian, &[9, 10, 10, 9, 9, 10]));
        assert!(!meets_requirements(Class::Barbarian, &[8, 10, 10, 9, 9, 10]));
        assert!(!meets_requirements(Class::Barbarian, &[9, 10, 10, 8, 9, 10]));
    }

    #[test]
    fn meets_dwarf_requirements() {
        assert!(meets_requirements(Class::Dwarf, &[10, 10, 10, 10, 9, 10]));
        assert!(!meets_requirements(Class::Dwarf, &[10, 10, 10, 10, 8, 10]));
    }

    #[test]
    fn apply_dwarf_modifiers() {
        let mut abilities = [10, 10, 10, 10, 10, 10];
        apply_racial_modifiers(Class::Dwarf, &mut abilities);
        assert_eq!(abilities, [10, 10, 10, 10, 11, 9]); // CON+1, CHA-1
    }

    #[test]
    fn apply_halfling_modifiers() {
        let mut abilities = [10, 10, 10, 10, 10, 10];
        apply_racial_modifiers(Class::Halfling, &mut abilities);
        assert_eq!(abilities, [9, 10, 10, 11, 10, 10]); // STR-1, DEX+1
    }

    #[test]
    fn apply_modifier_clamping() {
        let mut abilities = [3, 10, 10, 10, 10, 10];
        apply_racial_modifiers(Class::Halfling, &mut abilities);
        assert_eq!(abilities[0], 3); // STR can't go below 3
    }

    #[test]
    fn eligible_classes_average() {
        let abilities = [10, 10, 10, 10, 10, 10];
        let eligible = eligible_classes(&abilities);
        // Should include Fighter, Cleric, MU, Thief, etc.
        assert!(eligible.contains(&Class::Fighter));
        assert!(eligible.contains(&Class::Cleric));
        assert!(eligible.contains(&Class::MagicUser));
        assert!(eligible.contains(&Class::Thief));
        // Should not include classes with CON 9 requirement if CON is 10
        assert!(eligible.contains(&Class::Dwarf));
    }

    #[test]
    fn eligible_classes_low_stats() {
        let abilities = [3, 3, 3, 3, 3, 3];
        let eligible = eligible_classes(&abilities);
        // Only classes with no requirements
        assert!(eligible.contains(&Class::Fighter));
        assert!(eligible.contains(&Class::Cleric));
        assert!(eligible.contains(&Class::MagicUser));
        assert!(eligible.contains(&Class::Thief));
        assert!(!eligible.contains(&Class::Dwarf)); // needs CON 9
        assert!(!eligible.contains(&Class::Barbarian)); // needs STR/DEX/CON 9
    }

    // ── Capability tag tests ──────────────────────────────────

    #[test]
    fn has_thief_skills_tags() {
        let thief_classes = [
            Class::Thief, Class::Acrobat, Class::Assassin,
            Class::HalfOrc, Class::Bard,
        ];
        for c in Class::ALL {
            let def = class_def(c);
            if thief_classes.contains(&c) {
                assert!(def.has_thief_skills, "{:?} should have thief skills", c);
            } else {
                assert!(!def.has_thief_skills, "{:?} should NOT have thief skills", c);
            }
        }
    }

    #[test]
    fn can_backstab_tags() {
        let backstab_classes = [Class::Thief, Class::Assassin];
        for c in Class::ALL {
            let def = class_def(c);
            if backstab_classes.contains(&c) {
                assert!(def.can_backstab, "{:?} should be able to backstab", c);
            } else {
                assert!(!def.can_backstab, "{:?} should NOT be able to backstab", c);
            }
        }
    }

    #[test]
    fn can_turn_undead_tags() {
        let turn_classes = [Class::Cleric, Class::Paladin];
        for c in Class::ALL {
            let def = class_def(c);
            if turn_classes.contains(&c) {
                assert!(def.can_turn_undead, "{:?} should be able to turn undead", c);
            } else {
                assert!(!def.can_turn_undead, "{:?} should NOT be able to turn undead", c);
            }
        }
    }

    #[test]
    fn bx_equivalent_tags() {
        // B/X classes map to None (they ARE the base class)
        let bx_classes = [
            Class::Fighter, Class::Cleric, Class::MagicUser, Class::Thief,
            Class::Elf, Class::Dwarf, Class::Halfling,
        ];
        for c in Class::ALL {
            let def = class_def(c);
            if bx_classes.contains(&c) {
                assert_eq!(def.bx_equivalent, None, "{:?} is a B/X class, bx_equivalent should be None", c);
            } else {
                assert!(def.bx_equivalent.is_some(), "{:?} is AF, bx_equivalent should be Some", c);
            }
        }
        // Spot-check specific mappings
        assert_eq!(class_def(Class::Barbarian).bx_equivalent, Some(Class::Fighter));
        assert_eq!(class_def(Class::Acrobat).bx_equivalent, Some(Class::Thief));
        assert_eq!(class_def(Class::Druid).bx_equivalent, Some(Class::Cleric));
        assert_eq!(class_def(Class::Illusionist).bx_equivalent, Some(Class::MagicUser));
        assert_eq!(class_def(Class::Drow).bx_equivalent, Some(Class::Elf));
        assert_eq!(class_def(Class::Duergar).bx_equivalent, Some(Class::Dwarf));
        assert_eq!(class_def(Class::Gnome).bx_equivalent, Some(Class::Halfling));
    }

    // ── ClassId tests ─────────────────────────────────────────

    #[test]
    fn class_id_from_enum_roundtrip() {
        for &c in &Class::ALL {
            let id = ClassId::from_enum(c);
            assert_eq!(id.to_enum(), Some(c), "roundtrip failed for {:?}", c);
            assert_eq!(id.as_str(), c.name());
        }
    }

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
        assert_eq!(id.to_enum(), None);
    }

    #[test]
    fn class_id_from_trait() {
        let id: ClassId = Class::Fighter.into();
        assert_eq!(id.as_str(), "Fighter");

        let id: ClassId = "thief".into();
        assert_eq!(id.as_str(), "Thief");
    }

    #[test]
    fn class_id_display() {
        assert_eq!(format!("{}", ClassId::from_enum(Class::MagicUser)), "Magic-User");
        assert_eq!(format!("{}", ClassId::new("Necromancer")), "Necromancer");
    }

    #[test]
    fn class_id_equality() {
        // Same canonical form
        assert_eq!(ClassId::new("fighter"), ClassId::from_enum(Class::Fighter));
        assert_eq!(ClassId::new("MU"), ClassId::new("Magic-User"));
        // Different classes
        assert_ne!(ClassId::from_enum(Class::Fighter), ClassId::from_enum(Class::Thief));
    }

    #[test]
    fn class_id_serde_roundtrip() {
        let id = ClassId::from_enum(Class::MagicUser);
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
        assert_eq!(id.to_enum(), None);
    }

    #[test]
    fn class_id_hash_consistent() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ClassId::from_enum(Class::Fighter));
        assert!(set.contains(&ClassId::new("fighter")));
        assert!(!set.contains(&ClassId::new("Thief")));
    }

    // ── ClassRegistry tests ───────────────────────────────────

    #[test]
    fn registry_covers_all_classes() {
        let reg = class_registry();
        assert_eq!(reg.all().len(), 22);
        for &c in &Class::ALL {
            let def = reg.get(c);
            assert_eq!(def.class, c, "registry entry class mismatch for {:?}", c);
        }
    }

    #[test]
    fn registry_lookup_by_name() {
        let reg = class_registry();
        assert_eq!(reg.get_by_name("Fighter").unwrap().class, Class::Fighter);
        assert_eq!(reg.get_by_name("Magic-User").unwrap().class, Class::MagicUser);
        assert_eq!(reg.get_by_name("Half-Elf").unwrap().class, Class::HalfElf);
        assert!(reg.get_by_name("Nonexistent").is_none());
    }

    #[test]
    fn registry_lookup_by_class_id() {
        let reg = class_registry();
        let id = ClassId::from_enum(Class::Fighter);
        assert_eq!(reg.get_by_id(&id).unwrap().class, Class::Fighter);

        let id = ClassId::new("magic-user");
        assert_eq!(reg.get_by_id(&id).unwrap().class, Class::MagicUser);

        let id = ClassId::new("Necromancer");
        assert!(reg.get_by_id(&id).is_none());
    }

    #[test]
    fn registry_parity_with_native() {
        let reg = class_registry();
        for &c in &Class::ALL {
            let reg_def = reg.get(c);
            let native = native_class_def(c);
            assert_eq!(reg_def.hit_die, native.hit_die, "{:?} hit_die", c);
            assert_eq!(reg_def.combat_aptitude, native.combat_aptitude, "{:?} combat_aptitude", c);
            assert_eq!(reg_def.max_level, native.max_level, "{:?} max_level", c);
            assert_eq!(reg_def.armour, native.armour, "{:?} armour", c);
            assert_eq!(reg_def.weapons_any, native.weapons_any, "{:?} weapons_any", c);
            assert_eq!(reg_def.weapons_blunt_only, native.weapons_blunt_only, "{:?} weapons_blunt_only", c);
            assert_eq!(reg_def.save_category, native.save_category, "{:?} save_category", c);
            assert_eq!(reg_def.spell_progression, native.spell_progression, "{:?} spell_progression", c);
            assert_eq!(reg_def.spell_list, native.spell_list, "{:?} spell_list", c);
            assert_eq!(reg_def.is_demihuman, native.is_demihuman, "{:?} is_demihuman", c);
            assert_eq!(reg_def.has_thief_skills, native.has_thief_skills, "{:?} has_thief_skills", c);
            assert_eq!(reg_def.can_backstab, native.can_backstab, "{:?} can_backstab", c);
            assert_eq!(reg_def.can_turn_undead, native.can_turn_undead, "{:?} can_turn_undead", c);
            assert_eq!(reg_def.bx_equivalent, native.bx_equivalent, "{:?} bx_equivalent", c);
        }
    }
}

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_parity_tests {
    use super::*;

    #[test]
    fn dsl_class_def_parity_all_22() {
        // Explicitly evaluate each class through the DSL path and compare
        // field-by-field against the native definition.
        for &class in &Class::ALL {
            let dsl = dsl_gate::dsl_class_def(class);
            let dsl = match dsl {
                Some(d) => d,
                None => panic!("DSL class_def returned None for {:?}", class),
            };
            let native = native_class_def(class);

            assert_eq!(dsl.hit_die, native.hit_die, "{:?} hit_die", class);
            assert_eq!(dsl.combat_aptitude, native.combat_aptitude, "{:?} combat_aptitude", class);
            assert_eq!(dsl.max_level, native.max_level, "{:?} max_level", class);
            assert_eq!(dsl.armour, native.armour, "{:?} armour", class);
            assert_eq!(dsl.weapons_any, native.weapons_any, "{:?} weapons_any", class);
            assert_eq!(dsl.weapons_blunt_only, native.weapons_blunt_only, "{:?} weapons_blunt_only", class);
            assert_eq!(dsl.save_category, native.save_category, "{:?} save_category", class);
            assert_eq!(dsl.spell_progression, native.spell_progression, "{:?} spell_progression", class);
            assert_eq!(dsl.spell_list, native.spell_list, "{:?} spell_list", class);
            assert_eq!(dsl.is_demihuman, native.is_demihuman, "{:?} is_demihuman", class);
            assert_eq!(dsl.has_thief_skills, native.has_thief_skills, "{:?} has_thief_skills", class);
            assert_eq!(dsl.can_backstab, native.can_backstab, "{:?} can_backstab", class);
            assert_eq!(dsl.can_turn_undead, native.can_turn_undead, "{:?} can_turn_undead", class);
            assert_eq!(dsl.bx_equivalent, native.bx_equivalent, "{:?} bx_equivalent", class);
        }
    }

    #[test]
    fn dsl_enum_variants_lists_all_classes() {
        use crate::backend;
        let rt = backend::dsl().expect("DSL should load in test");
        let variants = rt.enum_variants("Class").expect("Class enum should exist");
        assert_eq!(variants.len(), 22, "should have 22 Class variants");
        // Verify all expected variants are present
        for &class in &Class::ALL {
            let name = format!("{:?}", class);
            assert!(variants.contains(&name), "missing variant: {}", name);
        }
    }
}
