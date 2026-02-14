use rand::Rng;
use std::collections::BTreeMap;

use crate::dice;
use crate::engine::result::EngineError;
use crate::rules::magic_item::{find_magic_item, find_magic_items_partial, search_magic_items};
use crate::rules::spell_data::{self, SpellList};
use crate::rules::treasure::{self, find_treasure_type, TreasureItemType};

use super::results::{
    GpValue, LookupItemMatch, LookupItemResult, LookupSpellResult, LookupTreasureTypeResult,
    RollTreasureResult, RolledTreasureItem, SearchItemEntry, SearchItemsResult,
    TreasureTypeEntryData,
};

pub fn action_lookup_item(name: &str) -> Result<LookupItemResult, EngineError> {
    let query = name.trim();

    if let Some(item) = find_magic_item(query) {
        return Ok(LookupItemResult {
            query: query.to_string(),
            item_match: LookupItemMatch::Single(
                crate::engine::lookup::results::MagicItemData::from_def(item),
            ),
        });
    }

    let matches = find_magic_items_partial(query);
    match matches.len() {
        0 => Err(EngineError::InvalidInput(format!(
            "no magic item found matching '{}'.",
            query
        ))),
        1 => Ok(LookupItemResult {
            query: query.to_string(),
            item_match: LookupItemMatch::Single(
                crate::engine::lookup::results::MagicItemData::from_def(matches[0]),
            ),
        }),
        n if n <= 10 => Ok(LookupItemResult {
            query: query.to_string(),
            item_match: LookupItemMatch::Multiple(
                matches.into_iter().map(|item| item.name.clone()).collect(),
            ),
        }),
        n => Ok(LookupItemResult {
            query: query.to_string(),
            item_match: LookupItemMatch::TooMany(n),
        }),
    }
}

pub fn action_search_items(query: &str) -> Result<SearchItemsResult, EngineError> {
    let query = query.trim();

    let mut by_category: BTreeMap<String, Vec<SearchItemEntry>> = BTreeMap::new();
    for item in search_magic_items(query) {
        by_category
            .entry(item.category.name().to_string())
            .or_default()
            .push(SearchItemEntry {
                name: item.name.clone(),
                cursed: item.cursed,
            });
    }

    Ok(SearchItemsResult {
        query: query.to_string(),
        by_category,
    })
}

pub fn action_lookup_treasure_type(letter: &str) -> Result<LookupTreasureTypeResult, EngineError> {
    let letter = letter.trim().to_uppercase();
    if letter.is_empty() {
        return Err(EngineError::InvalidInput(
            "treasure type must not be empty.".to_string(),
        ));
    }

    let treasure_type = find_treasure_type(&letter).ok_or_else(|| {
        EngineError::InvalidInput(format!(
            "unknown treasure type '{}'. Valid types are A-V.",
            letter
        ))
    })?;

    let entries = treasure_type
        .entries
        .iter()
        .map(|entry| TreasureTypeEntryData {
            chance: entry.chance,
            quantity: entry.quantity.clone(),
            item_type: entry.item_type.name().to_string(),
            restriction: entry.restriction.clone(),
            note: entry.note.clone(),
        })
        .collect::<Vec<_>>();

    let has_coins = treasure_type.entries.iter().any(|e| e.item_type.is_coin());
    let has_gems = treasure_type
        .entries
        .iter()
        .any(|e| e.item_type == TreasureItemType::Gems);
    let has_jewellery = treasure_type
        .entries
        .iter()
        .any(|e| e.item_type == TreasureItemType::Jewellery);
    let has_magic = treasure_type.entries.iter().any(|e| e.item_type.is_magic());

    Ok(LookupTreasureTypeResult {
        letter: treasure_type.letter.clone(),
        category: treasure_type.category.name().to_string(),
        average_gp: treasure_type.average_gp,
        entries,
        has_coins,
        has_gems,
        has_jewellery,
        has_magic,
    })
}

pub fn action_lookup_spell(
    name: &str,
    list: Option<SpellList>,
) -> Result<LookupSpellResult, EngineError> {
    let query = name.trim();
    if query.is_empty() {
        return Err(EngineError::InvalidInput(
            "spell name must not be empty.".to_string(),
        ));
    }

    let spell = spell_data::find_spell(query, list)
        .ok_or_else(|| EngineError::InvalidInput(format!("spell '{}' not found.", query)))?;

    Ok(LookupSpellResult {
        query: query.to_string(),
        name: spell.name.clone(),
        list: spell.list,
        level: spell.level,
        range: spell.range.clone(),
        duration: spell.duration.clone(),
        description: spell.description.clone(),
        reversible: spell.reversible,
        reversed_name: spell.reversed_name.clone(),
    })
}

