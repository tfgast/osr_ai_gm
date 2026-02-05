use super::{Command, CommandResult};
use crate::dice;
use crate::persist::GameState;
use crate::rules::treasure::{
    self, TreasureCategory, TreasureItemType, TreasureTypeDef,
};
use rand::Rng;

/// Parse a quantity string that may include a multiplier.
/// Supports formats like "1d6", "1d6 × 1000", "3", "1d4 × 10".
fn parse_quantity_with_multiplier(quantity: &str) -> Result<i32, String> {
    // Normalize the multiplication sign (× or x or *)
    let normalized = quantity
        .replace(['×', 'x'], "*")
        .replace(" ", "");

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

/// Result of rolling a single treasure entry.
#[derive(Debug)]
struct TreasureRollResult {
    item_type: TreasureItemType,
    quantity: i32,
    /// Individual values for gems/jewellery (empty for other types)
    values: Vec<u32>,
    /// Total value in GP (only for gems/jewellery)
    total_gp: u32,
    /// Note about the treasure (e.g., magic item restrictions)
    note: Option<String>,
}

/// Roll on a treasure type and generate the haul.
fn roll_treasure(treasure_type: &TreasureTypeDef) -> Vec<TreasureRollResult> {
    let mut rng = rand::thread_rng();
    let mut results = Vec::new();

    for entry in &treasure_type.entries {
        // Roll d100 to see if this entry appears
        let roll: u32 = rng.gen_range(1..=100);
        if roll > entry.chance {
            continue; // Didn't make the percentage roll
        }

        // Roll quantity
        let quantity = match parse_quantity_with_multiplier(&entry.quantity) {
            Ok(q) => q.max(1), // Ensure at least 1
            Err(_) => continue, // Skip bad entries
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

        let note = entry.restriction.clone().or_else(|| entry.note.clone());

        results.push(TreasureRollResult {
            item_type: entry.item_type,
            quantity,
            values,
            total_gp,
            note,
        });
    }

    results
}

/// Format the treasure haul for display.
fn format_treasure_haul(
    treasure_type: &TreasureTypeDef,
    results: &[TreasureRollResult],
) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "TREASURE TYPE {} ({})\n",
        treasure_type.letter,
        treasure_type.category.name()
    ));
    out.push_str(&format!(
        "Average value: {} gp\n",
        treasure_type.average_gp
    ));
    out.push_str("─────────────────────────────────\n");

    if results.is_empty() {
        out.push_str("Nothing found!\n");
        return out;
    }

    // Group results by type for nicer output
    let mut coins_total_gp = 0.0;

    // Coins section
    let coin_results: Vec<_> = results
        .iter()
        .filter(|r| r.item_type.is_coin())
        .collect();
    if !coin_results.is_empty() {
        out.push_str("COINS:\n");
        for r in &coin_results {
            let (abbrev, gp_value) = match r.item_type {
                TreasureItemType::Cp => ("cp", 0.01),
                TreasureItemType::Sp => ("sp", 0.1),
                TreasureItemType::Ep => ("ep", 0.5),
                TreasureItemType::Gp => ("gp", 1.0),
                TreasureItemType::Pp => ("pp", 5.0),
                _ => unreachable!(),
            };
            let value = r.quantity as f64 * gp_value;
            coins_total_gp += value;
            out.push_str(&format!(
                "  {:>8} {} ({:.0} gp value)\n",
                r.quantity, abbrev, value
            ));
        }
    }

    // Gems section
    let gem_results: Vec<_> = results
        .iter()
        .filter(|r| r.item_type == TreasureItemType::Gems)
        .collect();
    if !gem_results.is_empty() {
        out.push_str("GEMS:\n");
        for r in &gem_results {
            out.push_str(&format!("  {} gems, {} gp total\n", r.quantity, r.total_gp));
            // Show individual values if reasonable count
            if r.values.len() <= 12 {
                let values_str: Vec<String> =
                    r.values.iter().map(|v| format!("{}gp", v)).collect();
                out.push_str(&format!("    Values: {}\n", values_str.join(", ")));
            } else {
                // Summarize for large counts
                let mut counts = std::collections::HashMap::new();
                for v in &r.values {
                    *counts.entry(*v).or_insert(0) += 1;
                }
                let mut summary: Vec<_> = counts.into_iter().collect();
                summary.sort_by_key(|(v, _)| *v);
                let summary_str: Vec<String> = summary
                    .iter()
                    .map(|(v, c)| format!("{}×{}gp", c, v))
                    .collect();
                out.push_str(&format!("    Breakdown: {}\n", summary_str.join(", ")));
            }
        }
    }

    // Jewellery section
    let jewellery_results: Vec<_> = results
        .iter()
        .filter(|r| r.item_type == TreasureItemType::Jewellery)
        .collect();
    if !jewellery_results.is_empty() {
        out.push_str("JEWELLERY:\n");
        for r in &jewellery_results {
            out.push_str(&format!(
                "  {} pieces, {} gp total\n",
                r.quantity, r.total_gp
            ));
            // Show individual values
            if r.values.len() <= 12 {
                let values_str: Vec<String> =
                    r.values.iter().map(|v| format!("{}gp", v)).collect();
                out.push_str(&format!("    Values: {}\n", values_str.join(", ")));
            } else {
                let min = r.values.iter().min().unwrap_or(&0);
                let max = r.values.iter().max().unwrap_or(&0);
                out.push_str(&format!("    Range: {}gp - {}gp\n", min, max));
            }
        }
    }

    // Magic items section
    let magic_results: Vec<_> = results
        .iter()
        .filter(|r| r.item_type.is_magic())
        .collect();
    if !magic_results.is_empty() {
        out.push_str("MAGIC ITEMS:\n");
        for r in &magic_results {
            let type_name = match r.item_type {
                TreasureItemType::MagicItems => "any magic item",
                TreasureItemType::MagicWeapon => "magic weapon/armour",
                TreasureItemType::Potions => "potion",
                TreasureItemType::Scrolls => "scroll",
                _ => "magic item",
            };
            out.push_str(&format!("  {} × {}", r.quantity, type_name));
            if let Some(note) = &r.note {
                out.push_str(&format!(" ({})", note));
            }
            out.push('\n');
        }
        out.push_str("  (Use 'magic_item' command to generate specific items)\n");
    }

    // Total value summary
    out.push_str("─────────────────────────────────\n");
    let gems_gp: u32 = gem_results.iter().map(|r| r.total_gp).sum();
    let jewellery_gp: u32 = jewellery_results.iter().map(|r| r.total_gp).sum();
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

