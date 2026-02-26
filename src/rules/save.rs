//! Saving throw tables per OSE Reference Booklet p13.
//!
//! Supports DSL backend gate: when `OSR_BACKEND_SAVES=dsl` is set and the
//! `dsl-backend` feature is enabled, saving throw lookups are evaluated
//! through the DSL runtime instead of the hardcoded tables below.
//!
//! SavingThrows is a dynamic map (HashMap<String, u32>) so that different
//! game systems can define their own save categories (e.g. OSE has 5,
//! other systems may have 3 or 1).

use std::collections::HashMap;
use std::sync::Arc;

/// Dynamic saving throw map: save_name → target number.
///
/// OSE uses five saves (death, wands, paralysis, breath, spells).
/// Other game systems may define different save categories.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SavingThrows(pub HashMap<String, u32>);

impl SavingThrows {
    /// Create a new empty SavingThrows map.
    pub fn new_empty() -> Self {
        SavingThrows(HashMap::new())
    }

    /// Create SavingThrows from the standard OSE 5-save format.
    /// Convenience constructor preserving backward compatibility.
    pub fn new(d: u32, w: u32, p: u32, b: u32, s: u32) -> Self {
        let mut map = HashMap::new();
        map.insert("death".to_string(), d);
        map.insert("wands".to_string(), w);
        map.insert("paralysis".to_string(), p);
        map.insert("breath".to_string(), b);
        map.insert("spells".to_string(), s);
        SavingThrows(map)
    }

    /// Get a save value by name. Returns None if the save type doesn't exist.
    pub fn get(&self, name: &str) -> Option<u32> {
        self.0.get(name).copied()
    }

    /// Convenience accessors for OSE's five save types.
    pub fn death(&self) -> u32 { self.0.get("death").copied().unwrap_or(20) }
    pub fn wands(&self) -> u32 { self.0.get("wands").copied().unwrap_or(20) }
    pub fn paralysis(&self) -> u32 { self.0.get("paralysis").copied().unwrap_or(20) }
    pub fn breath(&self) -> u32 { self.0.get("breath").copied().unwrap_or(20) }
    pub fn spells(&self) -> u32 { self.0.get("spells").copied().unwrap_or(20) }
}

/// Each class maps to one of these save table groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SaveCategory {
    Thief,       // Acrobat, Assassin, Bard, Thief
    Barbarian,
    Cleric,      // Cleric, Druid
    Drow,
    Dwarf,       // Duergar, Dwarf, Halfling
    Elf,
    Fighter,     // Fighter, Knight, Ranger
    Gnome,
    HalfElf,
    HalfOrc,
    MagicUser,   // Illusionist, Magic-User
    Paladin,
    Svirfneblin,
}

impl SaveCategory {
    /// Canonical display name for this save category.
    pub fn name(self) -> &'static str {
        match self {
            SaveCategory::Thief => "Thief",
            SaveCategory::Barbarian => "Barbarian",
            SaveCategory::Cleric => "Cleric",
            SaveCategory::Drow => "Drow",
            SaveCategory::Dwarf => "Dwarf",
            SaveCategory::Elf => "Elf",
            SaveCategory::Fighter => "Fighter",
            SaveCategory::Gnome => "Gnome",
            SaveCategory::HalfElf => "Half-Elf",
            SaveCategory::HalfOrc => "Half-Orc",
            SaveCategory::MagicUser => "Magic-User",
            SaveCategory::Paladin => "Paladin",
            SaveCategory::Svirfneblin => "Svirfneblin",
        }
    }

    /// Parse save category name (case-insensitive, accepts common variants).
    pub fn parse(s: &str) -> Option<SaveCategory> {
        match s.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "thief" | "savethief" => Some(SaveCategory::Thief),
            "barbarian" | "savebarbarian" => Some(SaveCategory::Barbarian),
            "cleric" | "savecleric" => Some(SaveCategory::Cleric),
            "drow" | "savedrow" => Some(SaveCategory::Drow),
            "dwarf" | "savedwarf" => Some(SaveCategory::Dwarf),
            "elf" | "saveelf" => Some(SaveCategory::Elf),
            "fighter" | "savefighter" => Some(SaveCategory::Fighter),
            "gnome" | "savegnome" => Some(SaveCategory::Gnome),
            "halfelf" | "savehalfelf" => Some(SaveCategory::HalfElf),
            "halforc" | "savehalforc" => Some(SaveCategory::HalfOrc),
            "magicuser" | "mu" | "savemagicuser" => Some(SaveCategory::MagicUser),
            "paladin" | "savepaladin" => Some(SaveCategory::Paladin),
            "svirfneblin" | "deepgnome" | "savesvirfneblin" => Some(SaveCategory::Svirfneblin),
            _ => None,
        }
    }
}

