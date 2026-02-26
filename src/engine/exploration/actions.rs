use crate::engine::result::EngineError;
use crate::persist::GameState;
use crate::state::dungeon::{Door, DoorState, DungeonState, Room};
use crate::state::effect;
use crate::state::game::GameMode;
use crate::state::time::LightSourceKind;

use super::results::{
    AddDoorResult, AddRoomResult, AdvanceDungeonTurnResult, EnterDungeonResult, ForceDoorResult,
    ExplorationStatusResult, ListenAtDoorResult, LightResult, LookResult, MoveThroughDoorResult,
    OpenDoorResult, PickLockResult, RestResult, SearchRoomResult,
};
use super::{
    advance_dungeon_turn, exploration_status, force_door, listen_at_door, move_through_door,
    pick_lock, search_room,
};

/// Tick turn-based effects on all party members and global state.
/// Returns expiry messages to append to the action result.
fn tick_turn_effects(state: &mut GameState) -> Vec<String> {
    let mut messages = Vec::new();
    for member in &mut state.party.members {
        messages.extend(effect::tick_turn_effects(&mut member.effects, &member.name));
    }
    messages.extend(effect::tick_turn_effects(&mut state.effects, "the area"));
    messages
}

pub fn action_move_through_door(
    state: &mut GameState,
    door_id: u32,
) -> Result<MoveThroughDoorResult, EngineError> {
    let level = state.dungeon_level;
    let time = state
        .time
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("not in exploration mode.".to_string()))?;
    let dungeon = state
        .dungeon
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;

    let result = move_through_door(time, dungeon, level, door_id).map_err(EngineError::InvalidInput)?;

    // Sync GameState.dungeon_level after cross-level transition
    if let Some(ref dungeon) = state.dungeon {
        if dungeon.level != state.dungeon_level {
            state.dungeon_level = dungeon.level;
        }
    }

    Ok(MoveThroughDoorResult::from(result))
}

pub fn action_search_room(
    state: &mut GameState,
    is_elf: bool,
) -> Result<SearchRoomResult, EngineError> {
    let level = state.dungeon_level;
    let time = state
        .time
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("not in exploration mode.".to_string()))?;
    let dungeon = state
        .dungeon
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;

    let result = search_room(time, dungeon, level, is_elf);
    let mut result = SearchRoomResult::from(result);
    let expiry_msgs = tick_turn_effects(state);
    if !expiry_msgs.is_empty() {
        for msg in &expiry_msgs {
            result.messages.push(msg.clone());
        }
        result.message = result.messages.join("\n");
    }
    Ok(result)
}

pub fn action_listen_at_door(
    state: &mut GameState,
    is_demihuman: bool,
) -> Result<ListenAtDoorResult, EngineError> {
    let level = state.dungeon_level;
    let time = state
        .time
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("not in exploration mode.".to_string()))?;
    let dungeon = state
        .dungeon
        .as_ref()
        .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;

    let result = listen_at_door(time, dungeon, level, is_demihuman);
    let mut result = ListenAtDoorResult::from(result);
    let expiry_msgs = tick_turn_effects(state);
    if !expiry_msgs.is_empty() {
        for msg in &expiry_msgs {
            result.messages.push(msg.clone());
        }
        result.message = result.messages.join("\n");
    }
    Ok(result)
}

pub fn action_force_door(
    state: &mut GameState,
    door_id: u32,
    char_name: &str,
) -> Result<ForceDoorResult, EngineError> {
    let character = state.party.find_member(char_name).cloned().ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;
    let dungeon = state
        .dungeon
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;

    let message = force_door(dungeon, door_id, &character);
    let forced_open = message.contains("forces door") && message.contains("open");

    Ok(ForceDoorResult {
        message,
        door_id,
        character: character.name,
        forced_open,
    })
}

pub fn action_pick_lock(
    state: &mut GameState,
    door_id: u32,
    char_name: &str,
) -> Result<PickLockResult, EngineError> {
    let character = state.party.find_member(char_name).cloned().ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;
    let dungeon = state
        .dungeon
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;

    let result = pick_lock(dungeon, door_id, &character);

    Ok(PickLockResult {
        message: result.message,
        door_id,
        character: character.name,
        success: result.success,
    })
}

