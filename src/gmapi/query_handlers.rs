use crate::engine::{combat, exploration, lookup, party, rumor, wilderness_engine};
use crate::gmapi::protocol::GMResponse;
use crate::persist::GameState;
use crate::rules::alignment::Alignment;
use crate::rules::class::Class;
use crate::rules::encumbrance;
use crate::rules::spell_data;
use crate::state::effect::ActiveEffect;
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
    GMResponse::ok_with_data(id, "game state summary", state.mode, data)
}

/// Serialize an ActiveEffect to a JSON value for structured data payloads.
fn effect_to_json(e: &ActiveEffect) -> serde_json::Value {
    serde_json::json!({
        "id": e.id,
        "name": e.name,
        "source": e.source,
        "duration": format!("{}", e.duration),
        "modifiers": e.modifiers.iter().map(|m| serde_json::json!({
            "stat": format!("{}", m.stat),
            "value": m.value,
        })).collect::<Vec<_>>(),
        "notes": e.notes,
    })
}

pub(super) fn query_combat(id: &str, state: &GameState) -> GMResponse {
    match &state.combat {
        Some(combat_state) => {
            let mut status = combat::combat_status(combat_state, &state.party.members);
            let monsters: Vec<serde_json::Value> = combat_state.monsters.iter().enumerate().map(|(i, m)| {
                let mut val = serde_json::json!({
                    "index": i,
                    "name": m.name,
                    "hp": m.hp,
                    "max_hp": m.max_hp,
                    "ac": m.ac,
                    "alive": m.is_alive(),
                });
                if !m.effects.is_empty() {
                    val["effects"] = serde_json::json!(
                        m.effects.iter().map(effect_to_json).collect::<Vec<_>>()
                    );
                }
                val
            }).collect();

            // Build party effects for data
            let party_effects: Vec<serde_json::Value> = state.party.members.iter().filter_map(|c| {
                if c.effects.is_empty() {
                    None
                } else {
                    Some(serde_json::json!({
                        "character": c.name,
                        "effects": c.effects.iter().map(effect_to_json).collect::<Vec<_>>(),
                    }))
                }
            }).collect();

            // Build area/global effects
            let global_effects: Vec<serde_json::Value> = state.effects.iter().map(effect_to_json).collect();

            // Append effects to status message
            let has_any_effects = !party_effects.is_empty()
                || combat_state.monsters.iter().any(|m| !m.effects.is_empty())
                || !state.effects.is_empty();
            if has_any_effects {
                status.push_str("\n-- Effects --");
                for c in &state.party.members {
                    if !c.effects.is_empty() {
                        let summaries: Vec<String> = c.effects.iter().map(|e| e.summary_line()).collect();
                        status.push_str(&format!("\n  {}: {}", c.name, summaries.join(", ")));
                    }
                }
                for (i, m) in combat_state.monsters.iter().enumerate() {
                    if !m.effects.is_empty() {
                        let helpless_tag = if m.is_helpless() { " [HELPLESS]" } else { "" };
                        let summaries: Vec<String> = m.effects.iter().map(|e| e.summary_line()).collect();
                        status.push_str(&format!("\n  {} #{}: {}{}", m.name, i, summaries.join(", "), helpless_tag));
                    }
                }
                if !state.effects.is_empty() {
                    status.push_str("\n-- Area Effects --");
                    for e in &state.effects {
                        status.push_str(&format!("\n  {} (source: {})", e.summary_line(), e.source));
                    }
                }
            }

            let mut data = serde_json::json!({
                "round": combat_state.round,
                "distance": combat_state.distance,
                "party_initiative": combat_state.party_initiative,
                "monster_initiative": combat_state.monster_initiative,
                "monsters": monsters,
            });
            if !party_effects.is_empty() {
                data["party_effects"] = serde_json::json!(party_effects);
            }
            if !global_effects.is_empty() {
                data["area_effects"] = serde_json::json!(global_effects);
            }

            GMResponse::ok_with_data(id, status, state.mode, data)
        }
        None => GMResponse::err(id, "no active combat.", state.mode),
    }
}

pub(super) fn query_exploration(id: &str, state: &GameState) -> GMResponse {
    let time = match &state.time {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode),
    };
    let dungeon = match &state.dungeon {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode),
    };
    let mut status = exploration::exploration_status(time, dungeon);

    // Brief summary of active effects (turn-based are most relevant in exploration)
    let mut active: Vec<String> = Vec::new();
    for c in &state.party.members {
        for e in &c.effects {
            active.push(format!("{} on {} ({})", e.name, c.name, e.duration));
        }
    }
    for e in &state.effects {
        active.push(format!("{} ({})", e.name, e.duration));
    }
    if !active.is_empty() {
        status.push_str(&format!("\nEffects: {}", active.join(", ")));
    }

    let mut data = serde_json::json!({
        "dungeon_level": dungeon.level,
        "current_room": dungeon.current_room,
        "total_turns": time.total_turns,
        "has_light": time.has_light(),
    });
    let all_effects: Vec<serde_json::Value> = state.party.members.iter()
        .flat_map(|c| c.effects.iter().map(effect_to_json))
        .chain(state.effects.iter().map(effect_to_json))
        .collect();
    if !all_effects.is_empty() {
        data["effects"] = serde_json::json!(all_effects);
    }

    GMResponse::ok_with_data(id, status, state.mode, data)
}

