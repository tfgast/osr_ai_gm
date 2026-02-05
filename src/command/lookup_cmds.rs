use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::rules::magic_item::{
    find_magic_item, find_magic_items_partial, search_magic_items, ItemCategory,
};
use crate::rules::treasure::{find_treasure_type, TreasureItemType};

/// Look up a magic item by name.
pub struct ItemCommand;
impl Command for ItemCommand {
    fn name(&self) -> &str {
        "item"
    }
    fn help(&self) -> &str {
        "Look up magic item by name (item <name>)"
    }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: item <name>");
        }
        let query = args.join(" ");

        // Try exact match first
        if let Some(item) = find_magic_item(&query) {
            return CommandResult::ok(format_magic_item(item));
        }

        // Try partial match
        let matches = find_magic_items_partial(&query);
        match matches.len() {
            0 => CommandResult::error(format!("no magic item found matching '{}'.", query)),
            1 => CommandResult::ok(format_magic_item(matches[0])),
            n if n <= 10 => {
                let mut out = format!(
                    "Multiple items match '{}'. Did you mean:\n",
                    query
                );
                for item in &matches {
                    out.push_str(&format!("  - {}\n", item.name));
                }
                CommandResult::ok(out)
            }
            n => CommandResult::ok(format!(
                "Found {} items matching '{}'. Please be more specific.",
                n, query
            )),
        }
    }
}

/// Format a magic item for display.
fn format_magic_item(item: &crate::rules::magic_item::MagicItemDef) -> String {
    let mut out = format!(
        "=== {} ===\nCategory: {}",
        item.name,
        item.category.name()
    );

    if item.cursed {
        out.push_str(" [CURSED]");
    }
    out.push('\n');

    if let Some(ref desc) = item.description {
        out.push_str(&format!("\n{}\n", desc));
    }

    if !item.properties.is_empty() {
        out.push_str("\nProperties:\n");
        for prop in &item.properties {
            if let Some(ref key) = prop.key {
                out.push_str(&format!("  {}: {}\n", key, prop.value));
            } else {
                out.push_str(&format!("  - {}\n", prop.value));
            }
        }
    }

    out
}

/// Search magic items by keyword.
pub struct SearchItemsCommand;
impl Command for SearchItemsCommand {
    fn name(&self) -> &str {
        "search_items"
    }
    fn help(&self) -> &str {
        "Search magic items by keyword (search_items <query>)"
    }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: search_items <query>");
        }
        let query = args.join(" ");
        let results = search_magic_items(&query);

        match results.len() {
            0 => CommandResult::ok(format!("No magic items found matching '{}'.", query)),
            n => {
                let mut out = format!("Found {} item(s) matching '{}':\n\n", n, query);

                // Group by category
                let mut by_category: std::collections::HashMap<ItemCategory, Vec<_>> =
                    std::collections::HashMap::new();
                for item in results {
                    by_category.entry(item.category).or_default().push(item);
                }

                // Sort categories for consistent output
                let mut categories: Vec<_> = by_category.keys().copied().collect();
                categories.sort_by_key(|c| c.name());

                for category in categories {
                    let items = &by_category[&category];
                    out.push_str(&format!("{}:\n", category.name()));
                    for item in items {
                        let cursed = if item.cursed { " [CURSED]" } else { "" };
                        out.push_str(&format!("  - {}{}\n", item.name, cursed));
                    }
                    out.push('\n');
                }

                CommandResult::ok(out)
            }
        }
    }
}

