use crate::dice;
use crate::engine::result::EngineError;
use crate::engine::retainer::{self, LoyaltyResult};
use crate::model::{CombatState, Monster};
use crate::persist::GameState;
use crate::rules::class::Class;
use crate::rules::{ability, equipment, monster as monster_db, thief};

use super::results::{
    AttackResult, BackstabResult, CloseResult, CombatLogResult, DeclareSpellResult,
    EndCombatResult, FightingWithdrawalResult, InitiativeResult, InitiativeWinner,
    MonsterAttackResult, MoraleResult, RetainerLoyaltyCheckResult, RetainerLoyaltyOutcome,
    RetreatResult, SpawnEncounterResult, SpawnMonsterResult, TurnUndeadResult,
};
use super::{
    check_morale, close, combat_status, coup_de_grace, declare_spell, fighting_withdrawal,
    monster_attack, resolve_character_attack, resolve_turn_undead, retreat, roll_initiative,
};

fn no_active_combat() -> EngineError {
    EngineError::WrongState("no active combat.".to_string())
}

pub struct SpawnEncounterParams<'a> {
    pub name: &'a str,
    pub count: u32,
    pub hit_dice: &'a str,
    pub ac: i32,
    pub hp: i32,
    pub damage: &'a str,
    pub morale: u32,
    pub distance: u32,
    pub xp_value: Option<u64>,
}

pub fn action_spawn_encounter(
    state: &mut GameState,
    params: &SpawnEncounterParams<'_>,
) -> Result<SpawnEncounterResult, EngineError> {
    if state.combat.is_some() {
        return Err(EngineError::WrongState(
            "combat already active. Use 'end_combat' first.".to_string(),
        ));
    }
    if params.count == 0 {
        return Err(EngineError::InvalidInput(
            "count must be a positive integer".to_string(),
        ));
    }
    if params.hp < 1 {
        return Err(EngineError::InvalidInput(
            "hp must be a positive integer".to_string(),
        ));
    }
    if !(2..=12).contains(&params.morale) {
        return Err(EngineError::InvalidInput("morale must be 2-12".to_string()));
    }

    let xp_per_monster = params.xp_value.unwrap_or_else(|| {
        monster_db::find_monster(params.name)
            .map(|m| m.xp())
            .unwrap_or(0)
    });

    let mut monsters = Vec::new();
    for i in 0..params.count {
        let monster_name = if params.count > 1 {
            format!("{} {}", params.name, i + 1)
        } else {
            params.name.to_string()
        };
        let mut monster = Monster::new(&monster_name, params.hit_dice);
        monster.hp = params.hp;
        monster.max_hp = params.hp;
        monster.ac = params.ac;
        monster.damage = params.damage.to_string();
        monster.morale = params.morale;
        monster.xp_value = xp_per_monster;
        monster.attacks = vec!["attack".to_string()];
        monsters.push(monster);
    }

    let combat = CombatState::new(monsters, params.distance);
    let status = combat_status(&combat, &state.party.members);
    state.enter_combat(combat);

    Ok(SpawnEncounterResult {
        message: format!(
            "combat started: {} {}(s) at {}' distance.",
            params.count, params.name, params.distance
        ),
        encounter_name: params.name.to_string(),
        count: params.count,
        hit_dice: params.hit_dice.to_string(),
        ac: params.ac,
        hp: params.hp,
        damage: params.damage.to_string(),
        morale: params.morale,
        distance: params.distance,
        xp_per_monster,
        status,
    })
}

