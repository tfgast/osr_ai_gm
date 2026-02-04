/// Combat engine for OSE.
///
/// Implements the full combat round sequence per OSE Reference Booklet p116-124:
/// 1. Declarations (spell casters, retreats)
/// 2. Initiative (group d6, each side)
/// 3. Winning side acts: morale -> movement -> missile -> magic -> melee
/// 4. Losing side acts (same sub-phase order)
/// 5. End-of-round bookkeeping

use rand::Rng;
use std::fmt;

use crate::dice;
use crate::model::{Character, CombatState};
use crate::rules::attack;
use crate::rules::turn::{self, TurnResult};

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
            format!("{}", self.modifiers)
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

/// Result of a morale check.
#[derive(Debug, Clone)]
pub struct MoraleResult {
    pub roll: i32,
    pub morale_score: u32,
    pub passed: bool,
}

impl fmt::Display for MoraleResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.passed {
            write!(f, "Morale check: 2d6 = {} vs {} — HOLDS",
                self.roll, self.morale_score)
        } else {
            write!(f, "Morale check: 2d6 = {} vs {} — FLEES!",
                self.roll, self.morale_score)
        }
    }
}

/// Result of a turn undead attempt.
#[derive(Debug, Clone)]
pub struct TurnUndeadResult {
    pub cleric_name: String,
    pub cleric_level: u32,
    pub undead_type: String,
    pub undead_rank: u32,
    pub table_result: TurnResult,
    pub roll: Option<i32>,
    pub success: bool,
    pub hd_affected: u32,
    pub destroyed: bool,
}

impl fmt::Display for TurnUndeadResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} attempts to turn {} (rank {}): ",
            self.cleric_name, self.undead_type, self.undead_rank)?;
        match self.table_result {
            TurnResult::Impossible => {
                write!(f, "IMPOSSIBLE — cannot turn this undead type")
            }
            TurnResult::Roll(target) => {
                if let Some(roll) = self.roll {
                    if self.success {
                        write!(f, "2d6 = {} vs {} — TURNED! {} HD affected",
                            roll, target, self.hd_affected)
                    } else {
                        write!(f, "2d6 = {} vs {} — FAILED", roll, target)
                    }
                } else {
                    write!(f, "needs {} on 2d6", target)
                }
            }
            TurnResult::Turned => {
                write!(f, "AUTOMATIC TURN! {} HD affected", self.hd_affected)
            }
            TurnResult::Destroyed => {
                write!(f, "AUTOMATIC DESTRUCTION! {} HD affected", self.hd_affected)
            }
        }
    }
}

// =============================================================================
// Initiative
// =============================================================================

/// Roll group initiative (1d6 per side). Advances the round counter
/// and clears spell declarations/disruptions from the previous round.
pub fn roll_initiative(combat: &mut CombatState) -> (i32, i32) {
    roll_initiative_with(combat, &mut rand::thread_rng())
}

pub fn roll_initiative_with<R: Rng>(combat: &mut CombatState, rng: &mut R) -> (i32, i32) {
    let party = rng.gen_range(1..=6i32);
    let monsters = rng.gen_range(1..=6i32);
    combat.party_initiative = party;
    combat.monster_initiative = monsters;
    combat.round += 1;
    combat.spell_declarations.clear();
    combat.disrupted.clear();
    combat.phase = crate::model::CombatPhase::Morale;

    let winner = if party > monsters {
        "Party acts first"
    } else if monsters > party {
        "Monsters act first"
    } else {
        "Simultaneous actions"
    };
    let msg = format!("Round {} — Initiative: Party {} vs Monsters {} — {}",
        combat.round, party, monsters, winner);
    combat.log.push(msg);
    (party, monsters)
}

// =============================================================================
// Spell Declaration & Disruption
// =============================================================================

/// Declare a spell cast for a character (must be done during declaration phase).
/// If the caster takes damage before the magic phase, the spell is disrupted.
pub fn declare_spell(combat: &mut CombatState, character_name: &str, spell_name: &str) {
    combat.spell_declarations.push(character_name.to_string());
    combat.log.push(format!("{} declares: casting {}", character_name, spell_name));
}

