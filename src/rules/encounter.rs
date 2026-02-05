use crate::state::wilderness::Terrain;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

/// An entry from an encounter table.
#[derive(Debug, Clone, PartialEq)]
pub struct EncounterEntry {
    /// Monster name.
    pub name: String,
    /// Number appearing (dice notation).
    pub number: String,
    /// Optional HD variant (for monsters with multiple HD versions).
    pub hd: Option<String>,
}

impl EncounterEntry {
    pub fn new(name: &str, number: &str) -> Self {
        EncounterEntry {
            name: name.to_string(),
            number: number.to_string(),
            hd: None,
        }
    }

    pub fn with_hd(name: &str, number: &str, hd: &str) -> Self {
        EncounterEntry {
            name: name.to_string(),
            number: number.to_string(),
            hd: Some(hd.to_string()),
        }
    }
}

// ============================================================================
// JSON data structures for loading encounter tables
// ============================================================================

#[derive(Debug, Deserialize)]
struct EncounterData {
    dungeon: HashMap<String, HashMap<String, JsonMonsterEntry>>,
    wilderness: WildernessData,
}

#[derive(Debug, Deserialize)]
struct WildernessData {
    terrain_table: HashMap<String, HashMap<String, String>>,
    sub_tables: HashMap<String, HashMap<String, HashMap<String, String>>>,
}

#[derive(Debug, Deserialize)]
struct JsonMonsterEntry {
    monster: String,
    number: String,
    hd: Option<String>,
}

// ============================================================================
// Runtime data loading
// ============================================================================

static ENCOUNTER_DATA: OnceLock<EncounterData> = OnceLock::new();

fn load_encounter_data() -> &'static EncounterData {
    ENCOUNTER_DATA.get_or_init(|| {
        let json_str = include_str!("../../data/core/encounters.json");
        serde_json::from_str(json_str).expect("Failed to parse encounters.json")
    })
}

// ============================================================================
// Dungeon encounter tables
// Advanced Fantasy uses d4/d10 giving 40 entries per level
// Levels: 1, 2, 3, 4-5, 6-7, 8+
// ============================================================================

/// Get the dungeon level key for a given numeric level.
fn dungeon_level_key(level: u32) -> &'static str {
    match level {
        0 | 1 => "1",
        2 => "2",
        3 => "3",
        4 | 5 => "4-5",
        6 | 7 => "6-7",
        _ => "8+",
    }
}

/// Convert d4/d10 roll to lookup key (e.g., 2, 5 -> "2/5").
fn d4d10_key(d4: u32, d10: u32) -> String {
    format!("{}/{}", d4.clamp(1, 4), d10.clamp(0, 9))
}

/// Look up a dungeon encounter by level and d4/d10 roll.
///
/// # Arguments
/// * `level` - Dungeon level (1, 2, 3, 4-5, 6-7, or 8+)
/// * `d4` - d4 roll (1-4)
/// * `d10` - d10 roll (0-9)
pub fn dungeon_encounter(level: u32, d4: u32, d10: u32) -> Option<EncounterEntry> {
    let data = load_encounter_data();
    let level_key = dungeon_level_key(level);
    let roll_key = d4d10_key(d4, d10);

    data.dungeon.get(level_key)?.get(&roll_key).map(|e| {
        if let Some(ref hd) = e.hd {
            EncounterEntry::with_hd(&e.monster, &e.number, hd)
        } else {
            EncounterEntry::new(&e.monster, &e.number)
        }
    })
}

/// Look up a dungeon encounter by level and a combined d20-style roll (1-40).
/// This provides backward compatibility with simpler d20-based systems.
///
/// # Arguments
/// * `level` - Dungeon level
/// * `roll` - Roll from 1-40 (mapped to d4/d10)
pub fn dungeon_encounter_d40(level: u32, roll: u32) -> Option<EncounterEntry> {
    // Convert 1-40 to d4/d10: roll 1-10 -> 1/0-9, 11-20 -> 2/0-9, etc.
    let clamped = roll.clamp(1, 40);
    let d4 = ((clamped - 1) / 10) + 1;
    let d10 = (clamped - 1) % 10;
    dungeon_encounter(level, d4, d10)
}

/// Get the number of entries in a dungeon level table.
pub fn dungeon_table_size(level: u32) -> usize {
    let data = load_encounter_data();
    let level_key = dungeon_level_key(level);
    data.dungeon.get(level_key).map(|t| t.len()).unwrap_or(0)
}

// ============================================================================
// Wilderness encounter tables
// Two-step lookup: d8 roll -> sub-table code, d20 roll -> monster
// ============================================================================

