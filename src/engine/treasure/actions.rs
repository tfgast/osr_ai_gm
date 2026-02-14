use crate::dice;
use crate::engine::result::EngineError;
use crate::rules::treasure::{self, TreasureCategory, TreasureItemType, TreasureTypeDef};
use rand::Rng;

use super::results::{
    LookupTreasureTypeData, LookupTreasureTypeResult, RollTreasureData, RollTreasureItemData,
    RollTreasureResult, TreasureListResult, TreasureTypeEntryData,
};

/// Internal rolled treasure entry used for CLI formatting and API projection.
#[derive(Debug, Clone)]
struct TreasureRoll {
    item_type: TreasureItemType,
    quantity: i32,
    values: Vec<u32>,
    total_gp: u32,
    restriction: Option<String>,
    note: Option<String>,
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

        Ok(base.saturating_mul(multiplier))
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

fn roll_treasure_entries(treasure_type: &TreasureTypeDef) -> Vec<TreasureRoll> {
    let mut rng = rand::thread_rng();
    let mut results = Vec::new();

    for entry in &treasure_type.entries {
        let roll: u32 = rng.gen_range(1..=100);
        if roll > entry.chance {
            continue;
        }

        let quantity = match parse_quantity_with_multiplier(&entry.quantity) {
            Ok(q) => q.max(1),
            Err(_) => continue,
        };

        let (values, total_gp) = match entry.item_type {
            TreasureItemType::Gems => {
                let result = treasure::roll_gems(quantity as u32);
                (result.values, result.total_gp)
            }
            TreasureItemType::Jewellery => {
                let result = treasure::roll_jewellery(quantity as u32);
                (result.values, result.total_gp)
            }
            _ => (Vec::new(), 0),
        };

        results.push(TreasureRoll {
            item_type: entry.item_type,
            quantity,
            values,
            total_gp,
            restriction: entry.restriction.clone(),
            note: entry.note.clone(),
        });
    }

    results
}

fn to_api_items(results: &[TreasureRoll]) -> (Vec<RollTreasureItemData>, f64) {
    let mut items = Vec::new();
    let mut total_gp = 0.0;

    for result in results {
        let mut item = RollTreasureItemData {
            item_type: result.item_type.name().to_string(),
            quantity: result.quantity,
            gp_value: None,
            values: None,
            total_gp: None,
            restriction: None,
            note: result.note.clone(),
        };

        match result.item_type {
            TreasureItemType::Cp => {
                let gp_value = result.quantity as f64 * 0.01;
                total_gp += gp_value;
                item.gp_value = Some(serde_json::json!(gp_value));
            }
            TreasureItemType::Sp => {
                let gp_value = result.quantity as f64 * 0.1;
                total_gp += gp_value;
                item.gp_value = Some(serde_json::json!(gp_value));
            }
            TreasureItemType::Ep => {
                let gp_value = result.quantity as f64 * 0.5;
                total_gp += gp_value;
                item.gp_value = Some(serde_json::json!(gp_value));
            }
            TreasureItemType::Gp => {
                total_gp += result.quantity as f64;
                item.gp_value = Some(serde_json::json!(result.quantity));
            }
            TreasureItemType::Pp => {
                let gp_value = result.quantity as f64 * 5.0;
                total_gp += gp_value;
                item.gp_value = Some(serde_json::json!(gp_value));
            }
            TreasureItemType::Gems | TreasureItemType::Jewellery => {
                total_gp += result.total_gp as f64;
                item.values = Some(result.values.clone());
                item.total_gp = Some(result.total_gp);
            }
            _ => {
                item.restriction = result.restriction.clone();
            }
        }

        items.push(item);
    }

    (items, total_gp)
}

fn format_treasure_haul(treasure_type: &TreasureTypeDef, results: &[TreasureRoll]) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "TREASURE TYPE {} ({})\n",
        treasure_type.letter,
        treasure_type.category.name()
    ));
    out.push_str(&format!("Average value: {} gp\n", treasure_type.average_gp));
    out.push_str("─────────────────────────────────\n");

    if results.is_empty() {
        out.push_str("Nothing found!\n");
        return out;
    }

    let mut coins_total_gp = 0.0;

    let coin_results: Vec<_> = results.iter().filter(|r| r.item_type.is_coin()).collect();
    if !coin_results.is_empty() {
        out.push_str("COINS:\n");
        for result in &coin_results {
            let (abbrev, gp_value) = match result.item_type {
                TreasureItemType::Cp => ("cp", 0.01),
                TreasureItemType::Sp => ("sp", 0.1),
                TreasureItemType::Ep => ("ep", 0.5),
                TreasureItemType::Gp => ("gp", 1.0),
                TreasureItemType::Pp => ("pp", 5.0),
                _ => unreachable!(),
            };
            let value = result.quantity as f64 * gp_value;
            coins_total_gp += value;
            out.push_str(&format!(
                "  {:>8} {} ({:.0} gp value)\n",
                result.quantity, abbrev, value
            ));
        }
    }

    let gem_results: Vec<_> = results
        .iter()
        .filter(|r| r.item_type == TreasureItemType::Gems)
        .collect();
    if !gem_results.is_empty() {
        out.push_str("GEMS:\n");
        for result in &gem_results {
            out.push_str(&format!(
                "  {} gems, {} gp total\n",
                result.quantity, result.total_gp
            ));
            if result.values.len() <= 12 {
                let values_str: Vec<String> =
                    result.values.iter().map(|v| format!("{}gp", v)).collect();
                out.push_str(&format!("    Values: {}\n", values_str.join(", ")));
            } else {
                let mut counts = std::collections::HashMap::new();
                for value in &result.values {
                    *counts.entry(*value).or_insert(0) += 1;
                }
                let mut summary: Vec<_> = counts.into_iter().collect();
                summary.sort_by_key(|(value, _)| *value);
                let summary_str: Vec<String> = summary
                    .iter()
                    .map(|(value, count)| format!("{}×{}gp", count, value))
                    .collect();
                out.push_str(&format!("    Breakdown: {}\n", summary_str.join(", ")));
            }
        }
    }

    let jewellery_results: Vec<_> = results
        .iter()
        .filter(|r| r.item_type == TreasureItemType::Jewellery)
        .collect();
    if !jewellery_results.is_empty() {
        out.push_str("JEWELLERY:\n");
        for result in &jewellery_results {
            out.push_str(&format!(
                "  {} pieces, {} gp total\n",
                result.quantity, result.total_gp
            ));
            if result.values.len() <= 12 {
                let values_str: Vec<String> =
                    result.values.iter().map(|v| format!("{}gp", v)).collect();
                out.push_str(&format!("    Values: {}\n", values_str.join(", ")));
            } else {
                let min = result.values.iter().min().unwrap_or(&0);
                let max = result.values.iter().max().unwrap_or(&0);
                out.push_str(&format!("    Range: {}gp - {}gp\n", min, max));
            }
        }
    }

    let magic_results: Vec<_> = results.iter().filter(|r| r.item_type.is_magic()).collect();
    if !magic_results.is_empty() {
        out.push_str("MAGIC ITEMS:\n");
        for result in &magic_results {
            let type_name = match result.item_type {
                TreasureItemType::MagicItems => "any magic item",
                TreasureItemType::MagicWeapon => "magic weapon/armour",
                TreasureItemType::Potions => "potion",
                TreasureItemType::Scrolls => "scroll",
                _ => "magic item",
            };
            out.push_str(&format!("  {} × {}", result.quantity, type_name));
            if let Some(note) = result.restriction.clone().or(result.note.clone()) {
                out.push_str(&format!(" ({})", note));
            }
            out.push('\n');
        }
        out.push_str("  (Use 'magic_item' command to generate specific items)\n");
    }

    out.push_str("─────────────────────────────────\n");
    let gems_gp: u32 = gem_results.iter().map(|r| r.total_gp).fold(0u32, u32::saturating_add);
    let jewellery_gp: u32 = jewellery_results.iter().map(|r| r.total_gp).fold(0u32, u32::saturating_add);
    let total_gp = coins_total_gp + gems_gp as f64 + jewellery_gp as f64;
    out.push_str(&format!("TOTAL VALUE: {:.0} gp\n", total_gp));
    if coins_total_gp > 0.0 {
        out.push_str(&format!("  Coins: {:.0} gp\n", coins_total_gp));
    }
    if gems_gp > 0 {
        out.push_str(&format!("  Gems: {} gp\n", gems_gp));
    }
    if jewellery_gp > 0 {
        out.push_str(&format!("  Jewellery: {} gp\n", jewellery_gp));
    }

    out
}