pub(super) fn query_wilderness(id: &str, state: &GameState) -> GMResponse {
    let ws = match &state.wilderness {
        Some(w) => w,
        None => return GMResponse::err(id, "not in wilderness mode.", state.mode),
    };
    let party_movement = state.party.members.iter()
        .filter(|c| c.is_alive())
        .map(|c| c.movement_rate)
        .min()
        .unwrap_or(120);
    let status = wilderness_engine::wilderness_status(ws, &state.party, party_movement);
    GMResponse::ok_with_data(
        id, status, state.mode,
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
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode),
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
        state.mode,
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
struct EffectData {
    id: u32,
    name: String,
    source: String,
    duration: String,
    modifiers: Vec<ModifierData>,
    #[serde(skip_serializing_if = "String::is_empty")]
    notes: String,
}

#[derive(Serialize)]
struct ModifierData {
    stat: String,
    value: i32,
}

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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    effects: Vec<EffectData>,
}

#[derive(Serialize)]
struct QueryPartyData {
    members: Vec<QueryPartyMemberData>,
}

fn effect_to_typed(e: &ActiveEffect) -> EffectData {
    EffectData {
        id: e.id,
        name: e.name.clone(),
        source: e.source.clone(),
        duration: format!("{}", e.duration),
        modifiers: e.modifiers.iter().map(|m| ModifierData {
            stat: format!("{}", m.stat),
            value: m.value,
        }).collect(),
        notes: e.notes.clone(),
    }
}

fn query_party_member_data(member: &party::results::PartyMemberSummary, effects: &[ActiveEffect]) -> QueryPartyMemberData {
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
        effects: effects.iter().map(effect_to_typed).collect(),
    }
}

pub(super) fn query_party(id: &str, state: &GameState) -> GMResponse {
    match party::action_query_party(state) {
        Ok(result) => {
            let members: Vec<QueryPartyMemberData> = result
                .members
                .iter()
                .map(|m| {
                    let effects = state.party.find_member(&m.name)
                        .map(|c| c.effects.as_slice())
                        .unwrap_or(&[]);
                    query_party_member_data(m, effects)
                })
                .collect();

            // Build message with effects
            let mut message = if members.is_empty() {
                "no party members.".to_string()
            } else {
                format!("{} party members.", members.len())
            };

            // Append active effects detail per character
            for m in &result.members {
                let effs = state.party.find_member(&m.name)
                    .map(|c| &c.effects)
                    .filter(|e| !e.is_empty());
                if let Some(effects) = effs {
                    message.push_str(&format!("\n  {} — Active Effects:", m.name));
                    for e in effects {
                        message.push_str(&format!("\n    {}", e.detail_lines().replace('\n', "\n    ")));
                    }
                }
            }

            ok_with_typed_data(
                id,
                state,
                message,
                QueryPartyData { members },
            )
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
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
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
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
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

// =============================================================================
// Lookup & reference
// =============================================================================

pub(super) fn lookup_item(id: &str, state: &GameState, name: &str) -> GMResponse {
    match lookup::action_lookup_item(name) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result.api_payload()),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn search_items(id: &str, state: &GameState, query: &str) -> GMResponse {
    match lookup::action_search_items(query) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result.api_payload()),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn lookup_treasure_type(id: &str, state: &GameState, letter: &str) -> GMResponse {
    match lookup::action_lookup_treasure_type(letter) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result.api_payload()),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn roll_treasure(id: &str, state: &GameState, letter: &str) -> GMResponse {
    match lookup::action_roll_treasure(letter) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

// =============================================================================
// Rumors
// =============================================================================

pub(super) fn roll_rumor(id: &str, state: &GameState, table: &str) -> GMResponse {
    match rumor::action_roll_rumor(table) {
        Ok(result) => ok_with_typed_data(id, state, result.message, result.data),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn lookup_rumor_table(id: &str, state: &GameState, table: &str) -> GMResponse {
    match rumor::action_lookup_rumor_table(table) {
        Ok(result) => ok_with_typed_data(id, state, result.message, result.data),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

pub(super) fn list_rumor_tables(id: &str, state: &GameState) -> GMResponse {
    match rumor::action_list_rumor_tables() {
        Ok(result) => {
            let msg = format!("{} rumor tables available.", result.tables.len());
            ok_with_typed_data(id, state, msg, result)
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
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
                    state.mode,
                )
            }
        }
    };

    match lookup::action_lookup_spell(name, list) {
        Ok(result) => ok_with_typed_data(id, state, result.api_message(), result.api_payload()),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SpellInfoPayload {
    pub name: String,
    pub list: String,
    pub level: u32,
    pub duration: String,
    pub save_type: String,
    pub damage_dice: String,
}

pub(super) fn spell_info(id: &str, state: &GameState, name: &str) -> GMResponse {
    let spell = match spell_data::find_spell(name, None) {
        Some(s) => s,
        None => {
            return GMResponse::err(
                id,
                format!("spell not found: '{}'.", name),
                state.mode,
            )
        }
    };

    let save_type = spell_data::spell_save_type(&spell.name);
    let damage_dice = spell_data::spell_damage_dice(&spell.name);

    let msg = format!(
        "{} ({}L{}) — save: {}, damage: {}, duration: {}",
        spell.name,
        spell.list.name(),
        spell.level,
        save_type,
        if damage_dice.is_empty() { "none" } else { &damage_dice },
        spell.duration,
    );

    let payload = SpellInfoPayload {
        name: spell.name.clone(),
        list: spell.list.name().to_string(),
        level: spell.level,
        duration: spell.duration.clone(),
        save_type,
        damage_dice,
    };

    ok_with_typed_data(id, state, msg, payload)
}
