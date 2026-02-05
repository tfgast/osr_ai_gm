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

pub struct OpenCommand;
impl Command for OpenCommand {
    fn name(&self) -> &str { "open" }
    fn help(&self) -> &str { "Open/force a door and move through (open <door_id>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: open <door_id>");
        }
        let door_id: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("door_id must be a number"),
        };

        let dungeon = match state.dungeon.as_ref() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let door = match dungeon.doors.iter().find(|d| d.id == door_id) {
            Some(d) => d.clone(),
            None => return CommandResult::error(format!("Door {} not found.", door_id)),
        };

        if door.state == DoorState::Locked {
            return CommandResult::error(format!(
                "Door {} is locked. It must be unlocked before it can be opened.", door_id
            ));
        }

        let mut output = Vec::new();

        // If not passable (closed/stuck), force it with the strongest party member
        if !door.is_passable() {
            let strongest = state.party.members
                .iter()
                .filter(|c| c.hp > 0)
                .max_by_key(|c| c.abilities.strength)
                .cloned();
            let character = match strongest {
                Some(c) => c,
                None => return CommandResult::error("no living party members to force the door."),
            };
            let force_result = exploration::force_door(
                state.dungeon.as_mut().unwrap(), door_id, &character,
            );
            output.push(force_result);

            // Check if forcing succeeded
            let door_after = state.dungeon.as_ref().unwrap()
                .doors.iter().find(|d| d.id == door_id).unwrap();
            if !door_after.is_passable() {
                return CommandResult::ok(output.join("\n"));
            }
        }

        // Door is now open — move through it
        let level = state.dungeon_level;
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        let dungeon = state.dungeon.as_mut().unwrap();
        match exploration::move_through_door(time, dungeon, level, door_id) {
            Ok(result) => {
                output.push(format!("{}", result));
                CommandResult::ok(output.join("\n"))
            }
            Err(e) => {
                if output.is_empty() {
                    CommandResult::error(e)
                } else {
                    output.push(format!("Error: {}", e));
                    CommandResult::ok(output.join("\n"))
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    use crate::rules::class::Class;
    use crate::state::dungeon::{Door, Room, DungeonState};
    use crate::state::time::TimeTracker;

    fn dungeon_state_with_doors() -> GameState {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.abilities.strength = 14;
        state.party.add_member(c);

        let mut dungeon = DungeonState::new(1);
        dungeon.add_room(Room::new(0, "Start")).unwrap();
        dungeon.add_room(Room::new(1, "Next")).unwrap();
        dungeon.add_room(Room::new(2, "Locked Room")).unwrap();
        dungeon.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();
        dungeon.add_door(Door::new(1, 0, 2, DoorState::Locked).unwrap()).unwrap();
        dungeon.current_room = Some(0);
        dungeon.explored.insert(0);
        state.dungeon = Some(dungeon);
        state.time = Some(TimeTracker::new());
        state.time.as_mut().unwrap().light(
            crate::state::time::LightSourceKind::Torch, "Aldric",
        );
        state.mode = GameMode::Exploration;
        state.dungeon_level = 1;
        state
    }

    #[test]
    fn open_moves_through_open_door() {
        let cmd = OpenCommand;
        let mut state = dungeon_state_with_doors();
        let result = cmd.execute(&["0"], &mut state);
        assert!(!result.output.starts_with("Error:"));
        assert!(result.output.contains("Moved to Next"));
    }

    #[test]
    fn open_locked_door_rejected() {
        let cmd = OpenCommand;
        let mut state = dungeon_state_with_doors();
        let result = cmd.execute(&["1"], &mut state);
        assert!(result.output.contains("Error:"));
        assert!(result.output.contains("locked"));
    }

    #[test]
    fn open_no_args() {
        let cmd = OpenCommand;
        let mut state = dungeon_state_with_doors();
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error:"));
        assert!(result.output.contains("usage"));
    }

    #[test]
    fn open_closed_door_attempts_force() {
        let cmd = OpenCommand;
        let mut state = dungeon_state_with_doors();
        // Change door 0 to closed
        state.dungeon.as_mut().unwrap().find_door_mut(0).unwrap().state = DoorState::Closed;
        let result = cmd.execute(&["0"], &mut state);
        // Should attempt to force — output mentions Aldric (the forcer)
        assert!(result.output.contains("Aldric"));
    }
}