pub fn action_list_treasure_types() -> Result<TreasureListResult, EngineError> {
    let mut out = String::from("TREASURE TYPES\n");
    out.push_str("─────────────────────────────────\n");

    for category in [
        TreasureCategory::Hoard,
        TreasureCategory::Individual,
        TreasureCategory::Group,
    ] {
        out.push_str(&format!("\n{}:\n", category.name().to_uppercase()));
        for t in treasure::types_by_category(category) {
            out.push_str(&format!("  {} - avg {} gp\n", t.letter, t.average_gp));
        }
    }

    Ok(TreasureListResult { output: out })
}

pub fn action_lookup_treasure_type(letter: &str) -> Result<LookupTreasureTypeResult, EngineError> {
    let letter = letter.to_uppercase();
    let treasure_type = treasure::find_treasure_type(&letter).ok_or_else(|| {
        EngineError::InvalidInput(format!(
            "unknown treasure type '{}'. Valid types are A-V.",
            letter
        ))
    })?;

    let entries: Vec<TreasureTypeEntryData> = treasure_type
        .entries
        .iter()
        .map(|entry| TreasureTypeEntryData {
            chance: entry.chance,
            quantity: entry.quantity.clone(),
            item_type: entry.item_type.name().to_string(),
            restriction: entry.restriction.clone(),
            note: entry.note.clone(),
        })
        .collect();

    let mut message = format!(
        "treasure type {} ({}), avg {} gp.",
        treasure_type.letter,
        treasure_type.category.name(),
        treasure_type.average_gp
    );

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

    let mut contents = Vec::new();
    if has_coins {
        contents.push("coins");
    }
    if has_gems {
        contents.push("gems");
    }
    if has_jewellery {
        contents.push("jewellery");
    }
    if has_magic {
        contents.push("magic items");
    }
    if !contents.is_empty() {
        message.push_str(&format!(" May contain: {}.", contents.join(", ")));
    }

    Ok(LookupTreasureTypeResult {
        message,
        data: LookupTreasureTypeData {
            letter: treasure_type.letter.clone(),
            category: treasure_type.category.name().to_string(),
            average_gp: treasure_type.average_gp,
            entries,
        },
    })
}

