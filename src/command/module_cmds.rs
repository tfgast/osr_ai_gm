use super::{Command, CommandResult};
use crate::engine::module as module_engine;
use crate::persist::GameState;

/// Load a prewritten adventure module from a JSON file.
pub struct LoadModuleCommand;

impl Command for LoadModuleCommand {
    fn name(&self) -> &str {
        "load_module"
    }

    fn help(&self) -> &str {
        "Load an adventure module (load_module <path>)"
    }

    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: load_module <path>");
        }

        let path = args.join(" ");
        match module_engine::action_load_module(state, &path) {
            Ok(result) => CommandResult::ok(format!(
                "{}\n\
                 Use 'light torch <carrier>' or 'light lantern <carrier>' to light the way.\n\
                 Use 'exploration_status' to see current position.",
                result.message
            )),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::game::GameMode;

    #[test]
    fn load_module_requires_path() {
        let command = LoadModuleCommand;
        let mut state = GameState::new();
        let result = command.execute(&[], &mut state);
        assert!(!result.success);
        assert!(result.output.contains("usage: load_module <path>"));
    }

    #[test]
    fn load_module_success() {
        let command = LoadModuleCommand;
        let mut state = GameState::new();
        let result = command.execute(&["data/modules/sample_crypt/module.json"], &mut state);
        assert!(result.success, "load_module failed: {}", result.output);
        assert_eq!(state.mode, GameMode::Exploration);
        assert!(state.dungeon.is_some());
    }

    #[test]
    fn load_module_includes_onboarding_text() {
        let command = LoadModuleCommand;
        let mut state = GameState::new();
        let result = command.execute(&["data/modules/sample_crypt/module.json"], &mut state);
        assert!(result.success, "load_module failed: {}", result.output);
        assert!(
            result.output.contains("light torch"),
            "missing torch onboarding hint"
        );
        assert!(
            result.output.contains("exploration_status"),
            "missing exploration_status onboarding hint"
        );
    }
}
