use super::{Command, CommandResult};
use crate::engine::wilderness;
use crate::persist::GameState;
use crate::state::wilderness::Terrain;

pub struct EnterWildernessCommand;
impl Command for EnterWildernessCommand {
    fn name(&self) -> &str {
        "enter_wilderness"
    }
    fn help(&self) -> &str {
        "Enter wilderness travel mode (enter_wilderness <terrain>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let terrain: Terrain = if args.is_empty() {
            Terrain::default()
        } else {
            match args[0].parse() {
                Ok(t) => t,
                Err(e) => return CommandResult::error(e),
            }
        };
        match wilderness::action_enter_wilderness(state, terrain) {
            Ok(result) => CommandResult::ok(format!(
                "Entered wilderness. Starting hex: ({}, {}) — {}.\n\
                 Use 'add_hex' to build the map, 'travel' to move.",
                result.x,
                result.y,
                result.terrain.name()
            )),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct AddHexCommand;
impl Command for AddHexCommand {
    fn name(&self) -> &str {
        "add_hex"
    }
    fn help(&self) -> &str {
        "Add a hex to the wilderness map (add_hex <x> <y> <terrain>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 3 {
            return CommandResult::error("usage: add_hex <x> <y> <terrain>");
        }
        let x: i32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("x must be an integer"),
        };
        let y: i32 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("y must be an integer"),
        };
        let terrain: Terrain = match args[2].parse() {
            Ok(t) => t,
            Err(e) => return CommandResult::error(e),
        };
        match wilderness::action_add_hex(state, x, y, terrain) {
            Ok(result) => CommandResult::ok(format!(
                "Added hex ({}, {}) — {}.",
                result.x,
                result.y,
                result.terrain.name()
            )),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct TravelCommand;
impl Command for TravelCommand {
    fn name(&self) -> &str {
        "travel"
    }
    fn help(&self) -> &str {
        "Travel to a wilderness hex (travel <x> <y>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: travel <x> <y>");
        }
        let x: i32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("x must be an integer"),
        };
        let y: i32 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("y must be an integer"),
        };
        match wilderness::action_travel(state, x, y) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct ForageCommand;
impl Command for ForageCommand {
    fn name(&self) -> &str {
        "forage"
    }
    fn help(&self) -> &str {
        "Forage for food in the current hex (takes a full day)"
    }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match wilderness::action_forage(state) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct HuntCommand;
impl Command for HuntCommand {
    fn name(&self) -> &str {
        "hunt"
    }
    fn help(&self) -> &str {
        "Hunt for game in the current hex (takes a full day)"
    }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match wilderness::action_hunt(state) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct WildernessStatusCommand;
impl Command for WildernessStatusCommand {
    fn name(&self) -> &str {
        "wilderness_status"
    }
    fn help(&self) -> &str {
        "Show current wilderness travel status"
    }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match wilderness::action_wilderness_status(state) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct OrientCommand;
impl Command for OrientCommand {
    fn name(&self) -> &str {
        "orient"
    }
    fn help(&self) -> &str {
        "Attempt to find bearings when lost (takes a full day)"
    }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match wilderness::action_orient(state) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}