impl std::fmt::Display for SaveCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

// ── SaveCategoryId: string-based save category identifier ───

/// String-based save category identifier for data-driven lookups.
/// Wraps `Arc<str>` for O(1) clone. Canonical form is the display name
/// (e.g., "Fighter", "Magic-User", "Half-Elf").
/// Serializes transparently as a plain string for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct SaveCategoryId(Arc<str>);

impl<'de> serde::Deserialize<'de> for SaveCategoryId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(SaveCategoryId::new(&s))
    }
}

impl SaveCategoryId {
    /// Create a SaveCategoryId, normalizing to canonical form.
    /// Known categories are normalized via `SaveCategory::parse()`.
    /// Unknown names (homebrew categories) are stored as-is.
    pub fn new(s: &str) -> Self {
        if let Some(cat) = SaveCategory::parse(s) {
            SaveCategoryId(Arc::from(cat.name()))
        } else {
            SaveCategoryId(Arc::from(s))
        }
    }

    /// Create a SaveCategoryId from a `SaveCategory` enum variant.
    pub fn from_enum(cat: SaveCategory) -> Self {
        SaveCategoryId(Arc::from(cat.name()))
    }

    /// Try to resolve this SaveCategoryId back to a `SaveCategory` enum variant.
    /// Returns `None` for homebrew categories not in the core 13.
    pub fn to_enum(&self) -> Option<SaveCategory> {
        SaveCategory::parse(&self.0)
    }

    /// The canonical string form of this save category identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Display name for this save category (same as as_str).
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SaveCategoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<SaveCategory> for SaveCategoryId {
    fn from(cat: SaveCategory) -> Self {
        SaveCategoryId::from_enum(cat)
    }
}

impl From<&str> for SaveCategoryId {
    fn from(s: &str) -> Self {
        SaveCategoryId::new(s)
    }
}

/// Look up saving throws by category and character level.
///
/// When `dsl-backend` feature is enabled and `OSR_BACKEND_SAVES=dsl`,
/// delegates to the DSL runtime. Falls back to native on DSL failure.
pub fn saving_throws(cat: &SaveCategoryId, level: u32) -> SavingThrows {
    let cat_enum = cat.to_enum()
        .unwrap_or_else(|| panic!("unknown save category: {}", cat));
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Saves) {
        match dsl_saving_throws(cat_enum, level) {
            Some(saves) => return saves,
            None => eprintln!("DSL saves evaluation failed for {:?} L{}, falling back to native", cat_enum, level),
        }
    }
    native_saving_throws(cat_enum, level)
}

