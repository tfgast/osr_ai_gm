//! Attack resolution for characters and monsters (melee, missile, coup de grace).

use std::fmt;

use rand::Rng;

use crate::dice;
use crate::model::{Character, CombatState};
use crate::rules::{ability, attack, equipment};

use super::initiative::disrupt_caster;

// ── DSL attack_roll mechanic ─────────────────────────────────

/// Result of a DSL attack_roll evaluation: (hit, d20_roll, target_number).
#[cfg(feature = "dsl-backend")]
struct DslAttackResult {
    hit: bool,
    roll: u32,
    target_num: i32,
}

/// Try evaluating the attack via DSL `attack_roll` mechanic.
/// Returns None on DSL error (caller falls through to native).
#[cfg(feature = "dsl-backend")]
fn dsl_attack_roll(thac0: u32, target_ac: i32, modifiers: i32) -> Option<DslAttackResult> {
    use crate::backend::{self, MechanicGroup};
    if !backend::is_dsl(MechanicGroup::Combat) {
        return None;
    }
    let runtime = backend::dsl()?;
    let mut handler = backend::SimpleDiceHandler::new();
    use ttrpg_interp::value::Value;
    match runtime.evaluate_mechanic(
        &backend::NullState,
        &mut handler,
        "attack_roll",
        vec![
            Value::Int(thac0 as i64),
            Value::Int(target_ac as i64),
            Value::Int(modifiers as i64),
        ],
    ) {
        Ok(Value::EnumVariant { ref variant, .. }) => {
            let hit = variant.as_str() == "atk_hit";
            let roll = handler
                .rolls
                .first()
                .map(|r| r.unmodified as u32)
                .unwrap_or(0);
            let target_num = attack::target_number(thac0, target_ac);
            Some(DslAttackResult {
                hit,
                roll,
                target_num,
            })
        }
        _ => None,
    }
}

/// Result of a single attack (melee or missile).
#[derive(Debug, Clone)]
pub struct AttackResult {
    pub attacker: String,
    pub target: String,
    pub roll: u32,
    pub modifiers: i32,
    pub target_number: i32,
    pub hit: bool,
    pub damage: i32,
    pub damage_rolls: Vec<u32>,
    pub target_hp_after: i32,
    pub target_killed: bool,
}

impl fmt::Display for AttackResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mod_str = if self.modifiers > 0 {
            format!("+{}", self.modifiers)
        } else if self.modifiers < 0 {
            self.modifiers.to_string()
        } else {
            String::new()
        };
        if self.hit {
            write!(f, "{} attacks {} — d20: {}{} vs {} — HIT for {} damage",
                self.attacker, self.target, self.roll, mod_str,
                self.target_number, self.damage)?;
            if self.target_killed {
                write!(f, " [KILLED]")
            } else {
                write!(f, " [{} HP remaining]", self.target_hp_after)
            }
        } else {
            write!(f, "{} attacks {} — d20: {}{} vs {} — MISS",
                self.attacker, self.target, self.roll, mod_str,
                self.target_number)
        }
    }
}

/// Result of a coup de grace (auto-kill of a helpless creature).
#[derive(Debug, Clone)]
pub struct CoupDeGraceResult {
    pub attacker: String,
    pub target: String,
    pub target_was_helpless: bool,
}

impl fmt::Display for CoupDeGraceResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} dispatches the helpless {} — KILLED (auto-kill)",
            self.attacker, self.target)
    }
}

// =============================================================================
// Unified Character Attack Resolution
// =============================================================================

