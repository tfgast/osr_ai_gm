/// Treasure type definitions loaded from JSON data files.
/// Covers hoard treasures (A-O), individual treasures (P-T), and group treasures (U-V).

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
}
