use crate::engine::inventory;
use crate::gmapi::protocol::GMResponse;
use crate::persist::GameState;
use crate::state::game::GameMode;
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

pub(super) fn buy(id: &str, state: &mut GameState, character: &str, item_name: &str) -> GMResponse {
    match inventory::action_buy(state, character, item_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn drop(
    id: &str,
    state: &mut GameState,
    character: &str,
    item_name: &str,
) -> GMResponse {
    match inventory::action_drop(state, character, item_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn equip(
    id: &str,
    state: &mut GameState,
    character: &str,
    item_name: &str,
) -> GMResponse {
    match inventory::action_equip(state, character, item_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn loot(
    id: &str,
    state: &mut GameState,
    character: &str,
    item_name: &str,
    explicit_gp: Option<u32>,
) -> GMResponse {
    match inventory::action_loot(state, character, item_name, explicit_gp) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}