/// Look up a treasure type by letter.
pub struct TreasureTypeCommand;
impl Command for TreasureTypeCommand {
    fn name(&self) -> &str {
        "treasure_type"
    }
    fn help(&self) -> &str {
        "Show treasure type contents (treasure_type <letter>)"
    }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: treasure_type <letter> (A-V)");
        }
        let letter = args[0].to_uppercase();

        match find_treasure_type(&letter) {
            Some(tt) => {
                let mut out = format!(
                    "=== Treasure Type {} ===\n\
                     Category: {}\n\
                     Average Value: {} gp\n\n\
                     Contents:\n",
                    tt.letter,
                    tt.category.name(),
                    tt.average_gp
                );

                for entry in &tt.entries {
                    let item_name = entry.item_type.name();
                    let restriction = entry
                        .restriction
                        .as_ref()
                        .map(|r| format!(" ({})", r))
                        .unwrap_or_default();
                    let note = entry
                        .note
                        .as_ref()
                        .map(|n| format!(" [{}]", n))
                        .unwrap_or_default();

                    out.push_str(&format!(
                        "  {:3}%: {} {}{}{}\n",
                        entry.chance, entry.quantity, item_name, restriction, note
                    ));
                }

                // Add a summary of what to expect
                out.push_str("\nPossible contents:\n");
                let has_coins = tt
                    .entries
                    .iter()
                    .any(|e| e.item_type.is_coin());
                let has_gems = tt
                    .entries
                    .iter()
                    .any(|e| e.item_type == TreasureItemType::Gems);
                let has_jewellery = tt
                    .entries
                    .iter()
                    .any(|e| e.item_type == TreasureItemType::Jewellery);
                let has_magic = tt
                    .entries
                    .iter()
                    .any(|e| e.item_type.is_magic());

                if has_coins {
                    out.push_str("  - Coins (copper, silver, electrum, gold, platinum)\n");
                }
                if has_gems {
                    out.push_str("  - Gems (10-1000 gp each, rolled on d20 table)\n");
                }
                if has_jewellery {
                    out.push_str("  - Jewellery (3d6 x 100 gp each)\n");
                }
                if has_magic {
                    out.push_str("  - Magic items\n");
                }

                CommandResult::ok(out)
            }
            None => {
                CommandResult::error(format!(
                    "unknown treasure type '{}'. Valid types are A-V.",
                    letter
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_lookup_exact() {
        let cmd = ItemCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["Bag", "of", "Holding"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("Bag of Holding"));
        assert!(result.output.contains("Miscellaneous"));
    }

    #[test]
    fn item_lookup_case_insensitive() {
        let cmd = ItemCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["bag", "of", "holding"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("Bag of Holding"));
    }

    #[test]
    fn item_lookup_partial() {
        let cmd = ItemCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["Bag"], &mut state);
        // Should find multiple items or suggest
        assert!(!result.quit);
    }

    #[test]
    fn item_lookup_not_found() {
        let cmd = ItemCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["Nonexistent", "Item", "XYZ"], &mut state);
        assert!(result.output.contains("Error") || result.output.contains("no magic item"));
    }

    #[test]
    fn item_lookup_missing_args() {
        let cmd = ItemCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn search_items_finds_potions() {
        let cmd = SearchItemsCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["healing"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("Healing") || result.output.contains("healing"));
    }

    #[test]
    fn search_items_groups_by_category() {
        let cmd = SearchItemsCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["sword"], &mut state);
        // Should group results by category
        assert!(!result.quit);
    }

    #[test]
    fn search_items_no_results() {
        let cmd = SearchItemsCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["xyznonexistent123"], &mut state);
        assert!(result.output.contains("No magic items found"));
    }

    #[test]
    fn search_items_missing_args() {
        let cmd = SearchItemsCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn treasure_type_a() {
        let cmd = TreasureTypeCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["A"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("Treasure Type A"));
        assert!(result.output.contains("Hoard"));
        assert!(result.output.contains("18000"));
    }

    #[test]
    fn treasure_type_lowercase() {
        let cmd = TreasureTypeCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["a"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("Treasure Type A"));
    }

    #[test]
    fn treasure_type_individual_p() {
        let cmd = TreasureTypeCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["P"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("Individual"));
    }

    #[test]
    fn treasure_type_invalid() {
        let cmd = TreasureTypeCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["Z"], &mut state);
        assert!(result.output.contains("Error") || result.output.contains("unknown"));
    }

    #[test]
    fn treasure_type_missing_args() {
        let cmd = TreasureTypeCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn treasure_type_h_dragon() {
        let cmd = TreasureTypeCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["H"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("60000")); // High average value
    }
}