pub fn action_advance_dungeon_turn(
    state: &mut GameState,
) -> Result<AdvanceDungeonTurnResult, EngineError> {
    if state.mode != GameMode::Exploration {
        return Err(EngineError::WrongState(
            "AdvanceTurn is only available in dungeon exploration mode.".to_string(),
        ));
    }
    let level = state.dungeon_level;
    let time = state
        .time
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("not in exploration mode.".to_string()))?;
    let dungeon = state
        .dungeon
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;

    let result = advance_dungeon_turn(time, dungeon, level);
    let mut result = AdvanceDungeonTurnResult::from(result);
    let expiry_msgs = tick_turn_effects(state);
    if !expiry_msgs.is_empty() {
        for msg in &expiry_msgs {
            result.messages.push(msg.clone());
        }
        result.message = result.messages.join("\n");
    }
    Ok(result)
}

pub fn action_enter_dungeon(
    state: &mut GameState,
    level: u32,
    room_name: &str,
) -> Result<EnterDungeonResult, EngineError> {
    match state.mode {
        GameMode::Idle | GameMode::Downtime => {}
        GameMode::Wilderness => {
            return Err(EngineError::WrongState(
                "cannot enter dungeon while in wilderness mode. Use LeaveWilderness first."
                    .to_string(),
            ));
        }
        GameMode::Combat => {
            return Err(EngineError::WrongState(
                "cannot enter dungeon during combat. Use EndCombat first.".to_string(),
            ));
        }
        GameMode::Exploration => {
            return Err(EngineError::WrongState(
                "already in exploration mode. Use LeaveDungeon first.".to_string(),
            ));
        }
        GameMode::CharGen => {
            return Err(EngineError::WrongState(
                "cannot enter dungeon during character generation.".to_string(),
            ));
        }
    }

    if level == 0 {
        return Err(EngineError::InvalidInput(
            "level must be a positive integer.".to_string(),
        ));
    }

    let mut dungeon = DungeonState::new(level);
    dungeon
        .add_room(Room::new(0, room_name))
        .map_err(EngineError::Internal)?;
    dungeon.explore_current();

    state.enter_exploration(dungeon, level);

    Ok(EnterDungeonResult {
        message: format!(
            "entered dungeon level {}. starting room: {}.",
            level, room_name
        ),
        level,
        room_name: room_name.to_string(),
    })
}

pub fn action_light(
    state: &mut GameState,
    source: LightSourceKind,
    carrier: &str,
) -> Result<LightResult, EngineError> {
    let time = state
        .time
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("not in exploration mode.".to_string()))?;

    if time.lights.iter().any(|l| l.carrier == carrier) {
        return Err(EngineError::WrongState(format!(
            "{} already has an active light source.",
            carrier
        )));
    }

    time.light(source, carrier);
    Ok(LightResult {
        message: format!("{} lights a {} ({} turns).", carrier, source.name(), source.max_turns()),
        source: source.name().to_string(),
        carrier: carrier.to_string(),
        duration_turns: source.max_turns(),
    })
}

pub fn action_rest(state: &mut GameState) -> Result<RestResult, EngineError> {
    let time = state
        .time
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("not in exploration mode.".to_string()))?;

    time.rest();
    let total_turns = time.total_turns;
    let expiry_msgs = tick_turn_effects(state);
    let mut message = "party rests for one turn. activity counter reset.".to_string();
    for msg in &expiry_msgs {
        message.push('\n');
        message.push_str(msg);
    }
    Ok(RestResult {
        message,
        total_turns,
    })
}

pub fn action_add_room(
    state: &mut GameState,
    room_id: u32,
    name: &str,
) -> Result<AddRoomResult, EngineError> {
    let dungeon = state
        .dungeon
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;

    dungeon
        .add_room(Room::new(room_id, name))
        .map_err(EngineError::InvalidInput)?;

    Ok(AddRoomResult {
        message: format!("added room {}: {}.", room_id, name),
        room_id,
        name: name.to_string(),
    })
}

