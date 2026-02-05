pub mod command;
pub mod dice;
pub mod engine;
pub mod model;
pub mod persist;
pub mod rules;
pub mod state;
pub mod telemetry;

use command::CommandRegistry;
use command::party::*;
use command::combat_cmds::*;
use command::exploration_cmds::*;
use command::encounter_cmds::*;
use command::inventory_cmds::*;
use command::lookup_cmds::*;
use command::retainer_cmds::*;
use command::wilderness_cmds::*;
use command::treasure_cmds::TreasureCommand;
use command::gm_cmds::{AdvanceTurnCommand, AwardXpCommand, RulingCommand, HealCommand, DamageCommand, SetHpCommand, SetRationsCommand, AddRationsCommand};
use command::module_cmds::LoadModuleCommand;
use command::system::*;
use persist::GameState;
use std::io::{self, BufRead, Write};

/// Parse a command line into arguments, respecting quoted strings.
/// Supports both single and double quotes. Quotes can be escaped with backslash.
fn parse_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';
    let mut has_content = false; // Track if we've seen any content (including empty quotes)
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' if in_quotes => {
                // Handle escaped characters inside quotes
                if let Some(&next) = chars.peek() {
                    if next == quote_char || next == '\\' {
                        current.push(chars.next().unwrap());
                    } else {
                        current.push(c);
                    }
                } else {
                    current.push(c);
                }
            }
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = c;
                has_content = true; // Opening a quote means we have an arg (even if empty)
            }
            c if in_quotes && c == quote_char => {
                in_quotes = false;
            }
            ' ' | '\t' if !in_quotes => {
                if has_content {
                    args.push(std::mem::take(&mut current));
                    has_content = false;
                }
            }
            _ => {
                current.push(c);
                has_content = true;
            }
        }
    }

    if has_content {
        args.push(current);
    }

    args
}

fn build_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    // Character & Party
    registry.register(Box::new(ChargenCommand));
    registry.register(Box::new(ClassesCommand));
    registry.register(Box::new(EligibleCommand));
    registry.register(Box::new(PartyCommand));
    // Combat
    registry.register(Box::new(StartCombatCommand));
    registry.register(Box::new(InitiativeCommand));
    registry.register(Box::new(AttackCommand));
    registry.register(Box::new(MonsterAttackCommand));
    registry.register(Box::new(MoraleCommand));
    registry.register(Box::new(TurnUndeadCommand));
    registry.register(Box::new(CloseCommand));
    registry.register(Box::new(RetreatCommand));
    registry.register(Box::new(WithdrawalCommand));
    registry.register(Box::new(DeclareSpellCommand));
    registry.register(Box::new(CombatStatusCommand));
    registry.register(Box::new(CombatLogCommand));
    registry.register(Box::new(EndCombatCommand));
    registry.register(Box::new(SetHelplessCommand));
    registry.register(Box::new(KillCommand));
    // Inventory
    registry.register(Box::new(BuyCommand));
    registry.register(Box::new(DropCommand));
    registry.register(Box::new(LootCommand));
    registry.register(Box::new(EquipCommand));
    // Dungeon Exploration
    registry.register(Box::new(EnterDungeonCommand));
    registry.register(Box::new(LightCommand));
    registry.register(Box::new(ExploreCommand));
    registry.register(Box::new(SearchCommand));
    registry.register(Box::new(ListenCommand));
    registry.register(Box::new(ForceDoorCommand));
    registry.register(Box::new(AddRoomCommand));
    registry.register(Box::new(AddDoorCommand));
    registry.register(Box::new(MoveRoomCommand));
    registry.register(Box::new(RestCommand));
    registry.register(Box::new(ExplorationStatusCommand));
    registry.register(Box::new(LoadModuleCommand));
    // Encounter
    registry.register(Box::new(EncounterCommand));
    registry.register(Box::new(SurpriseCommand));
    registry.register(Box::new(ReactionCommand));
    registry.register(Box::new(EvadeCommand));
    // Treasure
    registry.register(Box::new(TreasureCommand));
    // Wilderness
    registry.register(Box::new(EnterWildernessCommand));
    registry.register(Box::new(AddHexCommand));
    registry.register(Box::new(TravelCommand));
    registry.register(Box::new(ForageCommand));
    registry.register(Box::new(HuntCommand));
    registry.register(Box::new(OrientCommand));
    registry.register(Box::new(WildernessStatusCommand));
    // Retainers
    registry.register(Box::new(HireCommand));
    registry.register(Box::new(RetainersCommand));
    registry.register(Box::new(DismissCommand));
    registry.register(Box::new(RetainerMoraleCommand));
    // GM
    registry.register(Box::new(AdvanceTurnCommand));
    registry.register(Box::new(AwardXpCommand));
    registry.register(Box::new(RulingCommand));
    registry.register(Box::new(HealCommand));
    registry.register(Box::new(DamageCommand));
    registry.register(Box::new(SetHpCommand));
    registry.register(Box::new(SetRationsCommand));
    registry.register(Box::new(AddRationsCommand));
    // Notes
    registry.register(Box::new(NoteCommand));
    registry.register(Box::new(NotesCommand));
    registry.register(Box::new(NoteDeleteCommand));
    // Lookup
    registry.register(Box::new(ItemCommand));
    registry.register(Box::new(SearchItemsCommand));
    registry.register(Box::new(TreasureTypeCommand));
    // System
    registry.register(Box::new(RollCommand));
    registry.register(Box::new(SaveCommand));
    registry.register(Box::new(LoadCommand));
    registry.register(Box::new(HelpCommand));
    registry.register(Box::new(QuitCommand));
    registry
}