pub fn action_spawn_monster(
    state: &mut GameState,
    name: &str,
    count: u32,
    distance: u32,
) -> Result<SpawnMonsterResult, EngineError> {
    if state.combat.is_some() {
        return Err(EngineError::WrongState(
            "combat already active.".to_string(),
        ));
    }

    let def = monster_db::find_monster(name).ok_or_else(|| {
        EngineError::InvalidInput(format!(
            "unknown monster '{}'. Use SpawnEncounter for custom monsters.",
            name
        ))
    })?;

    let mut monsters = Vec::new();
    for i in 0..count {
        let monster_name = if count > 1 {
            format!("{} {}", def.name, i + 1)
        } else {
            def.name.to_string()
        };
        let mut m = Monster::new(&monster_name, &def.hit_dice);
        let hd = crate::rules::attack::parse_monster_hd(&def.hit_dice);
        let hp = if hd == 0 {
            match crate::dice::roll_str("1d4") {
                Ok(r) => r.total.max(1),
                Err(_) => 2,
            }
        } else {
            match crate::dice::roll_str(&format!("{}d8", hd)) {
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
    let status = combat_status(&combat_state, &state.party.members);
    state.enter_combat(combat_state);

    let special = def.special();
    let mut msg = format!(
        "combat started: {} {}(s) at {}' distance.",
        count, def.name, distance
    );
    if !special.is_empty() {
        msg.push_str(&format!(" Special: {}", special));
    }

    Ok(SpawnMonsterResult {
        message: msg,
        monster_name: def.name.to_string(),
        count,
        hit_dice: def.hit_dice.clone(),
        ac: def.ac(),
        damage: def.damage(),
        morale: def.morale,
        distance,
        xp_per_monster: def.xp(),
        special,
        status,
    })
}

pub fn action_roll_initiative(state: &mut GameState) -> Result<InitiativeResult, EngineError> {
    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    let (party_initiative, monster_initiative) = roll_initiative(combat);
    let winner = if party_initiative > monster_initiative {
        InitiativeWinner::Party
    } else if monster_initiative > party_initiative {
        InitiativeWinner::Monsters
    } else {
        InitiativeWinner::Simultaneous
    };

    Ok(InitiativeResult {
        message: format!(
            "round {} initiative: party {} vs monsters {} — {} acts first.",
            combat.round,
            party_initiative,
            monster_initiative,
            winner.as_str()
        ),
        round: combat.round,
        party_initiative,
        monster_initiative,
        winner,
    })
}

pub fn action_attack(
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
    weapon_name: &str,
) -> Result<AttackResult, EngineError> {
    let weapon = equipment::find_weapon(weapon_name).ok_or_else(|| {
        EngineError::InvalidInput(format!(
            "unknown weapon '{}'. Try: sword, mace, dagger, short bow, etc.",
            weapon_name
        ))
    })?;

    let character = state.party.find_member(char_name).cloned().ok_or_else(|| {
        EngineError::InvalidInput(format!(
            "no party member named '{}'. Use 'party' to list members.",
            char_name
        ))
    })?;

    let rest_penalty = state.time.as_ref().map(|t| t.rest_penalty()).unwrap_or(0);
    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;

    if monster_idx < combat.monsters.len()
        && combat.monsters[monster_idx].is_alive()
        && combat.monsters[monster_idx].helpless
    {
        let result =
            coup_de_grace(combat, &character, monster_idx).map_err(EngineError::InvalidInput)?;
        return Ok(AttackResult::from(result));
    }

    let result = resolve_character_attack(combat, &character, monster_idx, weapon, rest_penalty)
        .map_err(EngineError::InvalidInput)?;
    Ok(AttackResult::from(result))
}

pub fn action_monster_attack(
    state: &mut GameState,
    monster_idx: usize,
    char_name: &str,
) -> Result<MonsterAttackResult, EngineError> {
    {
        let combat = state.combat.as_ref().ok_or_else(no_active_combat)?;
        if combat.monsters.is_empty() {
            return Err(EngineError::InvalidInput(
                "no monsters in combat".to_string(),
            ));
        }
        if monster_idx >= combat.monsters.len() {
            return Err(EngineError::InvalidInput(format!(
                "monster index {} out of range (0-{})",
                monster_idx,
                combat.monsters.len().saturating_sub(1)
            )));
        }
        if !combat.monsters[monster_idx].is_alive() {
            return Err(EngineError::InvalidInput(format!(
                "{} is dead.",
                combat.monsters[monster_idx].name
            )));
        }
    }

    let character = state.party.find_member_mut(char_name).ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;
    if !character.is_alive() {
        return Err(EngineError::InvalidInput(format!(
            "{} is already dead.",
            character.name
        )));
    }

    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    let result = monster_attack(combat, monster_idx, character);
    Ok(MonsterAttackResult::from(result))
}

pub fn action_morale(
    state: &mut GameState,
    selector: Option<&str>,
) -> Result<MoraleResult, EngineError> {
    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    if combat.living_monster_count() == 0 {
        return Err(EngineError::InvalidInput(
            "no living monsters to check morale for.".to_string(),
        ));
    }

    let morale_score = if let Some(name) = selector {
        let name_lower = name.to_lowercase();
        combat
            .monsters
            .iter()
            .find(|m| m.is_alive() && m.name.to_lowercase().starts_with(&name_lower))
            .map(|m| m.morale)
            .ok_or_else(|| {
                EngineError::InvalidInput(format!("no living monster named '{}'.", name))
            })?
    } else {
        combat
            .living_monsters()
            .first()
            .map(|(_, m)| m.morale)
            .unwrap_or(7)
    };

    Ok(MoraleResult::from(check_morale(combat, morale_score)))
}

pub fn action_turn_undead(
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
) -> Result<TurnUndeadResult, EngineError> {
    let character = state.party.find_member(char_name).cloned().ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;
    if !matches!(character.class, Class::Cleric | Class::Paladin) {
        return Err(EngineError::InvalidInput(format!(
            "{} ({}) cannot turn undead. Only Clerics and Paladins can turn undead.",
            character.name,
            character.class.name()
        )));
    }

    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    if monster_idx >= combat.monsters.len() {
        return Err(EngineError::InvalidInput(format!(
            "monster index {} out of range.",
            monster_idx
        )));
    }
    if !combat.monsters[monster_idx].is_alive() {
        return Err(EngineError::InvalidInput(
            "target is already dead.".to_string(),
        ));
    }

    let result = resolve_turn_undead(combat, &character, character.level, monster_idx);
    Ok(TurnUndeadResult::from(result))
}

pub fn action_close(
    state: &mut GameState,
    char_name: &str,
    feet: Option<u32>,
) -> Result<CloseResult, EngineError> {
    let character = state.party.find_member(char_name).cloned().ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;
    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    let old_distance = combat.distance;
    let message = close(combat, &character, feet).map_err(EngineError::InvalidInput)?;
    Ok(CloseResult {
        message,
        character: character.name,
        distance_closed: old_distance.saturating_sub(combat.distance),
        new_distance: combat.distance,
    })
}

pub fn action_retreat(
    state: &mut GameState,
    char_name: &str,
) -> Result<RetreatResult, EngineError> {
    if state.combat.is_none() {
        return Err(no_active_combat());
    }

    let character = state.party.find_member_mut(char_name).ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;
    if !character.is_alive() {
        return Err(EngineError::InvalidInput(format!(
            "{} is already dead.",
            character.name
        )));
    }

    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    Ok(RetreatResult::from(retreat(combat, character)))
}

pub fn action_fighting_withdrawal(
    state: &mut GameState,
    char_name: &str,
) -> Result<FightingWithdrawalResult, EngineError> {
    let character = state.party.find_member(char_name).cloned().ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;
    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    let old_distance = combat.distance;
    let message = fighting_withdrawal(combat, &character);
    Ok(FightingWithdrawalResult {
        message,
        withdrawer: character.name,
        distance_moved: old_distance.saturating_sub(combat.distance),
        new_distance: combat.distance,
    })
}

pub fn action_query_combat_log(state: &GameState) -> Result<CombatLogResult, EngineError> {
    let combat = state.combat.as_ref().ok_or_else(no_active_combat)?;
    if combat.log.is_empty() {
        return Ok(CombatLogResult {
            message: "no combat events logged yet.".to_string(),
            log: Vec::new(),
        });
    }

    let mut message = String::from("Combat Log:\n");
    for (i, entry) in combat.log.iter().enumerate() {
        message.push_str(&format!("  {}. {}\n", i + 1, entry));
    }
    Ok(CombatLogResult {
        message,
        log: combat.log.clone(),
    })
}

pub fn action_declare_spell(
    state: &mut GameState,
    char_name: &str,
    spell_name: &str,
) -> Result<DeclareSpellResult, EngineError> {
    if state.party.find_member(char_name).is_none() {
        return Err(EngineError::InvalidInput(format!(
            "no party member named '{}'.",
            char_name
        )));
    }

    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    declare_spell(combat, char_name, spell_name);

    Ok(DeclareSpellResult {
        message: format!(
            "{} declares: casting {}. Spell will be disrupted if {} takes damage before the magic phase.",
            char_name, spell_name, char_name
        ),
        character: char_name.to_string(),
        spell: spell_name.to_string(),
    })
}

pub fn action_end_combat(state: &mut GameState) -> Result<EndCombatResult, EngineError> {
    let combat = state.exit_combat().ok_or_else(no_active_combat)?;

    let rounds = combat.round;
    let monsters_defeated = combat.monsters.iter().filter(|m| !m.is_alive()).count();
    let total_monsters = combat.monsters.len();
    let total_xp = combat
        .monsters
        .iter()
        .filter(|m| !m.is_alive())
        .map(|m| m.xp_value)
        .sum();
    let party_casualties = state.party.members.iter().filter(|c| !c.is_alive()).count();

    if let Some(dungeon) = state.dungeon.as_mut() {
        if let Some(room_id) = dungeon.current_room {
            if let Some(room) = dungeon.find_room_mut(room_id) {
                room.monsters_cleared = true;
            }
        }
    }

    let living_retainers: Vec<(String, u32)> = state
        .retainers
        .iter()
        .filter(|r| r.is_alive())
        .map(|r| (r.name.clone(), r.loyalty))
        .collect();

    let retainer_xp_each = if total_xp > 0 && !living_retainers.is_empty() {
        Some(retainer::retainer_xp_share(total_xp))
    } else {
        None
    };
    let retainer_xp_recipients = if retainer_xp_each.is_some() {
        living_retainers
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    } else {
        Vec::new()
    };

    let retainer_loyalty_checks = living_retainers
        .iter()
        .map(|(name, loyalty)| {
            let outcome = match retainer::loyalty_check(*loyalty) {
                LoyaltyResult::Loyal => RetainerLoyaltyOutcome::Loyal,
                LoyaltyResult::Wavering => RetainerLoyaltyOutcome::Wavering,
                LoyaltyResult::Disloyal => RetainerLoyaltyOutcome::Disloyal,
            };
            RetainerLoyaltyCheckResult {
                name: name.clone(),
                loyalty: *loyalty,
                outcome,
            }
        })
        .collect();

    Ok(EndCombatResult {
        message: format!(
            "combat ended after {} rounds. {} of {} monsters defeated.",
            rounds, monsters_defeated, total_monsters
        ),
        rounds,
        monsters_defeated,
        total_monsters,
        total_xp,
        party_casualties,
        mode_after: state.mode.clone(),
        retainer_xp_each,
        retainer_xp_recipients,
        retainer_loyalty_checks,
    })
}

pub fn action_backstab(
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
    weapon_name: &str,
) -> Result<BackstabResult, EngineError> {
    let weapon = equipment::find_weapon(weapon_name).ok_or_else(|| {
        EngineError::InvalidInput(format!("unknown weapon '{}'.", weapon_name))
    })?;

    let character = state.party.find_member(char_name).cloned().ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;

    if !thief::can_backstab(character.class) {
        return Err(EngineError::InvalidInput(format!(
            "{} ({}) cannot backstab.",
            character.name,
            character.class.name()
        )));
    }

    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;

    if monster_idx >= combat.monsters.len() {
        return Err(EngineError::InvalidInput(format!(
            "monster index {} out of range.",
            monster_idx
        )));
    }
    if !combat.monsters[monster_idx].is_alive() {
        return Err(EngineError::InvalidInput(format!(
            "{} is already dead.",
            combat.monsters[monster_idx].name
        )));
    }

    let multiplier = thief::backstab_multiplier(character.level);
    let str_mod = ability::str_melee_mod(character.abilities.strength);
    let attack_bonus = thief::BACKSTAB_ATTACK_BONUS;

    let target_ac = combat.monsters[monster_idx].ac;
    let target_number =
        (character.thac0 as i32 - target_ac - attack_bonus - str_mod).clamp(2, 20);
    let attack_roll: i32 = rand::Rng::gen_range(&mut rand::thread_rng(), 1..=20);

    let hit = attack_roll == 20 || (attack_roll != 1 && attack_roll >= target_number);

    if hit {
        let base_damage = match dice::roll_str(weapon.damage_dice()) {
            Ok(r) => r.total.max(1),
            Err(_) => 1,
        };
        let total_damage = (base_damage.saturating_add(str_mod)).max(1).saturating_mul(multiplier as i32);
        combat.monsters[monster_idx].hp -= total_damage;
        let monster_name = combat.monsters[monster_idx].name.clone();
        let alive = combat.monsters[monster_idx].is_alive();
        combat.log.push(format!(
            "{} backstabs {} for {} damage (x{}){}",
            character.name,
            monster_name,
            total_damage,
            multiplier,
            if !alive { " — KILLED!" } else { "" }
        ));
        Ok(BackstabResult {
            message: format!(
                "{} backstabs {} (+{} to hit, x{} damage)! Rolled {} vs target {}: HIT for {} damage{}.",
                character.name, monster_name, attack_bonus, multiplier,
                attack_roll, target_number, total_damage,
                if !alive { " — KILLED!" } else { "" }
            ),
            hit: true,
            attack_roll,
            target_number,
            attack_bonus,
            multiplier,
            damage: Some(total_damage),
            monster_alive: Some(alive),
        })
    } else {
        combat.log.push(format!(
            "{} backstab attempt on {} missed",
            character.name, combat.monsters[monster_idx].name
        ));
        Ok(BackstabResult {
            message: format!(
                "{} backstab attempt: rolled {} vs target {} — MISS.",
                character.name, attack_roll, target_number
            ),
            hit: false,
            attack_roll,
            target_number,
            attack_bonus,
            multiplier,
            damage: None,
            monster_alive: None,
        })
    }
}

pub fn action_coup_de_grace(
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
) -> Result<AttackResult, EngineError> {
    let character = state.party.find_member(char_name).cloned().ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;
    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    let result =
        coup_de_grace(combat, &character, monster_idx).map_err(EngineError::InvalidInput)?;
    Ok(AttackResult::from(result))
}
