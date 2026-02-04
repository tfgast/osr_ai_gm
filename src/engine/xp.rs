/// XP engine: award XP, check level advancement, apply level-up changes.

use rand::Rng;
use crate::model::Character;
use crate::rules::class::class_def;
use crate::rules::xp::{check_level_up, prime_req_xp_modifier, adjust_xp};
use crate::rules::ability::con_hp_mod;
use crate::rules::save::saving_throws;

/// Result of awarding XP to a character.
#[derive(Debug)]
pub struct XpAwardResult {
    pub base_xp: u64,
    pub modifier_pct: i32,
    pub adjusted_xp: u64,
    pub new_total: u64,
    pub leveled_up: bool,
    pub new_level: u32,
    pub hp_gained: i32,
}

/// Award XP to a character with prime requisite modifier applied.
/// treasure_gp: gold pieces of treasure (1gp = 1xp).
/// monster_xp: XP from defeated monsters.
pub fn award_xp(character: &mut Character, treasure_gp: u64, monster_xp: u64) -> XpAwardResult {
    let mut rng = rand::thread_rng();
    award_xp_with(&mut rng, character, treasure_gp, monster_xp)
}

/// Testable version with explicit RNG.
pub fn award_xp_with<R: Rng>(rng: &mut R, character: &mut Character, treasure_gp: u64, monster_xp: u64) -> XpAwardResult {
    let base_xp = treasure_gp + monster_xp;

    let cls = character.class;
    let abilities = character.abilities.to_array();

    let modifier_pct = prime_req_xp_modifier(cls, &abilities);
    let adjusted_xp = adjust_xp(base_xp, modifier_pct);

    character.xp += adjusted_xp;
    let new_total = character.xp;

    // Check for level-up
    let mut leveled_up = false;
    let mut new_level = character.level;
    let mut hp_gained = 0i32;

    {
        while let Some(next_level) = check_level_up(cls, new_level, character.xp) {
            leveled_up = true;
            new_level = next_level;

            // Roll HP for new level
            let def = class_def(cls);
            let hp_roll = rng.gen_range(1..=def.hit_die as i32);
            let con_mod = con_hp_mod(character.abilities.constitution);
            let gained = (hp_roll + con_mod).max(1);
            hp_gained += gained;

            character.level = new_level;
            character.max_hp += gained;
            character.hp += gained;

            // Update THAC0
            character.thac0 = crate::engine::chargen::thac0(def.combat_aptitude, new_level);

            // Update saving throws
            let save_cat = def.save_category;
            character.saving_throws = Some(saving_throws(save_cat, new_level));
        }
    }

    XpAwardResult {
        base_xp,
        modifier_pct,
        adjusted_xp,
        new_total,
        leveled_up,
        new_level,
        hp_gained,
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
        let mut rng = StdRng::seed_from_u64(42);
        let mut fighter = make_fighter(0, 1);
        let result = award_xp_with(&mut rng, &mut fighter, 100, 0);
        // STR 16 = +10% XP modifier
        assert_eq!(result.base_xp, 100);
        assert_eq!(result.modifier_pct, 10);
        assert_eq!(result.adjusted_xp, 110);
        assert_eq!(fighter.xp, 110);
    }

    #[test]
    fn award_monster_xp() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut fighter = make_fighter(0, 1);
        let result = award_xp_with(&mut rng, &mut fighter, 0, 50);
        assert_eq!(result.base_xp, 50);
        assert_eq!(result.adjusted_xp, 55); // +10%
    }

    #[test]
    fn award_combined_xp() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut fighter = make_fighter(0, 1);
        let result = award_xp_with(&mut rng, &mut fighter, 500, 200);
        assert_eq!(result.base_xp, 700);
        assert_eq!(result.adjusted_xp, 770); // +10%
    }

    #[test]
    fn level_up_on_xp_award() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut fighter = make_fighter(1_900, 1);
        // Fighter needs 2000 XP for level 2. Give 100 treasure = 110 adjusted
        let result = award_xp_with(&mut rng, &mut fighter, 100, 0);
        assert!(result.leveled_up);
        assert_eq!(result.new_level, 2);
        assert_eq!(fighter.level, 2);
        assert!(result.hp_gained > 0);
        assert!(fighter.max_hp > 8);
    }

    #[test]
    fn no_level_up_insufficient_xp() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut fighter = make_fighter(0, 1);
        let result = award_xp_with(&mut rng, &mut fighter, 10, 0);
        assert!(!result.leveled_up);
        assert_eq!(result.new_level, 1);
        assert_eq!(fighter.level, 1);
    }

    #[test]
    fn hp_gain_minimum_1() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut c = Character::new("Sickly", Class::Fighter);
        c.abilities = AbilityScores {
            strength: 10, intelligence: 10, wisdom: 10,
            dexterity: 10, constitution: 3, charisma: 10, // CON 3 = -3 HP mod
        };
        c.xp = 1_999;
        c.level = 1;
        c.hp = 1;
        c.max_hp = 1;

        let result = award_xp_with(&mut rng, &mut c, 100, 0);
        // Even with CON -3, HP gained should be at least 1
        assert!(result.leveled_up);
        assert!(result.hp_gained >= 1);
    }

    #[test]
    fn low_prime_req_penalty() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut c = Character::new("Weak", Class::Fighter);
        c.abilities = AbilityScores {
            strength: 5, intelligence: 10, wisdom: 10,
            dexterity: 10, constitution: 10, charisma: 10,
        };
        c.level = 1;
        let result = award_xp_with(&mut rng, &mut c, 100, 0);
        assert_eq!(result.modifier_pct, -20);
        assert_eq!(result.adjusted_xp, 80);
    }

    #[test]
    fn thief_xp_with_dex_prime() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut thief = Character::new("Sneaky", Class::Thief);
        thief.abilities = AbilityScores {
            strength: 10, intelligence: 10, wisdom: 10,
            dexterity: 16, constitution: 10, charisma: 10,
        };
        thief.level = 1;
        thief.hp = 4;
        thief.max_hp = 4;
        let result = award_xp_with(&mut rng, &mut thief, 1000, 200);
        assert_eq!(result.modifier_pct, 10); // DEX 16 = +10%
        assert_eq!(result.adjusted_xp, 1320); // 1200 * 1.10
        // Thief needs 1200 XP for level 2
        assert!(result.leveled_up);
    }
}
