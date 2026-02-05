//! XP engine: award XP, check level advancement, apply level-up changes.

use rand::Rng;
use crate::model::Character;
use crate::rules::class::class_def;
use crate::rules::xp::{check_level_up, prime_req_xp_modifier, adjust_xp};
use crate::rules::ability::con_hp_mod;
use crate::rules::save::{saving_throws, SavingThrows};
use crate::rules::spell::{self, SpellSlots};

/// Result of awarding XP to a character (no level-up; use `train` for that).
#[derive(Debug)]
pub struct XpAwardResult {
    pub base_xp: u64,
    pub modifier_pct: i32,
    pub adjusted_xp: u64,
    pub new_total: u64,
    /// True if the character now has enough XP to train for the next level.
    pub ready_to_train: bool,
}

/// Result of applying a level-up via training.
#[derive(Debug)]
pub struct LevelUpResult {
    pub old_level: u32,
    pub new_level: u32,
    pub hp_gained: i32,
    pub new_thac0: u32,
    pub new_saves: SavingThrows,
    pub old_spell_slots: SpellSlots,
    pub new_spell_slots: SpellSlots,
}

/// Award XP to a character with prime requisite modifier applied.
/// Does NOT trigger level-up; use `apply_level_up` after training.
/// treasure_gp: gold pieces of treasure (1gp = 1xp).
/// monster_xp: XP from defeated monsters.
pub fn award_xp(character: &mut Character, treasure_gp: u64, monster_xp: u64) -> XpAwardResult {
    let base_xp = treasure_gp + monster_xp;
    let abilities = character.abilities.to_array();
    let modifier_pct = prime_req_xp_modifier(character.class, &abilities);
    let adjusted_xp = adjust_xp(base_xp, modifier_pct);

    character.xp += adjusted_xp;
    let new_total = character.xp;
    let ready_to_train = check_level_up(character.class, character.level, character.xp).is_some();

    XpAwardResult {
        base_xp,
        modifier_pct,
        adjusted_xp,
        new_total,
        ready_to_train,
    }
}

/// Apply one level-up to a character. Caller must verify XP is sufficient.
/// Rolls HP, updates level, THAC0, saving throws.
pub fn apply_level_up(character: &mut Character) -> LevelUpResult {
    let mut rng = rand::thread_rng();
    apply_level_up_with(&mut rng, character)
}

