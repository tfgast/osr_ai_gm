use crate::dice;
use crate::engine::{combat, encounter_engine};
use crate::gmapi::protocol::GMResponse;
use crate::model::{CombatState, Monster};
use crate::persist::GameState;
use crate::rules::{ability, equipment, monster, thief};
use crate::rules::class::Class;
use crate::state::game::GameMode;

pub(super) fn spawn_encounter(
    id: &str, state: &mut GameState, params: &crate::gmapi::protocol::EncounterParams,
) -> GMResponse {
    if state.combat.is_some() {
        return GMResponse::err(id, "combat already active.", state.mode.clone());
    }
    if !(2..=12).contains(&params.morale) {
        return GMResponse::err(id, "morale must be 2-12.", state.mode.clone());
    }
    // XP: use explicit value if provided, otherwise look up from monster database
    let xp = params.xp_value.unwrap_or_else(|| {
        crate::rules::monster::find_monster(&params.name)
            .map(|m| m.xp())
            .unwrap_or(0)
    });
    let mut monsters = Vec::new();
    for i in 0..params.count {
        let monster_name = if params.count > 1 {
            format!("{} {}", params.name, i + 1)
        } else {
            params.name.clone()
        };
        let hd_str = params.hit_dice.to_string();
        let mut m = Monster::new(&monster_name, &hd_str);
        m.hp = params.hp;
        m.max_hp = params.hp;
        m.ac = params.ac;
        m.damage = params.damage.clone();
        m.morale = params.morale;
        m.xp_value = xp;
        m.attacks = vec!["attack".to_string()];
        monsters.push(m);
    }

    let combat_state = CombatState::new(monsters, params.distance);
    let status = combat::combat_status(&combat_state, &state.party.members);
    state.combat = Some(combat_state);
    state.pre_combat_mode = Some(state.mode.clone());
    state.mode = GameMode::Combat;

    GMResponse::ok_with_data(
        id,
        format!("combat started: {} {}(s) at {}' distance.", params.count, params.name, params.distance),
        state.mode.clone(),
        serde_json::json!({ "status": status }),
    )
}

pub(super) fn roll_initiative(id: &str, state: &mut GameState) -> GMResponse {
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    let (p, m) = combat::roll_initiative(combat_state);
    let winner = if p > m { "party" } else if m > p { "monsters" } else { "simultaneous" };
    GMResponse::ok_with_data(
        id,
        format!("round {} initiative: party {} vs monsters {} — {} acts first.",
            combat_state.round, p, m, winner),
        state.mode.clone(),
        serde_json::json!({
            "round": combat_state.round,
            "party_initiative": p,
            "monster_initiative": m,
            "winner": winner,
        }),
    )
}

pub(super) fn attack(id: &str, state: &mut GameState, char_name: &str, monster_idx: usize, weapon_name: &str) -> GMResponse {
    let weapon = match equipment::find_weapon(weapon_name) {
        Some(w) => w,
        None => return GMResponse::err(id, format!("unknown weapon '{}'.", weapon_name), state.mode.clone()),
    };
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let rest_penalty = state.time.as_ref().map(|t| t.rest_penalty()).unwrap_or(0);
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };

    match combat::resolve_character_attack(combat_state, &character, monster_idx, weapon, rest_penalty) {
        Ok(result) => GMResponse::ok(id, result.to_string(), state.mode.clone()),
        Err(e) => GMResponse::err(id, e, state.mode.clone()),
    }
}

