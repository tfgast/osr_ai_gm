use crate::engine::combat::{self, SpawnEncounterParams};
use crate::engine::encounter;
use crate::gmapi::protocol::GMResponse;
use crate::persist::GameState;
use super::ok_with_typed_data;

pub(super) fn spawn_encounter(
    id: &str,
    state: &mut GameState,
    params: &crate::gmapi::protocol::EncounterParams,
) -> GMResponse {
    match combat::action_spawn_encounter(
        state,
        &SpawnEncounterParams {
            name: &params.name,
            count: params.count,
            hit_dice: &params.hit_dice,
            ac: params.ac,
            hp: params.hp,
            damage: &params.damage,
            morale: params.morale,
            distance: params.distance,
            xp_value: params.xp_value,
            undead: params.undead,
            immune_to_normal_weapons: params.immune_to_normal_weapons,
        },
    ) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn add_monster(
    id: &str,
    state: &mut GameState,
    params: &crate::gmapi::protocol::EncounterParams,
) -> GMResponse {
    match combat::action_add_monster(
        state,
        &SpawnEncounterParams {
            name: &params.name,
            count: params.count,
            hit_dice: &params.hit_dice,
            ac: params.ac,
            hp: params.hp,
            damage: &params.damage,
            morale: params.morale,
            distance: params.distance,
            xp_value: params.xp_value,
            undead: params.undead,
            immune_to_normal_weapons: params.immune_to_normal_weapons,
        },
    ) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn roll_initiative(id: &str, state: &mut GameState) -> GMResponse {
    match combat::action_roll_initiative(state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn next_phase(id: &str, state: &mut GameState) -> GMResponse {
    match combat::action_next_phase(state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
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
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn monster_attack(
    id: &str,
    state: &mut GameState,
    monster_idx: usize,
    char_name: &str,
) -> GMResponse {
    match combat::action_monster_attack(state, monster_idx, char_name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn check_morale(id: &str, state: &mut GameState) -> GMResponse {
    match combat::action_morale(state, None) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn turn_undead(
    id: &str,
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
) -> GMResponse {
    match combat::action_turn_undead(state, char_name, monster_idx) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn close(
    id: &str,
    state: &mut GameState,
    char_name: &str,
    feet: Option<u32>,
) -> GMResponse {
    match combat::action_close(state, char_name, feet) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

/// Resolve an optional character name to a concrete name.
/// If None, auto-selects when there is exactly one alive party member.
fn resolve_character(state: &GameState, char_name: Option<&str>) -> Result<String, String> {
    match char_name {
        Some(name) => Ok(name.to_string()),
        None => {
            let alive: Vec<&str> = state.party.members.iter()
                .filter(|c| c.is_alive())
                .map(|c| c.name.as_str())
                .collect();
            match alive.len() {
                0 => Err("no alive party members.".to_string()),
                1 => Ok(alive[0].to_string()),
                _ => Err(format!(
                    "multiple alive party members — specify which character: {}",
                    alive.join(", ")
                )),
            }
        }
    }
}

pub(super) fn retreat(id: &str, state: &mut GameState, char_name: Option<&str>) -> GMResponse {
    let name = match resolve_character(state, char_name) {
        Ok(n) => n,
        Err(e) => return GMResponse::err(id, e, state.mode),
    };
    match combat::action_retreat(state, &name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn fighting_withdrawal(id: &str, state: &mut GameState, char_name: Option<&str>) -> GMResponse {
    let name = match resolve_character(state, char_name) {
        Ok(n) => n,
        Err(e) => return GMResponse::err(id, e, state.mode),
    };
    match combat::action_fighting_withdrawal(state, &name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn query_combat_log(id: &str, state: &GameState) -> GMResponse {
    match combat::action_query_combat_log(state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn declare_spell(
    id: &str,
    state: &mut GameState,
    char_name: &str,
    spell_name: &str,
) -> GMResponse {
    match combat::action_declare_spell(state, char_name, spell_name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn cast_spell(id: &str, state: &mut GameState, char_name: &str) -> GMResponse {
    match combat::action_cast_spell(state, char_name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn end_combat(id: &str, state: &mut GameState, skip_xp: bool) -> GMResponse {
    match combat::action_end_combat(state, skip_xp) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
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
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
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
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn spawn_placed(
    id: &str,
    state: &mut GameState,
    distance: u32,
    name: Option<&str>,
) -> GMResponse {
    match combat::action_spawn_placed(state, distance, name) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn spawn_npc_party(
    id: &str,
    state: &mut GameState,
    party_type: &str,
    distance: u32,
) -> GMResponse {
    match encounter::action_spawn_npc_party(state, party_type, distance) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn roll_encounter(id: &str, state: &mut GameState) -> GMResponse {
    match encounter::action_roll_encounter(state) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn evade(
    id: &str,
    state: &GameState,
    monster_count: u32,
    monster_movement: u32,
) -> GMResponse {
    match encounter::action_evade(state, monster_count, monster_movement) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn roll_surprise(id: &str, state: &GameState) -> GMResponse {
    match encounter::action_roll_surprise(state) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn set_helpless(
    id: &str,
    state: &mut GameState,
    monster_idx: usize,
    helpless: bool,
) -> GMResponse {
    match combat::action_set_helpless(state, monster_idx, helpless) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn kill(
    id: &str,
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
) -> GMResponse {
    match combat::action_coup_de_grace(state, char_name, monster_idx) {
        Ok(result) => ok_with_typed_data(id, state, result.message.clone(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn roll_reaction(id: &str, state: &GameState, char_name: &str) -> GMResponse {
    match encounter::action_roll_reaction(state, char_name) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}