/// Testable version with explicit RNG.
pub fn apply_level_up_with<R: Rng>(rng: &mut R, character: &mut Character) -> LevelUpResult {
    let old_level = character.level;
    let new_level = old_level + 1;
    let def = class_def(character.class);

    // Capture old spell slots for report
    let old_spell_slots = spell::spell_slots(def.spell_progression, old_level);

    // Roll HP for new level
    let hp_roll = rng.gen_range(1..=def.hit_die as i32);
    let con_mod = con_hp_mod(character.abilities.constitution);
    let hp_gained = (hp_roll + con_mod).max(1);

    // Apply changes
    character.level = new_level;
    character.max_hp += hp_gained;
    character.hp += hp_gained;
    character.thac0 = crate::engine::chargen::thac0(def.combat_aptitude, new_level);
    let save_cat = def.save_category;
    let new_saves = saving_throws(save_cat, new_level);
    character.saving_throws = Some(new_saves.clone());

    let new_spell_slots = spell::spell_slots(def.spell_progression, new_level);

    LevelUpResult {
        old_level,
        new_level,
        hp_gained,
        new_thac0: character.thac0,
        new_saves,
        old_spell_slots,
        new_spell_slots,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AbilityScores;
    use crate::rules::class::Class;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn make_fighter(xp: u64, level: u32) -> Character {
        let mut c = Character::new("Test", Class::Fighter);
        c.abilities = AbilityScores {
            strength: 16, intelligence: 10, wisdom: 10,
            dexterity: 10, constitution: 14, charisma: 10,
        };
        c.xp = xp;
        c.level = level;
        c.hp = 8;
        c.max_hp = 8;
        c
    }

    #[test]
    fn award_treasure_xp() {
        let mut fighter = make_fighter(0, 1);
        let result = award_xp(&mut fighter, 100, 0);
        // STR 16 = +10% XP modifier
        assert_eq!(result.base_xp, 100);
        assert_eq!(result.modifier_pct, 10);
        assert_eq!(result.adjusted_xp, 110);
        assert_eq!(fighter.xp, 110);
        assert!(!result.ready_to_train);
    }

    #[test]
    fn award_monster_xp() {
        let mut fighter = make_fighter(0, 1);
        let result = award_xp(&mut fighter, 0, 50);
        assert_eq!(result.base_xp, 50);
        assert_eq!(result.adjusted_xp, 55); // +10%
    }

    #[test]
    fn award_combined_xp() {
        let mut fighter = make_fighter(0, 1);
        let result = award_xp(&mut fighter, 500, 200);
        assert_eq!(result.base_xp, 700);
        assert_eq!(result.adjusted_xp, 770); // +10%
    }

    #[test]
    fn ready_to_train_when_xp_sufficient() {
        let mut fighter = make_fighter(1_900, 1);
        // Fighter needs 2000 XP for level 2. Give 100 treasure = 110 adjusted
        let result = award_xp(&mut fighter, 100, 0);
        assert!(result.ready_to_train);
        // Character should NOT have leveled up automatically
        assert_eq!(fighter.level, 1);
    }

    #[test]
    fn not_ready_to_train_insufficient_xp() {
        let mut fighter = make_fighter(0, 1);
        let result = award_xp(&mut fighter, 10, 0);
        assert!(!result.ready_to_train);
        assert_eq!(fighter.level, 1);
    }

    #[test]
    fn apply_level_up_basic() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut fighter = make_fighter(2_100, 1);
        let result = apply_level_up_with(&mut rng, &mut fighter);
        assert_eq!(result.old_level, 1);
        assert_eq!(result.new_level, 2);
        assert_eq!(fighter.level, 2);
        assert!(result.hp_gained >= 1);
        assert!(fighter.max_hp > 8);
        assert_eq!(fighter.hp, fighter.max_hp); // hp also increased
    }

    #[test]
    fn hp_gain_minimum_1() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut c = Character::new("Sickly", Class::Fighter);
        c.abilities = AbilityScores {
            strength: 10, intelligence: 10, wisdom: 10,
            dexterity: 10, constitution: 3, charisma: 10, // CON 3 = -3 HP mod
        };
        c.xp = 2_100;
        c.level = 1;
        c.hp = 1;
        c.max_hp = 1;

        let result = apply_level_up_with(&mut rng, &mut c);
        // Even with CON -3, HP gained should be at least 1
        assert!(result.hp_gained >= 1);
        assert_eq!(c.level, 2);
    }

    #[test]
    fn low_prime_req_penalty() {
        let mut c = Character::new("Weak", Class::Fighter);
        c.abilities = AbilityScores {
            strength: 5, intelligence: 10, wisdom: 10,
            dexterity: 10, constitution: 10, charisma: 10,
        };
        c.level = 1;
        let result = award_xp(&mut c, 100, 0);
        assert_eq!(result.modifier_pct, -20);
        assert_eq!(result.adjusted_xp, 80);
    }

    #[test]
    fn thief_xp_with_dex_prime() {
        let mut thief = Character::new("Sneaky", Class::Thief);
        thief.abilities = AbilityScores {
            strength: 10, intelligence: 10, wisdom: 10,
            dexterity: 16, constitution: 10, charisma: 10,
        };
        thief.level = 1;
        thief.hp = 4;
        thief.max_hp = 4;
        let result = award_xp(&mut thief, 1000, 200);
        assert_eq!(result.modifier_pct, 10); // DEX 16 = +10%
        assert_eq!(result.adjusted_xp, 1320); // 1200 * 1.10
        // Thief needs 1200 XP for level 2 — should be ready to train
        assert!(result.ready_to_train);
        // But should NOT have leveled up
        assert_eq!(thief.level, 1);
    }

    #[test]
    fn level_up_updates_spell_slots_for_caster() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut mu = Character::new("Mage", Class::MagicUser);
        mu.abilities = AbilityScores {
            strength: 10, intelligence: 16, wisdom: 10,
            dexterity: 10, constitution: 10, charisma: 10,
        };
        mu.xp = 2_600;
        mu.level = 1;
        mu.hp = 3;
        mu.max_hp = 3;

        let result = apply_level_up_with(&mut rng, &mut mu);
        assert_eq!(result.old_level, 1);
        assert_eq!(result.new_level, 2);
        // Magic-User L1: [1,0,0,0,0,0], L2: [2,0,0,0,0,0]
        assert_eq!(result.old_spell_slots[0], 1);
        assert_eq!(result.new_spell_slots[0], 2);
    }

    #[test]
    fn level_up_no_spell_change_for_fighter() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut fighter = make_fighter(2_100, 1);
        let result = apply_level_up_with(&mut rng, &mut fighter);
        assert_eq!(result.old_spell_slots, [0; 6]);
        assert_eq!(result.new_spell_slots, [0; 6]);
    }

    #[test]
    fn level_up_updates_saves() {
        use crate::rules::save::{saving_throws, SaveCategory};
        let mut rng = StdRng::seed_from_u64(42);
        // Fighter saves change at level 4
        let mut fighter = make_fighter(8_100, 3);
        // Set saves for level 3 (Fighter L1-3 bracket)
        fighter.saving_throws = Some(saving_throws(SaveCategory::Fighter, 3));
        let old_saves = fighter.saving_throws.unwrap();
        let result = apply_level_up_with(&mut rng, &mut fighter);
        assert_eq!(result.new_level, 4);
        // Fighter saves improve at level 4 (new bracket)
        assert!(result.new_saves.death <= old_saves.death);
    }
}
