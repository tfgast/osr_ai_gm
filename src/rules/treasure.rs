//! Treasure type definitions loaded from JSON data files.
//! Covers hoard treasures (A-O), individual treasures (P-T), and group treasures (U-V).
//! Also includes gem and jewellery value tables.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// Treasure category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreasureCategory {
    Hoard,
    Individual,
    Group,
}

impl TreasureCategory {
    pub fn name(self) -> &'static str {
        match self {
            TreasureCategory::Hoard => "Hoard",
            TreasureCategory::Individual => "Individual",
            TreasureCategory::Group => "Group",
        }
    }
}

/// Type of treasure item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreasureItemType {
    Cp,           // Copper pieces
    Sp,           // Silver pieces
    Ep,           // Electrum pieces
    Gp,           // Gold pieces
    Pp,           // Platinum pieces
    Gems,         // Gems
    Jewellery,    // Jewellery
    MagicItems,   // Magic items (any)
    MagicWeapon,  // Magic sword, armour, or weapon
    Potions,      // Potions
    Scrolls,      // Scrolls
}

impl TreasureItemType {
    pub fn name(self) -> &'static str {
        match self {
            TreasureItemType::Cp => "Copper Pieces",
            TreasureItemType::Sp => "Silver Pieces",
            TreasureItemType::Ep => "Electrum Pieces",
            TreasureItemType::Gp => "Gold Pieces",
            TreasureItemType::Pp => "Platinum Pieces",
            TreasureItemType::Gems => "Gems",
            TreasureItemType::Jewellery => "Jewellery",
            TreasureItemType::MagicItems => "Magic Items",
            TreasureItemType::MagicWeapon => "Magic Weapon",
            TreasureItemType::Potions => "Potions",
            TreasureItemType::Scrolls => "Scrolls",
        }
    }

    /// Check if this is a coin type.
    pub fn is_coin(self) -> bool {
        matches!(self, TreasureItemType::Cp | TreasureItemType::Sp |
                      TreasureItemType::Ep | TreasureItemType::Gp |
                      TreasureItemType::Pp)
    }

    /// Check if this is a magic item type.
    pub fn is_magic(self) -> bool {
        matches!(self, TreasureItemType::MagicItems | TreasureItemType::MagicWeapon |
                      TreasureItemType::Potions | TreasureItemType::Scrolls)
    }
}

/// A single entry in a treasure type's contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasureEntry {
    /// Percentage chance of this entry appearing (1-100).
    pub chance: u32,
    /// Dice notation for quantity (e.g., "1d6", "2d4 × 1000").
    pub quantity: String,
    /// Type of treasure.
    #[serde(rename = "type")]
    pub item_type: TreasureItemType,
    /// Restrictions on magic items (e.g., "not weapons").
    #[serde(default)]
    pub restriction: Option<String>,
    /// Additional notes about this entry.
    #[serde(default)]
    pub note: Option<String>,
}

/// A treasure type definition loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasureTypeDef {
    /// Treasure type letter (A-V).
    pub letter: String,
    /// Average gold piece value.
    pub average_gp: f64,
    /// Category (hoard, individual, group).
    pub category: TreasureCategory,
    /// List of possible treasure entries.
    pub entries: Vec<TreasureEntry>,
}

/// JSON file format for treasure data.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TreasureFile {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    count: Option<usize>,
    treasure_types: Vec<TreasureTypeDef>,
}

/// Registry holding all loaded treasure types.
struct TreasureRegistry {
    types: Vec<TreasureTypeDef>,
    by_letter: HashMap<String, usize>,
    by_category: HashMap<TreasureCategory, Vec<usize>>,
}

impl TreasureRegistry {
    fn new() -> Self {
        Self {
            types: Vec::new(),
            by_letter: HashMap::new(),
            by_category: HashMap::new(),
        }
    }

