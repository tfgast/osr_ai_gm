use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::engine::exploration;
use crate::state::dungeon::{DungeonState, Room, Door, DoorState};
use crate::state::game::GameMode;
use crate::state::time::{TimeTracker, LightSourceKind};

pub struct EnterDungeonCommand;
impl Command for EnterDungeonCommand {
    fn name(&self) -> &str { "enter_dungeon" }
    fn help(&self) -> &str { "Enter dungeon exploration mode (enter_dungeon <level> <room_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: enter_dungeon <level> [room_name]");
        }
        let level: u32 = match args[0].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("level must be a positive integer"),
        };
        let room_name = if args.len() > 1 { args[1..].join(" ") } else { "Entrance".to_string() };

        let mut dungeon = DungeonState::new(level);
        dungeon.add_room(Room::new(0, &room_name)).unwrap();
        dungeon.explore_current();

        let time = TimeTracker::new();

        state.dungeon = Some(dungeon);
        state.time = Some(time);
        state.dungeon_level = level;
        state.mode = GameMode::Exploration;

        CommandResult::ok(format!(
            "Entered dungeon level {}. Starting room: {}.\n\
             Use 'light torch <carrier>' or 'light lantern <carrier>' to light the way.\n\
             Use 'explore' to advance a dungeon turn.",
            level, room_name
        ))
    }
}

pub struct LightCommand;
impl Command for LightCommand {
    fn name(&self) -> &str { "light" }
    fn help(&self) -> &str { "Light a torch or lantern (light torch|lantern <carrier_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: light torch|lantern <carrier_name>");
        }
        let kind: LightSourceKind = match args[0].parse() {
            Ok(k) => k,
            Err(e) => return CommandResult::error(e),
        };
        let carrier = args[1];
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode. Use 'enter_dungeon' first."),
        };
        time.light(kind, carrier);
        CommandResult::ok(format!(
            "{} lights a {} ({} turns).",
            carrier, kind.name(), kind.max_turns()
        ))
    }
}

pub struct ExploreCommand;
impl Command for ExploreCommand {
    fn name(&self) -> &str { "explore" }
    fn help(&self) -> &str { "Advance one dungeon turn of exploration" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let level = state.dungeon_level;
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let result = exploration::advance_dungeon_turn(time, dungeon, level);
        CommandResult::ok(format!("{}", result))
    }
}

pub struct SearchCommand;
impl Command for SearchCommand {
    fn name(&self) -> &str { "search" }
    fn help(&self) -> &str { "Search the current room (1-in-6, elves 2-in-6). Takes one turn." }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let is_elf = args.first().map(|a| a.eq_ignore_ascii_case("elf")).unwrap_or(false);
        let level = state.dungeon_level;
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let result = exploration::search_room(time, dungeon, level, is_elf);
        CommandResult::ok(format!("{}", result))
    }
}

pub struct ListenCommand;
impl Command for ListenCommand {
    fn name(&self) -> &str { "listen" }
    fn help(&self) -> &str { "Listen at a door (1-in-6, demihumans 2-in-6). Takes one turn." }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let is_demihuman = args.first()
            .map(|a| a.eq_ignore_ascii_case("demihuman") || a.eq_ignore_ascii_case("elf"))
            .unwrap_or(false);
        let level = state.dungeon_level;
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        let dungeon = match state.dungeon.as_ref() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let result = exploration::listen_at_door(time, dungeon, level, is_demihuman);
        CommandResult::ok(format!("{}", result))
    }
}

pub struct ForceDoorCommand;
impl Command for ForceDoorCommand {
    fn name(&self) -> &str { "force_door" }
    fn help(&self) -> &str { "Force open a door (force_door <door_id> <character_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: force_door <door_id> <character_name>");
        }
        let door_id: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("door_id must be a number"),
        };
        let char_name = args[1];
        let character = match state.party.find_member(char_name) {
            Some(c) => c.clone(),
            None => return CommandResult::error(format!("no party member named '{}'.", char_name)),
        };
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let result = exploration::force_door(dungeon, door_id, &character);
        CommandResult::ok(result)
    }
}

pub struct AddRoomCommand;
impl Command for AddRoomCommand {
    fn name(&self) -> &str { "add_room" }
    fn help(&self) -> &str { "Add a room to dungeon (add_room <id> <name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: add_room <id> <name>");
        }
        let id: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("room id must be a number"),
        };
        let name = args[1..].join(" ");
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        match dungeon.add_room(Room::new(id, &name)) {
            Ok(()) => CommandResult::ok(format!("Added room {}: {}", id, name)),
            Err(e) => CommandResult::error(e),
        }
    }
}

pub struct AddDoorCommand;
impl Command for AddDoorCommand {
    fn name(&self) -> &str { "add_door" }
    fn help(&self) -> &str { "Add a door (add_door <id> <room_a> <room_b> [open|closed|stuck|locked|secret|spiked])" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 3 {
            return CommandResult::error(
                "usage: add_door <id> <room_a> <room_b> [open|closed|stuck|locked|secret|spiked]"
            );
        }
        let id: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("door id must be a number"),
        };
        let room_a: u32 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("room_a must be a number"),
        };
        let room_b: u32 = match args[2].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("room_b must be a number"),
        };
        let door_state: DoorState = if args.len() > 3 {
            match args[3].parse() {
                Ok(ds) => ds,
                Err(e) => return CommandResult::error(e),
            }
        } else {
            DoorState::default()
        };
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let door = match Door::new(id, room_a, room_b, door_state) {
            Ok(d) => d,
            Err(e) => return CommandResult::error(e),
        };
        match dungeon.add_door(door) {
            Ok(()) => CommandResult::ok(format!("Added door {} between rooms {} and {} ({:?})", id, room_a, room_b, door_state)),
            Err(e) => CommandResult::error(e),
        }
    }
}

pub struct MoveRoomCommand;
impl Command for MoveRoomCommand {
    fn name(&self) -> &str { "move" }
    fn help(&self) -> &str { "Move through a door (move <door_id>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: move <door_id>");
        }
        let door_id: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("door_id must be a number"),
        };
        let level = state.dungeon_level;
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        match exploration::move_through_door(time, dungeon, level, door_id) {
            Ok(result) => CommandResult::ok(format!("{}", result)),
            Err(e) => CommandResult::error(e),
        }
    }
}

pub struct RestCommand;
impl Command for RestCommand {
    fn name(&self) -> &str { "rest" }
    fn help(&self) -> &str { "Rest for one turn (required after 5 turns of activity)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        time.rest();
        CommandResult::ok("Party rests for one turn. Activity counter reset.")
    }
}

pub struct ExplorationStatusCommand;
impl Command for ExplorationStatusCommand {
    fn name(&self) -> &str { "exploration_status" }
    fn help(&self) -> &str { "Show current exploration state (time, light, dungeon map)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let time = match state.time.as_ref() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        let dungeon = match state.dungeon.as_ref() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let status = exploration::exploration_status(time, dungeon);
        CommandResult::ok(status)
    }
}
