pub mod command;
pub mod dice;
pub mod engine;
pub mod model;
pub mod persist;
pub mod rules;

use command::{Command, CommandRegistry, CommandResult};
use std::io::{self, BufRead, Write};
use engine::chargen;
use rules::class::{self, Class};

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

struct ChargenCommand;
impl Command for ChargenCommand {
    fn name(&self) -> &str { "chargen" }
    fn help(&self) -> &str { "Create a character (chargen <name> <class> [alignment])" }
    fn execute(&self, args: &[&str]) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: chargen <name> <class> [alignment]\n  \
                 alignment: Lawful, Neutral, Chaotic (default: Neutral)\n  \
                 Use 'classes' to list available classes."
            );
        }
        let name = args[0];
        let class = match Class::parse(args[1]) {
            Some(c) => c,
            None => return CommandResult::error(format!(
                "unknown class '{}'. Use 'classes' to list available classes.", args[1]
            )),
        };
        let alignment = if args.len() >= 3 {
            match args[2].to_lowercase().as_str() {
                "lawful" | "l" => "Lawful",
                "neutral" | "n" => "Neutral",
                "chaotic" | "c" => "Chaotic",
                _ => return CommandResult::error(
                    "alignment must be Lawful (L), Neutral (N), or Chaotic (C)"
                ),
            }
        } else {
            "Neutral"
        };

        // Roll abilities
        let mut abilities = chargen::roll_abilities();
        let mut out = String::new();
        out.push_str(&format!("Rolled abilities: STR {} INT {} WIS {} DEX {} CON {} CHA {}\n",
            abilities[0], abilities[1], abilities[2],
            abilities[3], abilities[4], abilities[5]));

        // Apply racial modifiers for demihuman classes
        let def = class::class_def(class);
        if !def.racial_modifiers.is_empty() {
            class::apply_racial_modifiers(class, &mut abilities);
            out.push_str(&format!(
                "After racial modifiers: STR {} INT {} WIS {} DEX {} CON {} CHA {}\n",
                abilities[0], abilities[1], abilities[2],
                abilities[3], abilities[4], abilities[5]));
        }

        // Validate requirements
        if !class::meets_requirements(class, &abilities) {
            let eligible = class::eligible_classes(&abilities);
            let names: Vec<&str> = eligible.iter().map(|c| c.name()).collect();
            out.push_str(&format!(
                "\nAbilities do not meet requirements for {}.\nEligible classes: {}",
                class.name(), names.join(", ")
            ));
            return CommandResult::ok(out);
        }

        // Create character
        let c = chargen::create_character(name, class, abilities, alignment);
        out.push('\n');
        out.push_str(&chargen::character_sheet(&c));
        CommandResult::ok(out)
    }
}

struct ClassesCommand;
impl Command for ClassesCommand {
    fn name(&self) -> &str { "classes" }
    fn help(&self) -> &str { "List all character classes" }
    fn execute(&self, _args: &[&str]) -> CommandResult {
        let mut out = String::from("Character Classes (22):\n");
        for &c in &Class::ALL {
            let def = class::class_def(c);
            let reqs: Vec<String> = def.requirements.iter()
                .map(|&(idx, min)| {
                    let name = ["STR", "INT", "WIS", "DEX", "CON", "CHA"][idx];
                    format!("{} {}", name, min)
                })
                .collect();
            let req_str = if reqs.is_empty() { "none".to_string() } else { reqs.join(", ") };
            out.push_str(&format!("  {:14} HD: d{}  Req: {}",
                c.name(), def.hit_die, req_str));
            if def.is_demihuman {
                out.push_str("  [demihuman]");
            }
            out.push('\n');
        }
        CommandResult::ok(out)
    }
}

struct EligibleCommand;
impl Command for EligibleCommand {
    fn name(&self) -> &str { "eligible" }
    fn help(&self) -> &str { "Show eligible classes (eligible <STR> <INT> <WIS> <DEX> <CON> <CHA>)" }
    fn execute(&self, args: &[&str]) -> CommandResult {
        if args.len() < 6 {
            return CommandResult::error(
                "usage: eligible <STR> <INT> <WIS> <DEX> <CON> <CHA>"
            );
        }
        let mut abilities = [0i32; 6];
        for (i, &s) in args[..6].iter().enumerate() {
            match s.parse::<i32>() {
                Ok(v) if (3..=18).contains(&v) => abilities[i] = v,
                _ => return CommandResult::error(format!(
                    "ability scores must be 3-18, got '{}'", s
                )),
            }
        }
        let eligible = class::eligible_classes(&abilities);
        let names: Vec<&str> = eligible.iter().map(|c| c.name()).collect();
        CommandResult::ok(format!(
            "Abilities: STR {} INT {} WIS {} DEX {} CON {} CHA {}\nEligible classes ({}): {}",
            abilities[0], abilities[1], abilities[2],
            abilities[3], abilities[4], abilities[5],
            eligible.len(), names.join(", ")
        ))
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
        ("chargen".into(), "Create a character (chargen <name> <class> [alignment])".into()),
        ("classes".into(), "List all character classes".into()),
        ("eligible".into(), "Show eligible classes (eligible <STR> <INT> <WIS> <DEX> <CON> <CHA>)".into()),
        ("roll".into(), "Roll dice (e.g., roll 2d6+3, roll d%, roll 3-in-6)".into()),
        ("save".into(), "Save game state (e.g., save game.json)".into()),
        ("load".into(), "Load game state (e.g., load game.json)".into()),
        ("help".into(), "Show available commands".into()),
        ("quit".into(), "Exit the game".into()),
    ];

    let mut registry = CommandRegistry::new();
    registry.register(Box::new(ChargenCommand));
    registry.register(Box::new(ClassesCommand));
    registry.register(Box::new(EligibleCommand));
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