    fn load_file(&mut self, path: &Path) -> Result<usize, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let file: TreasureFile = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        let mut added = 0;
        for treasure_type in file.treasure_types {
            let letter = treasure_type.letter.clone();
            let category = treasure_type.category;
            let idx = self.types.len();

            // Index by letter
            self.by_letter.insert(letter, idx);

            // Index by category
            self.by_category.entry(category).or_default().push(idx);

            self.types.push(treasure_type);
            added += 1;
        }
        Ok(added)
    }

    fn find(&self, letter: &str) -> Option<&TreasureTypeDef> {
        self.by_letter.get(letter).map(|&idx| &self.types[idx])
    }

    fn by_category(&self, category: TreasureCategory) -> Vec<&TreasureTypeDef> {
        self.by_category
            .get(&category)
            .map(|indices| indices.iter().map(|&i| &self.types[i]).collect())
            .unwrap_or_default()
    }

    fn all(&self) -> &[TreasureTypeDef] {
        &self.types
    }
}

/// Global treasure registry.
static REGISTRY: OnceLock<TreasureRegistry> = OnceLock::new();

/// Initialize the treasure registry by loading data files.
fn init_registry() -> TreasureRegistry {
    let mut registry = TreasureRegistry::new();

    // Find data directory relative to executable or working directory
    let data_paths = [
        // Development: relative to working directory
        "data/core/treasure.json",
        // Installed: relative to executable
        "../data/core/treasure.json",
        // Alternative: in current directory
        "treasure.json",
    ];

    let mut loaded = false;
    for path_str in &data_paths {
        let path = Path::new(path_str);
        if path.exists() {
            match registry.load_file(path) {
                Ok(count) => {
                    eprintln!("Loaded {} treasure types from {}", count, path.display());
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
        eprintln!("Warning: No treasure data files found. Using empty registry.");
        eprintln!("Expected: data/core/treasure.json");
    }

    registry
}

/// Get the global treasure registry.
fn registry() -> &'static TreasureRegistry {
    REGISTRY.get_or_init(init_registry)
}

/// Find a treasure type by letter (A-V).
pub fn find_treasure_type(letter: &str) -> Option<&'static TreasureTypeDef> {
    registry().find(letter)
}

/// Get all treasure types in a category.
pub fn types_by_category(category: TreasureCategory) -> Vec<&'static TreasureTypeDef> {
    registry().by_category(category)
}

/// Get all treasure types.
pub fn all_treasure_types() -> &'static [TreasureTypeDef] {
    registry().all()
}

// =============================================================================
// Gem and Jewellery Value Tables (loaded from gems_jewellery.json)
// =============================================================================

/// Gem value table entry.
#[derive(Debug, Clone, Copy)]
pub struct GemValueEntry {
    pub min_roll: u32,
    pub max_roll: u32,
    pub value_gp: u32,
}

/// JSON format for a gem value table entry.
#[derive(Debug, Deserialize)]
struct GemValueEntryJson {
    min: u32,
    max: u32,
    value_gp: u32,
}

/// JSON format for the gem value table.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GemValueTableJson {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    die: Option<String>,
    entries: Vec<GemValueEntryJson>,
    average_gp: u32,
}

/// JSON format for jewellery data.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JewelleryJson {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    dice: Option<String>,
    multiplier: u32,
    average_gp: u32,
    damaged_modifier: f64,
    #[serde(default)]
    damaged_description: Option<String>,
}

/// JSON file format for gems_jewellery.json.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GemsJewelleryFile {
    #[serde(default)]
    source: Option<String>,
    gem_value_table: GemValueTableJson,
    jewellery: JewelleryJson,
    #[serde(default)]
    notes: Option<serde_json::Value>,
}

/// Runtime-loaded gem and jewellery data.
struct GemsJewelleryData {
    gem_entries: Vec<GemValueEntry>,
    gem_average_gp: u32,
    jewellery_multiplier: u32,
    jewellery_average_gp: u32,
    jewellery_damaged_modifier: f64,
}

