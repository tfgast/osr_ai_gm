use super::{Command, CommandResult};
use crate::engine::rumor;
use crate::persist::GameState;

/// Roll a random rumor from a named table.
pub struct RumorCommand;
impl Command for RumorCommand {
    fn name(&self) -> &str {
        "rumor"
    }

    fn help(&self) -> &str {
        "Roll a random rumor (rumor <table> | rumor list | rumor show <table>)"
    }

    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error(
                "usage: rumor <table> | rumor list | rumor show <table>",
            );
        }

        let subcommand = args[0].to_lowercase();

        if subcommand == "list" {
            return match rumor::action_list_rumor_tables() {
                Ok(result) => {
                    let mut out = String::from("RUMOR TABLES\n");
                    out.push_str("─────────────────────────────────\n");
                    for t in &result.tables {
                        out.push_str(&format!("  {} — {} rumors", t.name, t.entry_count));
                        if let Some(desc) = &t.description {
                            out.push_str(&format!(" ({})", desc));
                        }
                        out.push('\n');
                    }
                    CommandResult::ok(out)
                }
                Err(e) => CommandResult::error(e.to_string()),
            };
        }

        if subcommand == "show" {
            if args.len() < 2 {
                return CommandResult::error("usage: rumor show <table>");
            }
            let table_name = args[1];
            return match rumor::action_lookup_rumor_table(table_name) {
                Ok(result) => CommandResult::ok(result.cli_output),
                Err(e) => CommandResult::error(e.to_string()),
            };
        }

        // Default: roll a rumor from the named table
        match rumor::action_roll_rumor(&subcommand) {
            Ok(result) => CommandResult::ok(result.cli_output),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rumor_command_list() {
        let cmd = RumorCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["list"], &mut state);
        assert!(result.success);
        assert!(result.output.contains("RUMOR TABLES"));
        assert!(result.output.contains("tavern"));
        assert!(result.output.contains("market"));
        assert!(result.output.contains("docks"));
    }

    #[test]
    fn rumor_command_roll_tavern() {
        let cmd = RumorCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["tavern"], &mut state);
        assert!(result.success);
        assert!(result.output.contains("RUMOR"));
    }

    #[test]
    fn rumor_command_roll_case_insensitive() {
        let cmd = RumorCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["TAVERN"], &mut state);
        assert!(result.success);
        assert!(result.output.contains("RUMOR"));
    }

    #[test]
    fn rumor_command_show_tavern() {
        let cmd = RumorCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["show", "tavern"], &mut state);
        assert!(result.success);
        assert!(result.output.contains("RUMOR TABLE: TAVERN"));
    }

    #[test]
    fn rumor_command_unknown_table() {
        let cmd = RumorCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["nonexistent"], &mut state);
        assert!(!result.success);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn rumor_command_no_args() {
        let cmd = RumorCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&[], &mut state);
        assert!(!result.success);
        assert!(result.output.contains("usage"));
    }

    #[test]
    fn rumor_command_show_no_table() {
        let cmd = RumorCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["show"], &mut state);
        assert!(!result.success);
        assert!(result.output.contains("usage"));
    }
}