pub fn action_roll_treasure(letter: &str) -> Result<RollTreasureResult, EngineError> {
    let letter = letter.trim().to_uppercase();
    if letter.is_empty() {
        return Err(EngineError::InvalidInput(
            "treasure type must not be empty.".to_string(),
        ));
    }

    let treasure_type = find_treasure_type(&letter).ok_or_else(|| {
        EngineError::InvalidInput(format!(
            "unknown treasure type '{}'. Valid types are A-V.",
            letter
        ))
    })?;

    let mut rng = rand::thread_rng();
    let mut items = Vec::new();
    let mut total_gp = 0.0;

    for entry in &treasure_type.entries {
        let roll: u32 = rng.gen_range(1..=100);
        if roll > entry.chance {
            continue;
        }

        let quantity = match parse_quantity_with_multiplier(&entry.quantity) {
            Ok(q) => q.max(1),
            Err(_) => continue,
        };

        let mut item = RolledTreasureItem {
            item_type: entry.item_type.name().to_string(),
            quantity,
            gp_value: None,
            values: None,
            total_gp: None,
            restriction: None,
            note: None,
        };

        match entry.item_type {
            TreasureItemType::Cp => {
                let gp_value = quantity as f64 * 0.01;
                total_gp += gp_value;
                item.gp_value = Some(GpValue::Float(gp_value));
            }
            TreasureItemType::Sp => {
                let gp_value = quantity as f64 * 0.1;
                total_gp += gp_value;
                item.gp_value = Some(GpValue::Float(gp_value));
            }
            TreasureItemType::Ep => {
                let gp_value = quantity as f64 * 0.5;
                total_gp += gp_value;
                item.gp_value = Some(GpValue::Float(gp_value));
            }
            TreasureItemType::Gp => {
                total_gp += quantity as f64;
                item.gp_value = Some(GpValue::Int(quantity));
            }
            TreasureItemType::Pp => {
                let gp_value = quantity as f64 * 5.0;
                total_gp += gp_value;
                item.gp_value = Some(GpValue::Float(gp_value));
            }
            TreasureItemType::Gems => {
                let gem_result = treasure::roll_gems(quantity as u32);
                total_gp += gem_result.total_gp as f64;
                item.values = Some(gem_result.values);
                item.total_gp = Some(gem_result.total_gp);
            }
            TreasureItemType::Jewellery => {
                let jewellery_result = treasure::roll_jewellery(quantity as u32);
                total_gp += jewellery_result.total_gp as f64;
                item.values = Some(jewellery_result.values);
                item.total_gp = Some(jewellery_result.total_gp);
            }
            _ => {
                item.restriction = entry.restriction.clone();
            }
        }

        item.note = entry.note.clone();
        items.push(item);
    }

    Ok(RollTreasureResult {
        letter,
        category: treasure_type.category.name().to_string(),
        items,
        total_gp,
    })
}

/// Parse a quantity string that may include a multiplier.
/// Supports formats like "1d6", "1d6 × 1000", "3", "1d4 × 10".
fn parse_quantity_with_multiplier(quantity: &str) -> Result<i32, String> {
    let normalized = quantity.replace(['×', 'x'], "*").replace(' ', "");

    if let Some(pos) = normalized.find('*') {
        let dice_part = &normalized[..pos];
        let multiplier_part = &normalized[pos + 1..];

        let base = if dice_part.contains('d') || dice_part.contains('D') {
            dice::roll_str(dice_part)
                .map(|r| r.total)
                .map_err(|e| format!("bad dice expr '{}': {}", dice_part, e))?
        } else {
            dice_part
                .parse::<i32>()
                .map_err(|_| format!("invalid number '{}'", dice_part))?
        };

        let multiplier: i32 = multiplier_part
            .parse()
            .map_err(|_| format!("invalid multiplier '{}'", multiplier_part))?;

        Ok(base * multiplier)
    } else if normalized.contains('d') || normalized.contains('D') {
        dice::roll_str(&normalized)
            .map(|r| r.total)
            .map_err(|e| format!("bad dice expr '{}': {}", quantity, e))
    } else {
        normalized
            .parse::<i32>()
            .map_err(|_| format!("invalid quantity '{}'", quantity))
    }
}

pub fn parse_spell_list(s: &str) -> Option<SpellList> {
    match s.to_lowercase().as_str() {
        "cleric" => Some(SpellList::Cleric),
        "magicuser" | "magic-user" | "magic_user" | "mu" | "mage" => Some(SpellList::MagicUser),
        "druid" => Some(SpellList::Druid),
        "illusionist" => Some(SpellList::Illusionist),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::spell_data::SpellList;

    #[test]
    fn parse_spell_list_aliases() {
        assert_eq!(parse_spell_list("cleric"), Some(SpellList::Cleric));
        assert_eq!(parse_spell_list("mu"), Some(SpellList::MagicUser));
        assert_eq!(parse_spell_list("mage"), Some(SpellList::MagicUser));
        assert_eq!(parse_spell_list("bad"), None);
    }

    #[test]
    fn action_lookup_item_exact_match() {
        let result = action_lookup_item("Bag of Holding").unwrap();
        match result.item_match {
            LookupItemMatch::Single(item) => assert_eq!(item.name, "Bag of Holding"),
            _ => panic!("expected single item match"),
        }
    }

    #[test]
    fn action_search_items_finds_results() {
        let result = action_search_items("healing").unwrap();
        assert!(result.count() > 0);
    }

    #[test]
    fn action_lookup_treasure_type_returns_entries() {
        let result = action_lookup_treasure_type("A").unwrap();
        assert_eq!(result.letter, "A");
        assert!(!result.entries.is_empty());
    }

    #[test]
    fn action_lookup_spell_with_list() {
        let result = action_lookup_spell("Cure Light Wounds", Some(SpellList::Cleric)).unwrap();
        assert_eq!(result.name, "Cure Light Wounds");
        assert_eq!(result.list, SpellList::Cleric);
    }

    #[test]
    fn action_roll_treasure_type_p_has_copper() {
        let result = action_roll_treasure("P").unwrap();
        assert_eq!(result.letter, "P");
        assert!(result
            .items
            .iter()
            .any(|item| item.item_type == "Copper Pieces"));
    }
}