/// Hardcoded fallback values used when the JSON file is not found.
fn default_gems_jewellery() -> GemsJewelleryData {
    GemsJewelleryData {
        gem_entries: vec![
            GemValueEntry { min_roll: 1, max_roll: 4, value_gp: 10 },
            GemValueEntry { min_roll: 5, max_roll: 9, value_gp: 50 },
            GemValueEntry { min_roll: 10, max_roll: 15, value_gp: 100 },
            GemValueEntry { min_roll: 16, max_roll: 19, value_gp: 500 },
            GemValueEntry { min_roll: 20, max_roll: 20, value_gp: 1000 },
        ],
        gem_average_gp: 96,
        jewellery_multiplier: 100,
        jewellery_average_gp: 1050,
        jewellery_damaged_modifier: 0.5,
    }
}

/// Global gems/jewellery data.
static GEMS_JEWELLERY: OnceLock<GemsJewelleryData> = OnceLock::new();

/// Initialize gems/jewellery data by loading from JSON.
fn init_gems_jewellery() -> GemsJewelleryData {
    let data_paths = [
        "data/core/gems_jewellery.json",
        "../data/core/gems_jewellery.json",
        "gems_jewellery.json",
    ];

    for path_str in &data_paths {
        let path = Path::new(path_str);
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str::<GemsJewelleryFile>(&content) {
                    Ok(file) => {
                        let gem_entries = file
                            .gem_value_table
                            .entries
                            .iter()
                            .map(|e| GemValueEntry {
                                min_roll: e.min,
                                max_roll: e.max,
                                value_gp: e.value_gp,
                            })
                            .collect();

                        eprintln!("Loaded gems/jewellery data from {}", path.display());
                        return GemsJewelleryData {
                            gem_entries,
                            gem_average_gp: file.gem_value_table.average_gp,
                            jewellery_multiplier: file.jewellery.multiplier,
                            jewellery_average_gp: file.jewellery.average_gp,
                            jewellery_damaged_modifier: file.jewellery.damaged_modifier,
                        };
                    }
                    Err(e) => eprintln!("Warning: Failed to parse {}: {}", path.display(), e),
                },
                Err(e) => eprintln!("Warning: Failed to read {}: {}", path.display(), e),
            }
        }
    }

    eprintln!("Warning: No gems_jewellery data file found. Using defaults.");
    eprintln!("Expected: data/core/gems_jewellery.json");
    default_gems_jewellery()
}

/// Get the global gems/jewellery data.
fn gems_jewellery() -> &'static GemsJewelleryData {
    GEMS_JEWELLERY.get_or_init(init_gems_jewellery)
}

/// Get the gem value table entries.
pub fn gem_value_table() -> &'static [GemValueEntry] {
    &gems_jewellery().gem_entries
}

/// Roll a single gem's value using the loaded d20 table.
/// Returns the gem value in gold pieces.
pub fn roll_gem_value() -> u32 {
    let mut rng = rand::thread_rng();
    let roll: u32 = rng.gen_range(1..=20);
    gem_value_from_roll(roll)
}

/// Get gem value from a specific d20 roll (useful for testing/deterministic scenarios).
pub fn gem_value_from_roll(roll: u32) -> u32 {
    for entry in gem_value_table() {
        if roll >= entry.min_roll && roll <= entry.max_roll {
            return entry.value_gp;
        }
    }
    // Default to lowest value if roll is out of range
    10
}

/// Roll values for multiple gems.
/// Returns a vector of individual gem values in gold pieces.
/// Count is capped at 500 to prevent excessive allocation.
pub fn roll_gem_values(count: u32) -> Vec<u32> {
    (0..count.min(500)).map(|_| roll_gem_value()).collect()
}

/// Roll the value of a single piece of jewellery.
/// Uses the multiplier from gems_jewellery.json (default: 3d6 × 100 gp).
pub fn roll_jewellery_value() -> u32 {
    let mut rng = rand::thread_rng();
    let d1: u32 = rng.gen_range(1..=6);
    let d2: u32 = rng.gen_range(1..=6);
    let d3: u32 = rng.gen_range(1..=6);
    (d1 + d2 + d3) * gems_jewellery().jewellery_multiplier
}

