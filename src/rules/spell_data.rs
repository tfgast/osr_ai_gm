//! Spell definitions loaded from JSON data files.
//! Covers Cleric, Magic-User, Druid, and Illusionist spell lists.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// Which spell list a spell belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellList {
    Cleric,
    MagicUser,
    Druid,
    Illusionist,
}

impl SpellList {
    pub fn name(self) -> &'static str {
        match self {
            SpellList::Cleric => "Cleric",
            SpellList::MagicUser => "Magic-User",
            SpellList::Druid => "Druid",
            SpellList::Illusionist => "Illusionist",
        }
    }
}

/// A spell definition loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellDef {
    pub name: String,
    pub list: SpellList,
    pub level: u32,
    pub range: String,
    pub duration: String,
    pub description: String,
    #[serde(default)]
    pub reversible: bool,
    #[serde(default)]
    pub reversed_name: Option<String>,
    #[serde(default)]
    pub reversed_description: Option<String>,
}

/// JSON file format for spell data.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SpellFile {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    count: Option<usize>,
    spells: Vec<SpellDef>,
}

/// Registry holding all loaded spells.
struct SpellRegistry {
    spells: Vec<SpellDef>,
    by_name: HashMap<String, Vec<usize>>, // Maps lowercase name to indices (may have duplicates across lists)
    by_list_level: HashMap<(SpellList, u32), Vec<usize>>,
}

impl SpellRegistry {
    fn new() -> Self {
        Self {
            spells: Vec::new(),
            by_name: HashMap::new(),
            by_list_level: HashMap::new(),
        }
    }

    fn load_file(&mut self, path: &Path) -> Result<usize, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let file: SpellFile = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        let mut added = 0;
        for spell in file.spells {
            let key = spell.name.to_lowercase();
            let list_level = (spell.list, spell.level);
            let idx = self.spells.len();

            // Index by name (can have multiple spells with same name across lists)
            self.by_name.entry(key).or_default().push(idx);

            // Index by list and level
            self.by_list_level.entry(list_level).or_default().push(idx);

            self.spells.push(spell);
            added += 1;
        }
        Ok(added)
    }

    fn find(&self, name: &str, list: Option<SpellList>) -> Option<&SpellDef> {
        let key = name.to_lowercase();
        let indices = self.by_name.get(&key)?;

        for &idx in indices {
            let spell = &self.spells[idx];
            if list.is_none_or(|l| spell.list == l) {
                return Some(spell);
            }
        }
        None
    }

    fn by_list_and_level(&self, list: SpellList, level: u32) -> Vec<&SpellDef> {
        self.by_list_level
            .get(&(list, level))
            .map(|indices| indices.iter().map(|&i| &self.spells[i]).collect())
            .unwrap_or_default()
    }

    fn all(&self) -> &[SpellDef] {
        &self.spells
    }
}

/// Global spell registry.
static REGISTRY: OnceLock<SpellRegistry> = OnceLock::new();

/// Initialize the spell registry by loading data files.
fn init_registry() -> SpellRegistry {
    let mut registry = SpellRegistry::new();

    if let Some(path) = crate::manifest::game_data_file("spells") {
        if path.exists() {
            match registry.load_file(&path) {
                Ok(count) => {
                    eprintln!("Loaded {} spells from {}", count, path.display());
                    return registry;
                }
                Err(e) => {
                    eprintln!("Warning: {}", e);
                }
            }
        }
    }

    eprintln!("Warning: No spell data file found in game system manifest.");

    registry
}

/// Get the global spell registry.
fn registry() -> &'static SpellRegistry {
    REGISTRY.get_or_init(init_registry)
}

/// Get all spells for a given list and level.
pub fn spells_by_list_and_level(list: SpellList, level: u32) -> Vec<&'static SpellDef> {
    registry().by_list_and_level(list, level)
}

/// Find a spell by name (case-insensitive) and optional list.
pub fn find_spell(name: &str, list: Option<SpellList>) -> Option<&'static SpellDef> {
    registry().find(name, list)
}