pub struct TreasureCommand;
impl Command for TreasureCommand {
    fn name(&self) -> &str {
        "treasure"
    }

    fn help(&self) -> &str {
        "Generate treasure from a treasure type (treasure <type> | treasure list)"
    }

    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error(
                "usage: treasure <type> (A-V) or treasure list",
            );
        }

        let arg = args[0].to_uppercase();

        // Handle "list" subcommand
        if arg == "LIST" {
            let mut out = String::from("TREASURE TYPES\n");
            out.push_str("─────────────────────────────────\n");

            for category in [
                TreasureCategory::Hoard,
                TreasureCategory::Individual,
                TreasureCategory::Group,
            ] {
                out.push_str(&format!("\n{}:\n", category.name().to_uppercase()));
                for t in treasure::types_by_category(category) {
                    out.push_str(&format!(
                        "  {} - avg {} gp\n",
                        t.letter, t.average_gp
                    ));
                }
            }
            return CommandResult::ok(out);
        }

        // Look up the treasure type
        let treasure_type = match treasure::find_treasure_type(&arg) {
            Some(t) => t,
            None => {
                return CommandResult::error(format!(
                    "unknown treasure type '{}'. Use A-V or 'treasure list'.",
                    arg
                ));
            }
        };

        // Roll on the treasure type
        let results = roll_treasure(treasure_type);
        let output = format_treasure_haul(treasure_type, &results);

        CommandResult::ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_dice() {
        // Run multiple times since it's random
        for _ in 0..20 {
            let result = parse_quantity_with_multiplier("1d6").unwrap();
            assert!(result >= 1 && result <= 6, "got {}", result);
        }
    }

    #[test]
    fn parse_plain_number() {
        let result = parse_quantity_with_multiplier("3").unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn parse_dice_with_multiplier() {
        // 1d6 × 1000 should be 1000-6000
        for _ in 0..20 {
            let result = parse_quantity_with_multiplier("1d6 × 1000").unwrap();
            assert!(
                result >= 1000 && result <= 6000,
                "got {} for 1d6 × 1000",
                result
            );
        }
    }

    #[test]
    fn parse_dice_with_x_multiplier() {
        // Also support lowercase x
        for _ in 0..20 {
            let result = parse_quantity_with_multiplier("1d6 x 1000").unwrap();
            assert!(
                result >= 1000 && result <= 6000,
                "got {} for 1d6 x 1000",
                result
            );
        }
    }

    #[test]
    fn parse_dice_with_asterisk() {
        for _ in 0..20 {
            let result = parse_quantity_with_multiplier("1d6*1000").unwrap();
            assert!(
                result >= 1000 && result <= 6000,
                "got {} for 1d6*1000",
                result
            );
        }
    }

    #[test]
    fn parse_2d6_multiplier() {
        for _ in 0..20 {
            let result = parse_quantity_with_multiplier("2d6 × 1000").unwrap();
            assert!(
                result >= 2000 && result <= 12000,
                "got {} for 2d6 × 1000",
                result
            );
        }
    }

    #[test]
    fn parse_number_with_multiplier() {
        let result = parse_quantity_with_multiplier("3 × 1000").unwrap();
        assert_eq!(result, 3000);
    }

    #[test]
    fn treasure_command_list() {
        let cmd = TreasureCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["list"], &mut state);
        assert!(!result.quit);
        assert!(result.output.contains("TREASURE TYPES"));
        assert!(result.output.contains("HOARD"));
        assert!(result.output.contains("INDIVIDUAL"));
        assert!(result.output.contains("GROUP"));
        assert!(result.output.contains("A -"));
    }

    #[test]
    fn treasure_command_type_a() {
        let cmd = TreasureCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["A"], &mut state);
        assert!(!result.quit);
        assert!(result.output.contains("TREASURE TYPE A"));
        assert!(result.output.contains("Hoard"));
        assert!(result.output.contains("18000 gp"));
        // Either has treasure or "Nothing found"
        assert!(
            result.output.contains("TOTAL VALUE")
                || result.output.contains("Nothing found")
        );
    }

    #[test]
    fn treasure_command_lowercase() {
        let cmd = TreasureCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["a"], &mut state);
        assert!(!result.quit);
        assert!(result.output.contains("TREASURE TYPE A"));
    }

    #[test]
    fn treasure_command_individual_p() {
        let cmd = TreasureCommand;
        let mut state = GameState::new();
        // Type P is individual treasure - 100% chance of 3d8 cp
        let result = cmd.execute(&["P"], &mut state);
        assert!(!result.quit);
        assert!(result.output.contains("TREASURE TYPE P"));
        assert!(result.output.contains("Individual"));
        // Should always have copper pieces (100% chance)
        assert!(result.output.contains("cp"));
    }

    #[test]
    fn treasure_command_unknown_type() {
        let cmd = TreasureCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["Z"], &mut state);
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("unknown treasure type"));
    }

    #[test]
    fn treasure_command_no_args() {
        let cmd = TreasureCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("usage"));
    }

    #[test]
    fn roll_treasure_type_p_always_has_cp() {
        // Type P has 100% chance of 3d8 cp
        let treasure_type = treasure::find_treasure_type("P").unwrap();
        for _ in 0..10 {
            let results = roll_treasure(treasure_type);
            assert!(!results.is_empty(), "Type P should always have treasure");
            assert!(
                results.iter().any(|r| r.item_type == TreasureItemType::Cp),
                "Type P should always have copper pieces"
            );
        }
    }

    #[test]
    fn roll_treasure_gems_have_values() {
        // Type L has 50% gems only
        let treasure_type = treasure::find_treasure_type("L").unwrap();
        // Run until we get gems (may take a few tries due to 50% chance)
        for _ in 0..50 {
            let results = roll_treasure(treasure_type);
            if let Some(gem_result) = results
                .iter()
                .find(|r| r.item_type == TreasureItemType::Gems)
            {
                assert!(!gem_result.values.is_empty());
                assert!(gem_result.total_gp > 0);
                // Each gem should be a valid value
                for v in &gem_result.values {
                    assert!(
                        *v == 10 || *v == 50 || *v == 100 || *v == 500 || *v == 1000,
                        "Invalid gem value: {}",
                        v
                    );
                }
                return;
            }
        }
        // It's statistically very unlikely to not get gems in 50 tries at 50% chance
        panic!("Never got gems from Type L in 50 attempts");
    }

    #[test]
    fn format_haul_empty() {
        let treasure_type = treasure::find_treasure_type("A").unwrap();
        let output = format_treasure_haul(treasure_type, &[]);
        assert!(output.contains("Nothing found"));
    }
}
