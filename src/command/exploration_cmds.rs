use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::engine::exploration;
use crate::state::dungeon::DoorState;
use crate::state::time::LightSourceKind;

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

        match exploration::action_enter_dungeon(state, level, &room_name) {
            Ok(result) => CommandResult::ok(format!(
                "Entered dungeon level {}. Starting room: {}.\n\
                 Use 'light torch <carrier>' or 'light lantern <carrier>' to light the way.\n\
                 Use 'explore' to advance a dungeon turn.",
                result.level, result.room_name
            )),
            Err(e) => CommandResult::error(e.to_string()),
        }
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
        match exploration::action_light(state, kind, args[1]) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct ExploreCommand;
impl Command for ExploreCommand {
    fn name(&self) -> &str { "explore" }
    fn help(&self) -> &str { "Advance one dungeon turn of exploration" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match exploration::action_advance_dungeon_turn(state) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct SearchCommand;
impl Command for SearchCommand {
    fn name(&self) -> &str { "search" }
    fn help(&self) -> &str { "Search the current room (1-in-6, elves 2-in-6). Takes one turn." }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let is_elf = args.first().map(|a| a.eq_ignore_ascii_case("elf")).unwrap_or(false);
        match exploration::action_search_room(state, is_elf) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
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
        match exploration::action_listen_at_door(state, is_demihuman) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
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
        match exploration::action_force_door(state, door_id, args[1]) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
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
        match exploration::action_add_room(state, id, &name) {
            Ok(result) => CommandResult::ok(format!("Added room {}: {}", result.room_id, result.name)),
            Err(e) => CommandResult::error(e.to_string()),
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
        match exploration::action_add_door(state, id, room_a, room_b, door_state) {
            Ok(result) => CommandResult::ok(format!(
                "Added door {} between rooms {} and {} ({:?})",
                result.door_id, result.room_a, result.room_b, result.door_state
            )),
            Err(e) => CommandResult::error(e.to_string()),
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
        match exploration::action_move_through_door(state, door_id) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
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
        match exploration::action_open_door(state, door_id) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct PickLockCommand;
impl Command for PickLockCommand {
    fn name(&self) -> &str { "pick_lock" }
    fn help(&self) -> &str { "Thief picks a locked door (pick_lock <door_id> <character_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: pick_lock <door_id> <character_name>");
        }
        let door_id: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("door_id must be a number"),
        };
        let char_name = args[1..].join(" ");
        match exploration::action_pick_lock(state, door_id, &char_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct RestCommand;
impl Command for RestCommand {
    fn name(&self) -> &str { "rest" }
    fn help(&self) -> &str { "Rest for one turn (required after 5 turns of activity)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match exploration::action_rest(state) {
            Ok(_result) => CommandResult::ok("Party rests for one turn. Activity counter reset."),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct ExplorationStatusCommand;
impl Command for ExplorationStatusCommand {
    fn name(&self) -> &str { "exploration_status" }
    fn help(&self) -> &str { "Show current exploration state (time, light, dungeon map)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match exploration::action_exploration_status(state) {
            Ok(result) => {
                let mut out = result.message;
                // Append active effects (parity with GM API QueryExploration)
                let mut active: Vec<String> = Vec::new();
                for c in &state.party.members {
                    for e in &c.effects {
                        active.push(format!("{} on {} ({})", e.name, c.name, e.duration));
                    }
                }
                for e in &state.effects {
                    active.push(format!("{} ({})", e.name, e.duration));
                }
                if !active.is_empty() {
                    out.push_str(&format!("\nEffects: {}", active.join(", ")));
                }
                CommandResult::ok(out)
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct LookCommand;
impl Command for LookCommand {
    fn name(&self) -> &str { "look" }
    fn help(&self) -> &str { "Describe the current room (name, description, exits)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match exploration::action_look(state) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    
    use crate::state::game::GameMode;
    use crate::state::dungeon::{Door, Room, DungeonState};
    use crate::state::time::TimeTracker;

    fn dungeon_state_with_doors() -> GameState {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", "Fighter");
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

    #[test]
    fn pick_lock_no_args() {
        let cmd = PickLockCommand;
        let mut state = dungeon_state_with_doors();
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error:"));
        assert!(result.output.contains("usage"));
    }

    #[test]
    fn pick_lock_non_thief_rejected() {
        let cmd = PickLockCommand;
        let mut state = dungeon_state_with_doors();
        let result = cmd.execute(&["1", "Aldric"], &mut state);
        assert!(result.output.contains("does not have lockpicking"));
    }

    #[test]
    fn pick_lock_thief_attempts() {
        let cmd = PickLockCommand;
        let mut state = dungeon_state_with_doors();
        let mut thief = Character::new("Shade", "Thief");
        thief.abilities.dexterity = 16;
        state.party.add_member(thief);
        let result = cmd.execute(&["1", "Shade"], &mut state);
        // Either succeeds or fails, but should not error on input
        assert!(!result.output.contains("usage"));
    }
}
