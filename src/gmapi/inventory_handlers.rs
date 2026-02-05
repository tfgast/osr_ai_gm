use crate::gmapi::protocol::GMResponse;
use crate::model::Item;
use crate::persist::GameState;
use crate::rules::equipment;

/// Look up an item across all equipment tables.
/// Returns (canonical_name, cost_gp, weight).
fn find_buyable(name: &str) -> Option<(String, u32, f32)> {
    if let Some(w) = equipment::find_weapon(name) {
        return Some((w.name.clone(), w.cost_gp(), w.weight() as f32));
    }
    if let Some(a) = equipment::find_armour(name) {
        if a.cost_gp() > 0 {
            return Some((a.name.clone(), a.cost_gp(), a.weight() as f32));
        }
    }
    if let Some(g) = equipment::find_gear(name) {
        return Some((g.name.clone(), g.cost_gp(), 0.0));
    }
    if let Some(a) = equipment::ammunition().iter().find(|a| a.name.eq_ignore_ascii_case(name)) {
        return Some((a.name.clone(), a.cost_gp(), 0.0));
    }
    None
}

/// Suggest equipment names that contain the query as a substring (case-insensitive).
fn suggest_equipment(query: &str) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let mut suggestions: Vec<String> = Vec::new();
    for w in equipment::weapons() {
        if w.name.to_lowercase().contains(&query_lower) {
            suggestions.push(w.name.clone());
        }
    }
    for a in equipment::armour() {
        if a.cost_gp() > 0 && a.name.to_lowercase().contains(&query_lower) {
            suggestions.push(a.name.clone());
        }
    }
    for g in equipment::gear() {
        if g.name.to_lowercase().contains(&query_lower) {
            suggestions.push(g.name.clone());
        }
    }
    for a in equipment::ammunition() {
        if a.name.to_lowercase().contains(&query_lower) {
            suggestions.push(a.name.clone());
        }
    }
    suggestions.truncate(3);
    suggestions
}

pub(super) fn buy(id: &str, state: &mut GameState, character: &str, item_name: &str) -> GMResponse {
    let (canonical_name, cost, weight) = match find_buyable(item_name) {
        Some(info) => info,
        None => {
            let suggestions = suggest_equipment(item_name);
            return if suggestions.is_empty() {
                GMResponse::err(id, format!("unknown item '{}'. Check equipment tables.", item_name), state.mode.clone())
            } else {
                GMResponse::err(id, format!("unknown item '{}'. Did you mean: {}?", item_name, suggestions.join(", ")), state.mode.clone())
            };
        }
    };

    let character = match state.party.find_member_mut(character) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", character), state.mode.clone()),
    };

    if character.gold_gp < cost {
        return GMResponse::err(
            id,
            format!("{} has {} gp but {} costs {} gp.", character.name, character.gold_gp, canonical_name, cost),
            state.mode.clone(),
        );
    }

    character.gold_gp -= cost;
    character.inventory.push(Item::new(&canonical_name, weight, cost));

    GMResponse::ok_with_data(
        id,
        format!("{} buys {} for {} gp. ({} gp remaining)", character.name, canonical_name, cost, character.gold_gp),
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "item": canonical_name,
            "cost_gp": cost,
            "gold_remaining": character.gold_gp,
        }),
    )
}

pub(super) fn drop(id: &str, state: &mut GameState, character: &str, item_name: &str) -> GMResponse {
    let character = match state.party.find_member_mut(character) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", character), state.mode.clone()),
    };

    let idx = character.inventory.iter()
        .position(|i| i.name.eq_ignore_ascii_case(item_name));

    match idx {
        Some(i) => {
            let dropped = character.inventory.remove(i);
            GMResponse::ok_with_data(
                id,
                format!("{} drops {}.", character.name, dropped.name),
                state.mode.clone(),
                serde_json::json!({
                    "character": character.name,
                    "item": dropped.name,
                }),
            )
        }
        None => GMResponse::err(
            id,
            format!("{} does not have '{}'.", character.name, item_name),
            state.mode.clone(),
        ),
    }
}

pub(super) fn equip(id: &str, state: &mut GameState, character: &str, item_name: &str) -> GMResponse {
    let character = match state.party.find_member_mut(character) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", character), state.mode.clone()),
    };

    let idx = character.inventory.iter()
        .position(|i| i.name.eq_ignore_ascii_case(item_name));

    let idx = match idx {
        Some(i) => i,
        None => return GMResponse::err(
            id,
            format!("{} does not have '{}'.", character.name, item_name),
            state.mode.clone(),
        ),
    };

    let was_equipped = character.inventory[idx].equipped;
    character.inventory[idx].equipped = !was_equipped;

    let action = if was_equipped { "unequips" } else { "equips" };
    let item_display = character.inventory[idx].name.clone();

    // Recalculate AC from equipped armour
    let armour_ac = character.inventory.iter()
        .filter(|i| i.equipped)
        .filter_map(|i| equipment::find_armour(&i.name))
        .filter(|a| !a.is_shield())
        .map(|a| a.ac_descending())
        .min()
        .unwrap_or(9);

    let has_shield = character.inventory.iter()
        .any(|i| i.equipped && equipment::find_armour(&i.name).map(|a| a.is_shield()).unwrap_or(false));

    let dex_mod = crate::rules::ability::dex_ac_mod(character.abilities.dexterity);
    character.ac = equipment::calculate_ac(armour_ac, has_shield, dex_mod);

    GMResponse::ok_with_data(
        id,
        format!("{} {} {}. (AC {})", character.name, action, item_display, character.ac),
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "item": item_display,
            "action": action,
            "ac": character.ac,
        }),
    )
}

pub(super) fn loot(id: &str, state: &mut GameState, character: &str, item_name: &str, explicit_gp: Option<u32>) -> GMResponse {
    // If in a dungeon, validate item exists in current room's placed treasure
    let room_gp = if let Some(dungeon) = &mut state.dungeon {
        let current = match dungeon.current_room {
            Some(id) => id,
            None => return GMResponse::err(id, "no current room.", state.mode.clone()),
        };
        let room = match dungeon.find_room_mut(current) {
            Some(r) => r,
            None => return GMResponse::err(id, "current room not found.", state.mode.clone()),
        };
        let idx = room.placed_treasure.iter().position(|t| {
            !t.taken && t.description.eq_ignore_ascii_case(item_name)
        });
        match idx {
            Some(i) => {
                let gp = room.placed_treasure[i].gp_value;
                room.placed_treasure[i].taken = true;
                Some(gp)
            }
            None => return GMResponse::err(
                id,
                format!("no lootable item '{}' in this room.", item_name),
                state.mode.clone(),
            ),
        }
    } else {
        None
    };

    let value_gp = explicit_gp.unwrap_or(room_gp.unwrap_or(0) as u32);

    let character = match state.party.find_member_mut(character) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", character), state.mode.clone()),
    };

    character.inventory.push(Item::new(item_name, 0.0, value_gp));

    let mut msg = format!("{} picks up {}.", character.name, item_name);
    if value_gp > 0 {
        msg.push_str(&format!(" (worth {} gp)", value_gp));
    }

    GMResponse::ok_with_data(
        id,
        msg,
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "item": item_name,
            "value_gp": value_gp,
        }),
    )
}