// ── DSL backend ─────────────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
fn dsl_saving_throws(cat: SaveCategory, level: u32) -> Option<SavingThrows> {
    use std::collections::BTreeMap;
    use ttrpg_interp::effect::{Effect, EffectHandler, Response};
    use ttrpg_interp::reference_state::GameState;
    use ttrpg_interp::value::Value;

    struct NullHandler;
    impl EffectHandler for NullHandler {
        fn handle(&mut self, _: Effect) -> Response {
            Response::Acknowledged
        }
    }

    let runtime = crate::backend::dsl()?;
    let state = GameState::new();
    let mut handler = NullHandler;

    let cat_value = Value::EnumVariant {
        enum_name: "SaveCategory".into(),
        variant: dsl_variant_name(cat).into(),
        fields: BTreeMap::new(),
    };
    let level_value = Value::Int(level as i64);
    let args = || vec![cat_value.clone(), level_value.clone()];

    let death = as_u32(runtime.evaluate_derive(&state, &mut handler, "get_save_death", args()).ok()?)?;
    let wands = as_u32(runtime.evaluate_derive(&state, &mut handler, "get_save_wands", args()).ok()?)?;
    let paralysis = as_u32(runtime.evaluate_derive(&state, &mut handler, "get_save_paralysis", args()).ok()?)?;
    let breath = as_u32(runtime.evaluate_derive(&state, &mut handler, "get_save_breath", args()).ok()?)?;
    let spells = as_u32(runtime.evaluate_derive(&state, &mut handler, "get_save_spells", args()).ok()?)?;

    Some(SavingThrows::new(death, wands, paralysis, breath, spells))
}

#[cfg(feature = "dsl-backend")]
fn dsl_variant_name(cat: SaveCategory) -> &'static str {
    match cat {
        SaveCategory::Thief => "save_thief",
        SaveCategory::Barbarian => "save_barbarian",
        SaveCategory::Cleric => "save_cleric",
        SaveCategory::Drow => "save_drow",
        SaveCategory::Dwarf => "save_dwarf",
        SaveCategory::Elf => "save_elf",
        SaveCategory::Fighter => "save_fighter",
        SaveCategory::Gnome => "save_gnome",
        SaveCategory::HalfElf => "save_half_elf",
        SaveCategory::HalfOrc => "save_half_orc",
        SaveCategory::MagicUser => "save_magic_user",
        SaveCategory::Paladin => "save_paladin",
        SaveCategory::Svirfneblin => "save_svirfneblin",
    }
}

#[cfg(feature = "dsl-backend")]
fn as_u32(v: ttrpg_interp::value::Value) -> Option<u32> {
    if let ttrpg_interp::value::Value::Int(n) = v {
        Some(n as u32)
    } else {
        None
    }
}

// ── Native backend ──────────────────────────────────────────────