/// Resolve a character's attack against a monster, handling weapon type,
/// distance, ability modifiers, and rest penalty.
///
/// Both the CLI and GM API must call this function to ensure consistent
/// combat rules regardless of entry point.
pub fn resolve_character_attack(
    combat: &mut CombatState,
    character: &Character,
    monster_idx: usize,
    weapon: &equipment::WeaponDef,
    rest_penalty: i32,
) -> Result<AttackResult, String> {
    if monster_idx >= combat.monsters.len() {
        return Err(format!("monster index {} out of range (0-{})",
            monster_idx, combat.monsters.len() - 1));
    }
    if !combat.monsters[monster_idx].is_alive() {
        return Err(format!("{} is already dead.", combat.monsters[monster_idx].name));
    }
    if !character.is_alive() {
        return Err(format!("{} is dead and cannot attack.", character.name));
    }

    let qualities = weapon.weapon_qualities();
    if qualities.missile && !qualities.melee {
        // Pure missile weapon
        let dex_mod = ability::dex_missile_mod(character.abilities.dexterity) + rest_penalty;
        character_missile_attack(combat, character, monster_idx, weapon.damage_dice(), dex_mod, weapon.range_tuple())
    } else if qualities.missile && qualities.melee {
        // Versatile weapon (e.g., dagger, spear) — melee if close, missile if far
        if combat.distance <= 10 {
            let str_mod = ability::str_melee_mod(character.abilities.strength) + rest_penalty;
            Ok(character_melee_attack(combat, character, monster_idx, weapon.damage_dice(), str_mod))
        } else {
            let dex_mod = ability::dex_missile_mod(character.abilities.dexterity) + rest_penalty;
            character_missile_attack(combat, character, monster_idx, weapon.damage_dice(), dex_mod, weapon.range_tuple())
        }
    } else {
        // Pure melee weapon
        if combat.distance > 10 {
            // Find missile weapons in character's inventory
            let missile_weapons: Vec<&str> = character.inventory.iter()
                .filter_map(|item| equipment::find_weapon(&item.name))
                .filter(|w| w.weapon_qualities().missile)
                .map(|w| w.name.as_str())
                .collect();

            let missile_hint = if missile_weapons.is_empty() {
                String::new()
            } else {
                format!(" Available missile weapons: {}.", missile_weapons.join(", "))
            };

            return Err(format!(
                "{} is a melee weapon but monsters are {}' away. \
                Use \"close {}\" to move into melee range, or attack with a missile weapon.{}",
                weapon.name, combat.distance, character.name, missile_hint));
        }
        let str_mod = ability::str_melee_mod(character.abilities.strength) + rest_penalty;
        Ok(character_melee_attack(combat, character, monster_idx, weapon.damage_dice(), str_mod))
    }
}

// =============================================================================
// Coup de Grace (Auto-kill helpless creatures)
// =============================================================================

/// Automatically kill a helpless monster (sleeping, paralyzed, held, etc.).
///
/// Per OSE rules, helpless creatures can be dispatched without an attack roll.
/// The monster is killed instantly regardless of HP.
pub fn coup_de_grace(
    combat: &mut CombatState,
    character: &Character,
    monster_idx: usize,
) -> Result<CoupDeGraceResult, String> {
    if combat.monsters.is_empty() {
        return Err("no monsters in combat".to_string());
    }
    if monster_idx >= combat.monsters.len() {
        return Err(format!("monster index {} out of range (0-{})",
            monster_idx, combat.monsters.len() - 1));
    }
    if !combat.monsters[monster_idx].is_alive() {
        return Err(format!("{} is already dead.", combat.monsters[monster_idx].name));
    }
    if !character.is_alive() {
        return Err(format!("{} is dead and cannot attack.", character.name));
    }
    if !combat.monsters[monster_idx].helpless {
        return Err(format!("{} is not helpless. Use a normal attack.",
            combat.monsters[monster_idx].name));
    }

    let target_name = combat.monsters[monster_idx].name.clone();

    // Instant kill — set HP to 0 and clear helpless state
    combat.monsters[monster_idx].hp = 0;
    combat.monsters[monster_idx].helpless = false;

    let result = CoupDeGraceResult {
        attacker: character.name.clone(),
        target: target_name,
        target_was_helpless: true,
    };
    combat.log_event(result.to_string());
    Ok(result)
}

/// Mark a monster as helpless (sleeping, paralyzed, held, etc.).
/// Helpless monsters can be auto-killed with coup_de_grace.
pub fn set_monster_helpless(combat: &mut CombatState, monster_idx: usize, helpless: bool) -> Result<String, String> {
    if combat.monsters.is_empty() {
        return Err("no monsters in combat".to_string());
    }
    if monster_idx >= combat.monsters.len() {
        return Err(format!("monster index {} out of range (0-{})",
            monster_idx, combat.monsters.len() - 1));
    }
    if !combat.monsters[monster_idx].is_alive() {
        return Err(format!("{} is already dead.", combat.monsters[monster_idx].name));
    }

    combat.monsters[monster_idx].helpless = helpless;
    let name = &combat.monsters[monster_idx].name;
    let msg = if helpless {
        format!("{} is now helpless (can be auto-killed).", name)
    } else {
        format!("{} is no longer helpless.", name)
    };
    combat.log_event(msg.clone());
    Ok(msg)
}

// =============================================================================
// Melee Attack
// =============================================================================

/// Resolve a character's melee attack against a monster.
///
/// - `weapon_damage`: dice notation for the weapon (e.g., "1d8")
/// - `str_mod`: character's STR melee modifier (applied to both attack and damage)
pub fn character_melee_attack(
    combat: &mut CombatState,
    character: &Character,
    monster_idx: usize,
    weapon_damage: &str,
    str_mod: i32,
) -> AttackResult {
    character_melee_attack_with(combat, character, monster_idx, weapon_damage, str_mod,
        &mut rand::thread_rng())
}

