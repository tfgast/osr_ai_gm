use crate::dice;
use crate::engine::result::EngineError;
use crate::engine::retainer::{self, LoyaltyResult};
use crate::model::{CombatState, Monster};
use crate::persist::GameState;
use crate::rules::class::Class;
use crate::rules::{ability, equipment, monster as monster_db, thief};

use super::results::{
    AddMonsterResult, AttackResult, BackstabResult, CastSpellResult, CloseResult, CombatLogResult,
    CombatStatusResult, DeclareSpellResult, EndCombatResult, FightingWithdrawalResult,
    InitiativeResult, InitiativeWinner, MonsterAttackResult, MoraleResult,
    RetainerLoyaltyCheckResult, RetainerLoyaltyOutcome, RetreatResult, SetHelplessResult,
    SpawnEncounterResult, SpawnMonsterResult, TurnUndeadResult,
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
    pub hit_dice: &'a crate::rules::attack::HitDice,
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
    if params.name.trim().is_empty() {
        return Err(EngineError::InvalidInput(
            "monster name must not be empty.".to_string(),
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
        let mut monster = Monster::new(&monster_name, params.hit_dice.clone());
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
        hit_dice: params.hit_dice.clone(),
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
            "combat already active. Use 'end_combat' first.".to_string(),
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
        let mut m = Monster::new(&monster_name, def.hit_dice.clone());
        let dice_count = def.hit_dice.hp_dice_count();
        let hp_mod = def.hit_dice.hp_modifier();
        let hp = if dice_count == 0 {
            match crate::dice::roll_str("1d4") {
                Ok(r) => (r.total + hp_mod).max(1),
                Err(_) => 2,
            }
        } else {
            match crate::dice::roll_str(&format!("{}d8", dice_count)) {
                Ok(r) => (r.total + hp_mod).max(1),
                Err(_) => (dice_count as i32 * 4 + hp_mod).max(1),
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

pub fn action_add_monster(
    state: &mut GameState,
    params: &SpawnEncounterParams<'_>,
) -> Result<AddMonsterResult, EngineError> {
    if state.combat.is_none() {
        return Err(no_active_combat());
    }
    if params.name.trim().is_empty() {
        return Err(EngineError::InvalidInput(
            "monster name must not be empty.".to_string(),
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

    let combat = state.combat.as_mut().unwrap();
    let existing_count = combat.monsters.len();

    for i in 0..params.count {
        let monster_name = if params.count > 1 {
            format!("{} {}", params.name, i + 1)
        } else {
            params.name.to_string()
        };
        let mut monster = Monster::new(&monster_name, params.hit_dice.clone());
        monster.hp = params.hp;
        monster.max_hp = params.hp;
        monster.ac = params.ac;
        monster.damage = params.damage.to_string();
        monster.morale = params.morale;
        monster.xp_value = xp_per_monster;
        monster.attacks = vec!["attack".to_string()];
        combat.monsters.push(monster);
    }

    let total_monsters = combat.monsters.len();
    let status = combat_status(combat, &state.party.members);

    combat.log.push(format!(
        "{} {}(s) added to combat (indices {}-{}).",
        params.count,
        params.name,
        existing_count,
        total_monsters - 1,
    ));

    Ok(AddMonsterResult {
        message: format!(
            "{} {}(s) added to combat. Total monsters: {}.",
            params.count, params.name, total_monsters
        ),
        monster_name: params.name.to_string(),
        count: params.count,
        hit_dice: params.hit_dice.clone(),
        ac: params.ac,
        hp: params.hp,
        damage: params.damage.to_string(),
        morale: params.morale,
        xp_per_monster,
        total_monsters,
        status,
    })
}

pub fn action_roll_initiative(state: &mut GameState) -> Result<InitiativeResult, EngineError> {
    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;

    // Prevent rolling initiative twice without any intervening combat actions.
    if combat.round > 0 && combat.log.len() <= combat.log_len_at_initiative {
        return Err(EngineError::WrongState(
            "initiative already rolled for this round.".to_string(),
        ));
    }

    let (party_initiative, monster_initiative) = roll_initiative(combat);
    combat.log_len_at_initiative = combat.log.len();
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

    if combat.characters_acted.iter().any(|n| n.eq_ignore_ascii_case(char_name)) {
        return Err(EngineError::InvalidInput(format!(
            "{} has already acted this round.",
            char_name
        )));
    }

    if monster_idx < combat.monsters.len()
        && combat.monsters[monster_idx].is_alive()
        && combat.monsters[monster_idx].helpless
    {
        let result =
            coup_de_grace(combat, &character, monster_idx).map_err(EngineError::InvalidInput)?;
        combat.characters_acted.push(char_name.to_string());
        return Ok(AttackResult::from(result));
    }

    let result = resolve_character_attack(combat, &character, monster_idx, weapon, rest_penalty)
        .map_err(EngineError::InvalidInput)?;
    combat.characters_acted.push(char_name.to_string());
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
        if combat.monsters_attacked_this_round.contains(&monster_idx) {
            return Err(EngineError::InvalidInput(format!(
                "{} has already attacked this round.",
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
    combat.monsters_attacked_this_round.insert(monster_idx);
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

    if combat.spell_declarations.iter().any(|n| n.eq_ignore_ascii_case(char_name)) {
        return Err(EngineError::InvalidInput(format!(
            "{} has already declared a spell this round.",
            char_name
        )));
    }

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

pub fn action_cast_spell(
    state: &mut GameState,
    char_name: &str,
) -> Result<CastSpellResult, EngineError> {
    if state.party.find_member(char_name).is_none() {
        return Err(EngineError::InvalidInput(format!(
            "no party member named '{}'.",
            char_name
        )));
    }

    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;

    if combat.characters_acted.iter().any(|n| n.eq_ignore_ascii_case(char_name)) {
        return Err(EngineError::InvalidInput(format!(
            "{} has already acted this round.",
            char_name
        )));
    }

    // Find the pending spell for this character.
    let pending_idx = combat.pending_spells.iter().position(|(name, _)| {
        name.eq_ignore_ascii_case(char_name)
    });
    let pending_idx = match pending_idx {
        Some(idx) => idx,
        None => {
            return Err(EngineError::InvalidInput(format!(
                "{} has not declared a spell this round. Use DeclareSpell first.",
                char_name
            )));
        }
    };

    let (_, spell_name) = combat.pending_spells.remove(pending_idx);

    // Remove from spell_declarations too.
    if let Some(decl_idx) = combat.spell_declarations.iter().position(|n| {
        n.eq_ignore_ascii_case(char_name)
    }) {
        combat.spell_declarations.remove(decl_idx);
    }

    // Check if the spell was disrupted.
    let was_disrupted = super::is_disrupted(combat, char_name);

    // Mark character as having acted.
    combat.characters_acted.push(char_name.to_string());

    if was_disrupted {
        combat.log.push(format!(
            "{}'s {} fizzles — spell was disrupted!",
            char_name, spell_name
        ));
        Ok(CastSpellResult {
            message: format!(
                "{}'s {} was disrupted! The spell fails.",
                char_name, spell_name
            ),
            character: char_name.to_string(),
            spell: spell_name,
            cast: false,
            disrupted: true,
        })
    } else {
        combat.log.push(format!(
            "{} casts {}!",
            char_name, spell_name
        ));
        Ok(CastSpellResult {
            message: format!(
                "{} casts {}! Apply spell effects as appropriate.",
                char_name, spell_name
            ),
            character: char_name.to_string(),
            spell: spell_name,
            cast: true,
            disrupted: false,
        })
    }
}

pub fn action_end_combat(state: &mut GameState) -> Result<EndCombatResult, EngineError> {
    if state.combat.is_none() {
        return Err(no_active_combat());
    }
    let combat = state.exit_combat().unwrap();

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

    if combat.characters_acted.iter().any(|n| n.eq_ignore_ascii_case(char_name)) {
        return Err(EngineError::InvalidInput(format!(
            "{} has already acted this round.",
            char_name
        )));
    }

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

    combat.characters_acted.push(char_name.to_string());

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

pub fn action_combat_status(state: &GameState) -> Result<CombatStatusResult, EngineError> {
    let combat = state.combat.as_ref().ok_or_else(no_active_combat)?;
    let status = combat_status(combat, &state.party.members);
    Ok(CombatStatusResult {
        message: status.clone(),
        status,
    })
}

pub fn action_set_helpless(
    state: &mut GameState,
    monster_idx: usize,
    helpless: bool,
) -> Result<SetHelplessResult, EngineError> {
    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    let msg = super::set_monster_helpless(combat, monster_idx, helpless)
        .map_err(EngineError::InvalidInput)?;
    Ok(SetHelplessResult {
        message: msg,
        monster_idx,
        helpless,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Character, CombatState, Monster};
    use crate::rules::class::Class;
    use crate::state::game::GameMode;

    fn test_fighter() -> Character {
        let mut c = Character::new("Grond", Class::Fighter);
        c.hp = 12;
        c.max_hp = 12;
        c.ac = 4;
        c.level = 2;
        c.abilities.strength = 16;
        c
    }

    fn mk_monster(name: &str, hd: &str, hp: i32, ac: i32, morale: u32) -> Monster {
        let mut monster = Monster::new(name, hd.parse().unwrap());
        monster.hp = hp;
        monster.max_hp = hp;
        monster.ac = ac;
        monster.damage = "1d6".to_string();
        monster.morale = morale;
        monster.xp_value = 5;
        monster.attacks = vec!["weapon".to_string()];
        monster
    }

    fn state_with_combat() -> GameState {
        let mut state = GameState::new();
        let fighter = test_fighter();
        state.party.add_member(fighter);
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![
                mk_monster("Goblin 1", "1", 4, 6, 7),
                mk_monster("Goblin 2", "1", 4, 6, 7),
            ],
            60,
        ));
        state
    }

    // --- action_combat_status (oag-mol-jqd) ---

    #[test]
    fn combat_status_returns_status_string() {
        let state = state_with_combat();
        let result = action_combat_status(&state).unwrap();
        assert!(!result.status.is_empty());
        assert!(result.status.contains("Goblin 1"));
        assert!(result.status.contains("Goblin 2"));
        assert!(result.status.contains("Grond"));
        assert!(result.status.contains("60'")); // distance
    }

    #[test]
    fn combat_status_no_combat_error() {
        let state = GameState::new();
        let result = action_combat_status(&state);
        assert!(result.is_err());
    }

    // --- action_set_helpless (oag-mol-jqd) ---

    #[test]
    fn set_helpless_marks_monster() {
        let mut state = state_with_combat();
        let result = action_set_helpless(&mut state, 0, true).unwrap();
        assert!(result.helpless);
        assert_eq!(result.monster_idx, 0);
        assert!(result.message.contains("helpless"));
        assert!(state.combat.as_ref().unwrap().monsters[0].helpless);
    }

    #[test]
    fn set_helpless_unmarks_monster() {
        let mut state = state_with_combat();
        state.combat.as_mut().unwrap().monsters[0].helpless = true;
        let result = action_set_helpless(&mut state, 0, false).unwrap();
        assert!(!result.helpless);
        assert!(result.message.contains("no longer helpless"));
        assert!(!state.combat.as_ref().unwrap().monsters[0].helpless);
    }

    #[test]
    fn set_helpless_no_combat_error() {
        let mut state = GameState::new();
        let result = action_set_helpless(&mut state, 0, true);
        assert!(result.is_err());
    }

    #[test]
    fn set_helpless_dead_monster_error() {
        let mut state = state_with_combat();
        state.combat.as_mut().unwrap().monsters[0].hp = 0;
        let result = action_set_helpless(&mut state, 0, true);
        assert!(result.is_err());
    }

    #[test]
    fn set_helpless_out_of_range_error() {
        let mut state = state_with_combat();
        let result = action_set_helpless(&mut state, 99, true);
        assert!(result.is_err());
    }

    // --- action_roll_initiative duplicate guard (oag-vtkww) ---

    #[test]
    fn roll_initiative_twice_without_action_is_rejected() {
        let mut state = state_with_combat();
        let result = action_roll_initiative(&mut state);
        assert!(result.is_ok());
        assert_eq!(state.combat.as_ref().unwrap().round, 1);

        // Second call with no intervening action should fail
        let result = action_roll_initiative(&mut state);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("initiative already rolled"), "expected initiative guard, got: {}", err);
        // Round should NOT have advanced
        assert_eq!(state.combat.as_ref().unwrap().round, 1);
    }

    fn state_with_melee_combat() -> GameState {
        let mut state = GameState::new();
        let fighter = test_fighter();
        state.party.add_member(fighter);
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![
                mk_monster("Goblin 1", "1", 4, 6, 7),
                mk_monster("Goblin 2", "1", 4, 6, 7),
            ],
            5, // melee range
        ));
        state
    }

    #[test]
    fn roll_initiative_allowed_after_attack() {
        let mut state = state_with_melee_combat();
        let result = action_roll_initiative(&mut state);
        assert!(result.is_ok());

        // Perform a melee attack at close range (adds to log)
        let attack_result = action_attack(&mut state, "Grond", 0, "Sword");
        assert!(attack_result.is_ok(), "attack failed: {:?}", attack_result.err());

        // Now initiative should be allowed again
        let result = action_roll_initiative(&mut state);
        assert!(result.is_ok(), "second initiative failed: {:?}", result.err());
        assert_eq!(state.combat.as_ref().unwrap().round, 2);
    }

    #[test]
    fn roll_initiative_allowed_after_monster_attack() {
        let mut state = state_with_melee_combat();
        let result = action_roll_initiative(&mut state);
        assert!(result.is_ok());

        // Monster attacks at melee range (adds to log)
        let attack_result = action_monster_attack(&mut state, 0, "Grond");
        assert!(attack_result.is_ok(), "monster attack failed: {:?}", attack_result.err());

        // Initiative should be allowed again
        let result = action_roll_initiative(&mut state);
        assert!(result.is_ok(), "second initiative failed: {:?}", result.err());
        assert_eq!(state.combat.as_ref().unwrap().round, 2);
    }

    // --- monster_attack duplicate guard (oag-1vpmj) ---

    #[test]
    fn monster_attack_twice_same_round_is_rejected() {
        let mut state = state_with_melee_combat();
        let _ = action_roll_initiative(&mut state);

        // First attack should succeed
        let result = action_monster_attack(&mut state, 0, "Grond");
        assert!(result.is_ok(), "first monster attack should succeed");

        // Second attack by same monster in same round should fail
        let result = action_monster_attack(&mut state, 0, "Grond");
        assert!(result.is_err(), "second monster attack should be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("already attacked this round"),
            "expected 'already attacked' guard, got: {}",
            err
        );
    }

    #[test]
    fn different_monsters_can_attack_same_round() {
        let mut state = state_with_melee_combat();
        let _ = action_roll_initiative(&mut state);

        // Monster 0 attacks
        let result = action_monster_attack(&mut state, 0, "Grond");
        assert!(result.is_ok(), "monster 0 attack should succeed");

        // Monster 1 attacks (different monster, same round) — should succeed
        let result = action_monster_attack(&mut state, 1, "Grond");
        assert!(result.is_ok(), "monster 1 attack should succeed");
    }

    #[test]
    fn monster_attack_allowed_after_new_round() {
        let mut state = state_with_melee_combat();
        let _ = action_roll_initiative(&mut state);

        // Monster attacks in round 1
        let result = action_monster_attack(&mut state, 0, "Grond");
        assert!(result.is_ok());

        // New round
        let _ = action_roll_initiative(&mut state);

        // Same monster attacks again in round 2 — should succeed
        let result = action_monster_attack(&mut state, 0, "Grond");
        assert!(result.is_ok(), "monster should be allowed to attack in new round");
    }

    #[test]
    fn roll_initiative_allowed_after_morale_check() {
        let mut state = state_with_combat();
        let result = action_roll_initiative(&mut state);
        assert!(result.is_ok());

        // Morale check (adds to log)
        let _ = action_morale(&mut state, None);

        // Initiative should be allowed again
        let result = action_roll_initiative(&mut state);
        assert!(result.is_ok());
        assert_eq!(state.combat.as_ref().unwrap().round, 2);
    }

    // --- duplicate character attack guard (oag-456xr) ---

    #[test]
    fn same_character_cannot_attack_twice_per_round() {
        let mut state = state_with_melee_combat();
        let _ = action_roll_initiative(&mut state);

        // First attack succeeds
        let result = action_attack(&mut state, "Grond", 0, "Sword");
        assert!(result.is_ok(), "first attack should succeed: {:?}", result.err());

        // Second attack in same round is rejected
        let result = action_attack(&mut state, "Grond", 0, "Sword");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already acted"), "expected acted guard, got: {}", err);
    }

    #[test]
    fn character_can_attack_again_after_new_round() {
        let mut state = GameState::new();
        let fighter = test_fighter();
        state.party.add_member(fighter);
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        // Use a tough monster so it survives round 1
        state.combat = Some(CombatState::new(
            vec![mk_monster("Ogre", "4", 40, 5, 9)],
            5,
        ));

        let _ = action_roll_initiative(&mut state);

        // Attack in round 1
        let result = action_attack(&mut state, "Grond", 0, "Sword");
        assert!(result.is_ok());

        // New round clears the acted list
        let _ = action_roll_initiative(&mut state);

        // Attack in round 2 should succeed
        let result = action_attack(&mut state, "Grond", 0, "Sword");
        assert!(result.is_ok(), "attack in new round should succeed: {:?}", result.err());
    }

    #[test]
    fn attack_guard_is_case_insensitive() {
        let mut state = state_with_melee_combat();
        let _ = action_roll_initiative(&mut state);

        let result = action_attack(&mut state, "Grond", 0, "Sword");
        assert!(result.is_ok());

        // Differently-cased name should still be rejected
        let result = action_attack(&mut state, "grond", 0, "Sword");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already acted"), "expected acted guard, got: {}", err);
    }

    // --- duplicate spell declaration guard (oag-upoq4) ---

    fn state_with_caster_combat() -> GameState {
        let mut state = GameState::new();
        let mut caster = Character::new("Zara", Class::MagicUser);
        caster.hp = 6;
        caster.max_hp = 6;
        caster.level = 3;
        state.party.add_member(caster);
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![mk_monster("Goblin 1", "1", 4, 6, 7)],
            60,
        ));
        state
    }

    #[test]
    fn same_character_cannot_declare_spell_twice_per_round() {
        let mut state = state_with_caster_combat();
        let _ = action_roll_initiative(&mut state);

        // First declaration succeeds
        let result = action_declare_spell(&mut state, "Zara", "Sleep");
        assert!(result.is_ok(), "first declaration should succeed: {:?}", result.err());

        // Second declaration in same round is rejected
        let result = action_declare_spell(&mut state, "Zara", "Magic Missile");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("already declared a spell this round"),
            "expected spell guard, got: {}",
            err
        );
    }

    #[test]
    fn spell_declaration_guard_is_case_insensitive() {
        let mut state = state_with_caster_combat();
        let _ = action_roll_initiative(&mut state);

        let result = action_declare_spell(&mut state, "Zara", "Sleep");
        assert!(result.is_ok());

        // Differently-cased name should still be rejected
        let result = action_declare_spell(&mut state, "zara", "Magic Missile");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("already declared a spell this round"),
            "expected spell guard, got: {}",
            err
        );
    }

    #[test]
    fn spell_declaration_allowed_after_new_round() {
        let mut state = state_with_caster_combat();
        let _ = action_roll_initiative(&mut state);

        let result = action_declare_spell(&mut state, "Zara", "Sleep");
        assert!(result.is_ok());

        // New round clears spell declarations
        let _ = action_roll_initiative(&mut state);

        // Should be allowed again
        let result = action_declare_spell(&mut state, "Zara", "Magic Missile");
        assert!(result.is_ok(), "declaration in new round should succeed: {:?}", result.err());
    }

    // --- cast_spell (oag-aarw0) ---

    #[test]
    fn cast_spell_resolves_declared_spell() {
        let mut state = state_with_caster_combat();
        let _ = action_declare_spell(&mut state, "Zara", "Magic Missile");
        let _ = action_roll_initiative(&mut state);

        // Re-declare after initiative (initiative clears declarations)
        let _ = action_declare_spell(&mut state, "Zara", "Magic Missile");

        let result = action_cast_spell(&mut state, "Zara");
        assert!(result.is_ok(), "cast_spell should succeed: {:?}", result.err());
        let result = result.unwrap();
        assert!(result.cast);
        assert!(!result.disrupted);
        assert_eq!(result.spell, "Magic Missile");
        assert_eq!(result.character, "Zara");
    }

    #[test]
    fn cast_spell_fails_without_declaration() {
        let mut state = state_with_caster_combat();
        let _ = action_roll_initiative(&mut state);

        let result = action_cast_spell(&mut state, "Zara");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not declared"), "expected 'not declared' error, got: {}", err);
    }

    #[test]
    fn cast_spell_reports_disruption() {
        let mut state = state_with_caster_combat();
        let _ = action_roll_initiative(&mut state);
        let _ = action_declare_spell(&mut state, "Zara", "Sleep");

        // Manually disrupt the caster
        state.combat.as_mut().unwrap().disrupted.push("Zara".to_string());

        let result = action_cast_spell(&mut state, "Zara");
        assert!(result.is_ok(), "cast_spell should return Ok even when disrupted");
        let result = result.unwrap();
        assert!(!result.cast);
        assert!(result.disrupted);
        assert_eq!(result.spell, "Sleep");
    }

    #[test]
    fn cast_spell_clears_declaration() {
        let mut state = state_with_caster_combat();
        let _ = action_roll_initiative(&mut state);
        let _ = action_declare_spell(&mut state, "Zara", "Magic Missile");

        assert!(!state.combat.as_ref().unwrap().pending_spells.is_empty());
        assert!(!state.combat.as_ref().unwrap().spell_declarations.is_empty());

        let _ = action_cast_spell(&mut state, "Zara");

        assert!(state.combat.as_ref().unwrap().pending_spells.is_empty());
        assert!(state.combat.as_ref().unwrap().spell_declarations.is_empty());
    }

    #[test]
    fn cast_spell_marks_character_acted() {
        let mut state = state_with_caster_combat();
        let _ = action_roll_initiative(&mut state);
        let _ = action_declare_spell(&mut state, "Zara", "Magic Missile");

        let _ = action_cast_spell(&mut state, "Zara");

        // Character cannot attack after casting
        let result = action_attack(&mut state, "Zara", 0, "Dagger");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already acted"), "expected 'already acted', got: {}", err);
    }

    #[test]
    fn cast_spell_no_combat_error() {
        let mut state = GameState::new();
        let mut mage = crate::model::Character::new("Zara", crate::rules::class::Class::MagicUser);
        mage.hp = 3;
        mage.max_hp = 3;
        state.party.add_member(mage);

        let result = action_cast_spell(&mut state, "Zara");
        assert!(result.is_err());
    }

    #[test]
    fn cast_spell_unknown_character_error() {
        let mut state = state_with_caster_combat();
        let result = action_cast_spell(&mut state, "Nobody");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no party member"));
    }

    #[test]
    fn cast_spell_cannot_cast_twice_per_round() {
        let mut state = state_with_caster_combat();
        let _ = action_roll_initiative(&mut state);
        let _ = action_declare_spell(&mut state, "Zara", "Magic Missile");

        let result = action_cast_spell(&mut state, "Zara");
        assert!(result.is_ok());

        // Second cast attempt should fail (already acted)
        let _ = action_declare_spell(&mut state, "Zara", "Sleep");
        let result = action_cast_spell(&mut state, "Zara");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already acted"), "got: {}", err);
    }
}
