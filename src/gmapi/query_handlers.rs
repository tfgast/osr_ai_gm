use crate::engine::{combat, exploration, lookup, party, wilderness_engine};
use crate::gmapi::protocol::GMResponse;
use crate::persist::GameState;
use crate::rules::alignment::Alignment;
use crate::rules::class::Class;
use crate::rules::encumbrance;
use super::ok_with_typed_data;
use serde::Serialize;

pub(super) fn query_state(id: &str, state: &GameState) -> GMResponse {
    let data = serde_json::json!({
        "mode": state.mode.to_string(),
        "turn": state.turn(),
        "dungeon_level": state.dungeon_level,
        "party_size": state.party.members.len(),
        "has_combat": state.combat.is_some(),
        "has_dungeon": state.dungeon.is_some(),
        "has_wilderness": state.wilderness.is_some(),
        "notes": state.notes,
    });
    GMResponse::ok_with_data(id, "game state summary", state.mode.clone(), data)
}

pub(super) fn query_combat(id: &str, state: &GameState) -> GMResponse {
    match &state.combat {
        Some(combat_state) => {
            let status = combat::combat_status(combat_state, &state.party.members);
            let monsters: Vec<serde_json::Value> = combat_state.monsters.iter().enumerate().map(|(i, m)| {
                serde_json::json!({
                    "index": i,
                    "name": m.name,
                    "hp": m.hp,
                    "max_hp": m.max_hp,
                    "ac": m.ac,
                    "alive": m.is_alive(),
                })
            }).collect();
            GMResponse::ok_with_data(
                id, status, state.mode.clone(),
                serde_json::json!({
                    "round": combat_state.round,
                    "distance": combat_state.distance,
                    "party_initiative": combat_state.party_initiative,
                    "monster_initiative": combat_state.monster_initiative,
                    "monsters": monsters,
                }),
            )
        }
        None => GMResponse::err(id, "no active combat.", state.mode.clone()),
    }
}

pub(super) fn query_exploration(id: &str, state: &GameState) -> GMResponse {
    let time = match &state.time {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match &state.dungeon {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    let status = exploration::exploration_status(time, dungeon);
    GMResponse::ok_with_data(
        id, status, state.mode.clone(),
        serde_json::json!({
            "dungeon_level": dungeon.level,
            "current_room": dungeon.current_room,
            "total_turns": time.total_turns,
            "has_light": time.has_light(),
        }),
    )
}

pub(super) fn query_wilderness(id: &str, state: &GameState) -> GMResponse {
    let ws = match &state.wilderness {
        Some(w) => w,
        None => return GMResponse::err(id, "not in wilderness mode.", state.mode.clone()),
    };
    let party_movement = state.party.members.iter()
        .filter(|c| c.is_alive())
        .map(|c| c.movement_rate)
        .min()
        .unwrap_or(120);
    let status = wilderness_engine::wilderness_status(ws, &state.party, party_movement);
    GMResponse::ok_with_data(
        id, status, state.mode.clone(),
        serde_json::json!({
            "current_x": ws.current_x,
            "current_y": ws.current_y,
            "travel_day": ws.travel_day,
            "lost": ws.lost,
        }),
    )
}

pub(super) fn query_encumbrance(id: &str, state: &GameState, char_name: &str) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let item_weights: Vec<u32> = character.inventory.iter()
        .map(|item| (item.weight * 10.0) as u32) // weight is in pounds, convert to coins (10 cn = 1 lb)
        .collect();
    let total = encumbrance::total_weight(&item_weights, character.gold_gp);
    let level = encumbrance::encumbrance_level(total);
    let movement = encumbrance::movement_rate(total);
    GMResponse::ok_with_data(
        id,
        format!("{}: {} cn total, {} (movement {}').",
            character.name, total, level.name(), movement),
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "total_weight_cn": total,
            "encumbrance_level": level.name(),
            "movement_rate": movement,
            "max_capacity": encumbrance::MAX_CAPACITY_CN,
        }),
    )
}

// =============================================================================
// Party queries
// =============================================================================

