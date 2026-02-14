use crate::engine::combat::{self, SpawnEncounterParams};
use crate::engine::encounter;
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

pub(super) fn spawn_encounter(
    id: &str,
    state: &mut GameState,
    params: &crate::gmapi::protocol::EncounterParams,
) -> GMResponse {
    let hit_dice = params.hit_dice.to_string();
    match combat::action_spawn_encounter(
        state,
        &SpawnEncounterParams {
            name: &params.name,
            count: params.count,
            hit_dice: &hit_dice,
            ac: params.ac,
            hp: params.hp,
            damage: &params.damage,
            morale: params.morale,
            distance: params.distance,
            xp_value: params.xp_value,
        },
    ) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn roll_initiative(id: &str, state: &mut GameState) -> GMResponse {
    match combat::action_roll_initiative(state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn attack(
    id: &str,
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
    weapon_name: &str,
) -> GMResponse {
    match combat::action_attack(state, char_name, monster_idx, weapon_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn monster_attack(
    id: &str,
    state: &mut GameState,
    monster_idx: usize,
    char_name: &str,
) -> GMResponse {
    match combat::action_monster_attack(state, monster_idx, char_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn check_morale(id: &str, state: &mut GameState) -> GMResponse {
    match combat::action_morale(state, None) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn turn_undead(
    id: &str,
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
) -> GMResponse {
    match combat::action_turn_undead(state, char_name, monster_idx) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn close(
    id: &str,
    state: &mut GameState,
    char_name: &str,
    feet: Option<u32>,
) -> GMResponse {
    match combat::action_close(state, char_name, feet) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn retreat(id: &str, state: &mut GameState, char_name: &str) -> GMResponse {
    match combat::action_retreat(state, char_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn fighting_withdrawal(id: &str, state: &mut GameState, char_name: &str) -> GMResponse {
    match combat::action_fighting_withdrawal(state, char_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn query_combat_log(id: &str, state: &GameState) -> GMResponse {
    match combat::action_query_combat_log(state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn declare_spell(
    id: &str,
    state: &mut GameState,
    char_name: &str,
    spell_name: &str,
) -> GMResponse {
    match combat::action_declare_spell(state, char_name, spell_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn end_combat(id: &str, state: &mut GameState) -> GMResponse {
    match combat::action_end_combat(state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn backstab(
    id: &str,
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
    weapon_name: &str,
) -> GMResponse {
    match combat::action_backstab(state, char_name, monster_idx, weapon_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn spawn_monster(
    id: &str,
    state: &mut GameState,
    name: &str,
    count: u32,
    distance: u32,
) -> GMResponse {
    match combat::action_spawn_monster(state, name, count, distance) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn spawn_npc_party(
    id: &str,
    state: &mut GameState,
    party_type: &str,
    distance: u32,
) -> GMResponse {
    match encounter::action_spawn_npc_party(state, party_type, distance) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn roll_encounter(id: &str, state: &mut GameState) -> GMResponse {
    match encounter::action_roll_encounter(state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn evade(
    id: &str,
    state: &GameState,
    monster_count: u32,
    monster_movement: u32,
) -> GMResponse {
    match encounter::action_evade(state, monster_count, monster_movement) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn roll_surprise(id: &str, state: &GameState) -> GMResponse {
    match encounter::action_roll_surprise(state) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.api_message(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn set_helpless(
    id: &str,
    state: &mut GameState,
    monster_idx: usize,
    helpless: bool,
) -> GMResponse {
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    match combat::set_monster_helpless(combat_state, monster_idx, helpless) {
        Ok(msg) => GMResponse::ok_with_data(
            id,
            msg,
            state.mode.clone(),
            serde_json::json!({
                "monster_idx": monster_idx,
                "helpless": helpless,
            }),
        ),
        Err(e) => GMResponse::err(id, e, state.mode.clone()),
    }
}

pub(super) fn kill(
    id: &str,
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
) -> GMResponse {
    match combat::action_coup_de_grace(state, char_name, monster_idx) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn roll_reaction(id: &str, state: &GameState, char_name: &str) -> GMResponse {
    match encounter::action_roll_reaction(state, char_name) {
        Ok(result) => ok_with_typed_data(id, &state.mode, result.api_message(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}
