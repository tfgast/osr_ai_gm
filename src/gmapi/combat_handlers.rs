use crate::dice;
use crate::engine::{combat, encounter};
use crate::gmapi::protocol::GMResponse;
use crate::model::{CombatState, Monster};
use crate::persist::GameState;
use crate::rules::{ability, equipment, monster, thief};
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
        &params.name,
        params.count,
        &hit_dice,
        params.ac,
        params.hp,
        &params.damage,
        params.morale,
        params.distance,
        params.xp_value,
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
    let weapon = match equipment::find_weapon(weapon_name) {
        Some(w) => w,
        None => {
            return GMResponse::err(
                id,
                format!("unknown weapon '{}'.", weapon_name),
                state.mode.clone(),
            )
        }
    };
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => {
            return GMResponse::err(
                id,
                format!("no party member named '{}'.", char_name),
                state.mode.clone(),
            )
        }
    };
    if !thief::can_backstab(character.class) {
        return GMResponse::err(
            id,
            format!(
                "{} ({}) cannot backstab.",
                character.name,
                character.class.name()
            ),
            state.mode.clone(),
        );
    }
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if monster_idx >= combat_state.monsters.len() {
        return GMResponse::err(
            id,
            format!("monster index {} out of range.", monster_idx),
            state.mode.clone(),
        );
    }
    if !combat_state.monsters[monster_idx].is_alive() {
        return GMResponse::err(
            id,
            format!(
                "{} is already dead.",
                combat_state.monsters[monster_idx].name
            ),
            state.mode.clone(),
        );
    }

    let multiplier = thief::backstab_multiplier(character.level);
    let str_mod = ability::str_melee_mod(character.abilities.strength);
    let attack_bonus = thief::BACKSTAB_ATTACK_BONUS;

    // Roll attack with backstab bonus
    let target_ac = combat_state.monsters[monster_idx].ac;
    let target_number = (character.thac0 as i32 - target_ac - attack_bonus - str_mod).clamp(2, 20);
    let attack_roll: i32 = rand::Rng::gen_range(&mut rand::thread_rng(), 1..=20);

    let hit = attack_roll == 20 || (attack_roll != 1 && attack_roll >= target_number);

    if hit {
        // Roll damage and multiply
        let base_damage = match dice::roll_str(weapon.damage_dice()) {
            Ok(r) => r.total.max(1),
            Err(_) => 1,
        };
        let total_damage = (base_damage + str_mod).max(1) * multiplier as i32;
        combat_state.monsters[monster_idx].hp -= total_damage;
        let monster_name = combat_state.monsters[monster_idx].name.clone();
        let alive = combat_state.monsters[monster_idx].is_alive();
        combat_state.log.push(format!(
            "{} backstabs {} for {} damage (x{}){}",
            character.name,
            monster_name,
            total_damage,
            multiplier,
            if !alive { " — KILLED!" } else { "" }
        ));
        GMResponse::ok_with_data(
            id,
            format!("{} backstabs {} (+{} to hit, x{} damage)! Rolled {} vs target {}: HIT for {} damage{}.",
                character.name, monster_name, attack_bonus, multiplier,
                attack_roll, target_number, total_damage,
                if !alive { " — KILLED!" } else { "" }),
            state.mode.clone(),
            serde_json::json!({
                "hit": true,
                "attack_roll": attack_roll,
                "target_number": target_number,
                "damage": total_damage,
                "multiplier": multiplier,
                "monster_alive": alive,
            }),
        )
    } else {
        combat_state.log.push(format!(
            "{} backstab attempt on {} missed",
            character.name, combat_state.monsters[monster_idx].name
        ));
        GMResponse::ok_with_data(
            id,
            format!(
                "{} backstab attempt: rolled {} vs target {} — MISS.",
                character.name, attack_roll, target_number
            ),
            state.mode.clone(),
            serde_json::json!({
                "hit": false,
                "attack_roll": attack_roll,
                "target_number": target_number,
            }),
        )
    }
}

pub(super) fn spawn_monster(
    id: &str,
    state: &mut GameState,
    name: &str,
    count: u32,
    distance: u32,
) -> GMResponse {
    if state.combat.is_some() {
        return GMResponse::err(id, "combat already active.", state.mode.clone());
    }
    let def = match monster::find_monster(name) {
        Some(d) => d,
        None => {
            return GMResponse::err(
                id,
                format!(
                    "unknown monster '{}'. Use SpawnEncounter for custom monsters.",
                    name
                ),
                state.mode.clone(),
            )
        }
    };

    let mut monsters = Vec::new();
    for i in 0..count {
        let monster_name = if count > 1 {
            format!("{} {}", def.name, i + 1)
        } else {
            def.name.to_string()
        };
        let mut m = Monster::new(&monster_name, &def.hit_dice);
        // Roll HP from hit dice
        let hd = crate::rules::attack::parse_monster_hd(&def.hit_dice);
        let hp = if hd == 0 {
            // Half HD monsters (kobolds, etc): 1d4
            match dice::roll_str("1d4") {
                Ok(r) => r.total.max(1),
                Err(_) => 2,
            }
        } else {
            match dice::roll_str(&format!("{}d8", hd)) {
                Ok(r) => r.total.max(1),
                Err(_) => (hd as i32 * 4).max(1),
            }
        };
        m.hp = hp;
        m.max_hp = hp;
        m.ac = def.ac();
        m.damage = def.damage();
        m.morale = def.morale;
        m.xp_value = def.xp();
        m.attacks = def.attack_names();
        monsters.push(m);
    }

    let combat_state = CombatState::new(monsters, distance);
    let status = combat::combat_status(&combat_state, &state.party.members);
    state.combat = Some(combat_state);
    state.pre_combat_mode = Some(state.mode.clone());
    state.mode = GameMode::Combat;

    let mut msg = format!(
        "combat started: {} {}(s) at {}' distance.",
        count, def.name, distance
    );
    let special = def.special();
    if !special.is_empty() {
        msg.push_str(&format!(" Special: {}", special));
    }

    GMResponse::ok_with_data(
        id,
        msg,
        state.mode.clone(),
        serde_json::json!({
            "status": status,
            "monster": def.name,
            "hit_dice": def.hit_dice,
            "ac": def.ac(),
            "special": def.special(),
        }),
    )
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