fn main() {
    println!("OSR AI Game Master v{}", env!("CARGO_PKG_VERSION"));
    println!("Type 'help' for available commands, 'quit' to exit.\n");

    let registry = build_registry();
    let mut state = GameState::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            print!("> ");
            let _ = stdout.flush();
            continue;
        }

        let parts = parse_args(trimmed);
        if parts.is_empty() {
            print!("> ");
            let _ = stdout.flush();
            continue;
        }
        let cmd_name = &parts[0];
        let args: Vec<&str> = parts[1..].iter().map(|s| s.as_str()).collect();

        let result = registry.dispatch(cmd_name, &args, &mut state);
        println!("{}", result.output);

        if result.quit {
            break;
        }

        print!("> ");
        let _ = stdout.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_simple() {
        let args = parse_args("chargen Grond Fighter Lawful");
        assert_eq!(args, vec!["chargen", "Grond", "Fighter", "Lawful"]);
    }

    #[test]
    fn parse_args_double_quotes() {
        let args = parse_args(r#"chargen "Brother Marcus" Cleric Lawful"#);
        assert_eq!(args, vec!["chargen", "Brother Marcus", "Cleric", "Lawful"]);
    }

    #[test]
    fn parse_args_single_quotes() {
        let args = parse_args("chargen 'Brother Marcus' Cleric Lawful");
        assert_eq!(args, vec!["chargen", "Brother Marcus", "Cleric", "Lawful"]);
    }

    #[test]
    fn parse_args_mixed_quotes() {
        let args = parse_args(r#"chargen "Sir O'Brien" Fighter Lawful"#);
        assert_eq!(args, vec!["chargen", "Sir O'Brien", "Fighter", "Lawful"]);
    }

    #[test]
    fn parse_args_escaped_quote() {
        let args = parse_args(r#"note "The sign says \"Beware!\"""#);
        assert_eq!(args, vec!["note", r#"The sign says "Beware!""#]);
    }

    #[test]
    fn parse_args_empty() {
        let args = parse_args("");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_args_whitespace_only() {
        let args = parse_args("   ");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_args_extra_whitespace() {
        let args = parse_args("  chargen   Grond   Fighter  ");
        assert_eq!(args, vec!["chargen", "Grond", "Fighter"]);
    }

    #[test]
    fn parse_args_empty_quotes() {
        let args = parse_args(r#"chargen "" Fighter"#);
        assert_eq!(args, vec!["chargen", "", "Fighter"]);
    }

    #[test]
    fn parse_args_adjacent_to_quotes() {
        // "prefix"suffix should become prefixsuffix
        let args = parse_args(r#"chargen pre"fix suf"fix Fighter"#);
        assert_eq!(args, vec!["chargen", "prefix suffix", "Fighter"]);
    }
}