/// Check if a character's spell was disrupted this round.
pub fn is_disrupted(combat: &CombatState, character_name: &str) -> bool {
    combat.disrupted.iter().any(|n| n.eq_ignore_ascii_case(character_name))
}

/// Mark a spell-casting character as disrupted (called internally when they take damage).
fn disrupt_caster(combat: &mut CombatState, character_name: &str) {
    let is_casting = combat.spell_declarations.iter()
        .any(|n| n.eq_ignore_ascii_case(character_name));
    let already_disrupted = combat.disrupted.iter()
        .any(|n| n.eq_ignore_ascii_case(character_name));
    if is_casting && !already_disrupted {
        combat.disrupted.push(character_name.to_string());
        combat.log.push(format!("{}'s spell is DISRUPTED!", character_name));
    }
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
    let monster = &combat.monsters[monster_idx];
    let target_ac = monster.ac;
    let target_name = monster.name.clone();
    let thac0 = character.thac0;

    let roll = rng.gen_range(1..=20u32);
    let modifiers = str_mod;
    let target_num = attack::target_number(thac0, target_ac);
    let hit = attack::hits(thac0, target_ac, modifiers, roll);

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
    combat.log.push(format!("{}", result));
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
    let range_mod = attack::missile_range_modifier(combat.distance, range.0, range.1, range.2)
        .ok_or_else(|| format!("target out of range (distance: {}')", combat.distance))?;

    let monster = &combat.monsters[monster_idx];
    let target_ac = monster.ac;
    let target_name = monster.name.clone();
    let thac0 = character.thac0;

    let roll = rng.gen_range(1..=20u32);
    let modifiers = dex_mod + range_mod;
    let target_num = attack::target_number(thac0, target_ac);
    let hit = attack::hits(thac0, target_ac, modifiers, roll);

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
    combat.log.push(format!("{}", result));
    Ok(result)
}

// =============================================================================
// Monster Attack
// =============================================================================

/// Resolve a monster's attack against a character.
/// Applies damage to the character directly and handles spell disruption.
pub fn monster_attack(
    combat: &mut CombatState,
    monster_idx: usize,
    character: &mut Character,
) -> AttackResult {
    monster_attack_with(combat, monster_idx, character, &mut rand::thread_rng())
}

pub fn monster_attack_with<R: Rng>(
    combat: &mut CombatState,
    monster_idx: usize,
    character: &mut Character,
    rng: &mut R,
) -> AttackResult {
    let monster = &combat.monsters[monster_idx];
    let attacker_name = monster.name.clone();
    let hd = attack::parse_monster_hd(&monster.hit_dice);
    let thac0 = attack::monster_thac0(hd);
    let target_ac = character.ac;
    let damage_dice = if monster.damage.is_empty() { "1d6" } else { &monster.damage };

    let roll = rng.gen_range(1..=20u32);
    let modifiers = 0i32;
    let target_num = attack::target_number(thac0, target_ac);
    let hit = attack::hits(thac0, target_ac, modifiers, roll);

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
    combat.log.push(format!("{}", result));
    result
}

// =============================================================================
// Morale
// =============================================================================

/// Check morale for a specific monster type.
///
/// Per OSE, morale is checked per monster type — each type uses its own
/// morale score. Checked when:
/// - First monster in the group is killed
/// - Half or more of the group has been defeated
///
/// Roll 2d6: if result > morale score, monsters of that type flee.
/// Morale 2 = always flees, Morale 12 = never flees.
pub fn check_morale(combat: &mut CombatState, morale_score: u32) -> MoraleResult {
    check_morale_with(combat, morale_score, &mut rand::thread_rng())
}

