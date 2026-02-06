use crate::engine::{encounter_engine, exploration, module as module_engine, wilderness};
use crate::gmapi::protocol::GMResponse;
use crate::persist::GameState;
use crate::state::dungeon::DoorState;
use crate::state::game::GameMode;
use crate::state::time::LightSourceKind;
use crate::state::wilderness::Terrain;
use serde::Serialize;

fn ok_with_typed_data<T: Serialize>(
    id: &str,
    mode: &GameMode,
    message: String,
    payload: T,
) -> GMResponse {
    match serde_json::to_value(payload) {
        Ok(data) => GMResponse::ok_with_data(id, message, mode.clone(), data),
        Err(err) => GMResponse::err(
            id,
            format!("internal error: failed to serialize response: {err}"),
            mode.clone(),
        ),
    }
}

pub(super) fn enter_dungeon(id: &str, state: &mut GameState, level: u32, room_name: &str) -> GMResponse {
    match exploration::action_enter_dungeon(state, level, room_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn advance_turn(id: &str, state: &mut GameState) -> GMResponse {
    match exploration::action_advance_dungeon_turn(state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn add_room(id: &str, state: &mut GameState, room_id: u32, name: &str) -> GMResponse {
    match exploration::action_add_room(state, room_id, name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn add_door(id: &str, state: &mut GameState, door_id: u32, room_a: u32, room_b: u32, door_state: DoorState) -> GMResponse {
    match exploration::action_add_door(state, door_id, room_a, room_b, door_state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn move_room(id: &str, state: &mut GameState, door_id: u32) -> GMResponse {
    match exploration::action_move_through_door(state, door_id) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn search(id: &str, state: &mut GameState, is_elf: bool) -> GMResponse {
    match exploration::action_search_room(state, is_elf) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn light(id: &str, state: &mut GameState, source: LightSourceKind, carrier: &str) -> GMResponse {
    match exploration::action_light(state, source, carrier) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn load_module(id: &str, state: &mut GameState, path: &str) -> GMResponse {
    match module_engine::action_load_module(state, path) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn open_door(id: &str, state: &mut GameState, door_id: u32) -> GMResponse {
    match exploration::action_open_door(state, door_id) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn force_door(id: &str, state: &mut GameState, door_id: u32, char_name: &str) -> GMResponse {
    match exploration::action_force_door(state, door_id, char_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn listen(id: &str, state: &mut GameState, is_demihuman: bool) -> GMResponse {
    match exploration::action_listen_at_door(state, is_demihuman) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn rest(id: &str, state: &mut GameState) -> GMResponse {
    match exploration::action_rest(state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn enter_wilderness(id: &str, state: &mut GameState, terrain: Terrain) -> GMResponse {
    match wilderness::action_enter_wilderness(state, terrain) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn add_hex(id: &str, state: &mut GameState, x: i32, y: i32, terrain: Terrain) -> GMResponse {
    match wilderness::action_add_hex(state, x, y, terrain) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn travel(id: &str, state: &mut GameState, x: i32, y: i32) -> GMResponse {
    match wilderness::action_travel(state, x, y) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn orient(id: &str, state: &mut GameState) -> GMResponse {
    match wilderness::action_orient(state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn forage(id: &str, state: &mut GameState) -> GMResponse {
    match wilderness::action_forage(state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn hunt(id: &str, state: &mut GameState) -> GMResponse {
    match wilderness::action_hunt(state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}