/// Get all spells.
pub fn all_spells() -> &'static [SpellDef] {
    registry().all()
}

// ── Spell Metadata (DSL-backed) ───────────────────────────────

/// Look up the saving throw type for a spell via DSL.
/// Returns one of: "death", "wands", "paralysis", "breath", "spells", "none".
pub fn spell_save_type(spell_name: &str) -> String {
    #[cfg(feature = "dsl-backend")]
    {
        if let Some(val) = dsl_spell_save_type(spell_name) {
            return val;
        }
    }
    let _ = spell_name;
    "none".to_string()
}

/// Look up the damage dice formula for a spell via DSL.
/// Returns a descriptive string like "1d6 per caster level", or "" for non-damage spells.
pub fn spell_damage_dice(spell_name: &str) -> String {
    #[cfg(feature = "dsl-backend")]
    {
        if let Some(val) = dsl_spell_damage_dice(spell_name) {
            return val;
        }
    }
    let _ = spell_name;
    String::new()
}

#[cfg(feature = "dsl-backend")]
fn dsl_spell_save_type(spell_name: &str) -> Option<String> {
    use ttrpg_interp::value::Value;

    let rt = crate::backend::dsl()?;
    let args = vec![Value::Str(spell_name.to_string())];
    let result = rt
        .evaluate_derive(
            &crate::backend::NullState,
            &mut crate::backend::SimpleDiceHandler::new(),
            "spell_save_type",
            args,
        )
        .ok()?;
    match &result {
        Value::EnumVariant { variant, .. } => Some(match variant.as_str() {
            "sv_death" => "death".to_string(),
            "sv_wands" => "wands".to_string(),
            "sv_paralysis" => "paralysis".to_string(),
            "sv_breath" => "breath".to_string(),
            "sv_spells" => "spells".to_string(),
            _ => "none".to_string(),
        }),
        _ => None,
    }
}