pub fn action_add_door(
    state: &mut GameState,
    door_id: u32,
    room_a: u32,
    room_b: u32,
    door_state: DoorState,
) -> Result<AddDoorResult, EngineError> {
    let dungeon = state
        .dungeon
        .as_mut()
        .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;

    let door = Door::new(door_id, room_a, room_b, door_state).map_err(EngineError::InvalidInput)?;
    dungeon.add_door(door).map_err(EngineError::InvalidInput)?;

    Ok(AddDoorResult {
        message: format!(
            "added door {} between rooms {} and {} ({}).",
            door_id, room_a, room_b, door_state
        ),
        door_id,
        room_a,
        room_b,
        door_state,
    })
}

pub fn action_open_door(
    state: &mut GameState,
    door_id: u32,
) -> Result<OpenDoorResult, EngineError> {
    let door = {
        let dungeon = state
            .dungeon
            .as_ref()
            .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;
        dungeon
            .doors
            .iter()
            .find(|d| d.id == door_id)
            .cloned()
            .ok_or_else(|| EngineError::InvalidInput(format!("door {} not found.", door_id)))?
    };

    if door.state == DoorState::Locked {
        return Err(EngineError::InvalidInput(format!(
            "door {} is locked. it must be unlocked before it can be opened.",
            door_id
        )));
    }

    if door.state == DoorState::Stuck {
        return Err(EngineError::InvalidInput(format!(
            "door {} is stuck. Use ForceDoor to attempt to force it open.",
            door_id
        )));
    }

    let mut output = Vec::new();

    if !door.is_passable() {
        let strongest = state
            .party
            .members
            .iter()
            .filter(|c| c.hp > 0)
            .max_by_key(|c| c.abilities.strength)
            .cloned()
            .ok_or_else(|| {
                EngineError::InvalidInput("no living party members to force the door.".to_string())
            })?;

        let force_result = force_door(
            state
                .dungeon
                .as_mut()
                .expect("dungeon existence verified above"),
            door_id,
            &strongest,
        );
        output.push(force_result);

        let door_after = state
            .dungeon
            .as_ref()
            .expect("dungeon existence verified above")
            .doors
            .iter()
            .find(|d| d.id == door_id)
            .ok_or_else(|| EngineError::Internal(format!("door {} not found after forcing.", door_id)))?;

        if !door_after.is_passable() {
            let message = output.join("\n");
            return Ok(OpenDoorResult {
                message,
                door_id,
                steps: output,
                forced: true,
                moved: false,
            });
        }
    }

    match action_move_through_door(state, door_id) {
        Ok(result) => {
            output.push(result.message);
            let message = output.join("\n");
            Ok(OpenDoorResult {
                message,
                door_id,
                steps: output,
                forced: !door.is_passable(),
                moved: true,
            })
        }
        Err(e) => {
            if output.is_empty() {
                Err(e)
            } else {
                output.push(format!("error: {}", e));
                let message = output.join("\n");
                Ok(OpenDoorResult {
                    message,
                    door_id,
                    steps: output,
                    forced: true,
                    moved: false,
                })
            }
        }
    }
}

pub fn action_exploration_status(state: &GameState) -> Result<ExplorationStatusResult, EngineError> {
    let time = state
        .time
        .as_ref()
        .ok_or_else(|| EngineError::WrongState("not in exploration mode.".to_string()))?;
    let dungeon = state
        .dungeon
        .as_ref()
        .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;
    Ok(ExplorationStatusResult {
        message: exploration_status(time, dungeon),
    })
}

