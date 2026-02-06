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

pub(super) fn roll_encounter(id: &str, state: &mut GameState) -> GMResponse {
    use crate::rules::encounter as encounter_tables;

    let mut rng = rand::thread_rng();
    let table_roll: u32 = rand::Rng::gen_range(&mut rng, 1..=20);

    match state.mode {
        GameMode::Exploration => {
            let level = state.dungeon_level;
            if level == 0 {
                return GMResponse::err(
                    id,
                    "dungeon level not set. Use EnterDungeon first.",
                    state.mode.clone(),
                );
            }
            let entry = match encounter_tables::dungeon_encounter_d40(level, table_roll) {
                Some(e) => e,
                None => return GMResponse::err(id, "no encounter found for this roll.", state.mode.clone()),
            };
            let num_appearing = match roll_number_appearing(&entry.number) {
                Ok(n) => n,
                Err(e) => return GMResponse::err(id, e, state.mode.clone()),
            };
            let seq = encounter_engine::begin_encounter_dungeon();

            GMResponse::ok_with_data(
                id,
                format!(
                    "ENCOUNTER — Dungeon Level {}\n\
                     Table roll: {} → {}\n\
                     Number appearing: {} → {}\n\
                     Surprise: party {}, monsters {} — {}\n\
                     Distance: {}' feet",
                    level, table_roll, entry.name,
                    entry.number, num_appearing,
                    seq.party_surprise_roll, seq.monster_surprise_roll, seq.surprise,
                    seq.distance,
                ),
                state.mode.clone(),
                serde_json::json!({
                    "context": "dungeon",
                    "level": level,
                    "table_roll": table_roll,
                    "monster_name": entry.name,
                    "number_notation": entry.number,
                    "number_appearing": num_appearing,
                    "party_surprise_roll": seq.party_surprise_roll,
                    "monster_surprise_roll": seq.monster_surprise_roll,
                    "surprise": format!("{}", seq.surprise),
                    "distance": seq.distance,
                }),
            )
        }
        GameMode::Wilderness => {
            let ws = match state.wilderness.as_ref() {
                Some(w) => w,
                None => return GMResponse::err(id, "no wilderness state.", state.mode.clone()),
            };
            let hex = match ws.current_hex() {
                Some(h) => h,
                None => return GMResponse::err(id, "no current hex.", state.mode.clone()),
            };
            let terrain = hex.terrain;
            let entry = match encounter_tables::wilderness_encounter_simple(terrain, table_roll) {
                Some(e) => e,
                None => return GMResponse::err(id, "no encounter found for this terrain.", state.mode.clone()),
            };
            let num_appearing = match roll_number_appearing(&entry.number) {
                Ok(n) => n,
                Err(e) => return GMResponse::err(id, e, state.mode.clone()),
            };
            let seq = encounter_engine::begin_encounter_wilderness();

            GMResponse::ok_with_data(
                id,
                format!(
                    "ENCOUNTER — Wilderness ({})\n\
                     Table roll: {} → {}\n\
                     Number appearing: {} → {}\n\
                     Surprise: party {}, monsters {} — {}\n\
                     Distance: {} yards",
                    terrain.name(), table_roll, entry.name,
                    entry.number, num_appearing,
                    seq.party_surprise_roll, seq.monster_surprise_roll, seq.surprise,
                    seq.distance,
                ),
                state.mode.clone(),
                serde_json::json!({
                    "context": "wilderness",
                    "terrain": terrain.name(),
                    "table_roll": table_roll,
                    "monster_name": entry.name,
                    "number_notation": entry.number,
                    "number_appearing": num_appearing,
                    "party_surprise_roll": seq.party_surprise_roll,
                    "monster_surprise_roll": seq.monster_surprise_roll,
                    "surprise": format!("{}", seq.surprise),
                    "distance": seq.distance,
                }),
            )
        }
        _ => GMResponse::err(
            id,
            "encounter requires exploration or wilderness mode.",
            state.mode.clone(),
        ),
    }
}

/// Roll number appearing. Handles both dice notation ("2d4") and plain integers ("1").
fn roll_number_appearing(notation: &str) -> Result<i32, String> {
    if let Ok(n) = notation.parse::<i32>() {
        return Ok(n);
    }
    crate::dice::roll_str(notation)
        .map(|r| r.total)
        .map_err(|e| format!("bad dice expr '{}': {}", notation, e))
}

pub(super) fn evade(id: &str, state: &GameState, monster_count: u32, monster_movement: u32) -> GMResponse {
    let party_size = state.party.members.iter().filter(|c| c.is_alive()).count() as u32;
    if party_size == 0 {
        return GMResponse::err(id, "no living party members.", state.mode.clone());
    }
    let party_movement = state.party.members.iter()
        .filter(|c| c.is_alive())
        .map(|c| c.movement_rate)
        .min()
        .unwrap_or(120);
    let result = encounter_engine::attempt_evasion(
        party_size, party_movement, monster_count, monster_movement,
    );
    let escaped = matches!(result, encounter_engine::EvasionResult::Escaped);
    GMResponse::ok_with_data(
        id,
        format!(
            "Party ({} members, {}' movement) vs {} monsters ({}' movement)\n{}",
            party_size, party_movement, monster_count, monster_movement, result,
        ),
        state.mode.clone(),
        serde_json::json!({
            "escaped": escaped,
            "party_size": party_size,
            "party_movement": party_movement,
            "monster_count": monster_count,
            "monster_movement": monster_movement,
        }),
    )
}