pub(super) fn monster_attack(id: &str, state: &mut GameState, monster_idx: usize, char_name: &str) -> GMResponse {
    {
        let combat_state = match state.combat.as_ref() {
            Some(c) => c,
            None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
        };
        if monster_idx >= combat_state.monsters.len() {
            return GMResponse::err(id, format!("monster index {} out of range.", monster_idx), state.mode.clone());
        }
        if !combat_state.monsters[monster_idx].is_alive() {
            return GMResponse::err(id, format!("{} is dead.", combat_state.monsters[monster_idx].name), state.mode.clone());
        }
    }
    let character = match state.party.find_member_mut(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    if !character.is_alive() {
        return GMResponse::err(id, format!("{} is already dead.", character.name), state.mode.clone());
    }
    let combat_state = state.combat.as_mut().unwrap();
    let result = combat::monster_attack(combat_state, monster_idx, character);
    GMResponse::ok(id, result.to_string(), state.mode.clone())
}

pub(super) fn check_morale(id: &str, state: &mut GameState) -> GMResponse {
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if combat_state.living_monster_count() == 0 {
        return GMResponse::err(id, "no living monsters.", state.mode.clone());
    }
    let morale_score = combat_state.monsters.iter()
        .filter(|m| m.is_alive())
        .map(|m| m.morale)
        .max()
        .unwrap_or(6);
    let result = combat::check_morale(combat_state, morale_score);
    GMResponse::ok(id, result.to_string(), state.mode.clone())
}

pub(super) fn turn_undead(id: &str, state: &mut GameState, char_name: &str, monster_idx: usize) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    // Only Clerics and Paladins can turn undead per OSE rules
    if !matches!(character.class, Class::Cleric | Class::Paladin) {
        return GMResponse::err(id, format!("{} ({}) cannot turn undead. Only Clerics and Paladins can turn undead.", character.name, character.class.name()), state.mode.clone());
    }
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if monster_idx >= combat_state.monsters.len() {
        return GMResponse::err(id, format!("monster index {} out of range.", monster_idx), state.mode.clone());
    }
    if !combat_state.monsters[monster_idx].is_alive() {
        return GMResponse::err(id, "target is already dead.", state.mode.clone());
    }
    let result = combat::resolve_turn_undead(combat_state, &character, character.level, monster_idx);
    GMResponse::ok(id, result.to_string(), state.mode.clone())
}

pub(super) fn close(id: &str, state: &mut GameState, char_name: &str, feet: Option<u32>) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    match combat::close(combat_state, &character, feet) {
        Ok(msg) => GMResponse::ok_with_data(
            id, msg, state.mode.clone(),
            serde_json::json!({ "distance": combat_state.distance }),
        ),
        Err(e) => GMResponse::err(id, e, state.mode.clone()),
    }
}

pub(super) fn retreat(id: &str, state: &mut GameState, char_name: &str) -> GMResponse {
    if state.combat.is_none() {
        return GMResponse::err(id, "no active combat.", state.mode.clone());
    }
    let character = match state.party.find_member_mut(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    if !character.is_alive() {
        return GMResponse::err(id, format!("{} is already dead.", character.name), state.mode.clone());
    }
    let combat_state = state.combat.as_mut().unwrap();
    let result = combat::retreat(combat_state, character);
    GMResponse::ok_with_data(
        id,
        result.to_string(),
        state.mode.clone(),
        serde_json::json!({
            "retreater": result.retreater,
            "distance_moved": result.distance_moved,
            "new_distance": result.new_distance,
            "free_attacks": result.free_attacks.iter().map(|a| serde_json::json!({
                "attacker": a.attacker,
                "target": a.target,
                "roll": a.roll,
                "modifiers": a.modifiers,
                "target_number": a.target_number,
                "hit": a.hit,
                "damage": a.damage,
                "target_hp_after": a.target_hp_after,
                "target_killed": a.target_killed,
            })).collect::<Vec<_>>(),
        }),
    )
}

pub(super) fn fighting_withdrawal(id: &str, state: &mut GameState, char_name: &str) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    let msg = combat::fighting_withdrawal(combat_state, &character);
    GMResponse::ok(id, msg, state.mode.clone())
}

pub(super) fn query_combat_log(id: &str, state: &GameState) -> GMResponse {
    let combat_state = match state.combat.as_ref() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if combat_state.log.is_empty() {
        return GMResponse::ok_with_data(
            id,
            "no combat events logged yet.",
            state.mode.clone(),
            serde_json::json!({ "log": [] }),
        );
    }
    let mut out = String::from("Combat Log:\n");
    for (i, entry) in combat_state.log.iter().enumerate() {
        out.push_str(&format!("  {}. {}\n", i + 1, entry));
    }
    GMResponse::ok_with_data(
        id, out, state.mode.clone(),
        serde_json::json!({ "log": combat_state.log }),
    )
}

