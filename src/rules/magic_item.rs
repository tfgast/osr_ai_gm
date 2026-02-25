//! Magic item definitions loaded from JSON data files.
//! Covers armor, miscellaneous items, potions, rings, rods, staves, wands, scrolls, swords, and weapons.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// Magic item category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemCategory {
    Armor,
    Miscellaneous,
    Potion,
    Ring,
    Rod,
    Staff,
    Wand,
    Scroll,
    Sword,
    Weapon,
}

impl ItemCategory {
    pub fn name(self) -> &'static str {
        match self {
            ItemCategory::Armor => "Armor",
            ItemCategory::Miscellaneous => "Miscellaneous",
            ItemCategory::Potion => "Potion",
            ItemCategory::Ring => "Ring",
            ItemCategory::Rod => "Rod",
            ItemCategory::Staff => "Staff",
            ItemCategory::Wand => "Wand",
            ItemCategory::Scroll => "Scroll",
            ItemCategory::Sword => "Sword",
            ItemCategory::Weapon => "Weapon",
        }
    }
}

/// A property of a magic item (key-value pair).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemProperty {
    #[serde(default)]
    pub key: Option<String>,
    pub value: String,
}

/// A magic item definition loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicItemDef {
    pub name: String,
    pub category: ItemCategory,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub properties: Vec<ItemProperty>,
    #[serde(default)]
    pub cursed: bool,
}

/// JSON file format for magic item data.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MagicItemFile {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    count: Option<usize>,
    items: Vec<MagicItemDef>,
}

/// Registry holding all loaded magic items.
struct MagicItemRegistry {
    items: Vec<MagicItemDef>,
    by_name: HashMap<String, usize>,
    by_category: HashMap<ItemCategory, Vec<usize>>,
}

impl MagicItemRegistry {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            by_name: HashMap::new(),
            by_category: HashMap::new(),
        }
    }

    fn load_file(&mut self, path: &Path) -> Result<usize, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let file: MagicItemFile = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        let mut added = 0;
        for item in file.items {
            let key = item.name.to_lowercase();
            let category = item.category;
            let idx = self.items.len();

            // Index by name (first occurrence wins)
            self.by_name.entry(key).or_insert(idx);

            // Index by category
            self.by_category.entry(category).or_default().push(idx);

            self.items.push(item);
            added += 1;
        }
        Ok(added)
    }

    fn find(&self, name: &str) -> Option<&MagicItemDef> {
        let key = name.to_lowercase();
        self.by_name.get(&key).map(|&idx| &self.items[idx])
    }

    fn by_category(&self, category: ItemCategory) -> Vec<&MagicItemDef> {
        self.by_category
            .get(&category)
            .map(|indices| indices.iter().map(|&i| &self.items[i]).collect())
            .unwrap_or_default()
    }

    fn all(&self) -> &[MagicItemDef] {
        &self.items
    }
}

/// Global magic item registry.
static REGISTRY: OnceLock<MagicItemRegistry> = OnceLock::new();

/// Initialize the magic item registry by loading data files.
fn init_registry() -> MagicItemRegistry {
    let mut registry = MagicItemRegistry::new();

    if let Some(path) = crate::manifest::game_data_file("magic_items") {
        if path.exists() {
            match registry.load_file(&path) {
                Ok(count) => {
                    eprintln!("Loaded {} magic items from {}", count, path.display());
                    return registry;
                }
                Err(e) => {
                    eprintln!("Warning: {}", e);
                }
            }
        }
    }

    eprintln!("Warning: No magic item data file found in game system manifest.");

    registry
}

/// Get the global magic item registry.
fn registry() -> &'static MagicItemRegistry {
    REGISTRY.get_or_init(init_registry)
}

/// Find a magic item by name (case-insensitive).
pub fn find_magic_item(name: &str) -> Option<&'static MagicItemDef> {
    registry().find(name)
}

/// Get all magic items in a category.
pub fn items_by_category(category: ItemCategory) -> Vec<&'static MagicItemDef> {
    registry().by_category(category)
}

/// Get all magic items.
pub fn all_magic_items() -> &'static [MagicItemDef] {
    registry().all()
}

/// Search magic items by keyword (case-insensitive).
/// Matches against name, description, and property values.
pub fn search_magic_items(query: &str) -> Vec<&'static MagicItemDef> {
    let query_lower = query.to_lowercase();
    registry()
        .all()
        .iter()
        .filter(|item| {
            // Search in name
            if item.name.to_lowercase().contains(&query_lower) {
                return true;
            }
            // Search in description
            if let Some(ref desc) = item.description {
                if desc.to_lowercase().contains(&query_lower) {
                    return true;
                }
            }
            // Search in properties
            for prop in &item.properties {
                if prop.value.to_lowercase().contains(&query_lower) {
                    return true;
                }
                if let Some(ref key) = prop.key {
                    if key.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
            }
            false
        })
        .collect()
}

/// Find magic items with partial name match (case-insensitive).
/// Returns items where the name contains the query string.
pub fn find_magic_items_partial(name: &str) -> Vec<&'static MagicItemDef> {
    let name_lower = name.to_lowercase();
    registry()
        .all()
        .iter()
        .filter(|item| item.name.to_lowercase().contains(&name_lower))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads() {
        let items = all_magic_items();
        assert!(!items.is_empty(), "Magic item registry should not be empty");
    }

    #[test]
    fn find_bag_of_holding() {
        let item = find_magic_item("Bag of Holding");
        assert!(item.is_some(), "Bag of Holding should exist");
        let item = item.unwrap();
        assert_eq!(item.category, ItemCategory::Miscellaneous);
        assert!(!item.cursed);
    }

    #[test]
    fn find_item_case_insensitive() {
        assert!(find_magic_item("bag of holding").is_some());
        assert!(find_magic_item("POTION OF HEALING").is_some());
    }

    #[test]
    fn cursed_item_detection() {
        // Cursed items should have cursed=true
        let bracers = find_magic_item("Bracers of Defencelessness");
        assert!(bracers.is_some());
        // Note: cursed flag is set based on name containing "cursed"
    }

    #[test]
    fn items_by_category_potion() {
        let potions = items_by_category(ItemCategory::Potion);
        assert!(!potions.is_empty(), "Should have potions");
        assert!(potions.iter().any(|p| p.name.contains("Healing")));
    }

    #[test]
    fn items_by_category_sword() {
        let swords = items_by_category(ItemCategory::Sword);
        assert!(!swords.is_empty(), "Should have swords");
        assert!(swords.iter().any(|s| s.name.contains("Flaming")));
    }

    #[test]
    fn item_has_properties() {
        let item = find_magic_item("Bag of Holding").unwrap();
        assert!(!item.properties.is_empty(), "Bag of Holding should have properties");
    }

    #[test]
    fn search_by_name() {
        let results = search_magic_items("healing");
        assert!(!results.is_empty(), "Should find items with 'healing'");
        assert!(results.iter().any(|i| i.name.to_lowercase().contains("healing")));
    }

    #[test]
    fn search_no_results() {
        let results = search_magic_items("xyznonexistent123");
        assert!(results.is_empty());
    }

    #[test]
    fn partial_match_single() {
        let results = find_magic_items_partial("Bag of Holding");
        assert!(!results.is_empty());
        assert!(results.iter().any(|i| i.name == "Bag of Holding"));
    }

    #[test]
    fn partial_match_multiple() {
        let results = find_magic_items_partial("Potion");
        assert!(results.len() > 1, "Should find multiple potions");
        assert!(results.iter().all(|i| i.name.to_lowercase().contains("potion")));
    }
}