pub fn character_melee_attack_with<R: Rng>(
    combat: &mut CombatState,
    character: &Character,
    monster_idx: usize,
    weapon_damage: &str,
    str_mod: i32,
    rng: &mut R,
) -> AttackResult {
    assert!(
        monster_idx < combat.monsters.len() && combat.monsters[monster_idx].is_alive(),
        "cannot attack dead or nonexistent monster at index {}",
        monster_idx
    );
    let monster = &combat.monsters[monster_idx];
    let target_ac = monster.ac;
    let target_name = monster.name.clone();
    let thac0 = character.thac0;
    let modifiers = str_mod;

    // DSL gate: use attack_roll mechanic if available
    #[cfg(feature = "dsl-backend")]
    let (roll, target_num, hit) = {
        if let Some(dsl) = dsl_attack_roll(thac0, target_ac, modifiers) {
            (dsl.roll, dsl.target_num, dsl.hit)
        } else {
            let r = rng.gen_range(1..=20u32);
            let tn = attack::target_number(thac0, target_ac);
            (r, tn, attack::hits(thac0, target_ac, modifiers, r))
        }
    };
    #[cfg(all(not(feature = "dsl-backend"), feature = "legacy-native"))]
    let (roll, target_num, hit) = {
        let r = rng.gen_range(1..=20u32);
        let tn = attack::target_number(thac0, target_ac);
        (r, tn, attack::hits(thac0, target_ac, modifiers, r))
    };

    let (damage, damage_rolls) = if hit {
        let dmg_expr = dice::parse(weapon_damage)
            .unwrap_or(dice::DiceExpr::Standard { count: 1, sides: 6, modifier: 0 });
        let result = dice::roll_with(&dmg_expr, rng);
        let total = (result.total + str_mod).max(1);
        (total, result.rolls)
    } else {
        (0, vec![])
    };

    if hit {
        combat.monsters[monster_idx].hp -= damage;
    }
    let target_killed = !combat.monsters[monster_idx].is_alive();
    let target_hp_after = combat.monsters[monster_idx].hp;

    let result = AttackResult {
        attacker: character.name.clone(),
        target: target_name,
        roll,
        modifiers,
        target_number: target_num,
        hit,
        damage,
        damage_rolls,
        target_hp_after,
        target_killed,
    };
    combat.log_event(result.to_string());
    result
}

// =============================================================================
// Missile Attack
// =============================================================================

/// Resolve a character's missile attack against a monster.
///
/// Returns `Err` if the target is out of range.
pub fn character_missile_attack(
    combat: &mut CombatState,
    character: &Character,
    monster_idx: usize,
    weapon_damage: &str,
    dex_mod: i32,
    range: (u32, u32, u32),
) -> Result<AttackResult, String> {
    character_missile_attack_with(combat, character, monster_idx, weapon_damage, dex_mod, range,
        &mut rand::thread_rng())
}

pub fn character_missile_attack_with<R: Rng>(
    combat: &mut CombatState,
    character: &Character,
    monster_idx: usize,
    weapon_damage: &str,
    dex_mod: i32,
    range: (u32, u32, u32),
    rng: &mut R,
) -> Result<AttackResult, String> {
    if monster_idx >= combat.monsters.len() || !combat.monsters[monster_idx].is_alive() {
        return Err(format!("cannot attack dead or nonexistent monster at index {}", monster_idx));
    }
    let range_mod = attack::missile_range_modifier(combat.distance, range.0, range.1, range.2)
        .ok_or_else(|| {
            if combat.distance == 0 {
                "cannot use missile weapons in melee (distance: 0')".to_string()
            } else {
                format!("target out of range (distance: {}', max: {}')", combat.distance, range.2)
            }
        })?;

    let monster = &combat.monsters[monster_idx];
    let target_ac = monster.ac;
    let target_name = monster.name.clone();
    let thac0 = character.thac0;
    let modifiers = dex_mod + range_mod;

    // DSL gate: use attack_roll mechanic if available
    #[cfg(feature = "dsl-backend")]
    let (roll, target_num, hit) = {
        if let Some(dsl) = dsl_attack_roll(thac0, target_ac, modifiers) {
            (dsl.roll, dsl.target_num, dsl.hit)
        } else {
            let r = rng.gen_range(1..=20u32);
            let tn = attack::target_number(thac0, target_ac);
            (r, tn, attack::hits(thac0, target_ac, modifiers, r))
        }
    };
    #[cfg(all(not(feature = "dsl-backend"), feature = "legacy-native"))]
    let (roll, target_num, hit) = {
        let r = rng.gen_range(1..=20u32);
        let tn = attack::target_number(thac0, target_ac);
        (r, tn, attack::hits(thac0, target_ac, modifiers, r))
    };

    let (damage, damage_rolls) = if hit {
        let dmg_expr = dice::parse(weapon_damage)
            .unwrap_or(dice::DiceExpr::Standard { count: 1, sides: 6, modifier: 0 });
        let result = dice::roll_with(&dmg_expr, rng);
        // No STR mod for missile damage
        (result.total.max(1), result.rolls)
    } else {
        (0, vec![])
    };

    if hit {
        combat.monsters[monster_idx].hp -= damage;
    }
    let target_killed = !combat.monsters[monster_idx].is_alive();
    let target_hp_after = combat.monsters[monster_idx].hp;

    let result = AttackResult {
        attacker: character.name.clone(),
        target: target_name,
        roll,
        modifiers,
        target_number: target_num,
        hit,
        damage,
        damage_rolls,
        target_hp_after,
        target_killed,
    };
    combat.log_event(result.to_string());
    Ok(result)
}

