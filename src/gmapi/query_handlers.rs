use crate::engine::{combat, exploration, wilderness_engine};
use crate::gmapi::protocol::GMResponse;
use crate::persist::GameState;
use crate::rules::encumbrance;

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

pub(super) fn query_party(id: &str, state: &GameState) -> GMResponse {
    if state.party.members.is_empty() {
        return GMResponse::ok_with_data(
            id, "no party members.", state.mode.clone(),
            serde_json::json!({ "members": [] }),
        );
    }
    let members: Vec<serde_json::Value> = state.party.members.iter().map(|c| {
        serde_json::json!({
            "name": c.name,
            "class": c.class.name(),
            "level": c.level,
            "hp": c.hp,
            "max_hp": c.max_hp,
            "ac": c.ac,
            "thac0": c.thac0,
            "xp": c.xp,
            "alive": c.is_alive(),
            "alignment": c.alignment.name(),
            "movement_rate": c.movement_rate,
        })
    }).collect();
    GMResponse::ok_with_data(
        id, format!("{} party members.", members.len()), state.mode.clone(),
        serde_json::json!({ "members": members }),
    )
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