pub fn check_morale_with<R: Rng>(combat: &mut CombatState, morale_score: u32, rng: &mut R) -> MoraleResult {
    let d1 = rng.gen_range(1..=6i32);
    let d2 = rng.gen_range(1..=6i32);
    let roll = d1 + d2;
    let passed = roll <= morale_score as i32;

    let result = MoraleResult { roll, morale_score, passed };
    combat.log.push(format!("{}", result));
    result
}

// =============================================================================
// Turn Undead
// =============================================================================

/// Resolve a cleric's turn undead attempt against a monster.
pub fn resolve_turn_undead(
    combat: &mut CombatState,
    cleric: &Character,
    cleric_level: u32,
    target_monster_idx: usize,
) -> TurnUndeadResult {
    resolve_turn_undead_with(combat, cleric, cleric_level, target_monster_idx,
        &mut rand::thread_rng())
}

pub fn resolve_turn_undead_with<R: Rng>(
    combat: &mut CombatState,
    cleric: &Character,
    cleric_level: u32,
    target_monster_idx: usize,
    rng: &mut R,
) -> TurnUndeadResult {
    let monster = &combat.monsters[target_monster_idx];
    let hd = attack::parse_monster_hd(&monster.hit_dice);
    let rank = turn::undead_rank_from_hd(hd);
    let undead_type = monster.name.clone();

    let table_result = turn::turn_undead_result(cleric_level, rank);

    let (roll, success, hd_affected, destroyed) = match table_result {
        TurnResult::Impossible => (None, false, 0, false),
        TurnResult::Roll(target) => {
            let d1 = rng.gen_range(1..=6i32);
            let d2 = rng.gen_range(1..=6i32);
            let roll = d1 + d2;
            let success = roll >= target as i32;
            let hd_affected = if success {
                let h1 = rng.gen_range(1..=6u32);
                let h2 = rng.gen_range(1..=6u32);
                h1 + h2
            } else {
                0
            };
            (Some(roll), success, hd_affected, false)
        }
        TurnResult::Turned => {
            let h1 = rng.gen_range(1..=6u32);
            let h2 = rng.gen_range(1..=6u32);
            (None, true, h1 + h2, false)
        }
        TurnResult::Destroyed => {
            let h1 = rng.gen_range(1..=6u32);
            let h2 = rng.gen_range(1..=6u32);
            (None, true, h1 + h2, true)
        }
    };

    let result = TurnUndeadResult {
        cleric_name: cleric.name.clone(),
        cleric_level,
        undead_type,
        undead_rank: rank,
        table_result,
        roll,
        success,
        hd_affected,
        destroyed,
    };
    combat.log.push(format!("{}", result));
    result
}

// =============================================================================
// Movement
// =============================================================================

/// Resolve fighting withdrawal for a character.
/// Half encounter movement speed backward; no free attacks from enemies.
/// Character can still defend but cannot attack this round.
pub fn fighting_withdrawal(combat: &mut CombatState, character: &Character) -> String {
    let encounter_move = character.movement_rate / 3;
    let half_move = encounter_move / 2;
    let msg = format!("{} performs a fighting withdrawal ({}' backward)",
        character.name, half_move);
    combat.log.push(msg.clone());
    msg
}

/// Resolve retreat for a character.
/// Full encounter movement speed; enemies in melee get a free attack at +2.
pub fn retreat(combat: &mut CombatState, character: &Character) -> String {
    let encounter_move = character.movement_rate / 3;
    let msg = format!(
        "{} retreats at full speed ({}')! Enemies in melee get free attack at +2.",
        character.name, encounter_move);
    combat.log.push(msg.clone());
    msg
}

// =============================================================================
// Status Display
// =============================================================================