pub(super) fn declare_spell(id: &str, state: &mut GameState, char_name: &str, spell_name: &str) -> GMResponse {
    if state.party.find_member(char_name).is_none() {
        return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone());
    }
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    combat::declare_spell(combat_state, char_name, spell_name);
    GMResponse::ok(
        id,
        format!("{} declares: casting {}. Spell will be disrupted if {} takes damage before the magic phase.",
            char_name, spell_name, char_name),
        state.mode.clone(),
    )
}

pub(super) fn end_combat(id: &str, state: &mut GameState) -> GMResponse {
    if state.combat.is_none() {
        return GMResponse::err(id, "no active combat.", state.mode.clone());
    }
    let combat_state = state.combat.take().unwrap();
    let dead_monsters = combat_state.monsters.iter().filter(|m| !m.is_alive()).count();
    let total_xp: u64 = combat_state.monsters.iter()
        .filter(|m| !m.is_alive())
        .map(|m| m.xp_value)
        .sum();
    let dead_party = state.party.members.iter().filter(|c| !c.is_alive()).count();
    state.mode = state.pre_combat_mode.take().unwrap_or(GameMode::Idle);

    // Mark placed monsters as cleared in current room (module support)
    if let Some(dungeon) = state.dungeon.as_mut() {
        if let Some(room_id) = dungeon.current_room {
            if let Some(room) = dungeon.find_room_mut(room_id) {
                room.monsters_cleared = true;
            }
        }
    }

    GMResponse::ok_with_data(
        id,
        format!("combat ended after {} rounds. {} of {} monsters defeated.",
            combat_state.round, dead_monsters, combat_state.monsters.len()),
        state.mode.clone(),
        serde_json::json!({
            "rounds": combat_state.round,
            "monsters_defeated": dead_monsters,
            "total_xp": total_xp,
            "party_casualties": dead_party,
        }),
    )
}

pub(super) fn backstab(id: &str, state: &mut GameState, char_name: &str, monster_idx: usize, weapon_name: &str) -> GMResponse {
    let weapon = match equipment::find_weapon(weapon_name) {
        Some(w) => w,
        None => return GMResponse::err(id, format!("unknown weapon '{}'.", weapon_name), state.mode.clone()),
    };
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    if !thief::can_backstab(character.class) {
        return GMResponse::err(id, format!("{} ({}) cannot backstab.", character.name, character.class.name()), state.mode.clone());
    }
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if monster_idx >= combat_state.monsters.len() {
        return GMResponse::err(id, format!("monster index {} out of range.", monster_idx), state.mode.clone());
    }
    if !combat_state.monsters[monster_idx].is_alive() {
        return GMResponse::err(id, format!("{} is already dead.", combat_state.monsters[monster_idx].name), state.mode.clone());
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
            character.name, monster_name, total_damage, multiplier,
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
        combat_state.log.push(format!("{} backstab attempt on {} missed", character.name, combat_state.monsters[monster_idx].name));
        GMResponse::ok_with_data(
            id,
            format!("{} backstab attempt: rolled {} vs target {} — MISS.",
                character.name, attack_roll, target_number),
            state.mode.clone(),
            serde_json::json!({
                "hit": false,
                "attack_roll": attack_roll,
                "target_number": target_number,
            }),
        )
    }
}

