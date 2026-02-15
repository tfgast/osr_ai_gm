//! Monster stat blocks loaded from JSON data files.
//! Supports layered loading: core → modules → user customizations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use super::attack::HitDice;

/// XP value - can be single value or array (for variable HD monsters).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum XpValue {
    Single(u64),
    Multiple(Vec<u64>),
}

impl XpValue {
    /// Get the first/default XP value.
    pub fn value(&self) -> u64 {
        match self {
            XpValue::Single(v) => *v,
            XpValue::Multiple(v) => v.first().copied().unwrap_or(0),
        }
    }

    /// Get XP for a specific HD variant (0-indexed).
    pub fn for_variant(&self, idx: usize) -> u64 {
        match self {
            XpValue::Single(v) => *v,
            XpValue::Multiple(v) => v.get(idx).copied().unwrap_or_else(|| v.last().copied().unwrap_or(0)),
        }
    }
}

impl Default for XpValue {
    fn default() -> Self {
        XpValue::Single(0)
    }
}

/// Monster definition loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterDef {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub armor_class: i32,
    #[serde(default)]
    pub armor_class_ascending: Option<i32>,
    /// Structured hit dice (parsed from notation like "3+1*").
    pub hit_dice: HitDice,
    #[serde(default)]
    pub hp_typical: Option<String>,
    #[serde(default)]
    pub attacks: Vec<Attack>,
    #[serde(default)]
    pub thac0: Option<i32>,
    #[serde(default)]
    pub thac0_bonus: Option<i32>,
    #[serde(default)]
    pub movement: Movement,
    #[serde(default)]
    pub saves: Option<String>,
    pub morale: u32,
    #[serde(default)]
    pub alignment: Option<String>,
    #[serde(default)]
    pub xp_value: XpValue,
    #[serde(default)]
    pub num_appearing: Option<String>,
    #[serde(default)]
    pub treasure_type: Option<String>,
    #[serde(default)]
    pub special_abilities: Vec<String>,

    // Legacy fields for backward compatibility (from old hardcoded format)
    #[serde(default)]
    legacy_damage: Option<String>,
    #[serde(default)]
    legacy_special: Option<String>,
    #[serde(default)]
    legacy_attacks: Option<Vec<String>>,
}

/// Attack definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attack {
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub damage: Option<String>,
    #[serde(default)]
    pub raw: Option<String>,
}

/// Movement rates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Movement {
    #[serde(default)]
    pub base: u32,
    #[serde(default, rename = "fly")]
    pub flying: Option<u32>,
    #[serde(default)]
    pub burrow: Option<u32>,
    #[serde(default)]
    pub swim: Option<u32>,
}

impl MonsterDef {
    /// Get AC (descending, for compatibility).
    pub fn ac(&self) -> i32 {
        self.armor_class
    }

    /// Get base movement rate in feet per turn.
    pub fn movement_rate(&self) -> u32 {
        self.movement.base
    }

    /// Get damage string (combines attack damages for compatibility).
    pub fn damage(&self) -> String {
        if let Some(ref d) = self.legacy_damage {
            return d.clone();
        }
        // Combine damage from all attacks
        self.attacks
            .iter()
            .filter_map(|a| a.damage.as_deref())
            .collect::<Vec<_>>()
            .join(" / ")
    }

    /// Get special abilities as single string (for compatibility).
    pub fn special(&self) -> String {
        if let Some(ref s) = self.legacy_special {
            return s.clone();
        }
        self.special_abilities.join(". ")
    }

    /// Get attack names as strings (for compatibility).
    pub fn attack_names(&self) -> Vec<String> {
        if let Some(ref a) = self.legacy_attacks {
            return a.clone();
        }
        self.attacks
            .iter()
            .map(|a| {
                if a.name.is_empty() {
                    a.raw.clone().unwrap_or_default()
                } else {
                    a.name.clone()
                }
            })
            .collect()
    }

    /// Expand attacks into individual attack routines.
    /// Attack{count:2, name:"claw", damage:"1d3"} becomes [claw/1d3, claw/1d3].
    pub fn attack_routines(&self) -> Vec<crate::model::MonsterAttackRoutine> {
        let mut routines = Vec::new();
        for atk in &self.attacks {
            let name = if atk.name.is_empty() {
                atk.raw.clone().unwrap_or_else(|| "attack".to_string())
            } else {
                atk.name.clone()
            };
            let damage = atk.damage.clone().unwrap_or_else(|| "1d6".to_string());
            let count = atk.count.max(1);
            for _ in 0..count {
                routines.push(crate::model::MonsterAttackRoutine {
                    name: name.clone(),
                    damage: damage.clone(),
                });
            }
        }
        routines
    }

