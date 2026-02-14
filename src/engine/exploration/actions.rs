use crate::engine::result::EngineError;
use crate::persist::GameState;
use crate::state::dungeon::{Door, DoorState, DungeonState, Room};
use crate::state::time::LightSourceKind;

use super::results::{
    AddDoorResult, AddRoomResult, AdvanceDungeonTurnResult, EnterDungeonResult, ForceDoorResult,
    ExplorationStatusResult, ListenAtDoorResult, LightResult, MoveThroughDoorResult,
    OpenDoorResult, RestResult, SearchRoomResult,
};
use super::{
    advance_dungeon_turn, exploration_status, force_door, listen_at_door, move_through_door,
    search_room,
};

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
    Ok(SearchRoomResult::from(result))
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
    Ok(ListenAtDoorResult::from(result))
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

pub fn action_advance_dungeon_turn(
    state: &mut GameState,
) -> Result<AdvanceDungeonTurnResult, EngineError> {
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
    Ok(AdvanceDungeonTurnResult::from(result))
}

pub fn action_enter_dungeon(
    state: &mut GameState,
    level: u32,
    room_name: &str,
) -> Result<EnterDungeonResult, EngineError> {
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
    Ok(RestResult {
        message: "party rests for one turn. activity counter reset.".to_string(),
        total_turns: time.total_turns,
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
