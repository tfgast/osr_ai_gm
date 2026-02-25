//! Rumor table definitions loaded from JSON data files.
//! Provides adventure hooks and NPC gossip for OSR gameplay.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// A single rumor entry in a rumor table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RumorEntry {
    /// The rumor text that an NPC might share.
    pub text: String,
    /// Whether this rumor is true or false (GM knowledge only).
    pub true_rumor: bool,
    /// Tags for categorization (e.g., "monster", "treasure", "dungeon").
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A named rumor table with a collection of rumor entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RumorTableDef {
    /// Table name (e.g., "tavern", "market", "docks").
    pub name: String,
    /// Description of the table's context.
    #[serde(default)]
    pub description: Option<String>,
    /// The rumor entries in this table.
    pub entries: Vec<RumorEntry>,
}

/// JSON file format for rumors data.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RumorsFile {
    #[serde(default)]
    source: Option<String>,
    tables: Vec<RumorTableDef>,
}

/// Registry holding all loaded rumor tables.
struct RumorRegistry {
    tables: Vec<RumorTableDef>,
    by_name: HashMap<String, usize>,
}

impl RumorRegistry {
    fn new() -> Self {
        Self {
            tables: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    fn load_file(&mut self, path: &Path) -> Result<usize, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let file: RumorsFile = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        let mut added = 0;
        for table in file.tables {
            let name = table.name.to_lowercase();
            let idx = self.tables.len();
            self.by_name.insert(name, idx);
            self.tables.push(table);
            added += 1;
        }
        Ok(added)
    }

    fn find(&self, name: &str) -> Option<&RumorTableDef> {
        self.by_name.get(&name.to_lowercase()).map(|&idx| &self.tables[idx])
    }

    fn all(&self) -> &[RumorTableDef] {
        &self.tables
    }
}

/// Global rumor registry.
static REGISTRY: OnceLock<RumorRegistry> = OnceLock::new();

/// Initialize the rumor registry by loading data files.
fn init_registry() -> RumorRegistry {
    let mut registry = RumorRegistry::new();

    let data_paths = [
        "data/games/ose/data/rumors.json",
        "../data/games/ose/data/rumors.json",
        "rumors.json",
    ];

    let mut loaded = false;
    for path_str in &data_paths {
        let path = Path::new(path_str);
        if path.exists() {
            match registry.load_file(path) {
                Ok(count) => {
                    eprintln!("Loaded {} rumor tables from {}", count, path.display());
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
        eprintln!("Warning: No rumor data files found. Using empty registry.");
        eprintln!("Expected: data/games/ose/data/rumors.json");
    }

    registry
}

/// Get the global rumor registry.
fn registry() -> &'static RumorRegistry {
    REGISTRY.get_or_init(init_registry)
}

/// Find a rumor table by name.
pub fn find_rumor_table(name: &str) -> Option<&'static RumorTableDef> {
    registry().find(name)
}

/// Get all rumor tables.
pub fn all_rumor_tables() -> &'static [RumorTableDef] {
    registry().all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads() {
        let tables = all_rumor_tables();
        assert!(tables.len() >= 3, "Should have at least 3 rumor tables, got {}", tables.len());
    }

    #[test]
    fn find_tavern_table() {
        let t = find_rumor_table("tavern");
        assert!(t.is_some(), "Tavern table should exist");
        let t = t.unwrap();
        assert_eq!(t.name, "tavern");
        assert!(!t.entries.is_empty());
    }

    #[test]
    fn find_case_insensitive() {
        assert!(find_rumor_table("TAVERN").is_some());
        assert!(find_rumor_table("Tavern").is_some());
        assert!(find_rumor_table("tavern").is_some());
    }

    #[test]
    fn find_nonexistent_table() {
        assert!(find_rumor_table("nonexistent").is_none());
    }

    #[test]
    fn rumor_entries_have_text() {
        for table in all_rumor_tables() {
            for entry in &table.entries {
                assert!(!entry.text.is_empty(), "Rumor text should not be empty in table {}", table.name);
            }
        }
    }

    #[test]
    fn rumor_entries_have_truth_values() {
        let tavern = find_rumor_table("tavern").unwrap();
        let true_count = tavern.entries.iter().filter(|e| e.true_rumor).count();
        let false_count = tavern.entries.iter().filter(|e| !e.true_rumor).count();
        assert!(true_count > 0, "Should have some true rumors");
        assert!(false_count > 0, "Should have some false rumors");
    }

    #[test]
    fn market_table_exists() {
        let t = find_rumor_table("market");
        assert!(t.is_some(), "Market table should exist");
    }

    #[test]
    fn docks_table_exists() {
        let t = find_rumor_table("docks");
        assert!(t.is_some(), "Docks table should exist");
    }
}
