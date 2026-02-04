pub mod command;
pub mod dice;
pub mod model;
pub mod persist;

use command::{Command, CommandRegistry, CommandResult};
use std::io::{self, BufRead, Write};

// --- Built-in commands ---

struct RollCommand;
impl Command for RollCommand {
    fn name(&self) -> &str { "roll" }
    fn help(&self) -> &str { "Roll dice (e.g., roll 2d6+3, roll d%, roll 3-in-6)" }
    fn execute(&self, args: &[&str]) -> CommandResult {
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

struct HelpCommand {
    commands: Vec<(String, String)>,
}

impl Command for HelpCommand {
    fn name(&self) -> &str { "help" }
    fn help(&self) -> &str { "Show available commands" }
    fn execute(&self, _args: &[&str]) -> CommandResult {
        let mut out = String::from("Available commands:\n");
        for (name, help) in &self.commands {
            out.push_str(&format!("  {:12} {}\n", name, help));
        }
        CommandResult::ok(out)
    }
}

struct QuitCommand;
impl Command for QuitCommand {
    fn name(&self) -> &str { "quit" }
    fn help(&self) -> &str { "Exit the game" }
    fn execute(&self, _args: &[&str]) -> CommandResult {
        CommandResult::quit()
    }
}

struct SaveCommand;
impl Command for SaveCommand {
    fn name(&self) -> &str { "save" }
    fn help(&self) -> &str { "Save game state (e.g., save game.json)" }
    fn execute(&self, args: &[&str]) -> CommandResult {
        let path = args.first().copied().unwrap_or("save.json");
        let state = persist::GameState::new();
        match persist::save(&state, std::path::Path::new(path)) {
            Ok(()) => CommandResult::ok(format!("Game saved to {}", path)),
            Err(e) => CommandResult::error(format!("save failed: {}", e)),
        }
    }
}

struct LoadCommand;
impl Command for LoadCommand {
    fn name(&self) -> &str { "load" }
    fn help(&self) -> &str { "Load game state (e.g., load game.json)" }
    fn execute(&self, args: &[&str]) -> CommandResult {
        let path = args.first().copied().unwrap_or("save.json");
        match persist::load(std::path::Path::new(path)) {
            Ok(state) => CommandResult::ok(format!(
                "Loaded: turn {}, dungeon level {}, {} party members",
                state.turn, state.dungeon_level, state.party.members.len()
            )),
            Err(e) => CommandResult::error(format!("load failed: {}", e)),
        }
    }
}

fn build_registry() -> CommandRegistry {
    // Collect command info for help before registering
    let commands_info: Vec<(String, String)> = vec![
        ("roll".into(), "Roll dice (e.g., roll 2d6+3, roll d%, roll 3-in-6)".into()),
        ("save".into(), "Save game state (e.g., save game.json)".into()),
        ("load".into(), "Load game state (e.g., load game.json)".into()),
        ("help".into(), "Show available commands".into()),
        ("quit".into(), "Exit the game".into()),
    ];

    let mut registry = CommandRegistry::new();
    registry.register(Box::new(RollCommand));
    registry.register(Box::new(SaveCommand));
    registry.register(Box::new(LoadCommand));
    registry.register(Box::new(HelpCommand { commands: commands_info }));
    registry.register(Box::new(QuitCommand));
    registry
}

fn main() {
    println!("OSR AI Game Master v{}", env!("CARGO_PKG_VERSION"));
    println!("Type 'help' for available commands, 'quit' to exit.\n");

    let registry = build_registry();
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

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd_name = parts[0];
        let args = &parts[1..];

        let result = registry.dispatch(cmd_name, args);
        println!("{}", result.output);

        if result.quit {
            break;
        }

        print!("> ");
        let _ = stdout.flush();
    }
}
