pub mod party;
pub mod combat_cmds;
pub mod exploration_cmds;
pub mod encounter_cmds;
pub mod inventory_cmds;
pub mod lookup_cmds;
pub mod retainer_cmds;
pub mod wilderness_cmds;
pub mod treasure_cmds;
pub mod gm_cmds;
pub mod module_cmds;
pub mod system;

use std::collections::HashMap;
use crate::persist::GameState;

/// Result of executing a command.
#[derive(Debug)]
pub struct CommandResult {
    pub output: String,
    pub success: bool,
    pub quit: bool,
}

impl CommandResult {
    pub fn ok(output: impl Into<String>) -> Self {
        CommandResult {
            output: output.into(),
            success: true,
            quit: false,
        }
    }

    pub fn quit() -> Self {
        CommandResult {
            output: "Goodbye.".to_string(),
            success: true,
            quit: true,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        CommandResult {
            output: format!("Error: {}", msg.into()),
            success: false,
            quit: false,
        }
    }
}

/// A command that can be executed in the CLI shell.
pub trait Command {
    /// The command name (what the user types).
    fn name(&self) -> &str;

    /// Brief help text.
    fn help(&self) -> &str;

    /// Execute the command with the given arguments and mutable game state.
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult;
}

/// Resolve common aliases to canonical command names.
fn resolve_alias(name: &str) -> &str {
    match name {
        "look" | "l" => "exploration_status",
        "go" => "move",
        "status" => "exploration_status",
        "inventory" | "i" => "party",
        "examine" => "party",
        "take" | "grab" | "pick" => "loot",
        "level_up" | "levelup" => "train",
        _ => name,
    }
}

/// Registry that maps command names to command implementations.
pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        CommandRegistry {
            commands: HashMap::new(),
        }
    }

    pub fn register(&mut self, cmd: Box<dyn Command>) {
        self.commands.insert(cmd.name().to_string(), cmd);
    }

    pub fn dispatch(&self, name: &str, args: &[&str], state: &mut GameState) -> CommandResult {
        if name == "help" {
            let mut out = String::from("Available commands:\n");
            for (cmd_name, help) in self.commands() {
                out.push_str(&format!("  {:18} {}\n", cmd_name, help));
            }
            return CommandResult::ok(out);
        }
        let name = resolve_alias(name);
        let (result, category) = match self.commands.get(name) {
            Some(cmd) => (cmd.execute(args, state), "command_error"),
            None => (
                CommandResult::error(format!(
                    "unknown command: '{}'. Type 'help' for commands.",
                    name
                )),
                "unknown_command",
            ),
        };

        if !result.success {
            crate::telemetry::log_failed_command(&crate::telemetry::FailedCommand {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                raw_input: crate::telemetry::reconstruct_input(name, args),
                category,
                error_message: result.output.clone(),
                game_mode: state.mode.to_string(),
            });
        }

        result
    }

    pub fn commands(&self) -> Vec<(&str, &str)> {
        let mut cmds: Vec<(&str, &str)> = self.commands.values()
            .map(|c| (c.name(), c.help()))
            .collect();
        cmds.sort_by_key(|(name, _)| *name);
        cmds
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCmd;
    impl Command for TestCmd {
        fn name(&self) -> &str { "test" }
        fn help(&self) -> &str { "a test command" }
        fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
            CommandResult::ok(format!("args: {:?}", args))
        }
    }

    #[test]
    fn registry_dispatch() {
        let mut reg = CommandRegistry::new();
        reg.register(Box::new(TestCmd));
        let mut state = GameState::new();
        let result = reg.dispatch("test", &["a", "b"], &mut state);
        assert!(!result.quit);
        assert!(result.output.contains("a"));
    }

    #[test]
    fn unknown_command() {
        let reg = CommandRegistry::new();
        let mut state = GameState::new();
        let result = reg.dispatch("nope", &[], &mut state);
        assert!(result.output.contains("unknown command"));
    }

    #[test]
    fn unknown_command_is_error() {
        let reg = CommandRegistry::new();
        let mut state = GameState::new();
        let result = reg.dispatch("xyzzy", &[], &mut state);
        assert!(!result.success);
        assert!(result.output.starts_with("Error: "));
    }

    #[test]
    fn successful_command_has_success_true() {
        let mut reg = CommandRegistry::new();
        reg.register(Box::new(TestCmd));
        let mut state = GameState::new();
        let result = reg.dispatch("test", &[], &mut state);
        assert!(result.success);
    }

    #[test]
    fn alias_look_resolves() {
        assert_eq!(resolve_alias("look"), "exploration_status");
        assert_eq!(resolve_alias("l"), "exploration_status");
    }

    #[test]
    fn alias_go_resolves() {
        assert_eq!(resolve_alias("go"), "move");
    }

    #[test]
    fn alias_inventory_resolves() {
        assert_eq!(resolve_alias("inventory"), "party");
        assert_eq!(resolve_alias("i"), "party");
        assert_eq!(resolve_alias("examine"), "party");
    }

    #[test]
    fn alias_take_resolves() {
        assert_eq!(resolve_alias("take"), "loot");
        assert_eq!(resolve_alias("grab"), "loot");
        assert_eq!(resolve_alias("pick"), "loot");
    }

    #[test]
    fn alias_status_resolves() {
        assert_eq!(resolve_alias("status"), "exploration_status");
    }

    #[test]
    fn non_alias_passes_through() {
        assert_eq!(resolve_alias("attack"), "attack");
        assert_eq!(resolve_alias("chargen"), "chargen");
    }
}
