use super::{Command, CommandResult};
use crate::engine::lookup;
use crate::persist::GameState;

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
        match lookup::action_lookup_item(&query) {
            Ok(result) => CommandResult::ok(result.cli_output()),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
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
        match lookup::action_search_items(&query) {
            Ok(result) => CommandResult::ok(result.cli_output()),
            Err(e) => CommandResult::error(e.to_string()),
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
        match lookup::action_lookup_treasure_type(args[0]) {
            Ok(result) => CommandResult::ok(result.cli_output()),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

/// Look up a spell by name and optional list.
pub struct SpellCommand;
impl Command for SpellCommand {
    fn name(&self) -> &str {
        "spell"
    }
    fn help(&self) -> &str {
        "Look up spell by name (spell <name> [list])"
    }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error(
                "usage: spell <name> [list]\n  \
                 Lists: cleric, magicuser (or mu/mage), druid, illusionist",
            );
        }

        // Check if last arg is a spell list name
        let (name_args, list) = if args.len() >= 2 {
            match lookup::parse_spell_list(args[args.len() - 1]) {
                Some(l) => (&args[..args.len() - 1], Some(l)),
                None => (args, None),
            }
        } else {
            (args, None)
        };

        let query = name_args.join(" ");

        match lookup::action_lookup_spell(&query, list) {
            Ok(result) => CommandResult::ok(result.cli_output()),
            Err(e) => {
                if e.to_string().contains("not found") {
                    let list_hint = list
                        .map(|l| format!(" in {} list", l.name()))
                        .unwrap_or_default();
                    return CommandResult::error(format!(
                        "spell '{}' not found{}.",
                        query, list_hint
                    ));
                }
                CommandResult::error(e.to_string())
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

    #[test]
    fn spell_lookup_exact() {
        let cmd = SpellCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["Magic", "Missile"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("Magic Missile"));
        assert!(result.output.contains("Magic-User"));
        assert!(result.output.contains("Level 1"));
    }

    #[test]
    fn spell_lookup_case_insensitive() {
        let cmd = SpellCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["magic", "missile"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("Magic Missile"));
    }

    #[test]
    fn spell_lookup_with_list_filter() {
        let cmd = SpellCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["Cure", "Light", "Wounds", "cleric"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("Cure Light Wounds"));
        assert!(result.output.contains("Cleric"));
    }

    #[test]
    fn spell_lookup_with_mu_alias() {
        let cmd = SpellCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["Sleep", "mu"], &mut state);
        assert!(!result.output.contains("Error"), "{}", result.output);
        assert!(result.output.contains("Sleep"));
    }

    #[test]
    fn spell_lookup_not_found() {
        let cmd = SpellCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["Nonexistent", "Spell", "XYZ"], &mut state);
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("not found"));
    }

    #[test]
    fn spell_lookup_missing_args() {
        let cmd = SpellCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
    }
}
