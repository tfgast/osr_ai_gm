use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::engine::wilderness_engine;
use crate::state::game::GameMode;
use crate::state::wilderness::{WildernessState, HexCell, Terrain};

pub struct EnterWildernessCommand;
impl Command for EnterWildernessCommand {
    fn name(&self) -> &str { "enter_wilderness" }
    fn help(&self) -> &str { "Enter wilderness travel mode (enter_wilderness <terrain>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let terrain: Terrain = if args.is_empty() {
            Terrain::default()
        } else {
            match args[0].parse() {
                Ok(t) => t,
                Err(e) => return CommandResult::error(e),
            }
        };
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, terrain)).unwrap();
        state.wilderness = Some(ws);
        state.mode = GameMode::Wilderness;
        CommandResult::ok(format!(
            "Entered wilderness. Starting hex: (0, 0) — {}.\n\
             Use 'add_hex' to build the map, 'travel' to move.",
            terrain.name()
        ))
    }
}

pub struct AddHexCommand;
impl Command for AddHexCommand {
    fn name(&self) -> &str { "add_hex" }
    fn help(&self) -> &str { "Add a hex to the wilderness map (add_hex <x> <y> <terrain>)" }
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
        let ws = match state.wilderness.as_mut() {
            Some(w) => w,
            None => return CommandResult::error("not in wilderness mode."),
        };
        match ws.add_hex(HexCell::new(x, y, terrain)) {
            Ok(()) => CommandResult::ok(format!("Added hex ({}, {}) — {}.", x, y, terrain.name())),
            Err(e) => CommandResult::error(e),
        }
    }
}

pub struct TravelCommand;
impl Command for TravelCommand {
    fn name(&self) -> &str { "travel" }
    fn help(&self) -> &str { "Travel to a wilderness hex (travel <x> <y>)" }
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
        let party_movement = state.party.members.iter()
            .filter(|c| c.is_alive())
            .map(|c| c.movement_rate)
            .min()
            .unwrap_or(120);
        if state.wilderness.is_none() {
            return CommandResult::error("not in wilderness mode.");
        }
        let ws = state.wilderness.as_mut().unwrap();
        let result = wilderness_engine::travel_day(ws, &mut state.party, x, y, party_movement);
        CommandResult::ok(format!("{}", result))
    }
}

pub struct ForageCommand;
impl Command for ForageCommand {
    fn name(&self) -> &str { "forage" }
    fn help(&self) -> &str { "Forage for food in the current hex (takes a full day)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        if state.wilderness.is_none() {
            return CommandResult::error("not in wilderness mode.");
        }
        let ws = state.wilderness.as_ref().unwrap();
        let result = wilderness_engine::forage(ws, &mut state.party);
        CommandResult::ok(result.message)
    }
}

pub struct HuntCommand;
impl Command for HuntCommand {
    fn name(&self) -> &str { "hunt" }
    fn help(&self) -> &str { "Hunt for game in the current hex (takes a full day)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        if state.wilderness.is_none() {
            return CommandResult::error("not in wilderness mode.");
        }
        let ws = state.wilderness.as_ref().unwrap();
        let result = wilderness_engine::hunt(ws, &mut state.party);
        CommandResult::ok(result.message)
    }
}

pub struct WildernessStatusCommand;
impl Command for WildernessStatusCommand {
    fn name(&self) -> &str { "wilderness_status" }
    fn help(&self) -> &str { "Show current wilderness travel status" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let ws = match state.wilderness.as_ref() {
            Some(w) => w,
            None => return CommandResult::error("not in wilderness mode."),
        };
        let party_movement = state.party.members.iter()
            .filter(|c| c.is_alive())
            .map(|c| c.movement_rate)
            .min()
            .unwrap_or(120);
        let status = wilderness_engine::wilderness_status(ws, &state.party, party_movement);
        CommandResult::ok(status)
    }
}