/// Format combat status for display.
pub fn combat_status(combat: &CombatState, party: &[Character]) -> String {
    let mut out = String::new();
    let phase_name = match combat.phase {
        crate::model::CombatPhase::Declaration => "Declaration",
        crate::model::CombatPhase::Initiative => "Initiative",
        crate::model::CombatPhase::Morale => "Morale",
        crate::model::CombatPhase::Movement => "Movement",
        crate::model::CombatPhase::Missile => "Missile",
        crate::model::CombatPhase::Magic => "Magic",
        crate::model::CombatPhase::Melee => "Melee",
        crate::model::CombatPhase::EndOfRound => "End of Round",
    };
    out.push_str(&format!("=== Combat — Round {} ({}) ===\n", combat.round, phase_name));
    out.push_str(&format!("Distance: {}'\n", combat.distance));

    if combat.round > 0 {
        let winner = if combat.party_initiative > combat.monster_initiative {
            "Party"
        } else if combat.monster_initiative > combat.party_initiative {
            "Monsters"
        } else {
            "Simultaneous"
        };
        out.push_str(&format!("Initiative: Party {} vs Monsters {} — {} acts first\n",
            combat.party_initiative, combat.monster_initiative, winner));
    }

    out.push_str("\nParty:\n");
    for c in party {
        let status = if c.is_alive() {
            format!("HP {}/{}, AC {}", c.hp, c.max_hp, c.ac)
        } else {
            "DEAD".to_string()
        };
        out.push_str(&format!("  {} ({}) — {}\n", c.name, c.class, status));
    }

    out.push_str("\nMonsters:\n");
    for (i, m) in combat.monsters.iter().enumerate() {
        let status = if m.is_alive() {
            format!("HP {}/{}, AC {}", m.hp, m.max_hp, m.ac)
        } else {
            "DEAD".to_string()
        };
        out.push_str(&format!("  [{}] {} (HD {}) — {}\n", i, m.name, m.hit_dice, status));
    }

    if !combat.spell_declarations.is_empty() {
        out.push_str("\nSpell declarations: ");
        out.push_str(&combat.spell_declarations.join(", "));
        out.push('\n');
    }
    if !combat.disrupted.is_empty() {
        out.push_str("Disrupted: ");
        out.push_str(&combat.disrupted.join(", "));
        out.push('\n');
    }

    out
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AbilityScores, Monster};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn test_fighter() -> Character {
        Character {
            name: "Grond".to_string(),
            class: "Fighter".to_string(),
            level: 1,
            abilities: AbilityScores {
                strength: 16, intelligence: 10, wisdom: 10,
                dexterity: 12, constitution: 14, charisma: 10,
            },
            hp: 8, max_hp: 8, ac: 3, xp: 0,
            inventory: vec![], spells: vec![],
            alignment: "Neutral".to_string(), gold_gp: 100,
            saving_throws: None, thac0: 19, movement_rate: 120,
        }
    }

    fn test_cleric() -> Character {
        Character {
            name: "Brother Aldric".to_string(),
            class: "Cleric".to_string(),
            level: 3,
            abilities: AbilityScores {
                strength: 12, intelligence: 10, wisdom: 16,
                dexterity: 10, constitution: 13, charisma: 14,
            },
            hp: 14, max_hp: 14, ac: 4, xp: 0,
            inventory: vec![], spells: vec![],
            alignment: "Lawful".to_string(), gold_gp: 50,
            saving_throws: None, thac0: 19, movement_rate: 120,
        }
    }

    fn test_magic_user() -> Character {
        Character {
            name: "Elara".to_string(),
            class: "Magic-User".to_string(),
            level: 1,
            abilities: AbilityScores {
                strength: 8, intelligence: 16, wisdom: 10,
                dexterity: 14, constitution: 10, charisma: 11,
            },
            hp: 3, max_hp: 3, ac: 7, xp: 0,
            inventory: vec![], spells: vec![],
            alignment: "Neutral".to_string(), gold_gp: 40,
            saving_throws: None, thac0: 19, movement_rate: 120,
        }
    }

    fn test_goblin() -> Monster {
        Monster {
            name: "Goblin".to_string(),
            hit_dice: "1-1".to_string(),
            hp: 3, max_hp: 3, ac: 6,
            attacks: vec!["weapon".to_string()],
            damage: "1d6".to_string(),
            morale: 7, xp_value: 5,
        }
    }

    fn test_skeleton() -> Monster {
        Monster {
            name: "Skeleton".to_string(),
            hit_dice: "1".to_string(),
            hp: 4, max_hp: 4, ac: 7,
            attacks: vec!["weapon".to_string()],
            damage: "1d6".to_string(),
            morale: 12, xp_value: 10,
        }
    }

    fn test_ogre() -> Monster {
        Monster {
            name: "Ogre".to_string(),
            hit_dice: "4+1".to_string(),
            hp: 19, max_hp: 19, ac: 5,
            attacks: vec!["club".to_string()],
            damage: "1d10".to_string(),
            morale: 10, xp_value: 125,
        }
    }

    // --- Combat State ---

    #[test]
    fn combat_state_creation() {
        let combat = CombatState::new(vec![test_goblin(), test_goblin(), test_goblin()], 60);
        assert_eq!(combat.round, 0);
        assert_eq!(combat.monsters.len(), 3);
        assert_eq!(combat.distance, 60);
        assert_eq!(combat.living_monster_count(), 3);
    }

    #[test]
    fn living_monster_tracking() {
        let mut combat = CombatState::new(vec![test_goblin(), test_goblin()], 10);
        assert_eq!(combat.living_monster_count(), 2);
        combat.monsters[0].hp = 0;
        assert_eq!(combat.living_monster_count(), 1);
        let living = combat.living_monsters();
        assert_eq!(living.len(), 1);
        assert_eq!(living[0].0, 1); // index 1 is still alive
    }

    // --- Initiative ---

    #[test]
    fn initiative_roll_advances_round() {
        let mut combat = CombatState::new(vec![test_goblin()], 60);
        let mut rng = test_rng();
        let (p, m) = roll_initiative_with(&mut combat, &mut rng);
        assert!(p >= 1 && p <= 6);
        assert!(m >= 1 && m <= 6);
        assert_eq!(combat.round, 1);
        assert_eq!(combat.party_initiative, p);
        assert_eq!(combat.monster_initiative, m);
    }

    #[test]
    fn initiative_clears_spell_tracking() {
        let mut combat = CombatState::new(vec![test_goblin()], 60);
        combat.spell_declarations.push("Elara".to_string());
        combat.disrupted.push("Elara".to_string());
        roll_initiative_with(&mut combat, &mut test_rng());
        assert!(combat.spell_declarations.is_empty());
        assert!(combat.disrupted.is_empty());
    }

    // --- Melee Attack ---

    #[test]
    fn melee_attack_resolves() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let fighter = test_fighter();
        let mut rng = test_rng();
        let result = character_melee_attack_with(
            &mut combat, &fighter, 0, "1d8", 2, &mut rng,
        );
        assert_eq!(result.attacker, "Grond");
        assert_eq!(result.target, "Goblin");
        assert!(result.roll >= 1 && result.roll <= 20);
        assert_eq!(result.modifiers, 2);
        // target_number = 19 - 6 = 13
        assert_eq!(result.target_number, 13);
    }

    #[test]
    fn melee_attack_damage_applied() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let fighter = test_fighter();
        let mut rng = test_rng();
        let initial_hp = combat.monsters[0].hp;
        let mut total_damage = 0;
        // Run multiple attacks to get at least one hit
        for _ in 0..20 {
            combat.monsters[0].hp = initial_hp;
            let result = character_melee_attack_with(
                &mut combat, &fighter, 0, "1d8", 2, &mut rng,
            );
            if result.hit {
                total_damage += result.damage;
                assert!(result.damage >= 1);
                assert_eq!(combat.monsters[0].hp, initial_hp - result.damage);
                break;
            }
        }
        assert!(total_damage > 0, "should hit at least once in 20 tries");
    }

    #[test]
    fn melee_attack_logs() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let fighter = test_fighter();
        character_melee_attack_with(&mut combat, &fighter, 0, "1d8", 2, &mut test_rng());
        assert!(!combat.log.is_empty());
        assert!(combat.log.last().unwrap().contains("Grond"));
    }

    // --- Missile Attack ---

    #[test]
    fn missile_attack_in_range() {
        let mut combat = CombatState::new(vec![test_goblin()], 60);
        let fighter = test_fighter();
        let mut rng = test_rng();
        // Short bow range: 50/100/150
        let result = character_missile_attack_with(
            &mut combat, &fighter, 0, "1d6", 1, (50, 100, 150), &mut rng,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn missile_attack_out_of_range() {
        let mut combat = CombatState::new(vec![test_goblin()], 200);
        let fighter = test_fighter();
        let mut rng = test_rng();
        let result = character_missile_attack_with(
            &mut combat, &fighter, 0, "1d6", 1, (50, 100, 150), &mut rng,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of range"));
    }

    // --- Monster Attack ---

    #[test]
    fn monster_attack_resolves() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let mut fighter = test_fighter();
        let mut rng = test_rng();
        let result = monster_attack_with(&mut combat, 0, &mut fighter, &mut rng);
        assert_eq!(result.attacker, "Goblin");
        assert_eq!(result.target, "Grond");
        // Goblin HD "1-1" -> hd 0 -> THAC0 20
        // vs Fighter AC 3 -> target number 17
        assert_eq!(result.target_number, 17);
    }

    #[test]
    fn monster_attack_ogre_damage() {
        let mut combat = CombatState::new(vec![test_ogre()], 10);
        let mut fighter = test_fighter();
        let mut rng = test_rng();
        // Run several attacks
        for _ in 0..20 {
            fighter.hp = fighter.max_hp;
            let result = monster_attack_with(&mut combat, 0, &mut fighter, &mut rng);
            if result.hit {
                assert!(result.damage >= 1);
                assert_eq!(fighter.hp, fighter.max_hp - result.damage);
                break;
            }
        }
    }

    // --- Spell Disruption ---

    #[test]
    fn spell_declaration() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        declare_spell(&mut combat, "Elara", "Magic Missile");
        assert!(combat.spell_declarations.contains(&"Elara".to_string()));
        assert!(!is_disrupted(&combat, "Elara"));
    }

    #[test]
    fn spell_disruption_on_damage() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let mut mage = test_magic_user();
        declare_spell(&mut combat, "Elara", "Sleep");

        // Goblin attacks mage — if hit, spell is disrupted
        let mut rng = test_rng();
        // Force a hit by running multiple times
        for _ in 0..50 {
            mage.hp = mage.max_hp;
            combat.disrupted.clear();
            let result = monster_attack_with(&mut combat, 0, &mut mage, &mut rng);
            if result.hit {
                assert!(is_disrupted(&combat, "Elara"));
                return;
            }
        }
        // If we got here without a hit, the test is inconclusive (unlikely)
    }

    #[test]
    fn non_caster_not_disrupted() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let mut fighter = test_fighter();
        // Fighter didn't declare a spell
        let mut rng = test_rng();
        for _ in 0..20 {
            fighter.hp = fighter.max_hp;
            monster_attack_with(&mut combat, 0, &mut fighter, &mut rng);
        }
        assert!(!is_disrupted(&combat, "Grond"));
    }

    // --- Morale ---

    #[test]
    fn morale_check_per_type() {
        let mut combat = CombatState::new(vec![test_goblin(), test_skeleton()], 10);
        // Check goblin morale (7) separately from skeleton morale (12)
        let goblin_result = check_morale_with(&mut combat, 7, &mut test_rng());
        assert_eq!(goblin_result.morale_score, 7);
        let skeleton_result = check_morale_with(&mut combat, 12, &mut test_rng());
        assert_eq!(skeleton_result.morale_score, 12);
    }

    #[test]
    fn morale_check_bounds() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let mut rng = test_rng();
        for _ in 0..100 {
            let result = check_morale_with(&mut combat, 7, &mut rng);
            assert!(result.roll >= 2 && result.roll <= 12);
        }
    }

    // --- Turn Undead ---

    #[test]
    fn turn_undead_skeleton_level_3() {
        let mut combat = CombatState::new(vec![test_skeleton()], 10);
        let cleric = test_cleric();
        let result = resolve_turn_undead_with(
            &mut combat, &cleric, 3, 0, &mut test_rng(),
        );
        // Level 3 vs rank 1 skeleton: diff = 2, auto turn
        assert_eq!(result.table_result, TurnResult::Turned);
        assert!(result.success);
        assert!(result.hd_affected >= 2 && result.hd_affected <= 12);
    }

    #[test]
    fn turn_undead_impossible() {
        let mut vampire = test_skeleton();
        vampire.name = "Vampire".to_string();
        vampire.hit_dice = "8".to_string();
        let mut combat = CombatState::new(vec![vampire], 10);
        let cleric = test_cleric();
        // Level 3 vs rank 8: diff = -5, impossible
        let result = resolve_turn_undead_with(
            &mut combat, &cleric, 3, 0, &mut test_rng(),
        );
        assert_eq!(result.table_result, TurnResult::Impossible);
        assert!(!result.success);
    }

    // --- Movement ---

    #[test]
    fn fighting_withdrawal_speed() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let fighter = test_fighter(); // movement 120
        let msg = fighting_withdrawal(&mut combat, &fighter);
        // encounter movement = 120/3 = 40, half = 20
        assert!(msg.contains("20'"));
        assert!(msg.contains("Grond"));
    }

    #[test]
    fn retreat_speed() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let fighter = test_fighter();
        let msg = retreat(&mut combat, &fighter);
        // encounter movement = 120/3 = 40
        assert!(msg.contains("40'"));
        assert!(msg.contains("free attack"));
    }

    // --- Status Display ---

    #[test]
    fn combat_status_display() {
        let mut combat = CombatState::new(vec![test_goblin(), test_goblin()], 60);
        roll_initiative_with(&mut combat, &mut test_rng());
        let party = vec![test_fighter(), test_cleric()];
        let status = combat_status(&combat, &party);
        assert!(status.contains("Round 1"));
        assert!(status.contains("60'"));
        assert!(status.contains("Grond"));
        assert!(status.contains("Brother Aldric"));
        assert!(status.contains("Goblin"));
        assert!(status.contains("Party"));
        assert!(status.contains("Monsters"));
    }

    // --- Full Combat Round ---

    #[test]
    fn full_combat_round_sequence() {
        let mut combat = CombatState::new(
            vec![test_goblin(), test_goblin(), test_goblin()], 10,
        );
        let mut fighter = test_fighter();
        let mut cleric = test_cleric();
        let mut rng = test_rng();

        // 1. Roll initiative
        let (_p, _m) = roll_initiative_with(&mut combat, &mut rng);
        assert_eq!(combat.round, 1);

        // 2. Fighter attacks goblin 0
        let _r1 = character_melee_attack_with(
            &mut combat, &fighter, 0, "1d8", 2, &mut rng,
        );

        // 3. Cleric attacks goblin 1
        let _r2 = character_melee_attack_with(
            &mut combat, &cleric, 1, "1d6", 0, &mut rng,
        );

        // 4. Surviving goblins attack
        if combat.monsters[0].is_alive() {
            monster_attack_with(&mut combat, 0, &mut fighter, &mut rng);
        }
        if combat.monsters[1].is_alive() {
            monster_attack_with(&mut combat, 1, &mut cleric, &mut rng);
        }
        if combat.monsters[2].is_alive() {
            monster_attack_with(&mut combat, 2, &mut fighter, &mut rng);
        }

        // 5. Check morale if any goblins died (per-type: goblin morale is 7)
        let dead_goblins = combat.monsters.iter().filter(|m| !m.is_alive()).count();
        if dead_goblins > 0 {
            let _morale = check_morale_with(&mut combat, 7, &mut rng);
            // Morale result logged
        }

        // Verify combat log captured all events
        assert!(combat.log.len() >= 3, "should have at least 3 log entries");
        assert!(combat.log[0].contains("Round 1"));
    }
}