#[derive(Serialize)]
struct QueryPartyMemberData {
    name: String,
    class: Class,
    level: u32,
    hp: i32,
    max_hp: i32,
    ac: i32,
    thac0: u32,
    xp: u64,
    alive: bool,
    alignment: Alignment,
    movement_rate: u32,
}

#[derive(Serialize)]
struct QueryPartyData {
    members: Vec<QueryPartyMemberData>,
}

fn query_party_member_data(member: &party::results::PartyMemberSummary) -> QueryPartyMemberData {
    QueryPartyMemberData {
        name: member.name.clone(),
        class: member.class,
        level: member.level,
        hp: member.hp,
        max_hp: member.max_hp,
        ac: member.ac,
        thac0: member.thac0,
        xp: member.xp,
        alive: member.alive,
        alignment: member.alignment,
        movement_rate: member.movement_rate,
    }
}

pub(super) fn query_party(id: &str, state: &GameState) -> GMResponse {
    match party::action_query_party(state) {
        Ok(result) => {
            let members: Vec<QueryPartyMemberData> = result
                .members
                .iter()
                .map(query_party_member_data)
                .collect();

            let message = if members.is_empty() {
                "no party members.".to_string()
            } else {
                format!("{} party members.", members.len())
            };
            ok_with_typed_data(
                id,
                state,
                message,
                QueryPartyData { members },
            )
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

// =============================================================================
// Class queries
// =============================================================================

#[derive(Serialize)]
struct ListClassesData {
    classes: Vec<party::results::ClassSummary>,
}

#[derive(Serialize)]
struct EligibleAbilitiesData {
    #[serde(rename = "STR")]
    str_score: i32,
    #[serde(rename = "INT")]
    int_score: i32,
    #[serde(rename = "WIS")]
    wis_score: i32,
    #[serde(rename = "DEX")]
    dex_score: i32,
    #[serde(rename = "CON")]
    con_score: i32,
    #[serde(rename = "CHA")]
    cha_score: i32,
}

#[derive(Serialize)]
struct EligibleClassesData {
    abilities: EligibleAbilitiesData,
    eligible: Vec<Class>,
    count: usize,
}

pub(super) fn list_classes(id: &str, state: &GameState) -> GMResponse {
    match party::action_list_classes() {
        Ok(result) => ok_with_typed_data(
            id,
            state,
            format!("{} character classes available.", result.classes.len()),
            ListClassesData {
                classes: result.classes,
            },
        ),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn eligible_classes(id: &str, state: &GameState, abilities: &[i32; 6]) -> GMResponse {
    match party::action_eligible_classes(*abilities) {
        Ok(result) => {
            let eligible_count = result.eligible.len();
            ok_with_typed_data(
                id,
                state,
                format!(
                    "abilities STR {} INT {} WIS {} DEX {} CON {} CHA {}: {} eligible class(es).",
                    result.abilities[0],
                    result.abilities[1],
                    result.abilities[2],
                    result.abilities[3],
                    result.abilities[4],
                    result.abilities[5],
                    eligible_count
                ),
                EligibleClassesData {
                    abilities: EligibleAbilitiesData {
                        str_score: result.abilities[0],
                        int_score: result.abilities[1],
                        wis_score: result.abilities[2],
                        dex_score: result.abilities[3],
                        con_score: result.abilities[4],
                        cha_score: result.abilities[5],
                    },
                    eligible: result.eligible,
                    count: eligible_count,
                },
            )
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

// =============================================================================
// Lookup & reference
// =============================================================================

pub(super) fn lookup_item(id: &str, state: &GameState, name: &str) -> GMResponse {
    match lookup::action_lookup_item(name) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result.api_payload()),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn search_items(id: &str, state: &GameState, query: &str) -> GMResponse {
    match lookup::action_search_items(query) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result.api_payload()),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn lookup_treasure_type(id: &str, state: &GameState, letter: &str) -> GMResponse {
    match lookup::action_lookup_treasure_type(letter) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result.api_payload()),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

pub(super) fn roll_treasure(id: &str, state: &GameState, letter: &str) -> GMResponse {
    match lookup::action_roll_treasure(letter) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result),
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
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result.api_payload()),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}
