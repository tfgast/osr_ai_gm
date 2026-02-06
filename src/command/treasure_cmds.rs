use super::{Command, CommandResult};
use crate::engine::result::EngineError;
use crate::engine::treasure;
use crate::persist::GameState;

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
            return CommandResult::error("usage: treasure <type> (A-V) or treasure list");
        }

        let arg = args[0].to_uppercase();
        if arg == "LIST" {
            return match treasure::action_list_treasure_types() {
                Ok(result) => CommandResult::ok(result.output),
                Err(e) => CommandResult::error(e.to_string()),
            };
        }

        match treasure::action_roll_treasure(&arg) {
            Ok(result) => CommandResult::ok(result.cli_output),
            Err(EngineError::InvalidInput(msg)) if msg.starts_with("unknown treasure type '") => {
                CommandResult::error(format!(
                    "unknown treasure type '{}'. Use A-V or 'treasure list'.",
                    arg
                ))
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(
            result.output.contains("TOTAL VALUE") || result.output.contains("Nothing found")
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
        let result = cmd.execute(&["P"], &mut state);
        assert!(!result.quit);
        assert!(result.output.contains("TREASURE TYPE P"));
        assert!(result.output.contains("Individual"));
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
}
