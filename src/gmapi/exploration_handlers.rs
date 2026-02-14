use crate::engine::{exploration, module as module_engine, wilderness};
use crate::gmapi::protocol::GMResponse;
use crate::persist::GameState;
use crate::state::dungeon::DoorState;
use crate::state::time::LightSourceKind;
use crate::state::wilderness::Terrain;
use serde::Serialize;

fn ok_with_typed_data<T: Serialize>(
    id: &str,
    state: &GameState,
    message: String,
    payload: T,
) -> GMResponse {
    match serde_json::to_value(payload) {
        Ok(data) => GMResponse::ok_with_data(id, message, state.mode.clone(), data),
        Err(err) => GMResponse::err(
            id,
            format!("internal error: failed to serialize response: {err}"),
            state.mode.clone(),
        ),
    }
}

fn fail_with_typed_data<T: Serialize>(
    id: &str,
    state: &GameState,
    message: String,
    payload: T,
) -> GMResponse {
    match serde_json::to_value(payload) {
        Ok(data) => GMResponse::fail_with_data(id, message, state.mode.clone(), data),
        Err(err) => GMResponse::err(
            id,
            format!("internal error: failed to serialize response: {err}"),
            state.mode.clone(),
        ),
    }
}

// =============================================================================
// Dungeon exploration
// =============================================================================

pub(super) fn enter_dungeon(id: &str, state: &mut GameState, level: u32, room_name: &str) -> GMResponse {
    match exploration::action_enter_dungeon(state, level, room_name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn advance_turn(id: &str, state: &mut GameState) -> GMResponse {
    match exploration::action_advance_dungeon_turn(state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn add_room(id: &str, state: &mut GameState, room_id: u32, name: &str) -> GMResponse {
    match exploration::action_add_room(state, room_id, name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn add_door(id: &str, state: &mut GameState, door_id: u32, room_a: u32, room_b: u32, door_state: DoorState) -> GMResponse {
    match exploration::action_add_door(state, door_id, room_a, room_b, door_state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn move_room(id: &str, state: &mut GameState, door_id: u32) -> GMResponse {
    match exploration::action_move_through_door(state, door_id) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn search(id: &str, state: &mut GameState, is_elf: bool) -> GMResponse {
    match exploration::action_search_room(state, is_elf) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn light(id: &str, state: &mut GameState, source: LightSourceKind, carrier: &str) -> GMResponse {
    match exploration::action_light(state, source, carrier) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn load_module(id: &str, state: &mut GameState, path: &str) -> GMResponse {
    match module_engine::action_load_module(state, path) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn open_door(id: &str, state: &mut GameState, door_id: u32) -> GMResponse {
    match exploration::action_open_door(state, door_id) {
        Ok(result) if result.moved => ok_with_typed_data(id, state, result.message.clone(), result),
        Ok(result) => fail_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn force_door(id: &str, state: &mut GameState, door_id: u32, char_name: &str) -> GMResponse {
    match exploration::action_force_door(state, door_id, char_name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn pick_lock(id: &str, state: &mut GameState, door_id: u32, char_name: &str) -> GMResponse {
    match exploration::action_pick_lock(state, door_id, char_name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn listen(id: &str, state: &mut GameState, is_demihuman: bool) -> GMResponse {
    match exploration::action_listen_at_door(state, is_demihuman) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn rest(id: &str, state: &mut GameState) -> GMResponse {
    match exploration::action_rest(state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

// =============================================================================
// Wilderness
// =============================================================================

pub(super) fn enter_wilderness(id: &str, state: &mut GameState, terrain: Terrain) -> GMResponse {
    match wilderness::action_enter_wilderness(state, terrain) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn leave_wilderness(id: &str, state: &mut GameState) -> GMResponse {
    match wilderness::action_leave_wilderness(state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn add_hex(id: &str, state: &mut GameState, x: i32, y: i32, terrain: Terrain) -> GMResponse {
    match wilderness::action_add_hex(state, x, y, terrain) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn travel(id: &str, state: &mut GameState, x: i32, y: i32) -> GMResponse {
    match wilderness::action_travel(state, x, y) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn orient(id: &str, state: &mut GameState) -> GMResponse {
    match wilderness::action_orient(state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn forage(id: &str, state: &mut GameState) -> GMResponse {
    match wilderness::action_forage(state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn hunt(id: &str, state: &mut GameState) -> GMResponse {
    match wilderness::action_hunt(state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

// =============================================================================
// Inventory
// =============================================================================

pub(super) fn buy(id: &str, state: &mut GameState, character: &str, item_name: &str) -> GMResponse {
    match crate::engine::inventory::action_buy(state, character, item_name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn drop(id: &str, state: &mut GameState, character: &str, item_name: &str) -> GMResponse {
    match crate::engine::inventory::action_drop(state, character, item_name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn equip(id: &str, state: &mut GameState, character: &str, item_name: &str) -> GMResponse {
    match crate::engine::inventory::action_equip(state, character, item_name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn list_equipment(id: &str, state: &GameState, category: &Option<String>) -> GMResponse {
    let result = crate::engine::inventory::action_list_equipment();

    let filter = category.as_deref().map(|c| c.to_lowercase());

    let (weapons, armour, gear, ammunition) = match filter.as_deref() {
        Some("weapons" | "weapon") => (result.weapons, vec![], vec![], vec![]),
        Some("armour" | "armor") => (vec![], result.armour, vec![], vec![]),
        Some("gear" | "adventuring gear") => (vec![], vec![], result.gear, vec![]),
        Some("ammunition" | "ammo") => (vec![], vec![], vec![], result.ammunition),
        Some(unknown) => {
            return GMResponse::err(
                id,
                format!(
                    "unknown category '{}'. Valid categories: weapons, armour, gear, ammunition",
                    unknown
                ),
                state.mode.clone(),
            );
        }
        None => (result.weapons, result.armour, result.gear, result.ammunition),
    };

    let total = weapons.len() + armour.len() + gear.len() + ammunition.len();
    let message = format!("{} items available for purchase.", total);

    ok_with_typed_data(
        id,
        state,
        message,
        serde_json::json!({
            "weapons": weapons,
            "armour": armour,
            "gear": gear,
            "ammunition": ammunition,
            "total": total,
        }),
    )
}

pub(super) fn loot(id: &str, state: &mut GameState, character: &str, item_name: &str, explicit_gp: Option<u32>) -> GMResponse {
    match crate::engine::inventory::action_loot(state, character, item_name, explicit_gp) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}