pub fn action_look(state: &GameState) -> Result<LookResult, EngineError> {
    let dungeon = state
        .dungeon
        .as_ref()
        .ok_or_else(|| EngineError::WrongState("no dungeon state.".to_string()))?;
    let room_id = dungeon
        .current_room
        .ok_or_else(|| EngineError::WrongState("no current room.".to_string()))?;
    let room = dungeon
        .find_room(room_id)
        .ok_or_else(|| EngineError::Internal(format!("room {} not found.", room_id)))?;

    let mut lines = Vec::new();
    lines.push(format!("== {} (room {}) ==", room.name, room.id));

    if room.description.is_empty() {
        lines.push("No description available.".to_string());
    } else {
        lines.push(room.description.clone());
    }

    // Show exits
    let doors = dungeon.doors_from_current();
    if !doors.is_empty() {
        lines.push(String::new());
        lines.push("Exits:".to_string());
        for d in &doors {
            let dest = if d.room_a == room_id { d.room_b } else { d.room_a };
            let dest_name = dungeon
                .find_room(dest)
                .map(|r| r.name.as_str())
                .unwrap_or("?");
            lines.push(format!("  Door {} → {} ({}) [{}]", d.id, dest, dest_name, d.state));
        }
    }

    Ok(LookResult {
        message: lines.join("\n"),
        room_id,
        room_name: room.name.clone(),
        description: room.description.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    
    use crate::state::dungeon::{DungeonState, Room};
    use crate::state::effect::{ActiveEffect, EffectDuration};
    use crate::state::time::LightSourceKind;

    /// Build a minimal GameState in Exploration mode with one party member.
    fn exploration_state() -> GameState {
        let mut state = GameState::new();
        let mut ch = Character::new("Arden", "Fighter");
        ch.hp = 8;
        ch.max_hp = 8;
        state.party.add_member(ch);

        let mut dungeon = DungeonState::new(1);
        dungeon.add_room(Room::new(0, "Hall")).unwrap();
        state.enter_exploration(dungeon, 1);

        // Light a lantern so exploration isn't blocked by darkness.
        state.time.as_mut().unwrap().light(LightSourceKind::Lantern, "Arden");
        state
    }

    fn test_effect(name: &str, turns: u32) -> ActiveEffect {
        ActiveEffect {
            id: 1,
            name: name.to_string(),
            source: "test".to_string(),
            duration: EffectDuration::Turns(turns),
            modifiers: Vec::new(),
            notes: String::new(),
        }
    }

    #[test]
    fn advance_turn_ticks_turn_effects() {
        let mut state = exploration_state();
        state.party.members[0].effects.push(test_effect("Shield", 2));

        // First advance: Turns(2) → Turns(1)
        let r = action_advance_dungeon_turn(&mut state).unwrap();
        assert_eq!(state.party.members[0].effects.len(), 1);
        assert_eq!(state.party.members[0].effects[0].duration, EffectDuration::Turns(1));
        assert!(!r.message.contains("worn off"));

        // Second advance: Turns(1) → expired & removed
        let r = action_advance_dungeon_turn(&mut state).unwrap();
        assert!(state.party.members[0].effects.is_empty());
        assert!(r.message.contains("Shield has worn off"));
    }

    #[test]
    fn search_room_ticks_turn_effects() {
        let mut state = exploration_state();
        state.party.members[0].effects.push(test_effect("Light", 1));

        let r = action_search_room(&mut state, false).unwrap();
        assert!(state.party.members[0].effects.is_empty());
        assert!(r.message.contains("Light has worn off"));
    }

    #[test]
    fn listen_at_door_ticks_turn_effects() {
        let mut state = exploration_state();
        state.party.members[0].effects.push(test_effect("Detect Magic", 1));

        let r = action_listen_at_door(&mut state, false).unwrap();
        assert!(state.party.members[0].effects.is_empty());
        assert!(r.message.contains("Detect Magic has worn off"));
    }

    #[test]
    fn rest_ticks_turn_effects() {
        let mut state = exploration_state();
        state.party.members[0].effects.push(test_effect("Bless", 1));

        let r = action_rest(&mut state).unwrap();
        assert!(state.party.members[0].effects.is_empty());
        assert!(r.message.contains("Bless has worn off"));
    }

    #[test]
    fn global_effects_tick_during_exploration() {
        let mut state = exploration_state();
        state.effects.push(test_effect("Zone of Silence", 1));

        let r = action_advance_dungeon_turn(&mut state).unwrap();
        assert!(state.effects.is_empty());
        assert!(r.message.contains("Zone of Silence has worn off"));
    }

    #[test]
    fn round_effects_not_ticked_by_exploration() {
        let mut state = exploration_state();
        state.party.members[0].effects.push(ActiveEffect {
            id: 1,
            name: "Haste".to_string(),
            source: "test".to_string(),
            duration: EffectDuration::Rounds(3),
            modifiers: Vec::new(),
            notes: String::new(),
        });

        action_advance_dungeon_turn(&mut state).unwrap();
        // Round-based effect should be untouched by turn ticking.
        assert_eq!(state.party.members[0].effects.len(), 1);
        assert_eq!(
            state.party.members[0].effects[0].duration,
            EffectDuration::Rounds(3)
        );
    }
}