/// Roll jewellery value from a specific 3d6 total (useful for testing).
pub fn jewellery_value_from_roll(roll_3d6: u32) -> u32 {
    roll_3d6 * gems_jewellery().jewellery_multiplier
}

/// Roll values for multiple pieces of jewellery.
/// Returns a vector of individual jewellery values in gold pieces.
/// Count is capped at 500 to prevent excessive allocation.
pub fn roll_jewellery_values(count: u32) -> Vec<u32> {
    (0..count.min(500)).map(|_| roll_jewellery_value()).collect()
}

/// Calculate damaged jewellery value using the loaded modifier.
pub fn damaged_jewellery_value(normal_value: u32) -> u32 {
    (normal_value as f64 * gems_jewellery().jewellery_damaged_modifier) as u32
}

/// Result of rolling treasure valuables (gems or jewellery).
#[derive(Debug, Clone)]
pub struct ValuablesResult {
    /// Individual values of each item.
    pub values: Vec<u32>,
    /// Total value in gold pieces.
    pub total_gp: u32,
}

/// Roll gems and return detailed results.
pub fn roll_gems(count: u32) -> ValuablesResult {
    let values = roll_gem_values(count);
    let total_gp = values.iter().copied().fold(0u32, u32::saturating_add);
    ValuablesResult { values, total_gp }
}

/// Roll jewellery and return detailed results.
pub fn roll_jewellery(count: u32) -> ValuablesResult {
    let values = roll_jewellery_values(count);
    let total_gp = values.iter().copied().fold(0u32, u32::saturating_add);
    ValuablesResult { values, total_gp }
}

/// Average gem value (loaded from data file).
pub fn average_gem_value() -> u32 {
    gems_jewellery().gem_average_gp
}