fn native_saving_throws(cat: SaveCategory, level: u32) -> SavingThrows {
    use SaveCategory::*;
    match cat {
        Thief => match level {
            0..=4 => SavingThrows::new(13, 14, 13, 16, 15),
            5..=8 => SavingThrows::new(12, 13, 11, 14, 13),
            9..=12 => SavingThrows::new(10, 11, 9, 12, 10),
            _ => SavingThrows::new(8, 9, 7, 10, 8),
        },
        Barbarian => match level {
            0..=3 => SavingThrows::new(10, 13, 12, 15, 16),
            4..=6 => SavingThrows::new(8, 11, 10, 13, 13),
            7..=9 => SavingThrows::new(6, 9, 8, 10, 10),
            10..=12 => SavingThrows::new(4, 7, 6, 8, 7),
            _ => SavingThrows::new(3, 5, 4, 5, 5),
        },
        Cleric => match level {
            0..=4 => SavingThrows::new(11, 12, 14, 16, 15),
            5..=8 => SavingThrows::new(9, 10, 12, 14, 12),
            9..=12 => SavingThrows::new(6, 7, 9, 11, 9),
            _ => SavingThrows::new(3, 5, 7, 8, 7),
        },
        Drow => match level {
            0..=3 => SavingThrows::new(12, 13, 13, 15, 12),
            4..=6 => SavingThrows::new(10, 11, 11, 13, 10),
            7..=9 => SavingThrows::new(8, 9, 9, 10, 8),
            _ => SavingThrows::new(6, 7, 8, 8, 6),
        },
        Dwarf => match level {
            0..=3 => SavingThrows::new(8, 9, 10, 13, 12),
            4..=6 => SavingThrows::new(6, 7, 8, 10, 10),
            7..=9 => SavingThrows::new(4, 5, 6, 7, 8),
            _ => SavingThrows::new(2, 3, 4, 4, 6),
        },
        Elf => match level {
            0..=3 => SavingThrows::new(12, 13, 13, 15, 15),
            4..=6 => SavingThrows::new(10, 11, 11, 13, 12),
            7..=9 => SavingThrows::new(8, 9, 9, 10, 10),
            _ => SavingThrows::new(6, 7, 8, 8, 8),
        },
        Fighter => match level {
            0..=3 => SavingThrows::new(12, 13, 14, 15, 16),
            4..=6 => SavingThrows::new(10, 11, 12, 13, 14),
            7..=9 => SavingThrows::new(8, 9, 10, 10, 12),
            10..=12 => SavingThrows::new(6, 7, 8, 8, 10),
            _ => SavingThrows::new(4, 5, 6, 5, 8),
        },
        Gnome => match level {
            0..=5 => SavingThrows::new(8, 9, 10, 14, 11),
            _ => SavingThrows::new(6, 7, 8, 11, 9),
        },
        HalfElf => match level {
            0..=3 => SavingThrows::new(12, 13, 13, 15, 15),
            4..=6 => SavingThrows::new(10, 11, 11, 13, 12),
            7..=9 => SavingThrows::new(8, 9, 9, 10, 10),
            _ => SavingThrows::new(6, 7, 8, 8, 8),
        },
        HalfOrc => match level {
            0..=4 => SavingThrows::new(13, 14, 13, 16, 15),
            _ => SavingThrows::new(12, 13, 11, 14, 13),
        },
        MagicUser => match level {
            0..=5 => SavingThrows::new(13, 14, 13, 16, 15),
            6..=10 => SavingThrows::new(11, 12, 11, 14, 12),
            _ => SavingThrows::new(8, 9, 8, 11, 8),
        },
        Paladin => match level {
            0..=3 => SavingThrows::new(10, 11, 12, 13, 14),
            4..=6 => SavingThrows::new(8, 9, 10, 11, 12),
            7..=9 => SavingThrows::new(6, 7, 8, 8, 10),
            10..=12 => SavingThrows::new(4, 5, 6, 6, 8),
            _ => SavingThrows::new(2, 3, 4, 3, 6),
        },
        Svirfneblin => match level {
            0..=3 => SavingThrows::new(8, 9, 10, 14, 11),
            4..=6 => SavingThrows::new(6, 7, 8, 11, 9),
            _ => SavingThrows::new(4, 5, 6, 9, 7),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_0_gets_worst_saves() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Fighter), 0);
        assert_eq!(s, SavingThrows::new(12, 13, 14, 15, 16));
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Thief), 0);
        assert_eq!(s, SavingThrows::new(13, 14, 13, 16, 15));
    }

    #[test]
    fn thief_saves_level_1() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Thief), 1);
        assert_eq!(s, SavingThrows::new(13, 14, 13, 16, 15));
    }

    #[test]
    fn thief_saves_level_5() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Thief), 5);
        assert_eq!(s, SavingThrows::new(12, 13, 11, 14, 13));
    }

    #[test]
    fn fighter_saves_level_1() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Fighter), 1);
        assert_eq!(s, SavingThrows::new(12, 13, 14, 15, 16));
    }

    #[test]
    fn fighter_saves_level_13() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Fighter), 13);
        assert_eq!(s, SavingThrows::new(4, 5, 6, 5, 8));
    }

    #[test]
    fn cleric_saves_level_4() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Cleric), 4);
        assert_eq!(s, SavingThrows::new(11, 12, 14, 16, 15));
    }

    #[test]
    fn cleric_saves_level_5() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Cleric), 5);
        assert_eq!(s, SavingThrows::new(9, 10, 12, 14, 12));
    }

    #[test]
    fn dwarf_saves_level_1() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Dwarf), 1);
        assert_eq!(s, SavingThrows::new(8, 9, 10, 13, 12));
    }

    #[test]
    fn magic_user_saves_level_1() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::MagicUser), 1);
        assert_eq!(s, SavingThrows::new(13, 14, 13, 16, 15));
    }

    #[test]
    fn magic_user_saves_level_6() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::MagicUser), 6);
        assert_eq!(s, SavingThrows::new(11, 12, 11, 14, 12));
    }

    #[test]
    fn paladin_saves_level_13() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Paladin), 13);
        assert_eq!(s, SavingThrows::new(2, 3, 4, 3, 6));
    }

    #[test]
    fn barbarian_saves_level_7() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Barbarian), 7);
        assert_eq!(s, SavingThrows::new(6, 9, 8, 10, 10));
    }

    #[test]
    fn drow_saves_level_10() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Drow), 10);
        assert_eq!(s, SavingThrows::new(6, 7, 8, 8, 6));
    }

    #[test]
    fn svirfneblin_saves_level_7() {
        let s = saving_throws(&SaveCategoryId::from_enum(SaveCategory::Svirfneblin), 7);
        assert_eq!(s, SavingThrows::new(4, 5, 6, 9, 7));
    }

    #[test]
    fn dynamic_map_access() {
        let s = SavingThrows::new(12, 13, 14, 15, 16);
        assert_eq!(s.get("death"), Some(12));
        assert_eq!(s.get("wands"), Some(13));
        assert_eq!(s.get("paralysis"), Some(14));
        assert_eq!(s.get("breath"), Some(15));
        assert_eq!(s.get("spells"), Some(16));
        assert_eq!(s.get("nonexistent"), None);
    }

    #[test]
    fn convenience_accessors_match_map() {
        let s = SavingThrows::new(12, 13, 14, 15, 16);
        assert_eq!(s.death(), 12);
        assert_eq!(s.wands(), 13);
        assert_eq!(s.paralysis(), 14);
        assert_eq!(s.breath(), 15);
        assert_eq!(s.spells(), 16);
    }

    #[test]
    fn missing_save_defaults_to_20() {
        let s = SavingThrows::new_empty();
        assert_eq!(s.death(), 20);
        assert_eq!(s.wands(), 20);
    }

    // ── SaveCategoryId tests ──

    #[test]
    fn save_category_id_from_enum() {
        let id = SaveCategoryId::from_enum(SaveCategory::Fighter);
        assert_eq!(id.as_str(), "Fighter");
        assert_eq!(id.to_enum(), Some(SaveCategory::Fighter));
    }

    #[test]
    fn save_category_id_new_normalizes() {
        let id = SaveCategoryId::new("fighter");
        assert_eq!(id.as_str(), "Fighter");
        assert_eq!(id.to_enum(), Some(SaveCategory::Fighter));
    }

    #[test]
    fn save_category_id_hyphenated() {
        let id = SaveCategoryId::new("Half-Elf");
        assert_eq!(id.as_str(), "Half-Elf");
        assert_eq!(id.to_enum(), Some(SaveCategory::HalfElf));
    }

    #[test]
    fn save_category_id_dsl_name() {
        let id = SaveCategoryId::new("save_magic_user");
        assert_eq!(id.as_str(), "Magic-User");
        assert_eq!(id.to_enum(), Some(SaveCategory::MagicUser));
    }

    #[test]
    fn save_category_id_unknown_stored_as_is() {
        let id = SaveCategoryId::new("Homebrew");
        assert_eq!(id.as_str(), "Homebrew");
        assert_eq!(id.to_enum(), None);
    }

    #[test]
    fn save_category_id_serde_roundtrip() {
        let id = SaveCategoryId::from_enum(SaveCategory::MagicUser);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"Magic-User\"");
        let parsed: SaveCategoryId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn save_category_id_from_trait() {
        let id: SaveCategoryId = SaveCategory::Thief.into();
        assert_eq!(id.as_str(), "Thief");
        let id2: SaveCategoryId = "dwarf".into();
        assert_eq!(id2.as_str(), "Dwarf");
    }
}
