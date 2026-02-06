use crate::engine::lookup;
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

pub(super) fn lookup_item(id: &str, state: &GameState, name: &str) -> GMResponse {
    match lookup::action_lookup_item(name) {
        Ok(result) => {
            ok_with_typed_data(id, &state.mode, result.api_message(), result.api_payload())
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn search_items(id: &str, state: &GameState, query: &str) -> GMResponse {
    match lookup::action_search_items(query) {
        Ok(result) => {
            ok_with_typed_data(id, &state.mode, result.api_message(), result.api_payload())
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn lookup_treasure_type(id: &str, state: &GameState, letter: &str) -> GMResponse {
    match lookup::action_lookup_treasure_type(letter) {
        Ok(result) => {
            ok_with_typed_data(id, &state.mode, result.api_message(), result.api_payload())
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn roll_treasure(id: &str, state: &GameState, letter: &str) -> GMResponse {
    match lookup::action_roll_treasure(letter) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.api_message(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn lookup_spell(id: &str, state: &GameState, name: &str, list_name: &str) -> GMResponse {
    let list = if list_name.is_empty() {
        None
    } else {
        match lookup::parse_spell_list(list_name) {
            Some(list) => Some(list),
            None => {
                return GMResponse::err(
                    id,
                    format!("unknown spell list '{}'.", list_name),
                    state.mode.clone(),
                )
            }
        }
    };

    match lookup::action_lookup_spell(name, list) {
        Ok(result) => {
            ok_with_typed_data(id, &state.mode, result.api_message(), result.api_payload())
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}