/// Average jewellery value (loaded from data file).
pub fn average_jewellery_value() -> u32 {
    gems_jewellery().jewellery_average_gp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads() {
        let types = all_treasure_types();
        assert_eq!(types.len(), 22, "Should have 22 treasure types (A-V)");
    }

    #[test]
    fn find_type_a() {
        let t = find_treasure_type("A");
        assert!(t.is_some(), "Type A should exist");
        let t = t.unwrap();
        assert_eq!(t.category, TreasureCategory::Hoard);
        assert_eq!(t.average_gp, 18000.0);
        assert!(!t.entries.is_empty());
    }

    #[test]
    fn find_type_case_sensitive() {
        // Treasure types should be uppercase
        assert!(find_treasure_type("A").is_some());
        assert!(find_treasure_type("a").is_none());
    }

    #[test]
    fn type_h_has_high_value() {
        let t = find_treasure_type("H").unwrap();
        assert_eq!(t.average_gp, 60000.0);
    }

    #[test]
    fn individual_treasure_p() {
        let t = find_treasure_type("P").unwrap();
        assert_eq!(t.category, TreasureCategory::Individual);
        assert_eq!(t.average_gp, 0.1);
        // Individual treasure has 100% chance entries
        assert!(t.entries.iter().all(|e| e.chance == 100));
    }

    #[test]
    fn group_treasure_u() {
        let t = find_treasure_type("U").unwrap();
        assert_eq!(t.category, TreasureCategory::Group);
    }

    #[test]
    fn hoard_types_count() {
        let hoards = types_by_category(TreasureCategory::Hoard);
        assert_eq!(hoards.len(), 15, "Should have 15 hoard types (A-O)");
    }

    #[test]
    fn individual_types_count() {
        let individual = types_by_category(TreasureCategory::Individual);
        assert_eq!(individual.len(), 5, "Should have 5 individual types (P-T)");
    }

    #[test]
    fn group_types_count() {
        let group = types_by_category(TreasureCategory::Group);
        assert_eq!(group.len(), 2, "Should have 2 group types (U-V)");
    }

    #[test]
    fn type_a_has_magic_items() {
        let t = find_treasure_type("A").unwrap();
        let magic_entry = t.entries.iter().find(|e| e.item_type == TreasureItemType::MagicItems);
        assert!(magic_entry.is_some(), "Type A should have magic items");
    }

    #[test]
    fn coin_type_detection() {
        assert!(TreasureItemType::Gp.is_coin());
        assert!(TreasureItemType::Cp.is_coin());
        assert!(!TreasureItemType::Gems.is_coin());
        assert!(!TreasureItemType::MagicItems.is_coin());
    }

    #[test]
    fn magic_type_detection() {
        assert!(TreasureItemType::MagicItems.is_magic());
        assert!(TreasureItemType::Potions.is_magic());
        assert!(!TreasureItemType::Gp.is_magic());
        assert!(!TreasureItemType::Gems.is_magic());
    }

    // Gem and jewellery tests

    #[test]
    fn gem_value_table_coverage() {
        // Verify all d20 rolls map to a value
        for roll in 1..=20 {
            let value = gem_value_from_roll(roll);
            assert!(value > 0, "Roll {} should have a value", roll);
        }
    }

    #[test]
    fn gem_value_boundaries() {
        // Test boundary values
        assert_eq!(gem_value_from_roll(1), 10);
        assert_eq!(gem_value_from_roll(4), 10);
        assert_eq!(gem_value_from_roll(5), 50);
        assert_eq!(gem_value_from_roll(9), 50);
        assert_eq!(gem_value_from_roll(10), 100);
        assert_eq!(gem_value_from_roll(15), 100);
        assert_eq!(gem_value_from_roll(16), 500);
        assert_eq!(gem_value_from_roll(19), 500);
        assert_eq!(gem_value_from_roll(20), 1000);
    }

    #[test]
    fn gem_value_distribution() {
        // Verify the possible values
        let possible_values: Vec<u32> = gem_value_table().iter().map(|e| e.value_gp).collect();
        assert_eq!(possible_values, vec![10, 50, 100, 500, 1000]);
    }

    #[test]
    fn jewellery_value_range() {
        // 3d6 × 100 = 300 to 1800 gp
        assert_eq!(jewellery_value_from_roll(3), 300);   // Minimum (3)
        assert_eq!(jewellery_value_from_roll(18), 1800); // Maximum (18)
        assert_eq!(jewellery_value_from_roll(10), 1000); // Near average
    }

    #[test]
    fn damaged_jewellery_halves_value() {
        assert_eq!(damaged_jewellery_value(1000), 500);
        assert_eq!(damaged_jewellery_value(300), 150);
        assert_eq!(damaged_jewellery_value(1800), 900);
    }

    #[test]
    fn roll_multiple_gems() {
        let result = roll_gems(5);
        assert_eq!(result.values.len(), 5);
        assert_eq!(result.total_gp, result.values.iter().sum::<u32>());
        // Each gem should be a valid value
        for v in &result.values {
            assert!(
                *v == 10 || *v == 50 || *v == 100 || *v == 500 || *v == 1000,
                "Invalid gem value: {}", v
            );
        }
    }

    #[test]
    fn roll_multiple_jewellery() {
        let result = roll_jewellery(3);
        assert_eq!(result.values.len(), 3);
        assert_eq!(result.total_gp, result.values.iter().sum::<u32>());
        // Each jewellery piece should be in valid range
        for v in &result.values {
            assert!(*v >= 300 && *v <= 1800, "Invalid jewellery value: {}", v);
            assert!(*v % 100 == 0, "Jewellery value should be multiple of 100");
        }
    }

    #[test]
    fn average_values_are_reasonable() {
        // Average gem: 96 gp (calculated from distribution)
        assert_eq!(average_gem_value(), 96);
        // Average jewellery: 3d6 avg = 10.5, × 100 = 1050
        assert_eq!(average_jewellery_value(), 1050);
    }

    #[test]
    fn gems_jewellery_loads_from_file() {
        // Verify the data file is loaded (not just defaults)
        let table = gem_value_table();
        assert_eq!(table.len(), 5, "Should have 5 gem value entries");
        assert_eq!(table[0].value_gp, 10);
        assert_eq!(table[4].value_gp, 1000);
    }
}
