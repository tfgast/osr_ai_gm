use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::{dice, persist};

pub struct RollCommand;
impl Command for RollCommand {
    fn name(&self) -> &str { "roll" }
    fn help(&self) -> &str { "Roll dice (e.g., roll 2d6+3, roll d%, roll 3-in-6)" }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: roll <dice expression>");
        }
        let notation = args.join("");
        match dice::roll_str(&notation) {
            Ok(result) => CommandResult::ok(format!("{}", result)),
            Err(e) => CommandResult::error(format!("{}", e)),
        }
    }
}

pub struct SaveCommand;
impl Command for SaveCommand {
    fn name(&self) -> &str { "save" }
    fn help(&self) -> &str { "Save game state (e.g., save game.json)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let path = args.first().copied().unwrap_or("save.json");
        match persist::save(state, std::path::Path::new(path)) {
            Ok(()) => CommandResult::ok(format!("Game saved to {}", path)),
            Err(e) => CommandResult::error(format!("save failed: {}", e)),
        }
    }
}

pub struct LoadCommand;
impl Command for LoadCommand {
    fn name(&self) -> &str { "load" }
    fn help(&self) -> &str { "Load game state (e.g., load game.json)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let path = args.first().copied().unwrap_or("save.json");
        match persist::load(std::path::Path::new(path)) {
            Ok(loaded) => {
                let msg = format!(
                    "Loaded: turn {}, dungeon level {}, {} party members{}",
                    loaded.turn(), loaded.dungeon_level, loaded.party.members.len(),
                    if loaded.combat.is_some() { ", combat active" } else { "" }
                );
                *state = loaded;
                CommandResult::ok(msg)
            }
            Err(e) => CommandResult::error(format!("load failed: {}", e)),
        }
    }
}

pub struct HelpCommand {
    pub commands: Vec<(String, String)>,
}

impl Command for HelpCommand {
    fn name(&self) -> &str { "help" }
    fn help(&self) -> &str { "Show available commands" }
    fn execute(&self, _args: &[&str], _state: &mut GameState) -> CommandResult {
        let mut out = String::from("Available commands:\n");
        for (name, help) in &self.commands {
            out.push_str(&format!("  {:18} {}\n", name, help));
        }
        CommandResult::ok(out)
    }
}

pub struct QuitCommand;
impl Command for QuitCommand {
    fn name(&self) -> &str { "quit" }
    fn help(&self) -> &str { "Exit the game" }
    fn execute(&self, _args: &[&str], _state: &mut GameState) -> CommandResult {
        CommandResult::quit()
    }
}