/// Get the terrain name used in the JSON data for a given Terrain enum.
fn terrain_to_json_name(terrain: Terrain) -> &'static str {
    match terrain {
        Terrain::Clear => "Clear, Grasslands",
        Terrain::City => "City",
        Terrain::Barren | Terrain::Hills | Terrain::Mountains => "Barren, Hills, Mountains",
        Terrain::Forest => "Forest",
        Terrain::Desert => "Desert",
        Terrain::Swamp => "Swamp",
        Terrain::Jungle => "Jungle",
        Terrain::Ocean => "Ocean, Sea",
        Terrain::River => "Lake, River",
    }
}

/// Look up the sub-table code for a terrain and d8 roll.
pub fn wilderness_subtable_code(terrain: Terrain, d8: u32) -> Option<String> {
    let data = load_encounter_data();
    let terrain_name = terrain_to_json_name(terrain);
    let roll_key = d8.clamp(1, 8).to_string();

    data.wilderness
        .terrain_table
        .get(terrain_name)?
        .get(&roll_key)
        .cloned()
}

/// Parse a sub-table code like "B-Animal" or "1-Dragon" into (table_id, category).
fn parse_subtable_code(code: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = code.splitn(2, '-').collect();
    if parts.len() == 2 {
        Some((parts[0], parts[1]))
    } else {
        None
    }
}

/// Look up a monster from a wilderness sub-table.
///
/// # Arguments
/// * `code` - Sub-table code like "B-Animal" or "1-Dragon"
/// * `d20` - d20 roll (1-20)
pub fn wilderness_subtable_lookup(code: &str, d20: u32) -> Option<String> {
    let data = load_encounter_data();
    let (table_id, category) = parse_subtable_code(code)?;
    let roll_key = d20.clamp(1, 20).to_string();

    data.wilderness
        .sub_tables
        .get(table_id)?
        .get(category)?
        .get(&roll_key)
        .cloned()
}

/// Full wilderness encounter lookup.
///
/// # Arguments
/// * `terrain` - The terrain type
/// * `d8` - d8 roll for terrain table (1-8)
/// * `d20` - d20 roll for sub-table (1-20)
///
/// # Returns
/// The monster name if found, None otherwise.
pub fn wilderness_encounter(terrain: Terrain, d8: u32, d20: u32) -> Option<String> {
    let code = wilderness_subtable_code(terrain, d8)?;
    wilderness_subtable_lookup(&code, d20)
}

/// Simplified wilderness encounter for backward compatibility.
/// Uses d8=4 (human/humanoid) and maps d20 roll.
pub fn wilderness_encounter_simple(terrain: Terrain, roll: u32) -> Option<EncounterEntry> {
    // Try d8=4 (Human) for most terrains, then use the d20 roll
    let d8 = 4; // Human sub-table
    let d20 = roll.clamp(1, 20);

    let code = wilderness_subtable_code(terrain, d8)?;
    let monster = wilderness_subtable_lookup(&code, d20)?;

    Some(EncounterEntry::new(&monster, "1d6"))
}

/// Get available sub-table categories for a terrain.
pub fn wilderness_categories(terrain: Terrain) -> Vec<String> {
    let data = load_encounter_data();
    let terrain_name = terrain_to_json_name(terrain);

    let mut categories = Vec::new();
    if let Some(terrain_data) = data.wilderness.terrain_table.get(terrain_name) {
        for code in terrain_data.values() {
            if let Some((table_id, category)) = parse_subtable_code(code) {
                let full_name = format!("{}-{}", table_id, category);
                if !categories.contains(&full_name) {
                    categories.push(full_name);
                }
            }
        }
    }
    categories.sort();
    categories
}

// ============================================================================
// Legacy API compatibility (deprecated, for gradual migration)
// ============================================================================

/// Legacy: Get the dungeon encounter table for a given dungeon level.
/// Returns a Vec of 20 encounter entries for backward compatibility.
#[deprecated(note = "Use dungeon_encounter_d40 instead")]
pub fn dungeon_table(level: u32) -> Vec<EncounterEntry> {
    (1..=20)
        .filter_map(|roll| dungeon_encounter_d40(level, roll))
        .collect()
}