    /// Get XP value (default/first value for variable HD monsters).
    pub fn xp(&self) -> u64 {
        self.xp_value.value()
    }

    /// Check if this monster is undead based on special_abilities.
    /// Undead monsters have a special ability starting with "Undead:".
    pub fn is_undead(&self) -> bool {
        self.special_abilities.iter().any(|s| s.starts_with("Undead:"))
    }

    /// Create a minimal MonsterDef for testing.
    #[cfg(test)]
    pub fn test_def(name: &str, ac: i32, morale: u32, xp: u64) -> MonsterDef {
        MonsterDef {
            name: name.to_string(),
            description: None,
            armor_class: ac,
            armor_class_ascending: None,
            hit_dice: "1".parse().unwrap(),
            hp_typical: None,
            attacks: vec![Attack { count: 1, name: "attack".to_string(), damage: Some("1d6".to_string()), raw: None }],
            thac0: None,
            thac0_bonus: None,
            movement: Movement::default(),
            saves: None,
            morale,
            alignment: None,
            xp_value: XpValue::Single(xp),
            num_appearing: None,
            treasure_type: None,
            special_abilities: Vec::new(),
            legacy_damage: None,
            legacy_special: None,
            legacy_attacks: None,
        }
    }

    /// Check if this monster is immune to non-magical weapons.
    /// Detected from special abilities mentioning magical attack requirements.
    pub fn immune_to_normal_weapons(&self) -> bool {
        self.special_abilities.iter().any(|s| {
            let lower = s.to_lowercase();
            lower.contains("only be harmed by magical")
                || lower.contains("only be hit by magical")
                || lower.contains("mundane damage immunity")
                || lower.contains("immune to non-magical")
                || lower.contains("immune to normal weapons")
        })
    }
}

/// Container for loaded monster data.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct MonsterFile {
    #[serde(default)]
    pub(crate) source: Option<String>,
    #[serde(default)]
    pub(crate) count: usize,
    pub(crate) monsters: Vec<MonsterDef>,
}

/// Monster registry - holds all loaded monsters.
struct MonsterRegistry {
    monsters: Vec<MonsterDef>,
    by_name: HashMap<String, usize>, // name (lowercase) -> index
}

impl MonsterRegistry {
    fn new() -> Self {
        Self {
            monsters: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    fn load_file(&mut self, path: &Path) -> Result<usize, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let file: MonsterFile = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        let mut added = 0;
        for monster in file.monsters {
            let key = monster.name.to_lowercase();
            if self.by_name.contains_key(&key) {
                // Override existing entry (for module/user overrides)
                let idx = self.by_name[&key];
                self.monsters[idx] = monster;
            } else {
                let idx = self.monsters.len();
                self.by_name.insert(key, idx);
                self.monsters.push(monster);
                added += 1;
            }
        }
        Ok(added)
    }

    fn find(&self, name: &str) -> Option<&MonsterDef> {
        let key = name.to_lowercase();
        self.by_name.get(&key).map(|&idx| &self.monsters[idx])
    }

    fn all(&self) -> &[MonsterDef] {
        &self.monsters
    }
}

/// Global monster registry.
static REGISTRY: OnceLock<MonsterRegistry> = OnceLock::new();

/// Initialize the monster registry by loading data files.
fn init_registry() -> MonsterRegistry {
    let mut registry = MonsterRegistry::new();

    // Find data directory relative to executable or working directory
    let data_paths = [
        // Development: relative to working directory
        "data/core/monsters.json",
        // Installed: relative to executable
        "../data/core/monsters.json",
        // Alternative: in current directory
        "monsters.json",
    ];

    let mut loaded = false;
    for path_str in &data_paths {
        let path = Path::new(path_str);
        if path.exists() {
            match registry.load_file(path) {
                Ok(count) => {
                    eprintln!("Loaded {} monsters from {}", count, path.display());
                    loaded = true;
                    break;
                }
                Err(e) => {
                    eprintln!("Warning: {}", e);
                }
            }
        }
    }

    if !loaded {
        eprintln!("Warning: No monster data files found. Using empty registry.");
        eprintln!("Expected: data/core/monsters.json");
    }

    // TODO: Load module data from data/modules/*/monsters.json
    // TODO: Load user data from ~/.osr_data/custom/monsters.json

    registry
}

/// Get the global monster registry.
fn registry() -> &'static MonsterRegistry {
    REGISTRY.get_or_init(init_registry)
}