#[cfg(feature = "dsl-backend")]
fn dsl_spell_damage_dice(spell_name: &str) -> Option<String> {
    use ttrpg_interp::value::Value;

    let rt = crate::backend::dsl()?;
    let args = vec![Value::Str(spell_name.to_string())];
    let result = rt
        .evaluate_derive(
            &crate::backend::NullState,
            &mut crate::backend::SimpleDiceHandler::new(),
            "spell_damage_dice",
            args,
        )
        .ok()?;
    match &result {
        Value::Str(s) => Some(s.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads() {
        let spells = all_spells();
        assert!(!spells.is_empty(), "Spell registry should not be empty");
    }

    #[test]
    fn find_magic_missile() {
        let spell = find_spell("Magic Missile", None);
        assert!(spell.is_some(), "Magic Missile should exist");
        let spell = spell.unwrap();
        assert_eq!(spell.level, 1);
        assert_eq!(spell.list, SpellList::MagicUser);
    }

    #[test]
    fn find_spell_case_insensitive() {
        assert!(find_spell("magic missile", None).is_some());
        assert!(find_spell("CURE LIGHT WOUNDS", None).is_some());
    }

    #[test]
    fn find_spell_with_list_filter() {
        // Both cleric and MU have "Detect Magic" - filter by list
        let cleric = find_spell("Detect Magic", Some(SpellList::Cleric));
        assert!(cleric.is_some());
        assert_eq!(cleric.unwrap().list, SpellList::Cleric);

        let mu = find_spell("Detect Magic", Some(SpellList::MagicUser));
        assert!(mu.is_some());
        assert_eq!(mu.unwrap().list, SpellList::MagicUser);
    }

    #[test]
    fn cleric_level_1_spells() {
        let spells = spells_by_list_and_level(SpellList::Cleric, 1);
        assert!(!spells.is_empty(), "Should have cleric level 1 spells");
        assert!(spells.iter().any(|s| s.name == "Cure Light Wounds"));
    }

    #[test]
    fn magic_user_level_3_has_fireball() {
        let spells = spells_by_list_and_level(SpellList::MagicUser, 3);
        // Note: In JSON it's "Fire Ball" (two words)
        assert!(spells.iter().any(|s| s.name.contains("Fire") && s.name.contains("Ball")));
    }

    #[test]
    fn reversible_spells_exist() {
        let all = all_spells();
        let reversible: Vec<_> = all.iter().filter(|s| s.reversible).collect();
        assert!(!reversible.is_empty(), "Should have some reversible spells");

        // Cure Light Wounds is reversible
        let clw = find_spell("Cure Light Wounds", None).unwrap();
        assert!(clw.reversible);
        assert_eq!(clw.reversed_name.as_deref(), Some("Cause Light Wounds"));
    }

    #[test]
    fn spell_save_type_returns_value() {
        // Fireball requires save vs spells
        let save = spell_save_type("Fire Ball");
        // Without DSL backend, returns "none" (fallback).
        // With DSL backend, returns "spells".
        assert!(save == "spells" || save == "none");
    }

    #[test]
    fn spell_save_type_no_save_default() {
        // Magic Missile has no save
        let save = spell_save_type("Magic Missile");
        assert!(save == "none");
    }

    #[test]
    fn spell_damage_dice_returns_value() {
        let dice = spell_damage_dice("Fire Ball");
        // Without DSL backend, returns "" (fallback).
        // With DSL backend, returns "1d6 per caster level".
        assert!(dice == "1d6 per caster level" || dice.is_empty());
    }

    #[test]
    fn spell_damage_dice_no_damage_default() {
        // Detect Magic does no damage
        let dice = spell_damage_dice("Detect Magic");
        assert!(dice.is_empty());
    }
}

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_tests {
    use super::*;

    // Only runs when DSL backend is available. Tests that the
    // ose_spells.ttrpg file parses and evaluates correctly.

    fn dsl_available() -> bool {
        crate::backend::dsl().is_some()
    }

    #[test]
    fn dsl_spell_save_type_fireball() {
        if !dsl_available() {
            eprintln!("DSL not available, skipping");
            return;
        }
        assert_eq!(spell_save_type("Fire Ball"), "spells");
    }

    #[test]
    fn dsl_spell_save_type_cloudkill() {
        if !dsl_available() {
            eprintln!("DSL not available, skipping");
            return;
        }
        assert_eq!(spell_save_type("Cloudkill"), "death");
    }

    #[test]
    fn dsl_spell_save_type_disintegrate() {
        if !dsl_available() {
            eprintln!("DSL not available, skipping");
            return;
        }
        assert_eq!(spell_save_type("Disintegrate"), "death");
    }

    #[test]
    fn dsl_spell_save_type_no_save() {
        if !dsl_available() {
            eprintln!("DSL not available, skipping");
            return;
        }
        assert_eq!(spell_save_type("Magic Missile"), "none");
        assert_eq!(spell_save_type("Sleep"), "none");
        assert_eq!(spell_save_type("Detect Magic"), "none");
    }

    #[test]
    fn dsl_spell_damage_dice_fireball() {
        if !dsl_available() {
            eprintln!("DSL not available, skipping");
            return;
        }
        assert_eq!(spell_damage_dice("Fire Ball"), "1d6 per caster level");
    }

    #[test]
    fn dsl_spell_damage_dice_magic_missile() {
        if !dsl_available() {
            eprintln!("DSL not available, skipping");
            return;
        }
        assert_eq!(spell_damage_dice("Magic Missile"), "1d6+1 per missile");
    }

    #[test]
    fn dsl_spell_damage_dice_no_damage() {
        if !dsl_available() {
            eprintln!("DSL not available, skipping");
            return;
        }
        assert_eq!(spell_damage_dice("Charm Person"), "");
    }

    #[test]
    fn dsl_spell_save_type_charm_person() {
        if !dsl_available() {
            eprintln!("DSL not available, skipping");
            return;
        }
        assert_eq!(spell_save_type("Charm Person"), "spells");
    }
}