// =============================================================================
// Monster Attack
// =============================================================================

/// Resolve a monster's attack against a character using a specific attack routine.
/// `routine_idx` selects which attack routine to use for damage dice.
pub fn monster_attack(
    combat: &mut CombatState,
    monster_idx: usize,
    character: &mut Character,
    routine_idx: usize,
) -> AttackResult {
    monster_attack_modified_with(combat, monster_idx, character, 0, Some(routine_idx), &mut rand::thread_rng())
}

pub fn monster_attack_with<R: Rng>(
    combat: &mut CombatState,
    monster_idx: usize,
    character: &mut Character,
    rng: &mut R,
) -> AttackResult {
    monster_attack_modified_with(combat, monster_idx, character, 0, None, rng)
}

/// Resolve a monster's attack against a character with an explicit hit modifier.
/// Used for normal attacks (modifier=0) and free attacks on retreat (modifier=+2).
/// When `routine_idx` is Some, uses that attack routine's damage dice; otherwise
/// falls back to the monster's default damage string.
pub(super) fn monster_attack_modified_with<R: Rng>(
    combat: &mut CombatState,
    monster_idx: usize,
    character: &mut Character,
    modifier: i32,
    routine_idx: Option<usize>,
    rng: &mut R,
) -> AttackResult {
    assert!(
        monster_idx < combat.monsters.len() && combat.monsters[monster_idx].is_alive(),
        "cannot attack with dead or nonexistent monster at index {}",
        monster_idx
    );
    let monster = &combat.monsters[monster_idx];
    let attacker_name = monster.name.clone();
    let hd = monster.hit_dice.combat_hd();
    let thac0 = attack::monster_thac0(hd);
    let target_ac = character.ac;
    let routine_damage = routine_idx.and_then(|i| monster.attack_routines.get(i).map(|r| r.damage.clone()));
    let damage_dice: &str = match &routine_damage {
        Some(d) => d,
        None => if monster.damage.is_empty() { "1d6" } else { &monster.damage },
    };
    let modifiers = modifier;

    // DSL gate: use attack_roll mechanic if available
    #[cfg(feature = "dsl-backend")]
    let (roll, target_num, hit) = {
        if let Some(dsl) = dsl_attack_roll(thac0, target_ac, modifiers) {
            (dsl.roll, dsl.target_num, dsl.hit)
        } else {
            let r = rng.gen_range(1..=20u32);
            let tn = attack::target_number(thac0, target_ac);
            (r, tn, attack::hits(thac0, target_ac, modifiers, r))
        }
    };
    #[cfg(all(not(feature = "dsl-backend"), feature = "legacy-native"))]
    let (roll, target_num, hit) = {
        let r = rng.gen_range(1..=20u32);
        let tn = attack::target_number(thac0, target_ac);
        (r, tn, attack::hits(thac0, target_ac, modifiers, r))
    };

    let (damage, damage_rolls) = if hit {
        let dmg_expr = dice::parse(damage_dice)
            .unwrap_or(dice::DiceExpr::Standard { count: 1, sides: 6, modifier: 0 });
        let result = dice::roll_with(&dmg_expr, rng);
        (result.total.max(1), result.rolls)
    } else {
        (0, vec![])
    };

    if hit {
        character.hp -= damage;
        disrupt_caster(combat, &character.name);
    }
    let target_killed = !character.is_alive();
    let target_hp_after = character.hp;

    let result = AttackResult {
        attacker: attacker_name,
        target: character.name.clone(),
        roll,
        modifiers,
        target_number: target_num,
        hit,
        damage,
        damage_rolls,
        target_hp_after,
        target_killed,
    };
    combat.log_event(result.to_string());
    result
}