pub(super) fn spawn_monster(id: &str, state: &mut GameState, name: &str, count: u32, distance: u32) -> GMResponse {
    if state.combat.is_some() {
        return GMResponse::err(id, "combat already active.", state.mode.clone());
    }
    let def = match monster::find_monster(name) {
        Some(d) => d,
        None => return GMResponse::err(id, format!("unknown monster '{}'. Use SpawnEncounter for custom monsters.", name), state.mode.clone()),
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

    let mut msg = format!("combat started: {} {}(s) at {}' distance.", count, def.name, distance);
    let special = def.special();
    if !special.is_empty() {
        msg.push_str(&format!(" Special: {}", special));
    }

    GMResponse::ok_with_data(
        id, msg, state.mode.clone(),
        serde_json::json!({
            "status": status,
            "monster": def.name,
            "hit_dice": def.hit_dice,
            "ac": def.ac(),
            "special": def.special(),
        }),
    )
}

pub(super) fn spawn_npc_party(id: &str, state: &mut GameState, party_type: &str, distance: u32) -> GMResponse {
    use crate::rules::npc_party;

    if state.combat.is_some() {
        return GMResponse::err(id, "combat already active.", state.mode.clone());
    }

    let mut rng = rand::thread_rng();
    let party = match party_type {
        "basic" => npc_party::generate_basic_party(&mut rng),
        "expert" => npc_party::generate_expert_party(&mut rng),
        "cleric" => npc_party::generate_high_level_cleric_party(&mut rng),
        "fighter" => npc_party::generate_high_level_fighter_party(&mut rng),
        "mage" => npc_party::generate_high_level_magic_user_party(&mut rng),
        _ => return GMResponse::err(
            id,
            format!("unknown party type '{}'. Valid: basic, expert, cleric, fighter, mage.", party_type),
            state.mode.clone(),
        ),
    };

    let member_info: Vec<serde_json::Value> = party.members.iter().map(|m| {
        serde_json::json!({
            "class": m.class,
            "level": m.level,
            "alignment": m.alignment.to_string(),
            "role": m.role,
        })
    }).collect();

    let monsters: Vec<Monster> = party.members.iter()
        .map(|m| npc_party::npc_member_to_monster(m))
        .collect();

    let count = monsters.len();
    let combat_state = CombatState::new(monsters, distance);
    let status = combat::combat_status(&combat_state, &state.party.members);
    state.combat = Some(combat_state);
    state.pre_combat_mode = Some(state.mode.clone());
    state.mode = GameMode::Combat;

    let mut msg = format!(
        "combat started: {} NPC adventurers ({}) at {}' distance.",
        count, party.party_type, distance
    );
    if party.mounted {
        msg.push_str(" Party is mounted.");
    }
    for note in &party.notes {
        msg.push_str(&format!(" {}", note));
    }

    GMResponse::ok_with_data(
        id, msg, state.mode.clone(),
        serde_json::json!({
            "status": status,
            "party_type": party.party_type,
            "member_count": count,
            "members": member_info,
            "mounted": party.mounted,
        }),
    )
}

pub(super) fn roll_surprise(id: &str, state: &GameState) -> GMResponse {
    let (result, p, m) = encounter_engine::check_surprise();
    GMResponse::ok_with_data(
        id,
        format!("party roll: {} monster roll: {} — {}", p, m, result),
        state.mode.clone(),
        serde_json::json!({
            "party_roll": p,
            "monster_roll": m,
            "result": result.to_string(),
        }),
    )
}

pub(super) fn set_helpless(id: &str, state: &mut GameState, monster_idx: usize, helpless: bool) -> GMResponse {
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    match combat::set_monster_helpless(combat_state, monster_idx, helpless) {
        Ok(msg) => GMResponse::ok_with_data(
            id, msg, state.mode.clone(),
            serde_json::json!({
                "monster_idx": monster_idx,
                "helpless": helpless,
            }),
        ),
        Err(e) => GMResponse::err(id, e, state.mode.clone()),
    }
}

pub(super) fn kill(id: &str, state: &mut GameState, char_name: &str, monster_idx: usize) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    match combat::coup_de_grace(combat_state, &character, monster_idx) {
        Ok(result) => GMResponse::ok_with_data(
            id, result.to_string(), state.mode.clone(),
            serde_json::json!({
                "attacker": result.attacker,
                "target": result.target,
            }),
        ),
        Err(e) => GMResponse::err(id, e, state.mode.clone()),
    }
}

pub(super) fn roll_reaction(id: &str, state: &GameState, char_name: &str) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let cha = character.abilities.charisma;
    let (reaction, raw, modified) = encounter_engine::reaction_roll(cha);
    let cha_mod = ability::cha_reaction_mod(cha);
    GMResponse::ok_with_data(
        id,
        format!("{} speaks (CHA {}, modifier {:+}). reaction roll: {} {:+} = {} — {}",
            character.name, cha, cha_mod, raw, cha_mod, modified, reaction),
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "charisma": cha,
            "cha_modifier": cha_mod,
            "raw_roll": raw,
            "modified_roll": modified,
            "reaction": reaction.to_string(),
        }),
    )
}