pub fn action_roll_treasure(letter: &str) -> Result<RollTreasureResult, EngineError> {
    let letter = letter.to_uppercase();
    let treasure_type = treasure::find_treasure_type(&letter).ok_or_else(|| {
        EngineError::InvalidInput(format!("unknown treasure type '{}'.", letter))
    })?;

    let results = roll_treasure_entries(treasure_type);
    let cli_output = format_treasure_haul(treasure_type, &results);
    let (items, total_gp) = to_api_items(&results);

    let message = if items.is_empty() {
        format!("rolled on treasure type {}: nothing found.", letter)
    } else {
        format!(
            "rolled on treasure type {}: {} item(s), {:.0} gp total value.",
            letter,
            items.len(),
            total_gp
        )
    };

    Ok(RollTreasureResult {
        message,
        cli_output,
        data: RollTreasureData {
            letter,
            category: treasure_type.category.name().to_string(),
            items,
            total_gp,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_dice() {
        for _ in 0..20 {
            let result = parse_quantity_with_multiplier("1d6").unwrap();
            assert!((1..=6).contains(&result), "got {}", result);
        }
    }

    #[test]
    fn parse_plain_number() {
        let result = parse_quantity_with_multiplier("3").unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn parse_dice_with_multiplier() {
        for _ in 0..20 {
            let result = parse_quantity_with_multiplier("1d6 × 1000").unwrap();
            assert!((1000..=6000).contains(&result), "got {} for 1d6 × 1000", result);
        }
    }

    #[test]
    fn parse_dice_with_x_multiplier() {
        for _ in 0..20 {
            let result = parse_quantity_with_multiplier("1d6 x 1000").unwrap();
            assert!((1000..=6000).contains(&result), "got {} for 1d6 x 1000", result);
        }
    }

    #[test]
    fn parse_dice_with_asterisk() {
        for _ in 0..20 {
            let result = parse_quantity_with_multiplier("1d6*1000").unwrap();
            assert!((1000..=6000).contains(&result), "got {} for 1d6*1000", result);
        }
    }

    #[test]
    fn parse_2d6_multiplier() {
        for _ in 0..20 {
            let result = parse_quantity_with_multiplier("2d6 × 1000").unwrap();
            assert!((2000..=12000).contains(&result), "got {} for 2d6 × 1000", result);
        }
    }

    #[test]
    fn parse_number_with_multiplier() {
        let result = parse_quantity_with_multiplier("3 × 1000").unwrap();
        assert_eq!(result, 3000);
    }

    #[test]
    fn action_list_treasure_types_contains_categories() {
        let result = action_list_treasure_types().unwrap();
        assert!(result.output.contains("TREASURE TYPES"));
        assert!(result.output.contains("HOARD"));
        assert!(result.output.contains("INDIVIDUAL"));
        assert!(result.output.contains("GROUP"));
        assert!(result.output.contains("A -"));
    }

    #[test]
    fn action_lookup_treasure_type_a() {
        let result = action_lookup_treasure_type("A").unwrap();
        assert!(result.message.contains("treasure type A"));
        assert_eq!(result.data.letter, "A");
        assert_eq!(result.data.category, "Hoard");
        assert_eq!(result.data.average_gp, 18000.0);
        assert!(!result.data.entries.is_empty());
    }

    #[test]
    fn action_roll_treasure_type_p_always_has_items() {
        for _ in 0..10 {
            let result = action_roll_treasure("P").unwrap();
            assert!(
                !result.data.items.is_empty(),
                "Type P should always have treasure"
            );
        }
    }

    #[test]
    fn action_roll_treasure_type_a_cli_output() {
        let result = action_roll_treasure("A").unwrap();
        assert!(result.cli_output.contains("TREASURE TYPE A"));
        assert!(result.cli_output.contains("Hoard"));
        assert!(result.cli_output.contains("18000 gp"));
        assert!(
            result.cli_output.contains("TOTAL VALUE") || result.cli_output.contains("Nothing found")
        );
    }

    #[test]
    fn format_haul_empty() {
        let treasure_type = treasure::find_treasure_type("A").unwrap();
        let output = format_treasure_haul(treasure_type, &[]);
        assert!(output.contains("Nothing found"));
    }
}
