//! Combat engine for OSE.
//!
//! Implements the full combat round sequence per OSE Reference Booklet p116-124:
//! 1. Declarations (spell casters, retreats)
//! 2. Initiative (group d6, each side)
//! 3. Winning side acts: morale -> movement -> missile -> magic -> melee
//! 4. Losing side acts (same sub-phase order)
//! 5. End-of-round bookkeeping

mod attack;
mod initiative;
mod morale;
mod movement;
pub mod results;
mod turn_undead;

pub use attack::{
    AttackResult, CoupDeGraceResult, character_melee_attack, character_melee_attack_with,
    character_missile_attack, character_missile_attack_with, coup_de_grace,
    monster_attack, monster_attack_with, resolve_character_attack, set_monster_helpless,
};
pub use initiative::{declare_spell, is_disrupted, roll_initiative, roll_initiative_with};
pub use morale::{MoraleResult, check_morale, check_morale_with};
pub use movement::{RetreatResult, close, fighting_withdrawal, retreat, retreat_with};
pub use turn_undead::{TurnUndeadResult, resolve_turn_undead, resolve_turn_undead_with};

use crate::model::{Character, CombatState};

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
        out.push_str(&format!("  {} ({}) — {}\n", c.name, c.class.name(), status));
    }

    out.push_str("\nMonsters:\n");
    for (i, m) in combat.monsters.iter().enumerate() {
        let status = if m.is_alive() {
            let mut s = format!("HP {}/{}, AC {}", m.hp, m.max_hp, m.ac);
            if m.helpless {
                s.push_str(" [HELPLESS]");
            }
            if m.turned {
                s.push_str(" [TURNED]");
            }
            s
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
    use crate::rules::alignment::Alignment;
    use crate::rules::class::Class;
    use crate::rules::equipment;
    use crate::rules::turn::TurnResult;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn test_fighter() -> Character {
        Character {
            name: "Grond".to_string(),
            class: Class::Fighter,
            level: 1,
            abilities: AbilityScores {
                strength: 16, intelligence: 10, wisdom: 10,
                dexterity: 12, constitution: 14, charisma: 10,
            },
            hp: 8, max_hp: 8, ac: 3, xp: 0,
            inventory: vec![], spells: vec![],
            alignment: Alignment::Neutral, gold_gp: 100,
            saving_throws: None, thac0: 19, movement_rate: 120,
        }
    }

    fn test_cleric() -> Character {
        Character {
            name: "Brother Aldric".to_string(),
            class: Class::Cleric,
            level: 3,
            abilities: AbilityScores {
                strength: 12, intelligence: 10, wisdom: 16,
                dexterity: 10, constitution: 13, charisma: 14,
            },
            hp: 14, max_hp: 14, ac: 4, xp: 0,
            inventory: vec![], spells: vec![],
            alignment: Alignment::Lawful, gold_gp: 50,
            saving_throws: None, thac0: 19, movement_rate: 120,
        }
    }

    fn test_magic_user() -> Character {
        Character {
            name: "Elara".to_string(),
            class: Class::MagicUser,
            level: 1,
            abilities: AbilityScores {
                strength: 8, intelligence: 16, wisdom: 10,
                dexterity: 14, constitution: 10, charisma: 11,
            },
            hp: 3, max_hp: 3, ac: 7, xp: 0,
            inventory: vec![], spells: vec![],
            alignment: Alignment::Neutral, gold_gp: 40,
            saving_throws: None, thac0: 19, movement_rate: 120,
        }
    }

    fn test_goblin() -> Monster {
        Monster {
            name: "Goblin".to_string(),
            hit_dice: "1-1".parse().unwrap(),
            hp: 3, max_hp: 3, ac: 6,
            attacks: vec!["weapon".to_string()],
            damage: "1d6".to_string(),
            morale: 7, xp_value: 5,
            turned: false,
            helpless: false,
        }
    }

    fn test_skeleton() -> Monster {
        Monster {
            name: "Skeleton".to_string(),
            hit_dice: "1".parse().unwrap(),
            hp: 4, max_hp: 4, ac: 7,
            attacks: vec!["weapon".to_string()],
            damage: "1d6".to_string(),
            morale: 12, xp_value: 10,
            turned: false,
            helpless: false,
        }
    }

    fn test_ogre() -> Monster {
        Monster {
            name: "Ogre".to_string(),
            hit_dice: "4+1".parse().unwrap(),
            hp: 19, max_hp: 19, ac: 5,
            attacks: vec!["club".to_string()],
            damage: "1d10".to_string(),
            morale: 10, xp_value: 125,
            turned: false,
            helpless: false,
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

    #[test]
    fn melee_range_allows_10_feet() {
        // Per OSR rules, melee combat is allowed at 5-10' distance
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let fighter = test_fighter();
        let sword = equipment::find_weapon("Sword").unwrap();
        let result = resolve_character_attack(&mut combat, &fighter, 0, &sword, 0);
        assert!(result.is_ok(), "melee attack should succeed at 10' distance");
    }

    #[test]
    fn melee_range_fails_beyond_10_feet() {
        // Melee weapons cannot reach beyond 10'
        let mut combat = CombatState::new(vec![test_goblin()], 11);
        let fighter = test_fighter();
        let sword = equipment::find_weapon("Sword").unwrap();
        let result = resolve_character_attack(&mut combat, &fighter, 0, &sword, 0);
        assert!(result.is_err(), "melee attack should fail at 11' distance");
        assert!(result.unwrap_err().contains("melee weapon"));
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

    #[test]
    fn missile_attack_melee_range_error() {
        let mut combat = CombatState::new(vec![test_goblin()], 0);
        let fighter = test_fighter();
        let mut rng = test_rng();
        let result = character_missile_attack_with(
            &mut combat, &fighter, 0, "1d6", 1, (50, 100, 150), &mut rng,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot use missile weapons in melee"));
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
        panic!("expected at least one hit in 50 attempts with seeded RNG");
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
        vampire.hit_dice = "8".parse().unwrap();
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
        assert_eq!(combat.distance, 30); // 10 + 20
    }

    #[test]
    fn retreat_speed() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let mut fighter = test_fighter();
        let result = retreat_with(&mut combat, &mut fighter, &mut test_rng());
        // encounter movement = 120/3 = 40
        assert_eq!(result.distance_moved, 40);
        assert_eq!(result.new_distance, 50); // 10 + 40
        assert_eq!(combat.distance, 50);
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

    // --- Multi-round combat through all phases ---

    #[test]
    fn multi_round_all_phases() {
        // Full multi-round combat: Declaration -> Initiative -> Morale ->
        // Movement -> Missile -> Magic -> Melee -> EndOfRound
        let mut combat = CombatState::new(
            vec![test_goblin(), test_goblin(), test_goblin()], 60,
        );
        let mut fighter = test_fighter();
        let mut rng = test_rng();

        // === ROUND 1 ===
        // Declaration phase
        assert_eq!(combat.phase, crate::model::CombatPhase::Declaration);
        declare_spell(&mut combat, "Elara", "Sleep");
        combat.advance_phase(); // -> Initiative

        // Initiative
        assert_eq!(combat.phase, crate::model::CombatPhase::Initiative);
        let (_p, _m) = roll_initiative_with(&mut combat, &mut rng);
        // roll_initiative sets phase to Morale
        assert_eq!(combat.phase, crate::model::CombatPhase::Morale);
        assert_eq!(combat.round, 1);

        // Morale (no deaths yet, skip)
        assert!(!combat.should_check_morale());
        combat.advance_phase(); // -> Movement

        // Movement
        assert_eq!(combat.phase, crate::model::CombatPhase::Movement);
        combat.advance_phase(); // -> Missile

        // Missile phase — shoot at goblin
        assert_eq!(combat.phase, crate::model::CombatPhase::Missile);
        let missile_result = character_missile_attack_with(
            &mut combat, &fighter, 0, "1d6", 1, (50, 100, 150), &mut rng,
        );
        assert!(missile_result.is_ok());
        combat.advance_phase(); // -> Magic

        // Magic phase — spell resolves (not disrupted)
        assert_eq!(combat.phase, crate::model::CombatPhase::Magic);
        assert!(!is_disrupted(&combat, "Elara"));
        combat.advance_phase(); // -> Melee

        // Melee
        assert_eq!(combat.phase, crate::model::CombatPhase::Melee);
        // Goblins attack in melee
        for i in 0..3 {
            if combat.monsters[i].is_alive() {
                monster_attack_with(&mut combat, i, &mut fighter, &mut rng);
            }
        }
        combat.advance_phase(); // -> EndOfRound

        // End of round
        assert_eq!(combat.phase, crate::model::CombatPhase::EndOfRound);
        combat.advance_phase(); // -> Declaration (new round)

        // === ROUND 2 ===
        assert_eq!(combat.phase, crate::model::CombatPhase::Declaration);
        // Initiative
        combat.advance_phase();
        let (_p2, _m2) = roll_initiative_with(&mut combat, &mut rng);
        assert_eq!(combat.round, 2);
        // Spell declarations should be cleared from round 1
        assert!(combat.spell_declarations.is_empty());
        assert!(combat.disrupted.is_empty());

        // Melee — fighter attacks, try to kill a goblin
        for _ in 0..10 {
            if combat.monsters[0].is_alive() {
                combat.monsters[0].hp = combat.monsters[0].max_hp;
                let r = character_melee_attack_with(
                    &mut combat, &fighter, 0, "1d8", 2, &mut rng,
                );
                if r.target_killed { break; }
            } else {
                break;
            }
        }

        // Verify log has been growing across rounds
        assert!(combat.log.len() >= 5, "combat log should have multiple entries: got {}", combat.log.len());
    }

    // --- Phase advancement cycle ---

    #[test]
    fn phase_advancement_full_cycle() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        assert_eq!(combat.phase, crate::model::CombatPhase::Declaration);
        combat.advance_phase();
        assert_eq!(combat.phase, crate::model::CombatPhase::Initiative);
        combat.advance_phase();
        assert_eq!(combat.phase, crate::model::CombatPhase::Morale);
        combat.advance_phase();
        assert_eq!(combat.phase, crate::model::CombatPhase::Movement);
        combat.advance_phase();
        assert_eq!(combat.phase, crate::model::CombatPhase::Missile);
        combat.advance_phase();
        assert_eq!(combat.phase, crate::model::CombatPhase::Magic);
        combat.advance_phase();
        assert_eq!(combat.phase, crate::model::CombatPhase::Melee);
        combat.advance_phase();
        assert_eq!(combat.phase, crate::model::CombatPhase::EndOfRound);
        combat.advance_phase();
        // Wraps back to Declaration
        assert_eq!(combat.phase, crate::model::CombatPhase::Declaration);
    }

    // --- Morale trigger conditions ---

    #[test]
    fn morale_trigger_first_death() {
        let mut combat = CombatState::new(
            vec![test_goblin(), test_goblin(), test_goblin(), test_goblin()], 10,
        );
        assert!(!combat.should_check_morale()); // no deaths
        combat.monsters[0].hp = 0; // kill first goblin
        assert!(combat.should_check_morale()); // first death triggers
        assert!(combat.first_death_checked);
        // Second call should not trigger again
        assert!(!combat.should_check_morale());
    }

    #[test]
    fn morale_trigger_half_killed() {
        let mut combat = CombatState::new(
            vec![test_goblin(), test_goblin(), test_goblin(), test_goblin()], 10,
        );
        combat.monsters[0].hp = 0; // kill first
        let _ = combat.should_check_morale(); // consume first-death trigger
        assert!(!combat.should_check_morale()); // only 1 of 4 dead

        combat.monsters[1].hp = 0; // kill second — now 2 of 4 = 50%
        assert!(combat.should_check_morale()); // half-killed triggers
        assert!(combat.half_killed_checked);
        // Should not trigger again
        assert!(!combat.should_check_morale());
    }

    #[test]
    fn morale_trigger_simultaneous_first_and_half() {
        // 2-monster group: killing the first means 50% are dead too
        let mut combat = CombatState::new(vec![test_goblin(), test_goblin()], 10);
        combat.monsters[0].hp = 0;
        // First call triggers first_death
        assert!(combat.should_check_morale());
        // Second call triggers half_killed (1 of 2 = 50%)
        assert!(combat.should_check_morale());
        // No more triggers
        assert!(!combat.should_check_morale());
    }

    #[test]
    fn morale_score_2_always_flees() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let mut rng = test_rng();
        // Morale 2: any 2d6 roll (2-12) is > 2 except exactly 2
        // Run 100 checks — the vast majority should fail
        let mut fled_count = 0;
        for _ in 0..100 {
            let result = check_morale_with(&mut combat, 2, &mut rng);
            if !result.passed { fled_count += 1; }
        }
        // With morale 2, probability of fleeing = P(2d6 > 2) = 35/36 ≈ 97%
        assert!(fled_count > 90, "morale 2 should flee almost always, fled {} of 100", fled_count);
    }

    #[test]
    fn morale_score_12_never_flees() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let mut rng = test_rng();
        // Morale 12: max 2d6 is 12, so roll <= 12 always passes
        for _ in 0..100 {
            let result = check_morale_with(&mut combat, 12, &mut rng);
            assert!(result.passed, "morale 12 should never flee (rolled {})", result.roll);
        }
    }

    // --- Spell disruption edge cases ---

    #[test]
    fn spell_disruption_multiple_casters() {
        let mut combat = CombatState::new(vec![test_goblin(), test_goblin()], 10);
        let mut mage = test_magic_user();
        declare_spell(&mut combat, "Elara", "Sleep");
        declare_spell(&mut combat, "Brother Aldric", "Cure Light Wounds");
        assert_eq!(combat.spell_declarations.len(), 2);

        // Goblin hits mage — only mage's spell disrupted
        let mut rng = test_rng();
        for _ in 0..50 {
            mage.hp = mage.max_hp;
            combat.disrupted.clear();
            let result = monster_attack_with(&mut combat, 0, &mut mage, &mut rng);
            if result.hit {
                assert!(is_disrupted(&combat, "Elara"));
                assert!(!is_disrupted(&combat, "Brother Aldric"));
                return;
            }
        }
    }

    #[test]
    fn spell_not_disrupted_if_missed() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let mut fighter_as_mage = test_fighter();
        // Declare spell for the fighter (pretending they can cast)
        declare_spell(&mut combat, "Grond", "Fireball");
        // Attack but ensure it misses — use fighter's good AC
        fighter_as_mage.ac = -5; // impossible to hit except nat 20
        let mut rng = StdRng::seed_from_u64(12345);
        // Run a few attacks — most should miss with AC -5
        let mut any_miss = false;
        for _ in 0..20 {
            fighter_as_mage.hp = fighter_as_mage.max_hp;
            combat.disrupted.clear();
            let result = monster_attack_with(&mut combat, 0, &mut fighter_as_mage, &mut rng);
            if !result.hit {
                assert!(!is_disrupted(&combat, "Grond"), "spell should not be disrupted on a miss");
                any_miss = true;
                break;
            }
        }
        assert!(any_miss, "should get at least one miss with AC -5");
    }

    #[test]
    fn spell_disruption_cleared_on_new_round() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        declare_spell(&mut combat, "Elara", "Sleep");
        combat.disrupted.push("Elara".to_string());
        assert!(is_disrupted(&combat, "Elara"));

        // New round clears disruption
        roll_initiative_with(&mut combat, &mut test_rng());
        assert!(!is_disrupted(&combat, "Elara"));
        assert!(combat.spell_declarations.is_empty());
    }

    // --- Initiative ties ---

    #[test]
    fn initiative_tie_simultaneous() {
        let _combat = CombatState::new(vec![test_goblin()], 10);
        // Use a fixed RNG that produces a tie
        // We'll run enough times to get a tie
        let mut rng = test_rng();
        let mut found_tie = false;
        for _ in 0..100 {
            let mut c = CombatState::new(vec![test_goblin()], 10);
            let (p, m) = roll_initiative_with(&mut c, &mut rng);
            if p == m {
                // Tied — log should say "Simultaneous"
                assert!(c.log.last().unwrap().contains("Simultaneous"),
                    "tied initiative should log 'Simultaneous', got: {}",
                    c.log.last().unwrap());
                found_tie = true;
                break;
            }
        }
        assert!(found_tie, "should find at least one initiative tie in 100 rolls");
    }

    #[test]
    fn initiative_determines_action_order() {
        let _combat = CombatState::new(vec![test_goblin()], 10);
        let mut rng = test_rng();
        let mut party_first = false;
        let mut monster_first = false;
        for _ in 0..100 {
            let mut c = CombatState::new(vec![test_goblin()], 10);
            let (p, m) = roll_initiative_with(&mut c, &mut rng);
            if p > m {
                assert!(c.log.last().unwrap().contains("Party acts first"));
                party_first = true;
            } else if m > p {
                assert!(c.log.last().unwrap().contains("Monsters act first"));
                monster_first = true;
            }
            if party_first && monster_first { break; }
        }
        assert!(party_first, "should find party-first at least once");
        assert!(monster_first, "should find monster-first at least once");
    }

    // --- Turn undead with mixed types ---

    #[test]
    fn turn_undead_mixed_types() {
        // Mix of skeletons (rank 1) and a zombie (rank 2)
        let mut zombie = test_skeleton();
        zombie.name = "Zombie".to_string();
        zombie.hit_dice = "2".parse().unwrap();
        zombie.hp = 9;
        zombie.max_hp = 9;
        let mut combat = CombatState::new(
            vec![test_skeleton(), test_skeleton(), zombie], 10,
        );
        let cleric = test_cleric(); // level 3

        // Level 3 vs skeleton (rank 1): diff=2, auto turn
        let result = resolve_turn_undead_with(
            &mut combat, &cleric, 3, 0, &mut test_rng(),
        );
        assert!(result.success);
        assert_eq!(result.table_result, TurnResult::Turned);
        // Should have turned some monsters
        let turned_count = combat.monsters.iter().filter(|m| m.turned).count();
        assert!(turned_count > 0, "should have turned at least one monster");
    }

    #[test]
    fn turn_undead_hd_exhaustion() {
        // Create high-HD undead: a single 6 HD mummy
        let mummy = Monster {
            name: "Mummy".to_string(),
            hit_dice: "5+1".parse().unwrap(),
            hp: 24, max_hp: 24, ac: 3,
            attacks: vec!["touch".to_string()],
            damage: "1d12".to_string(),
            morale: 12, xp_value: 500,
            turned: false,
            helpless: false,
        };
        // Also add a weak skeleton
        let mut combat = CombatState::new(
            vec![test_skeleton(), mummy], 10,
        );
        let cleric = test_cleric();
        // Level 3 vs skeleton (rank 1): auto turn, 2d6 HD affected
        // The skeleton is 1 HD, so it should be turned if roll >= 1
        // The mummy is 5 HD (rank 5), level 3 vs rank 5: diff = -2, need 11
        let result = resolve_turn_undead_with(
            &mut combat, &cleric, 3, 0, &mut test_rng(),
        );
        assert!(result.success);
        // Skeleton should be turned (1 HD, easily fits in budget)
        assert!(combat.monsters[0].turned, "skeleton should be turned");
        // Mummy may or may not be affected depending on HD budget
    }

    #[test]
    fn turn_undead_destroy_kills_monsters() {
        // High-level cleric vs skeletons: should destroy (HP = 0)
        let mut combat = CombatState::new(
            vec![test_skeleton(), test_skeleton(), test_skeleton()], 10,
        );
        let cleric = test_cleric();
        // Level 4+ vs skeleton (rank 1): diff >= 3, auto destroy
        let result = resolve_turn_undead_with(
            &mut combat, &cleric, 5, 0, &mut test_rng(),
        );
        assert!(result.success);
        assert!(result.destroyed, "should be auto-destroy at level 5 vs rank 1");
        // At least some skeletons should have HP = 0
        let dead = combat.monsters.iter().filter(|m| !m.is_alive()).count();
        assert!(dead > 0, "destroyed undead should have 0 HP");
    }

    #[test]
    fn turn_undead_skips_already_turned() {
        let mut combat = CombatState::new(
            vec![test_skeleton(), test_skeleton(), test_skeleton()], 10,
        );
        // Pre-turn the first skeleton
        combat.monsters[0].turned = true;
        let cleric = test_cleric();
        let result = resolve_turn_undead_with(
            &mut combat, &cleric, 3, 0, &mut test_rng(),
        );
        assert!(result.success);
        // First skeleton was already turned, should be skipped
        // Second or third should be newly turned
        let newly_turned = combat.monsters.iter().skip(1).filter(|m| m.turned).count();
        assert!(newly_turned > 0, "should turn additional skeletons");
    }

    // --- Retreat & fighting withdrawal ---

    #[test]
    fn fighting_withdrawal_no_free_attacks() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let fighter = test_fighter();
        let msg = fighting_withdrawal(&mut combat, &fighter);
        assert!(!msg.contains("free attack"), "fighting withdrawal should not mention free attacks");
        assert!(msg.contains("fighting withdrawal"));
    }

    #[test]
    fn retreat_grants_free_attacks() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let mut fighter = test_fighter();
        let result = retreat_with(&mut combat, &mut fighter, &mut test_rng());
        // The goblin should have executed a free attack
        assert_eq!(result.free_attacks.len(), 1, "goblin should get one free attack");
        assert_eq!(result.free_attacks[0].modifiers, 2, "free attack should be at +2");
        assert_eq!(result.free_attacks[0].attacker, "Goblin");
        assert_eq!(result.free_attacks[0].target, "Grond");
    }

    #[test]
    fn retreat_resolves_multiple_free_attacks() {
        let mut combat = CombatState::new(vec![test_goblin(), test_goblin(), test_goblin()], 10);
        let mut fighter = test_fighter();
        let result = retreat_with(&mut combat, &mut fighter, &mut test_rng());
        // All 3 goblins should attack
        assert_eq!(result.free_attacks.len(), 3, "all 3 goblins should get free attacks");
        for atk in &result.free_attacks {
            assert_eq!(atk.modifiers, 2, "each free attack should be at +2");
        }
    }

    #[test]
    fn retreat_dead_monsters_dont_attack() {
        let mut combat = CombatState::new(vec![test_goblin(), test_goblin()], 10);
        // Kill the first goblin
        combat.monsters[0].hp = 0;
        let mut fighter = test_fighter();
        let result = retreat_with(&mut combat, &mut fighter, &mut test_rng());
        // Only living goblin attacks
        assert_eq!(result.free_attacks.len(), 1, "only living goblin should attack");
        assert_eq!(result.free_attacks[0].attacker, "Goblin"); // second goblin also named "Goblin"
    }

    #[test]
    fn retreat_stops_if_character_dies() {
        // Create multiple strong monsters
        let ogre = Monster {
            name: "Ogre".to_string(),
            hit_dice: "4+1".parse().unwrap(),
            hp: 20, max_hp: 20, ac: 5,
            attacks: vec!["club".to_string()],
            damage: "1d10".to_string(),
            morale: 10, xp_value: 125,
            turned: false,
            helpless: false,
        };
        let mut combat = CombatState::new(vec![ogre.clone(), ogre.clone(), ogre], 10);
        // Create a very weak character (1 HP, bad AC)
        let mut weakling = test_fighter();
        weakling.hp = 1;
        weakling.max_hp = 1;
        weakling.ac = 9; // very bad AC, easier to hit

        // Run multiple times to statistically ensure behavior
        // Use seeded RNG for reproducibility
        let mut rng = StdRng::seed_from_u64(12345);
        let result = retreat_with(&mut combat, &mut weakling, &mut rng);

        // At least one attack should happen (3 ogres)
        assert!(!result.free_attacks.is_empty(), "at least one attack should happen");

        // If character died during retreat, attacks should stop early
        if !weakling.is_alive() {
            // Find the killing attack
            let killing_idx = result.free_attacks.iter()
                .position(|a| a.target_killed)
                .expect("should have a killing attack if character is dead");
            // No attacks after the killing blow
            assert_eq!(result.free_attacks.len(), killing_idx + 1,
                "should stop after killing character");
        }
        // Otherwise all 3 attacks should have been made
        else {
            assert_eq!(result.free_attacks.len(), 3, "all ogres should attack if character survives");
        }
    }

    #[test]
    fn distance_tracking_across_movements() {
        let mut combat = CombatState::new(vec![test_goblin()], 0);
        let mut fighter = test_fighter(); // movement 120, encounter = 40

        // Fighting withdrawal from distance 0
        fighting_withdrawal(&mut combat, &fighter);
        assert_eq!(combat.distance, 20); // half encounter move = 20

        // Another fighting withdrawal
        fighting_withdrawal(&mut combat, &fighter);
        assert_eq!(combat.distance, 40);

        // Full retreat
        retreat_with(&mut combat, &mut fighter, &mut test_rng());
        assert_eq!(combat.distance, 80); // + 40 encounter move
    }

    #[test]
    fn distance_saturating_add() {
        // Distance should use saturating_add to prevent overflow
        let mut combat = CombatState::new(vec![test_goblin()], u32::MAX - 10);
        let mut fighter = test_fighter(); // encounter move = 40
        retreat_with(&mut combat, &mut fighter, &mut test_rng());
        assert_eq!(combat.distance, u32::MAX); // saturated, not wrapped
    }

    // --- Close ---

    #[test]
    fn close_to_melee() {
        let mut combat = CombatState::new(vec![test_goblin()], 60);
        let fighter = test_fighter(); // movement 120, encounter = 40
        let msg = close(&mut combat, &fighter, None).unwrap();
        assert!(msg.contains("closes 40'"));
        assert_eq!(combat.distance, 20); // 60 - 40
    }

    #[test]
    fn close_specific_feet() {
        let mut combat = CombatState::new(vec![test_goblin()], 100);
        let fighter = test_fighter(); // encounter move = 40
        let msg = close(&mut combat, &fighter, Some(30)).unwrap();
        assert!(msg.contains("30'"));
        assert_eq!(combat.distance, 70); // 100 - 30
    }

    #[test]
    fn close_capped_by_encounter_move() {
        let mut combat = CombatState::new(vec![test_goblin()], 100);
        let fighter = test_fighter(); // encounter move = 40
        let result = close(&mut combat, &fighter, Some(50));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too far"));
        assert_eq!(combat.distance, 100); // unchanged
    }

    #[test]
    fn close_none_caps_at_encounter_move() {
        // When no distance specified, close up to encounter move
        let mut combat = CombatState::new(vec![test_goblin()], 100);
        let fighter = test_fighter(); // encounter move = 40
        let msg = close(&mut combat, &fighter, None).unwrap();
        assert!(msg.contains("40'"));
        assert_eq!(combat.distance, 60); // 100 - 40
    }

    #[test]
    fn close_none_reaches_zero_when_closer() {
        let mut combat = CombatState::new(vec![test_goblin()], 30);
        let fighter = test_fighter(); // encounter move = 40, but only 30' away
        let msg = close(&mut combat, &fighter, None).unwrap();
        assert!(msg.contains("30'"));
        assert_eq!(combat.distance, 0);
    }

    #[test]
    fn close_already_at_zero() {
        let mut combat = CombatState::new(vec![test_goblin()], 0);
        let fighter = test_fighter();
        let result = close(&mut combat, &fighter, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("melee range"));
    }

    #[test]
    fn close_logs_entry() {
        let mut combat = CombatState::new(vec![test_goblin()], 60);
        let fighter = test_fighter();
        close(&mut combat, &fighter, Some(20)).unwrap();
        assert!(combat.log.last().unwrap().contains("Grond"));
        assert!(combat.log.last().unwrap().contains("20'"));
    }

    // --- Monster attack with multiple routines ---

    #[test]
    fn monster_multiple_attacks_same_round() {
        // Simulate a monster attacking multiple party members in one round
        let mut combat = CombatState::new(vec![test_ogre()], 10);
        let mut fighter = test_fighter();
        let mut cleric = test_cleric();
        let mut rng = test_rng();

        // Ogre attacks fighter
        let r1 = monster_attack_with(&mut combat, 0, &mut fighter, &mut rng);
        assert_eq!(r1.attacker, "Ogre");
        assert_eq!(r1.target, "Grond");

        // Same ogre attacks cleric in same round
        let r2 = monster_attack_with(&mut combat, 0, &mut cleric, &mut rng);
        assert_eq!(r2.attacker, "Ogre");
        assert_eq!(r2.target, "Brother Aldric");
    }

    #[test]
    fn monster_attack_disrupts_declared_spell() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let mut mage = test_magic_user();
        declare_spell(&mut combat, "Elara", "Sleep");

        let mut rng = test_rng();
        let mut disrupted = false;
        for _ in 0..50 {
            mage.hp = mage.max_hp;
            combat.disrupted.clear();
            let result = monster_attack_with(&mut combat, 0, &mut mage, &mut rng);
            if result.hit {
                assert!(is_disrupted(&combat, "Elara"),
                    "hitting a spell-declaring caster should disrupt their spell");
                disrupted = true;
                break;
            }
        }
        assert!(disrupted, "should hit at least once in 50 tries");
    }

    // --- Edge cases ---

    #[test]
    fn tpk_detection() {
        // All party members at 0 HP = TPK
        let mut fighter = test_fighter();
        let mut cleric = test_cleric();
        let mut mage = test_magic_user();
        fighter.hp = 0;
        cleric.hp = 0;
        mage.hp = 0;
        let party = vec![fighter, cleric, mage];
        let all_dead = party.iter().all(|c| !c.is_alive());
        assert!(all_dead, "all party members dead = TPK");
    }

    #[test]
    fn last_monster_killed_mid_round() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let fighter = test_fighter();
        let mut rng = test_rng();

        roll_initiative_with(&mut combat, &mut rng);

        // Kill the only goblin
        combat.monsters[0].hp = 1; // set to 1 HP for easy kill
        for _ in 0..20 {
            combat.monsters[0].hp = 1;
            let result = character_melee_attack_with(
                &mut combat, &fighter, 0, "1d8", 2, &mut rng,
            );
            if result.target_killed {
                assert_eq!(combat.living_monster_count(), 0);
                assert!(combat.living_monsters().is_empty());
                return;
            }
        }
        panic!("should kill a 1 HP goblin within 20 tries");
    }

    #[test]
    fn hp_goes_negative_clamped_to_dead() {
        // Monster HP can go negative from overkill damage
        let mut combat = CombatState::new(vec![test_goblin()], 10); // 3 HP goblin
        let fighter = test_fighter();
        let mut rng = test_rng();

        // Force a hit with high damage
        combat.monsters[0].hp = 1;
        for _ in 0..20 {
            combat.monsters[0].hp = 1;
            let result = character_melee_attack_with(
                &mut combat, &fighter, 0, "1d8", 2, &mut rng,
            );
            if result.hit {
                // Even if damage > remaining HP, monster is dead
                assert!(!combat.monsters[0].is_alive());
                assert!(result.target_hp_after <= 0);
                return;
            }
        }
    }

    #[test]
    fn character_hp_goes_negative() {
        // Character HP can go negative from big damage
        let mut combat = CombatState::new(vec![test_ogre()], 10);
        let mut mage = test_magic_user(); // 3 HP, AC 7
        let mut rng = test_rng();

        for _ in 0..50 {
            mage.hp = 1; // set to 1 HP
            let result = monster_attack_with(&mut combat, 0, &mut mage, &mut rng);
            if result.hit {
                assert!(!mage.is_alive());
                assert!(result.target_hp_after <= 0);
                return;
            }
        }
    }

    #[test]
    #[should_panic(expected = "cannot attack dead or nonexistent monster")]
    fn melee_attack_dead_monster_panics() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        combat.monsters[0].hp = 0;
        let fighter = test_fighter();
        let mut rng = test_rng();
        character_melee_attack_with(&mut combat, &fighter, 0, "1d8", 2, &mut rng);
    }

    #[test]
    fn missile_attack_dead_monster_returns_err() {
        let mut combat = CombatState::new(vec![test_goblin()], 60);
        combat.monsters[0].hp = 0;
        let fighter = test_fighter();
        let mut rng = test_rng();
        let result = character_missile_attack_with(
            &mut combat, &fighter, 0, "1d6", 1, (50, 100, 150), &mut rng,
        );
        assert!(result.is_err());
    }

    #[test]
    fn missile_attack_nonexistent_monster_returns_err() {
        let mut combat = CombatState::new(vec![test_goblin()], 60);
        let fighter = test_fighter();
        let mut rng = test_rng();
        let result = character_missile_attack_with(
            &mut combat, &fighter, 5, "1d6", 1, (50, 100, 150), &mut rng,
        );
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "cannot attack with dead or nonexistent monster")]
    fn dead_monster_cannot_attack() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        combat.monsters[0].hp = 0;
        let mut fighter = test_fighter();
        let mut rng = test_rng();
        monster_attack_with(&mut combat, 0, &mut fighter, &mut rng);
    }

    #[test]
    fn combat_with_single_monster_morale() {
        // Single monster group: killing it means 100% dead, both triggers fire
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        assert!(!combat.should_check_morale());
        combat.monsters[0].hp = 0;
        assert!(combat.should_check_morale()); // first death
        assert!(combat.should_check_morale()); // half killed (1/1 = 100%)
        assert!(!combat.should_check_morale()); // no more triggers
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

    // --- Coup de Grace (Auto-kill helpless) ---

    #[test]
    fn coup_de_grace_kills_helpless_monster() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        combat.monsters[0].helpless = true;
        let fighter = test_fighter();

        let result = coup_de_grace(&mut combat, &fighter, 0).unwrap();
        assert!(result.target_was_helpless);
        assert_eq!(result.attacker, "Grond");
        assert_eq!(result.target, "Goblin");
        assert!(!combat.monsters[0].is_alive());
        assert_eq!(combat.monsters[0].hp, 0);
    }

    #[test]
    fn coup_de_grace_fails_on_non_helpless() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        let fighter = test_fighter();

        let result = coup_de_grace(&mut combat, &fighter, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not helpless"));
    }

    #[test]
    fn coup_de_grace_fails_on_dead_monster() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        combat.monsters[0].hp = 0;
        combat.monsters[0].helpless = true;
        let fighter = test_fighter();

        let result = coup_de_grace(&mut combat, &fighter, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already dead"));
    }

    #[test]
    fn coup_de_grace_fails_on_dead_character() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        combat.monsters[0].helpless = true;
        let mut fighter = test_fighter();
        fighter.hp = 0;

        let result = coup_de_grace(&mut combat, &fighter, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("dead and cannot attack"));
    }

    #[test]
    fn coup_de_grace_clears_helpless_flag() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        combat.monsters[0].helpless = true;
        let fighter = test_fighter();

        coup_de_grace(&mut combat, &fighter, 0).unwrap();
        assert!(!combat.monsters[0].helpless);
    }

    #[test]
    fn coup_de_grace_logs_result() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        combat.monsters[0].helpless = true;
        let fighter = test_fighter();

        coup_de_grace(&mut combat, &fighter, 0).unwrap();
        assert!(combat.log.last().unwrap().contains("dispatches"));
        assert!(combat.log.last().unwrap().contains("auto-kill"));
    }

    #[test]
    fn set_helpless_marks_monster() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        assert!(!combat.monsters[0].helpless);

        let msg = set_monster_helpless(&mut combat, 0, true).unwrap();
        assert!(msg.contains("helpless"));
        assert!(combat.monsters[0].helpless);
    }

    #[test]
    fn set_helpless_unmarks_monster() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        combat.monsters[0].helpless = true;

        let msg = set_monster_helpless(&mut combat, 0, false).unwrap();
        assert!(msg.contains("no longer helpless"));
        assert!(!combat.monsters[0].helpless);
    }

    #[test]
    fn set_helpless_fails_on_dead() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        combat.monsters[0].hp = 0;

        let result = set_monster_helpless(&mut combat, 0, true);
        assert!(result.is_err());
    }

    #[test]
    fn combat_status_shows_helpless() {
        let mut combat = CombatState::new(vec![test_goblin()], 10);
        combat.monsters[0].helpless = true;
        let party = vec![test_fighter()];

        let status = combat_status(&combat, &party);
        assert!(status.contains("[HELPLESS]"));
    }

    #[test]
    fn coup_de_grace_display() {
        let result = CoupDeGraceResult {
            attacker: "Grond".to_string(),
            target: "Goblin".to_string(),
            target_was_helpless: true,
        };
        let display = result.to_string();
        assert!(display.contains("dispatches"));
        assert!(display.contains("Grond"));
        assert!(display.contains("Goblin"));
        assert!(display.contains("auto-kill"));
    }

    #[test]
    fn melee_at_distance_error_suggests_close_and_missile_weapons() {
        use crate::model::Item;

        let mut combat = CombatState::new(vec![test_goblin()], 60);
        let mut fighter = test_fighter();
        // Give the fighter a melee weapon and a missile weapon
        fighter.inventory.push(Item::new("Sword", 6.0, 10));
        fighter.inventory.push(Item::new("Short bow", 3.0, 25));

        let sword = equipment::find_weapon("Sword").unwrap();
        let result = resolve_character_attack(&mut combat, &fighter, 0, sword, 0);

        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should suggest using close command
        assert!(err.contains("close Grond"), "Error should suggest close command: {}", err);
        // Should list available missile weapons
        assert!(err.contains("Short bow"), "Error should list missile weapons: {}", err);
    }
}