/// Legacy: Get the wilderness encounter table for a given terrain type.
#[deprecated(note = "Use wilderness_encounter instead")]
pub fn wilderness_table(terrain: Terrain) -> Vec<EncounterEntry> {
    // Return a simplified 20-entry table using the Human sub-table
    (1..=20)
        .filter_map(|roll| wilderness_encounter_simple(terrain, roll))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dungeon_level_1_has_40_entries() {
        assert_eq!(dungeon_table_size(1), 40);
    }

    #[test]
    fn dungeon_level_8_has_40_entries() {
        assert_eq!(dungeon_table_size(8), 40);
    }

    #[test]
    fn dungeon_encounter_roll_1_0() {
        let e = dungeon_encounter(1, 1, 0).unwrap();
        assert_eq!(e.name, "Acolyte");
        assert_eq!(e.number, "1d8");
    }

    #[test]
    fn dungeon_encounter_roll_4_9() {
        let e = dungeon_encounter(1, 4, 9).unwrap();
        assert_eq!(e.name, "Zombie");
        assert_eq!(e.number, "1d4");
    }

    #[test]
    fn dungeon_encounter_d40_roll_1() {
        let e = dungeon_encounter_d40(1, 1).unwrap();
        assert_eq!(e.name, "Acolyte");
    }

    #[test]
    fn dungeon_encounter_d40_roll_40() {
        let e = dungeon_encounter_d40(1, 40).unwrap();
        assert_eq!(e.name, "Zombie");
    }

    #[test]
    fn dungeon_encounter_level_8_has_vampires() {
        let e = dungeon_encounter(8, 4, 9).unwrap();
        assert_eq!(e.name, "Vampire");
    }

    #[test]
    fn dungeon_encounter_with_hd_variant() {
        // Ankheg at level 2 has HD 3
        let e = dungeon_encounter(2, 1, 0).unwrap();
        assert_eq!(e.name, "Ankheg");
        assert_eq!(e.hd, Some("3".to_string()));
    }

    #[test]
    fn wilderness_subtable_code_forest() {
        let code = wilderness_subtable_code(Terrain::Forest, 1).unwrap();
        assert_eq!(code, "F-Animal");
    }

    #[test]
    fn wilderness_subtable_code_forest_dragon() {
        let code = wilderness_subtable_code(Terrain::Forest, 2).unwrap();
        assert_eq!(code, "1-Dragon");
    }

    #[test]
    fn wilderness_subtable_lookup_dragon() {
        let monster = wilderness_subtable_lookup("1-Dragon", 1).unwrap();
        assert_eq!(monster, "Chimera");
    }

    #[test]
    fn wilderness_subtable_lookup_forest_animal() {
        let monster = wilderness_subtable_lookup("F-Animal", 1).unwrap();
        assert_eq!(monster, "Bear, Grizzly");
    }

    #[test]
    fn wilderness_full_encounter_forest() {
        // Forest, d8=1 (F-Animal), d20=1 -> Bear, Grizzly
        let monster = wilderness_encounter(Terrain::Forest, 1, 1).unwrap();
        assert_eq!(monster, "Bear, Grizzly");
    }

    #[test]
    fn wilderness_full_encounter_desert() {
        // Desert, d8=1 (D-Animal), d20=1 -> Camel
        let monster = wilderness_encounter(Terrain::Desert, 1, 1).unwrap();
        assert_eq!(monster, "Camel");
    }

    #[test]
    fn wilderness_full_encounter_ocean() {
        // Ocean, d8=4 (O-Swimmer), d20=1 -> Dragon Turtle
        let monster = wilderness_encounter(Terrain::Ocean, 4, 1).unwrap();
        assert_eq!(monster, "Dragon Turtle");
    }

    #[test]
    fn wilderness_categories_forest() {
        let cats = wilderness_categories(Terrain::Forest);
        assert!(cats.contains(&"F-Animal".to_string()));
        assert!(cats.contains(&"1-Dragon".to_string()));
        assert!(cats.contains(&"F-Monster".to_string()));
    }

    #[test]
    fn all_dungeon_levels_populated() {
        for level_key in &["1", "2", "3", "4-5", "6-7", "8+"] {
            let level = match *level_key {
                "1" => 1,
                "2" => 2,
                "3" => 3,
                "4-5" => 4,
                "6-7" => 6,
                "8+" => 8,
                _ => unreachable!(),
            };
            assert_eq!(
                dungeon_table_size(level),
                40,
                "Level {} should have 40 entries",
                level_key
            );
        }
    }

    #[test]
    fn all_wilderness_terrains_have_codes() {
        for terrain in &[
            Terrain::Clear,
            Terrain::Forest,
            Terrain::Hills,
            Terrain::Mountains,
            Terrain::Desert,
            Terrain::Swamp,
            Terrain::Jungle,
            Terrain::Ocean,
            Terrain::River,
        ] {
            for d8 in 1..=8 {
                let code = wilderness_subtable_code(*terrain, d8);
                assert!(
                    code.is_some(),
                    "{:?} d8={} should have a sub-table code",
                    terrain,
                    d8
                );
            }
        }
    }
}
