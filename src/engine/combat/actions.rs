use crate::dice;
use crate::engine::result::EngineError;
use crate::engine::retainer::{self, LoyaltyResult};
use crate::model::{CombatState, Monster};
use crate::persist::GameState;
use crate::rules::class::{class_def, Class};
use crate::rules::{ability, equipment, monster as monster_db, spell, spell_data, thief};
use crate::state::effect;

use super::results::{
    AddMonsterResult, AttackResult, BackstabResult, CastSpellResult, CloseResult, CombatLogResult,
    CombatStatusResult, CombatXpAward, DeclareSpellResult, EndCombatResult,
    FightingWithdrawalResult, InitiativeResult, InitiativeWinner, MonsterAttackResult,
    MoraleResult, NextPhaseResult, RetainerLoyaltyCheckResult, RetainerLoyaltyOutcome,
    RetreatResult, SetHelplessResult, SpawnEncounterResult, SpawnMonsterResult,
    SpawnPlacedDetail, SpawnPlacedResult, TurnUndeadResult,
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
    /// If provided, overrides the monster-DB lookup for the undead flag.
    /// Module monsters not in the core DB should set this explicitly.
    pub undead: Option<bool>,
    /// If provided, overrides the monster-DB lookup for weapon immunity.
    pub immune_to_normal_weapons: Option<bool>,
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

    let db_entry = monster_db::find_monster(params.name);
    let xp_per_monster = params.xp_value.unwrap_or_else(|| {
        db_entry.map(|m| m.xp()).unwrap_or(0)
    });
    let is_undead = params.undead.unwrap_or_else(|| {
        db_entry.map(|m| m.is_undead()).unwrap_or(false)
    });
    let is_immune = params.immune_to_normal_weapons.unwrap_or_else(|| {
        db_entry.map(|m| m.immune_to_normal_weapons()).unwrap_or(false)
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
        monster.attacks = db_entry.map(|d| d.attack_names()).unwrap_or_else(|| vec!["attack".to_string()]);
        monster.attack_routines = db_entry.map(|d| d.attack_routines()).unwrap_or_else(|| {
            vec![crate::model::MonsterAttackRoutine {
                name: "attack".to_string(),
                damage: params.damage.to_string(),
            }]
        });
        monster.undead = is_undead;
        monster.immune_to_normal_weapons = is_immune;
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
        m.attack_routines = def.attack_routines();
        m.undead = def.is_undead();
        m.immune_to_normal_weapons = def.immune_to_normal_weapons();
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

/// Spawn placed module monsters from the current room into combat.
///
/// Resolves monster definitions from module_monsters (loaded from companion
/// monsters.json) first, then falls back to the core monster database.
/// HP is rolled from hit dice per B/X rules.
pub fn action_spawn_placed(
    state: &mut GameState,
    distance: u32,
    name_filter: Option<&str>,
) -> Result<SpawnPlacedResult, EngineError> {
    if state.combat.is_some() {
        return Err(EngineError::WrongState(
            "combat already active. Use 'end_combat' first.".to_string(),
        ));
    }

    let dungeon = state.dungeon.as_ref().ok_or_else(|| {
        EngineError::WrongState("not in exploration mode.".to_string())
    })?;

    let room_id = dungeon.current_room.ok_or_else(|| {
        EngineError::WrongState("no current room.".to_string())
    })?;

    let room = dungeon.find_room(room_id).ok_or_else(|| {
        EngineError::InvalidInput(format!("room {} not found.", room_id))
    })?;

    // Collect unspawned placed monsters (optionally filtered by name)
    let targets: Vec<_> = room.placed_monsters.iter()
        .filter(|m| !m.spawned)
        .filter(|m| match name_filter {
            Some(filter) => m.name.eq_ignore_ascii_case(filter),
            None => true,
        })
        .cloned()
        .collect();

    if targets.is_empty() {
        return Err(EngineError::InvalidInput(
            match name_filter {
                Some(name) => format!("no unspawned placed monster named '{}' in this room.", name),
                None => "no unspawned placed monsters in this room.".to_string(),
            }
        ));
    }

    // Resolve monster definitions and build Monster objects
    let mut monsters = Vec::new();
    let mut spawn_details = Vec::new();
    let mut unresolved = Vec::new();

    for target in &targets {
        let key = target.name.to_lowercase();
        let def = state.module_monsters.get(&key)
            .or_else(|| monster_db::find_monster(&target.name));

        match def {
            Some(def) => {
                let is_undead = target.undead.unwrap_or_else(|| def.is_undead());
                let is_immune = def.immune_to_normal_weapons();
                let source = if state.module_monsters.contains_key(&key) {
                    "module"
                } else {
                    "core"
                };

                for i in 0..target.count {
                    let monster_name = if target.count > 1 {
                        format!("{} {}", def.name, i + 1)
                    } else {
                        def.name.to_string()
                    };

                    // Roll HP from HD (same logic as action_spawn_monster)
                    let dice_count = def.hit_dice.hp_dice_count();
                    let hp_mod = def.hit_dice.hp_modifier();
                    let hp = if dice_count == 0 {
                        match dice::roll_str("1d4") {
                            Ok(r) => (r.total + hp_mod).max(1),
                            Err(_) => 2,
                        }
                    } else {
                        match dice::roll_str(&format!("{}d8", dice_count)) {
                            Ok(r) => (r.total + hp_mod).max(1),
                            Err(_) => (dice_count as i32 * 4 + hp_mod).max(1),
                        }
                    };

                    let mut m = Monster::new(&monster_name, def.hit_dice.clone());
                    m.hp = hp;
                    m.max_hp = hp;
                    m.ac = def.ac();
                    m.damage = def.damage();
                    m.morale = def.morale;
                    m.xp_value = def.xp();
                    m.attacks = def.attack_names();
                    m.attack_routines = def.attack_routines();
                    m.undead = is_undead;
                    m.immune_to_normal_weapons = is_immune;
                    monsters.push(m);
                }

                spawn_details.push(SpawnPlacedDetail {
                    name: def.name.clone(),
                    count: target.count,
                    hit_dice: def.hit_dice.clone(),
                    ac: def.ac(),
                    morale: def.morale,
                    xp_per_monster: def.xp(),
                    special: def.special(),
                    source: source.to_string(),
                });
            }
            None => {
                unresolved.push(target.name.clone());
            }
        }
    }

    if monsters.is_empty() {
        return Err(EngineError::InvalidInput(format!(
            "no monster definitions found for: {}. Use SpawnEncounter with manual stats.",
            unresolved.join(", ")
        )));
    }

    // Mark placed monsters as spawned
    let dungeon = state.dungeon.as_mut().unwrap();
    if let Some(room) = dungeon.find_room_mut(room_id) {
        for pm in &mut room.placed_monsters {
            if targets.iter().any(|t| t.name == pm.name) && !pm.spawned {
                pm.spawned = true;
            }
        }
    }

    // Enter combat
    let combat = CombatState::new(monsters, distance);
    let status = combat_status(&combat, &state.party.members);
    state.enter_combat(combat);

    let mut msg = format!(
        "combat started: placed monsters at {}' distance.",
        distance
    );
    for detail in &spawn_details {
        msg.push_str(&format!(
            " {} x{} (HD {}, AC {}, Morale {}, {} XP) [{}].",
            detail.name, detail.count, detail.hit_dice, detail.ac,
            detail.morale, detail.xp_per_monster, detail.source
        ));
        if !detail.special.is_empty() {
            msg.push_str(&format!(" Special: {}", detail.special));
        }
    }
    if !unresolved.is_empty() {
        msg.push_str(&format!(
            " WARNING: {} not found in DB, skipped. Use AddMonster for them.",
            unresolved.join(", ")
        ));
    }

    Ok(SpawnPlacedResult {
        message: msg,
        distance,
        spawned: spawn_details,
        unresolved,
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

    let db_entry = monster_db::find_monster(params.name);
    let xp_per_monster = params.xp_value.unwrap_or_else(|| {
        db_entry.map(|m| m.xp()).unwrap_or(0)
    });
    let is_undead = params.undead.unwrap_or_else(|| {
        db_entry.map(|m| m.is_undead()).unwrap_or(false)
    });
    let is_immune = params.immune_to_normal_weapons.unwrap_or_else(|| {
        db_entry.map(|m| m.immune_to_normal_weapons()).unwrap_or(false)
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
        monster.attacks = db_entry.map(|d| d.attack_names()).unwrap_or_else(|| vec!["attack".to_string()]);
        monster.attack_routines = db_entry.map(|d| d.attack_routines()).unwrap_or_else(|| {
            vec![crate::model::MonsterAttackRoutine {
                name: "attack".to_string(),
                damage: params.damage.to_string(),
            }]
        });
        monster.undead = is_undead;
        monster.immune_to_normal_weapons = is_immune;
        combat.monsters.push(monster);
    }

    let total_monsters = combat.monsters.len();
    let status = combat_status(combat, &state.party.members);

    combat.log_event(format!(
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

    // Tick round-based effects before advancing to the new round.
    // This happens at the EndOfRound→Declaration transition point.
    if combat.round > 0 {
        tick_round_effects_on_combat(state);
    }

    let combat = state.combat.as_mut().unwrap();
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

/// Tick all Rounds-based effects on monsters, party members, and global effects.
/// Logs expiry messages to the combat log.
fn tick_round_effects_on_combat(state: &mut GameState) {
    let mut all_messages = Vec::new();

    // Tick effects on all monsters in combat
    if let Some(ref mut combat) = state.combat {
        for monster in &mut combat.monsters {
            let messages = effect::tick_round_effects(&mut monster.effects, &monster.name);
            all_messages.extend(messages);
        }
    }

    // Tick effects on all party members
    for member in &mut state.party.members {
        let messages = effect::tick_round_effects(&mut member.effects, &member.name);
        all_messages.extend(messages);
    }

    // Tick global effects
    {
        let messages = effect::tick_round_effects(&mut state.effects, "the battlefield");
        all_messages.extend(messages);
    }

    // Log all expiry messages to the combat log
    if let Some(ref mut combat) = state.combat {
        for msg in all_messages {
            combat.log_event(msg);
        }
    }
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

    // Check weapon immunity: monsters immune to normal weapons cannot be harmed
    // by non-magical weapons.
    if monster_idx < combat.monsters.len()
        && combat.monsters[monster_idx].is_alive()
        && combat.monsters[monster_idx].immune_to_normal_weapons
        && !equipment::is_magical_weapon(weapon_name)
    {
        return Err(EngineError::InvalidInput(format!(
            "{} is immune to normal weapons! {} has no effect. Use a magical weapon.",
            combat.monsters[monster_idx].name, weapon_name
        )));
    }

    if monster_idx < combat.monsters.len()
        && combat.monsters[monster_idx].is_alive()
        && combat.monsters[monster_idx].helpless
    {
        // Coup de grace requires melee range (≤10')
        if combat.distance > 10 {
            return Err(EngineError::InvalidInput(format!(
                "cannot dispatch helpless {} at {}' distance. Use \"close {}\" to move into melee range.",
                combat.monsters[monster_idx].name, combat.distance, char_name
            )));
        }
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
        if combat.monsters[monster_idx].turned {
            return Err(EngineError::InvalidInput(format!(
                "{} is turned and cannot attack.",
                combat.monsters[monster_idx].name
            )));
        }
        let attacks_used = combat.monsters_attacked_this_round.get(&monster_idx).copied().unwrap_or(0);
        let max_attacks = combat.monsters[monster_idx].attack_routines.len().max(1);
        if attacks_used >= max_attacks {
            return Err(EngineError::InvalidInput(format!(
                "{} has already used all {} attack(s) this round.",
                combat.monsters[monster_idx].name, max_attacks
            )));
        }
        // Monsters use melee attacks — require melee distance (≤10')
        if combat.distance > 10 {
            return Err(EngineError::InvalidInput(format!(
                "{} cannot melee attack at {}' distance. Monsters must be within 10' for melee.",
                combat.monsters[monster_idx].name, combat.distance
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
    let attacks_used = combat.monsters_attacked_this_round.get(&monster_idx).copied().unwrap_or(0);
    let max_attacks = combat.monsters[monster_idx].attack_routines.len().max(1);
    let result = monster_attack(combat, monster_idx, character, attacks_used);
    *combat.monsters_attacked_this_round.entry(monster_idx).or_insert(0) += 1;
    let attacks_remaining = max_attacks - (attacks_used + 1);
    let routine_name = combat.monsters[monster_idx].attack_routines
        .get(attacks_used)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "attack".to_string());
    Ok(MonsterAttackResult::new(result, routine_name, attacks_remaining))
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
    if !combat.monsters[monster_idx].undead {
        return Err(EngineError::InvalidInput(format!(
            "{} is not undead.",
            combat.monsters[monster_idx].name
        )));
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
        distance_moved: combat.distance.saturating_sub(old_distance),
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
        log: combat.log.iter().map(|e| e.message.clone()).collect(),
    })
}

pub fn action_declare_spell(
    state: &mut GameState,
    char_name: &str,
    spell_name: &str,
) -> Result<DeclareSpellResult, EngineError> {
    let character = state.party.find_member(char_name).ok_or_else(|| {
        EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
    })?;

    let cdef = class_def(character.class);
    if !spell::can_cast(cdef.spell_progression, character.level) {
        return Err(EngineError::InvalidInput(format!(
            "{} ({}) cannot cast spells.",
            character.name,
            character.class.name()
        )));
    }

    // Check spell slot availability via DSL-gated casting rules
    if let Some(spell_def) = spell_data::find_spell(spell_name, None) {
        let max_slots = spell::spell_slots(cdef.spell_progression, character.level);
        if !spell::can_cast_spell(&character.spell_slots_used, &max_slots, spell_def.level) {
            let idx = (spell_def.level - 1) as usize;
            let (used, max) = if idx < 6 {
                (character.spell_slots_used[idx], max_slots[idx])
            } else {
                (0, 0)
            };
            return Err(EngineError::InvalidInput(format!(
                "{} has no level {} spell slots remaining ({} of {} used).",
                character.name, spell_def.level, used, max
            )));
        }
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

    let result = if was_disrupted {
        combat.log_event(format!(
            "{}'s {} fizzles — spell was disrupted!",
            char_name, spell_name
        ));
        CastSpellResult {
            message: format!(
                "{}'s {} was disrupted! The spell fails.",
                char_name, spell_name
            ),
            character: char_name.to_string(),
            spell: spell_name,
            cast: false,
            disrupted: true,
        }
    } else {
        combat.log_event(format!(
            "{} casts {}!",
            char_name, spell_name
        ));
        CastSpellResult {
            message: format!(
                "{} casts {}! Apply spell effects as appropriate.",
                char_name, spell_name
            ),
            character: char_name.to_string(),
            spell: spell_name,
            cast: true,
            disrupted: false,
        }
    };

    // Consume spell slot via DSL-gated cost (whether disrupted or not — per B/X, attempted casting uses the slot)
    if let Some(spell_def) = spell_data::find_spell(&result.spell, None) {
        let cost = spell::cast_cost(spell_def.level);
        let idx = (spell_def.level - 1) as usize;
        if idx < 6 {
            if let Some(character) = state.party.find_member_mut(char_name) {
                character.spell_slots_used[idx] += cost;
            }
        }
    }

    Ok(result)
}

pub fn action_end_combat(state: &mut GameState, skip_xp: bool) -> Result<EndCombatResult, EngineError> {
    if state.combat.is_none() {
        return Err(no_active_combat());
    }
    let combat = state.exit_combat().unwrap();

    let rounds = combat.round;
    let monsters_defeated = combat.monsters.iter().filter(|m| !m.is_alive()).count();
    let total_monsters = combat.monsters.len();
    let total_xp: u64 = combat
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

    // Auto-distribute monster XP equally among surviving party members (B/X rules).
    // skip_xp allows the GM to handle XP manually via AwardTreasureXp (avoids double-award).
    let survivors: Vec<usize> = state
        .party
        .members
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_alive())
        .map(|(i, _)| i)
        .collect();
    let survivor_count = survivors.len() as u64;
    let xp_per_survivor = if !skip_xp && total_xp > 0 && survivor_count > 0 {
        total_xp / survivor_count
    } else {
        0
    };

    let mut xp_awards = Vec::new();
    if xp_per_survivor > 0 {
        for &idx in &survivors {
            let character = &mut state.party.members[idx];
            let result = crate::engine::xp::award_xp(character, 0, xp_per_survivor);
            xp_awards.push(CombatXpAward {
                character: character.name.clone(),
                base_xp: result.base_xp,
                modifier_pct: result.modifier_pct,
                adjusted_xp: result.adjusted_xp,
                total_xp: result.new_total,
                ready_to_train: result.ready_to_train,
            });
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
        xp_per_survivor,
        xp_awards,
        party_casualties,
        mode_after: state.mode,
        retainer_xp_each,
        retainer_xp_recipients,
        retainer_loyalty_checks,
    })
}

pub fn action_next_phase(state: &mut GameState) -> Result<NextPhaseResult, EngineError> {
    let combat = state.combat.as_mut().ok_or_else(no_active_combat)?;
    let previous = combat.phase.clone();
    combat.advance_phase();
    let current = combat.phase.clone();
    let msg = format!(
        "Phase: {} → {}",
        crate::model::phase_display_name(&previous),
        crate::model::phase_display_name(&current),
    );
    combat.log_event(msg.clone());

    Ok(NextPhaseResult {
        message: msg,
        previous_phase: previous,
        current_phase: current,
        round: combat.round,
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

    // Check weapon immunity
    if combat.monsters[monster_idx].immune_to_normal_weapons
        && !equipment::is_magical_weapon(weapon_name)
    {
        return Err(EngineError::InvalidInput(format!(
            "{} is immune to normal weapons! {} has no effect.",
            combat.monsters[monster_idx].name, weapon_name
        )));
    }

    // Backstab is a melee attack — enforce melee distance check
    let qualities = weapon.weapon_qualities();
    if !qualities.missile || qualities.melee {
        // Pure melee or versatile weapons require melee range for backstab
        if combat.distance > 10 {
            return Err(EngineError::InvalidInput(format!(
                "{} is a melee weapon but monsters are {}' away. \
                Use \"close {}\" to move into melee range.",
                weapon.name, combat.distance, character.name
            )));
        }
    } else {
        // Pure missile weapons cannot be used for backstab
        return Err(EngineError::InvalidInput(format!(
            "{} is a missile weapon and cannot be used for backstab. \
            Backstab requires a melee weapon.",
            weapon.name
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
        let total_damage = base_damage.saturating_mul(multiplier as i32).saturating_add(str_mod).max(1);
        combat.monsters[monster_idx].hp -= total_damage;
        let monster_name = combat.monsters[monster_idx].name.clone();
        let alive = combat.monsters[monster_idx].is_alive();
        combat.log_event(format!(
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
        combat.log_event(format!(
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
    Ok(CombatStatusResult { status })
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
            err.contains("already used all"),
            "expected 'already used all attacks' guard, got: {}",
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

    // --- turned monster attack guard (oag-pjcjk) ---

    #[test]
    fn turned_monster_cannot_attack() {
        let mut state = state_with_melee_combat();
        let _ = action_roll_initiative(&mut state);

        // Mark monster as turned
        state.combat.as_mut().unwrap().monsters[0].turned = true;

        // Turned monster should not be allowed to attack
        let result = action_monster_attack(&mut state, 0, "Grond");
        assert!(result.is_err(), "turned monster should not be able to attack");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("turned") && err.contains("cannot attack"),
            "expected turned guard, got: {}",
            err
        );
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

        // Advance through all phases to reach Declaration for the new round.
        // EndOfRound → Declaration clears spell declarations.
        let combat = state.combat.as_mut().unwrap();
        while combat.phase != "EndOfRound" {
            combat.advance_phase();
        }
        combat.advance_phase(); // EndOfRound → Declaration (clears spell state)

        // Should be allowed again in the new declaration phase
        let result = action_declare_spell(&mut state, "Zara", "Magic Missile");
        assert!(result.is_ok(), "declaration in new round should succeed: {:?}", result.err());
    }

    // --- cast_spell (oag-aarw0) ---

    #[test]
    fn cast_spell_resolves_declared_spell() {
        let mut state = state_with_caster_combat();
        let _ = action_declare_spell(&mut state, "Zara", "Magic Missile");
        let _ = action_roll_initiative(&mut state);

        // Declaration survives initiative — no re-declaration needed
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

    // --- backstab damage cap (oag-bxwm9) ---

    fn test_thief(name: &str, str_score: i32) -> Character {
        let mut c = Character::new(name, Class::Thief);
        c.hp = 6;
        c.max_hp = 6;
        c.ac = 6;
        c.level = 1;
        c.abilities.strength = str_score;
        c
    }

    #[test]
    fn backstab_damage_does_not_exceed_weapon_max_times_multiplier_plus_str() {
        // Short sword = 1d6, level 1 = x2 multiplier, STR 10 = +0 mod
        // Max damage should be 6*2 + 0 = 12
        let max_expected = 12;
        let mut state = GameState::new();
        state.party.add_member(test_thief("Stabby", 10));
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![mk_monster("Dummy", "10", 999, 9, 7)], // high HP so it survives
            5,
        ));

        for _ in 0..200 {
            state.combat.as_mut().unwrap().monsters[0].hp = 999;
            state.combat.as_mut().unwrap().characters_acted.clear();
            let result = action_backstab(&mut state, "Stabby", 0, "Short sword");
            assert!(result.is_ok(), "backstab failed: {:?}", result.err());
            let result = result.unwrap();
            if result.hit {
                let dmg = result.damage.unwrap();
                assert!(dmg <= max_expected,
                    "backstab damage {} exceeds max {} (1d6 x2 + STR 10 mod 0)",
                    dmg, max_expected);
                assert!(dmg >= 2,
                    "backstab damage {} below minimum 2 (1*2 + 0)", dmg);
            }
        }
    }

    #[test]
    fn backstab_str_mod_not_multiplied() {
        // Short sword = 1d6, level 1 = x2 multiplier, STR 18 = +3 mod
        // Correct max: 6*2 + 3 = 15 (multiplier on dice only)
        // Wrong max (old bug): (6+3)*2 = 18
        let max_correct = 15;
        let mut state = GameState::new();
        state.party.add_member(test_thief("Strong Thief", 18));
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![mk_monster("Dummy", "10", 999, 9, 7)],
            5,
        ));

        for _ in 0..200 {
            state.combat.as_mut().unwrap().monsters[0].hp = 999;
            state.combat.as_mut().unwrap().characters_acted.clear();
            let result = action_backstab(&mut state, "Strong Thief", 0, "Short sword");
            assert!(result.is_ok());
            let result = result.unwrap();
            if result.hit {
                let dmg = result.damage.unwrap();
                assert!(dmg <= max_correct,
                    "backstab damage {} exceeds correct max {} — STR mod may be multiplied (1d6 x2 + STR 18 mod +3)",
                    dmg, max_correct);
                assert!(dmg >= 5,
                    "backstab damage {} below minimum 5 (1*2 + 3)", dmg);
            }
        }
    }

    // --- backstab melee distance check (oag-mxw8t) ---

    #[test]
    fn backstab_rejected_at_ranged_distance_with_melee_weapon() {
        let mut state = GameState::new();
        state.party.add_member(test_thief("Stabby", 10));
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![mk_monster("Goblin", "1", 4, 6, 7)],
            20, // out of melee range
        ));

        let result = action_backstab(&mut state, "Stabby", 0, "Short sword");
        assert!(result.is_err(), "backstab should fail at 20' distance with melee weapon");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("melee weapon"), "expected melee weapon error, got: {}", err);
        assert!(err.contains("20'"), "expected distance in error, got: {}", err);
    }

    #[test]
    fn backstab_succeeds_at_melee_distance() {
        let mut state = GameState::new();
        state.party.add_member(test_thief("Stabby", 10));
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![mk_monster("Goblin", "1", 999, 6, 7)],
            5, // melee range
        ));

        // Should succeed (at least not fail due to distance)
        let result = action_backstab(&mut state, "Stabby", 0, "Short sword");
        assert!(result.is_ok(), "backstab should succeed at 5' distance: {:?}", result.err());
    }

    #[test]
    fn backstab_rejected_with_pure_missile_weapon() {
        let mut state = GameState::new();
        state.party.add_member(test_thief("Stabby", 10));
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![mk_monster("Goblin", "1", 4, 6, 7)],
            5,
        ));

        let result = action_backstab(&mut state, "Stabby", 0, "Short bow");
        assert!(result.is_err(), "backstab should fail with missile weapon");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("missile weapon"), "expected missile weapon error, got: {}", err);
    }

    #[test]
    fn backstab_with_dagger_at_melee_range_succeeds() {
        // Dagger is versatile (melee+missile) — should work at melee range
        let mut state = GameState::new();
        state.party.add_member(test_thief("Stabby", 10));
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![mk_monster("Goblin", "1", 999, 6, 7)],
            5,
        ));

        let result = action_backstab(&mut state, "Stabby", 0, "Dagger");
        assert!(result.is_ok(), "backstab with dagger at melee range should work: {:?}", result.err());
    }

    #[test]
    fn backstab_with_dagger_at_ranged_distance_rejected() {
        // Dagger is versatile but backstab requires melee range
        let mut state = GameState::new();
        state.party.add_member(test_thief("Stabby", 10));
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![mk_monster("Goblin", "1", 4, 6, 7)],
            20,
        ));

        let result = action_backstab(&mut state, "Stabby", 0, "Dagger");
        assert!(result.is_err(), "backstab with dagger at 20' should fail");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("melee weapon"), "expected melee weapon error, got: {}", err);
    }

    // --- fighting_withdrawal distance_moved (oag-09jya) ---

    #[test]
    fn fighting_withdrawal_distance_moved_is_nonzero() {
        let mut state = state_with_combat(); // distance = 60, fighter movement_rate = 120
        // encounter_move = 120/3 = 40, half = 20
        let result = action_fighting_withdrawal(&mut state, "Grond").unwrap();
        assert_eq!(result.distance_moved, 20, "distance_moved should be 20 (half encounter move)");
        assert_eq!(result.new_distance, 80, "new distance should be 60 + 20 = 80");
    }

    // --- spell slot tracking (oag-t8467) ---

    #[test]
    fn declare_spell_rejected_when_slots_exhausted() {
        // Level 1 MU has 1 first-level slot. After casting, no more slots.
        let mut state = GameState::new();
        let mut caster = Character::new("Zara", Class::MagicUser);
        caster.hp = 6;
        caster.max_hp = 6;
        caster.level = 1; // ArcaneFullCaster level 1 = [1, 0, 0, 0, 0, 0]
        state.party.add_member(caster);
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![mk_monster("Goblin 1", "1", 4, 6, 7)],
            60,
        ));

        // First declaration succeeds
        let _ = action_roll_initiative(&mut state);
        let result = action_declare_spell(&mut state, "Zara", "Sleep");
        assert!(result.is_ok(), "first declare should succeed: {:?}", result.err());

        // Cast the spell (consumes the slot)
        let result = action_cast_spell(&mut state, "Zara");
        assert!(result.is_ok(), "cast should succeed: {:?}", result.err());

        // Verify slot was consumed
        assert_eq!(state.party.find_member("Zara").unwrap().spell_slots_used[0], 1);

        // New round, try to declare again — should fail (no slots left)
        let _ = action_roll_initiative(&mut state);
        let result = action_declare_spell(&mut state, "Zara", "Sleep");
        assert!(result.is_err(), "second declare should fail (no slots remaining)");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("no level 1 spell slots remaining"),
            "expected slot exhaustion error, got: {}", err);
    }

    #[test]
    fn multiple_slots_allow_multiple_casts() {
        // Level 3 MU has [2, 1, 0, 0, 0, 0] — 2 first-level slots
        let mut state = state_with_caster_combat(); // level 3 MU
        let _ = action_roll_initiative(&mut state);

        // First cast
        let _ = action_declare_spell(&mut state, "Zara", "Sleep");
        let _ = action_cast_spell(&mut state, "Zara");

        // New round — second cast should succeed (2 slots available)
        let _ = action_roll_initiative(&mut state);
        let result = action_declare_spell(&mut state, "Zara", "Sleep");
        assert!(result.is_ok(), "second declare should succeed (2 slots): {:?}", result.err());
        let _ = action_cast_spell(&mut state, "Zara");

        // Third round — should fail (both slots used)
        let _ = action_roll_initiative(&mut state);
        let result = action_declare_spell(&mut state, "Zara", "Sleep");
        assert!(result.is_err(), "third declare should fail (slots exhausted)");
    }

    #[test]
    fn disrupted_spell_still_consumes_slot() {
        let mut state = GameState::new();
        let mut caster = Character::new("Zara", Class::MagicUser);
        caster.hp = 6;
        caster.max_hp = 6;
        caster.level = 1;
        state.party.add_member(caster);
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(
            vec![mk_monster("Goblin 1", "1", 4, 6, 7)],
            60,
        ));

        let _ = action_roll_initiative(&mut state);
        let _ = action_declare_spell(&mut state, "Zara", "Sleep");

        // Manually disrupt the caster
        state.combat.as_mut().unwrap().disrupted.push("Zara".to_string());

        let result = action_cast_spell(&mut state, "Zara");
        assert!(result.is_ok());
        assert!(result.unwrap().disrupted);

        // Slot should still be consumed
        assert_eq!(state.party.find_member("Zara").unwrap().spell_slots_used[0], 1,
            "disrupted spell should still consume a slot");

        // Next round — should fail (no slots left)
        let _ = action_roll_initiative(&mut state);
        let result = action_declare_spell(&mut state, "Zara", "Sleep");
        assert!(result.is_err(), "should have no slots after disrupted cast");
    }

    // --- end_combat auto-distributes monster XP (oag-f5d2j) ---

    #[test]
    fn end_combat_distributes_xp_to_survivors() {
        let mut state = GameState::new();
        let mut fighter = test_fighter();
        fighter.abilities.strength = 10; // no prime req bonus
        state.party.add_member(fighter);
        let mut cleric = Character::new("Mira", Class::Cleric);
        cleric.hp = 8;
        cleric.max_hp = 8;
        cleric.level = 1;
        cleric.abilities.wisdom = 10;
        state.party.add_member(cleric);

        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        let mut m1 = mk_monster("Goblin 1", "1", 4, 6, 7);
        m1.xp_value = 10;
        m1.hp = 0; // dead
        let mut m2 = mk_monster("Goblin 2", "1", 4, 6, 7);
        m2.xp_value = 10;
        m2.hp = 0; // dead
        state.combat = Some(CombatState::new(vec![m1, m2], 5));

        let result = action_end_combat(&mut state, false).unwrap();
        assert_eq!(result.total_xp, 20);
        assert_eq!(result.xp_per_survivor, 10); // 20 / 2 survivors
        assert_eq!(result.xp_awards.len(), 2);
        assert_eq!(result.xp_awards[0].character, "Grond");
        assert_eq!(result.xp_awards[0].base_xp, 10);
        assert_eq!(result.xp_awards[1].character, "Mira");
        assert_eq!(result.xp_awards[1].base_xp, 10);

        // Verify XP was actually awarded to characters
        assert_eq!(state.party.find_member("Grond").unwrap().xp, 10);
        assert_eq!(state.party.find_member("Mira").unwrap().xp, 10);
    }

    #[test]
    fn end_combat_xp_excludes_dead_party_members() {
        let mut state = GameState::new();
        let mut alive = test_fighter();
        alive.abilities.strength = 10;
        state.party.add_member(alive);
        let mut dead = Character::new("Fallen", Class::Fighter);
        dead.hp = 0;
        dead.max_hp = 8;
        dead.level = 1;
        state.party.add_member(dead);

        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        let mut m = mk_monster("Orc", "1", 4, 6, 7);
        m.xp_value = 30;
        m.hp = 0;
        state.combat = Some(CombatState::new(vec![m], 5));

        let result = action_end_combat(&mut state, false).unwrap();
        assert_eq!(result.total_xp, 30);
        assert_eq!(result.xp_per_survivor, 30); // only 1 survivor
        assert_eq!(result.xp_awards.len(), 1);
        assert_eq!(result.xp_awards[0].character, "Grond");
        assert_eq!(state.party.find_member("Grond").unwrap().xp, 30);
        assert_eq!(state.party.find_member("Fallen").unwrap().xp, 0);
    }

    #[test]
    fn end_combat_no_xp_when_no_kills() {
        let mut state = state_with_combat(); // 2 goblins alive, xp_value=5 each
        let result = action_end_combat(&mut state, false).unwrap();
        assert_eq!(result.total_xp, 0);
        assert_eq!(result.xp_per_survivor, 0);
        assert!(result.xp_awards.is_empty());
        assert_eq!(state.party.find_member("Grond").unwrap().xp, 0);
    }

    #[test]
    fn end_combat_xp_applies_prime_req_modifier() {
        let mut state = GameState::new();
        let fighter = test_fighter(); // STR 16 = +10% for Fighter
        state.party.add_member(fighter);

        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        let mut m = mk_monster("Orc", "1", 4, 6, 7);
        m.xp_value = 100;
        m.hp = 0;
        state.combat = Some(CombatState::new(vec![m], 5));

        let result = action_end_combat(&mut state, false).unwrap();
        assert_eq!(result.xp_per_survivor, 100);
        assert_eq!(result.xp_awards.len(), 1);
        assert_eq!(result.xp_awards[0].modifier_pct, 10);
        assert_eq!(result.xp_awards[0].adjusted_xp, 110); // 100 + 10%
        assert_eq!(state.party.find_member("Grond").unwrap().xp, 110);
    }

    // --- round-based effect ticking (oag-1damc) ---

    fn add_effect(effects: &mut Vec<crate::state::effect::ActiveEffect>, name: &str, duration: crate::state::effect::EffectDuration) {
        use crate::state::effect::{ActiveEffect, next_effect_id};
        let id = next_effect_id(effects);
        effects.push(ActiveEffect {
            id,
            name: name.to_string(),
            source: "test".to_string(),
            duration,
            modifiers: Vec::new(),
            notes: String::new(),
        });
    }

    #[test]
    fn tick_rounds_effect_on_monster_decrements_and_expires() {
        use crate::state::effect::EffectDuration;

        let mut state = state_with_melee_combat();
        // Add a Rounds(2) effect to monster 0
        add_effect(&mut state.combat.as_mut().unwrap().monsters[0].effects, "Hold Person", EffectDuration::Rounds(2));

        // Round 1 initiative (first round, no tick yet since round==0 before)
        let _ = action_roll_initiative(&mut state);
        // Effect should still be there (round was 0 before, no tick)
        assert_eq!(state.combat.as_ref().unwrap().monsters[0].effects.len(), 1);

        // Need an action between rounds to allow next initiative
        state.combat.as_mut().unwrap().log_event("dummy action".into());

        // Round 2 initiative — ticks the effect from Rounds(2) -> Rounds(1)
        let _ = action_roll_initiative(&mut state);
        assert_eq!(state.combat.as_ref().unwrap().monsters[0].effects.len(), 1);
        assert_eq!(
            state.combat.as_ref().unwrap().monsters[0].effects[0].duration,
            EffectDuration::Rounds(1)
        );

        state.combat.as_mut().unwrap().log_event("dummy action".into());

        // Round 3 initiative — ticks Rounds(1) -> Rounds(0) and expires
        let _ = action_roll_initiative(&mut state);
        assert!(state.combat.as_ref().unwrap().monsters[0].effects.is_empty());
        // Expiry message should be in the combat log
        let log_text: String = state.combat.as_ref().unwrap().log.iter()
            .map(|e| e.message.clone()).collect::<Vec<_>>().join("\n");
        assert!(log_text.contains("Hold Person"), "expiry message should mention effect name");
        assert!(log_text.contains("worn off"), "expiry message should say worn off");
    }

    #[test]
    fn tick_rounds_effect_on_character_in_combat() {
        use crate::state::effect::EffectDuration;

        let mut state = state_with_melee_combat();
        // Add a Rounds(1) effect to the party member
        add_effect(&mut state.party.members[0].effects, "Bless", EffectDuration::Rounds(1));

        // Round 1
        let _ = action_roll_initiative(&mut state);
        // No tick yet (was round 0)
        assert_eq!(state.party.members[0].effects.len(), 1);

        state.combat.as_mut().unwrap().log_event("dummy action".into());

        // Round 2 — ticks Rounds(1) -> expired
        let _ = action_roll_initiative(&mut state);
        assert!(state.party.members[0].effects.is_empty());
        let log_text: String = state.combat.as_ref().unwrap().log.iter()
            .map(|e| e.message.clone()).collect::<Vec<_>>().join("\n");
        assert!(log_text.contains("Bless"), "expiry message should mention Bless");
    }

    #[test]
    fn turns_permanent_concentration_not_ticked_by_round() {
        use crate::state::effect::EffectDuration;

        let mut state = state_with_melee_combat();
        let monster = &mut state.combat.as_mut().unwrap().monsters[0];
        add_effect(&mut monster.effects, "Light", EffectDuration::Turns(6));
        add_effect(&mut monster.effects, "Curse", EffectDuration::Permanent);
        add_effect(&mut monster.effects, "Detect Magic", EffectDuration::Concentration);

        // Round 1
        let _ = action_roll_initiative(&mut state);
        state.combat.as_mut().unwrap().log_event("dummy action".into());

        // Round 2 — tick should NOT affect Turns/Permanent/Concentration
        let _ = action_roll_initiative(&mut state);

        let effects = &state.combat.as_ref().unwrap().monsters[0].effects;
        assert_eq!(effects.len(), 3, "all non-round effects should remain");
        assert_eq!(effects[0].duration, EffectDuration::Turns(6), "Turns effect should not be ticked");
        assert_eq!(effects[1].duration, EffectDuration::Permanent, "Permanent should not be ticked");
        assert_eq!(effects[2].duration, EffectDuration::Concentration, "Concentration should not be ticked");
    }

    #[test]
    fn expiry_messages_in_combat_log() {
        use crate::state::effect::EffectDuration;

        let mut state = state_with_melee_combat();
        // Add effects that expire at different times
        add_effect(&mut state.combat.as_mut().unwrap().monsters[0].effects, "Sleep", EffectDuration::Rounds(1));
        add_effect(&mut state.combat.as_mut().unwrap().monsters[1].effects, "Web", EffectDuration::Rounds(1));

        // Round 1 (no tick, round was 0)
        let _ = action_roll_initiative(&mut state);
        state.combat.as_mut().unwrap().log_event("dummy".into());

        let log_before = state.combat.as_ref().unwrap().log.len();

        // Round 2 — both effects expire
        let _ = action_roll_initiative(&mut state);

        let log_after = state.combat.as_ref().unwrap().log.len();
        // Should have at least 2 new expiry messages plus the initiative message
        assert!(log_after > log_before + 2, "should have logged expiry messages");

        let log_text: String = state.combat.as_ref().unwrap().log.iter()
            .map(|e| e.message.clone()).collect::<Vec<_>>().join("\n");
        assert!(log_text.contains("Sleep"), "log should mention Sleep expiry");
        assert!(log_text.contains("Web"), "log should mention Web expiry");
    }

    #[test]
    fn multiple_effects_across_characters_and_monsters() {
        use crate::state::effect::EffectDuration;

        let mut state = state_with_melee_combat();

        // Add effects to monster 0
        add_effect(&mut state.combat.as_mut().unwrap().monsters[0].effects, "Slow", EffectDuration::Rounds(2));
        // Add effects to monster 1
        add_effect(&mut state.combat.as_mut().unwrap().monsters[1].effects, "Blindness", EffectDuration::Rounds(3));
        // Add effect to party member
        add_effect(&mut state.party.members[0].effects, "Shield", EffectDuration::Rounds(2));
        // Add global effect
        add_effect(&mut state.effects, "Darkness", EffectDuration::Rounds(1));

        // Round 1 (no tick)
        let _ = action_roll_initiative(&mut state);
        state.combat.as_mut().unwrap().log_event("dummy".into());

        // Round 2 — tick all: Slow 2->1, Blindness 3->2, Shield 2->1, Darkness 1->0 (expires)
        let _ = action_roll_initiative(&mut state);

        assert_eq!(state.combat.as_ref().unwrap().monsters[0].effects[0].duration, EffectDuration::Rounds(1));
        assert_eq!(state.combat.as_ref().unwrap().monsters[1].effects[0].duration, EffectDuration::Rounds(2));
        assert_eq!(state.party.members[0].effects[0].duration, EffectDuration::Rounds(1));
        assert!(state.effects.is_empty(), "Darkness should have expired");

        state.combat.as_mut().unwrap().log_event("dummy".into());

        // Round 3 — Slow 1->0 (expires), Shield 1->0 (expires), Blindness 2->1
        let _ = action_roll_initiative(&mut state);

        assert!(state.combat.as_ref().unwrap().monsters[0].effects.is_empty(), "Slow should have expired");
        assert_eq!(state.combat.as_ref().unwrap().monsters[1].effects[0].duration, EffectDuration::Rounds(1));
        assert!(state.party.members[0].effects.is_empty(), "Shield should have expired");
    }

    // --- action_spawn_placed tests ---

    use crate::state::dungeon::{DungeonState, PlacedMonsterInstance, Room};

    fn exploration_state_with_placed(
        monsters: Vec<PlacedMonsterInstance>,
    ) -> GameState {
        let mut state = GameState::new();
        state.party.add_member(test_fighter());

        let mut dungeon = DungeonState::new(1);
        let mut room = Room::new(0, "Guard Chamber");
        room.placed_monsters = monsters;
        dungeon.add_room(room).unwrap();
        dungeon.current_room = Some(0);

        state.enter_exploration(dungeon, 1);
        state
    }

    #[test]
    fn spawn_placed_core_db_monster() {
        if monster_db::find_monster("Skeleton").is_none() {
            return; // skip if no data
        }
        let placed = vec![PlacedMonsterInstance::new("Skeleton", 3)];
        let mut state = exploration_state_with_placed(placed);

        let result = action_spawn_placed(&mut state, 10, None).unwrap();
        assert_eq!(result.distance, 10);
        assert_eq!(result.spawned.len(), 1);
        assert_eq!(result.spawned[0].name, "Skeleton");
        assert_eq!(result.spawned[0].count, 3);
        assert_eq!(result.spawned[0].source, "core");
        assert!(result.unresolved.is_empty());
        assert_eq!(state.mode, GameMode::Combat);
        assert_eq!(state.combat.as_ref().unwrap().monsters.len(), 3);
    }

    #[test]
    fn spawn_placed_module_monster() {
        let placed = vec![PlacedMonsterInstance::new("Frost Giant", 2)];
        let mut state = exploration_state_with_placed(placed);

        // Add module monster definition
        let def = crate::rules::monster::MonsterDef::test_def("Frost Giant", 10, 4, 12);
        state.module_monsters.insert("frost giant".to_string(), def);

        let result = action_spawn_placed(&mut state, 20, None).unwrap();
        assert_eq!(result.spawned[0].source, "module");
        assert_eq!(result.spawned[0].count, 2);
        assert_eq!(state.combat.as_ref().unwrap().monsters.len(), 2);
    }

    #[test]
    fn spawn_placed_no_dungeon_error() {
        let mut state = GameState::new();
        state.party.add_member(test_fighter());

        let result = action_spawn_placed(&mut state, 10, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not in exploration"));
    }

    #[test]
    fn spawn_placed_combat_active_error() {
        let placed = vec![PlacedMonsterInstance::new("Skeleton", 1)];
        let mut state = exploration_state_with_placed(placed);
        state.enter_combat(CombatState::new(vec![mk_monster("Orc", "1", 4, 6, 8)], 30));

        let result = action_spawn_placed(&mut state, 10, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("combat already active"));
    }

    #[test]
    fn spawn_placed_no_unspawned_error() {
        let mut placed = PlacedMonsterInstance::new("Skeleton", 1);
        placed.spawned = true;
        let mut state = exploration_state_with_placed(vec![placed]);

        let result = action_spawn_placed(&mut state, 10, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no unspawned"));
    }

    #[test]
    fn spawn_placed_unresolved_monster_error() {
        let placed = vec![PlacedMonsterInstance::new("Unknown Beast", 1)];
        let mut state = exploration_state_with_placed(placed);

        let result = action_spawn_placed(&mut state, 10, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown Beast"));
    }

    #[test]
    fn spawn_placed_name_filter() {
        if monster_db::find_monster("Skeleton").is_none() {
            return;
        }
        let placed = vec![
            PlacedMonsterInstance::new("Skeleton", 3),
            PlacedMonsterInstance::new("Zombie", 2),
        ];
        let mut state = exploration_state_with_placed(placed);

        // Only spawn skeletons
        let result = action_spawn_placed(&mut state, 10, Some("Skeleton")).unwrap();
        assert_eq!(result.spawned.len(), 1);
        assert_eq!(result.spawned[0].name, "Skeleton");
        assert_eq!(state.combat.as_ref().unwrap().monsters.len(), 3);

        // Check that Zombie is still unspawned
        let dungeon = state.dungeon.as_ref().unwrap();
        let room = dungeon.find_room(0).unwrap();
        let zombie = room.placed_monsters.iter().find(|m| m.name == "Zombie").unwrap();
        assert!(!zombie.spawned);
    }

    #[test]
    fn spawn_placed_marks_spawned() {
        if monster_db::find_monster("Skeleton").is_none() {
            return;
        }
        let placed = vec![PlacedMonsterInstance::new("Skeleton", 3)];
        let mut state = exploration_state_with_placed(placed);

        let _ = action_spawn_placed(&mut state, 10, None).unwrap();

        let dungeon = state.dungeon.as_ref().unwrap();
        let room = dungeon.find_room(0).unwrap();
        assert!(room.placed_monsters[0].spawned);
    }

    #[test]
    fn spawn_placed_undead_from_placed_monster() {
        let mut placed = PlacedMonsterInstance::new("Custom Undead", 2);
        placed.undead = Some(true);
        let mut state = exploration_state_with_placed(vec![placed]);

        let def = crate::rules::monster::MonsterDef::test_def("Custom Undead", 3, 7, 12);
        state.module_monsters.insert("custom undead".to_string(), def);

        let _result = action_spawn_placed(&mut state, 10, None).unwrap();
        let combat = state.combat.as_ref().unwrap();
        assert!(combat.monsters[0].undead, "should be undead from placed monster flag");
    }

    #[test]
    fn spawn_placed_partial_success_with_unresolved() {
        if monster_db::find_monster("Skeleton").is_none() {
            return;
        }
        let placed = vec![
            PlacedMonsterInstance::new("Skeleton", 2),
            PlacedMonsterInstance::new("Unknown Horror", 1),
        ];
        let mut state = exploration_state_with_placed(placed);

        let result = action_spawn_placed(&mut state, 10, None).unwrap();
        assert_eq!(result.spawned.len(), 1);
        assert_eq!(result.spawned[0].name, "Skeleton");
        assert_eq!(result.unresolved, vec!["Unknown Horror"]);
        assert!(result.message.contains("WARNING"));
        assert_eq!(state.combat.as_ref().unwrap().monsters.len(), 2);
    }

    // --- multi-attack tests (oag-jukza) ---

    fn mk_multi_attack_monster(name: &str) -> Monster {
        let mut m = Monster::new(name, "5".parse().unwrap());
        m.hp = 20;
        m.max_hp = 20;
        m.ac = 5;
        m.damage = "1d8 / 1d8 / 2d6".to_string();
        m.morale = 9;
        m.xp_value = 175;
        m.attacks = vec!["claw".to_string(), "claw".to_string(), "bite".to_string()];
        m.attack_routines = vec![
            crate::model::MonsterAttackRoutine { name: "claw".to_string(), damage: "1d8".to_string() },
            crate::model::MonsterAttackRoutine { name: "claw".to_string(), damage: "1d8".to_string() },
            crate::model::MonsterAttackRoutine { name: "bite".to_string(), damage: "2d6".to_string() },
        ];
        m
    }

    fn state_with_multi_attack_monster() -> GameState {
        let mut state = GameState::new();
        let mut fighter = test_fighter();
        fighter.hp = 100;
        fighter.max_hp = 100;
        state.party.add_member(fighter);
        state.mode = GameMode::Combat;
        state.pre_combat_mode = Some(GameMode::Idle);
        state.combat = Some(CombatState::new(vec![mk_multi_attack_monster("Cave Bear")], 5));
        state
    }

    #[test]
    fn multi_attack_monster_can_attack_three_times() {
        let mut state = state_with_multi_attack_monster();
        let _ = action_roll_initiative(&mut state);

        // First attack (claw 1)
        let r1 = action_monster_attack(&mut state, 0, "Grond");
        assert!(r1.is_ok(), "first attack should succeed");
        let r1 = r1.unwrap();
        assert_eq!(r1.attack_routine, "claw");
        assert_eq!(r1.attacks_remaining, 2);

        // Second attack (claw 2)
        let r2 = action_monster_attack(&mut state, 0, "Grond");
        assert!(r2.is_ok(), "second attack should succeed");
        let r2 = r2.unwrap();
        assert_eq!(r2.attack_routine, "claw");
        assert_eq!(r2.attacks_remaining, 1);

        // Third attack (bite)
        let r3 = action_monster_attack(&mut state, 0, "Grond");
        assert!(r3.is_ok(), "third attack should succeed");
        let r3 = r3.unwrap();
        assert_eq!(r3.attack_routine, "bite");
        assert_eq!(r3.attacks_remaining, 0);

        // Fourth attack should be rejected
        let r4 = action_monster_attack(&mut state, 0, "Grond");
        assert!(r4.is_err(), "fourth attack should be rejected");
        assert!(r4.unwrap_err().to_string().contains("already used all 3 attack(s)"));
    }

    #[test]
    fn single_attack_monster_still_blocked_after_one() {
        let mut state = state_with_melee_combat();
        let _ = action_roll_initiative(&mut state);

        let r1 = action_monster_attack(&mut state, 0, "Grond");
        assert!(r1.is_ok());
        let r1 = r1.unwrap();
        assert_eq!(r1.attacks_remaining, 0);

        let r2 = action_monster_attack(&mut state, 0, "Grond");
        assert!(r2.is_err());
    }

    #[test]
    fn multi_attack_resets_on_new_round() {
        let mut state = state_with_multi_attack_monster();
        let _ = action_roll_initiative(&mut state);

        // Use all 3 attacks
        let _ = action_monster_attack(&mut state, 0, "Grond");
        let _ = action_monster_attack(&mut state, 0, "Grond");
        let _ = action_monster_attack(&mut state, 0, "Grond");

        // New round
        let _ = action_roll_initiative(&mut state);

        // Should be able to attack again
        let r = action_monster_attack(&mut state, 0, "Grond");
        assert!(r.is_ok(), "monster should attack in new round");
        assert_eq!(r.unwrap().attacks_remaining, 2);
    }
}