/// Look up a monster definition by name (case-insensitive).
pub fn find_monster(name: &str) -> Option<&'static MonsterDef> {
    registry().find(name)
}

/// All monster definitions.
pub fn all_monsters() -> &'static [MonsterDef] {
    registry().all()
}

/// Reload monster data (useful for testing or hot-reloading).
/// Note: This is a no-op after initial load due to OnceLock.
/// For true hot-reloading, would need RwLock instead.
pub fn reload_monsters() {
    // OnceLock can only be set once, so this is informational only
    let _ = registry();
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests require data/core/monsters.json to exist
    // If file is missing, tests that depend on specific monsters will be skipped

    fn has_data() -> bool {
        Path::new("data/core/monsters.json").exists()
    }

    #[test]
    fn registry_initializes() {
        let _ = registry();
        // Should not panic
    }

    #[test]
    fn find_case_insensitive() {
        if !has_data() {
            eprintln!("Skipping: no data file");
            return;
        }
        // These should work if the data file has these monsters
        if let Some(m) = find_monster("basilisk") {
            assert!(find_monster("BASILISK").is_some());
            assert!(find_monster("Basilisk").is_some());
            assert_eq!(m.name, "Basilisk");
        }
    }

    #[test]
    fn find_nonexistent() {
        assert!(find_monster("NonexistentMonster12345").is_none());
    }

    #[test]
    fn all_monsters_accessible() {
        let monsters = all_monsters();
        // Either we have data or we don't - shouldn't panic either way
        for m in monsters {
            assert!(!m.name.is_empty());
            assert!(m.hit_dice.base > 0 || m.hit_dice.fractional);
        }
    }

    #[test]
    fn monster_has_required_fields() {
        if !has_data() {
            return;
        }
        for m in all_monsters() {
            assert!(!m.name.is_empty(), "monster has empty name");
            assert!(m.hit_dice.base > 0 || m.hit_dice.fractional, "{} has invalid hit_dice", m.name);
            // xp_value can be 0 for special monsters, but morale should be set
            assert!(m.morale >= 1 && m.morale <= 12, "{} has invalid morale: {}", m.name, m.morale);
        }
    }

    #[test]
    fn damage_compatibility() {
        if !has_data() {
            return;
        }
        if let Some(m) = find_monster("Basilisk") {
            let damage = m.damage();
            // Should have some damage string
            assert!(!damage.is_empty() || m.attacks.is_empty());
        }
    }

    #[test]
    fn special_compatibility() {
        if !has_data() {
            return;
        }
        if let Some(m) = find_monster("Basilisk") {
            let special = m.special();
            // Basilisk should have petrification in special abilities
            assert!(
                special.to_lowercase().contains("petrif") || m.special_abilities.is_empty(),
                "Basilisk special: {}",
                special
            );
        }
    }

    #[test]
    fn is_undead_detection() {
        if !has_data() {
            return;
        }
        // Known undead monsters should be detected
        for name in &["Skeleton", "Zombie", "Ghoul", "Wraith", "Vampire", "Mummy", "Spectre", "Wight"] {
            if let Some(m) = find_monster(name) {
                assert!(m.is_undead(), "{} should be detected as undead", name);
            }
        }
        // Known living monsters should not be detected as undead
        for name in &["Goblin", "Orc", "Basilisk", "Ogre", "Dragon, Red"] {
            if let Some(m) = find_monster(name) {
                assert!(!m.is_undead(), "{} should NOT be detected as undead", name);
            }
        }
    }

    #[test]
    fn immune_to_normal_weapons_detection() {
        if !has_data() {
            return;
        }
        // Known immune monsters
        for name in &["Gargoyle", "Wraith", "Spectre", "Shadow", "Will-o'-the-Wisp"] {
            if let Some(m) = find_monster(name) {
                assert!(m.immune_to_normal_weapons(), "{} should be immune to normal weapons", name);
            }
        }
        // Known non-immune monsters
        for name in &["Goblin", "Orc", "Skeleton", "Zombie", "Ogre"] {
            if let Some(m) = find_monster(name) {
                assert!(!m.immune_to_normal_weapons(), "{} should NOT be immune to normal weapons", name);
            }
        }
    }
}
