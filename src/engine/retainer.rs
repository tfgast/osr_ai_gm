//! Retainer/hireling system per OSE Rules Tome.
//! CHA-based max retainers, hiring reaction rolls, loyalty/morale.

use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::rules::ability::{cha_max_retainers, cha_loyalty, cha_reaction_mod};
use crate::rules::class::ClassId;

/// A retainer (hired NPC follower).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retainer {
    pub name: String,
    pub class: ClassId,
    pub level: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub loyalty: u32,  // base loyalty score (2-12)
    pub wage_gp: u32,  // monthly wage in gold pieces
}

impl Retainer {
    pub fn new(name: &str, class: impl Into<ClassId>, level: u32, hp: i32, loyalty: u32, wage_gp: u32) -> Self {
        Retainer {
            name: name.to_string(),
            class: class.into(),
            level,
            hp,
            max_hp: hp,
            loyalty,
            wage_gp,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

/// Hiring reaction roll result (2d6 + CHA modifier).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HireReaction {
    Refused,    // 2-3: refuses, bad reputation spreads
    Reluctant,  // 4-5: refuses
    Uncertain,  // 6-8: re-roll with better offer
    Accepts,    // 9-11: accepts
    Eager,      // 12+: accepts, +1 loyalty
}

impl HireReaction {
    pub fn name(self) -> &'static str {
        match self {
            HireReaction::Refused => "Refused (bad reputation)",
            HireReaction::Reluctant => "Refused",
            HireReaction::Uncertain => "Uncertain (try better offer)",
            HireReaction::Accepts => "Accepts",
            HireReaction::Eager => "Eager (bonus loyalty)",
        }
    }
}

/// Roll a hiring reaction: 2d6 + CHA modifier.
pub fn hiring_reaction(cha_score: i32) -> HireReaction {
    let mut rng = rand::thread_rng();
    hiring_reaction_with(&mut rng, cha_score)
}

/// Testable version with explicit RNG.
pub fn hiring_reaction_with<R: Rng>(rng: &mut R, cha_score: i32) -> HireReaction {
    let roll: i32 = rng.gen_range(1..=6) + rng.gen_range(1..=6);
    let modified = roll + cha_reaction_mod(cha_score);
    hiring_reaction_from_roll(modified)
}

/// Convert a modified 2d6 roll to a hiring reaction.
pub fn hiring_reaction_from_roll(modified_roll: i32) -> HireReaction {
    match modified_roll {
        i32::MIN..=3 => HireReaction::Refused,
        4..=5 => HireReaction::Reluctant,
        6..=8 => HireReaction::Uncertain,
        9..=11 => HireReaction::Accepts,
        _ => HireReaction::Eager,
    }
}

/// Check how many retainers a character can have based on CHA.
pub fn max_retainers(cha_score: i32) -> u32 {
    cha_max_retainers(cha_score)
}

/// Get base loyalty score from CHA.
pub fn base_loyalty(cha_score: i32) -> u32 {
    cha_loyalty(cha_score)
}

/// Loyalty check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoyaltyResult {
    Loyal,      // Roll <= loyalty: stays loyal
    Wavering,   // Roll = loyalty + 1 or 2: uncertain
    Disloyal,   // Roll > loyalty + 2: leaves or betrays
}

/// Perform a loyalty check (2d6 vs loyalty score).
pub fn loyalty_check(loyalty: u32) -> LoyaltyResult {
    let mut rng = rand::thread_rng();
    loyalty_check_with(&mut rng, loyalty)
}

/// Testable version with explicit RNG.
pub fn loyalty_check_with<R: Rng>(rng: &mut R, loyalty: u32) -> LoyaltyResult {
    let roll: u32 = rng.gen_range(1..=6) + rng.gen_range(1..=6);
    loyalty_check_from_roll(roll, loyalty)
}

/// Convert a 2d6 roll + loyalty score to a result.
pub fn loyalty_check_from_roll(roll: u32, loyalty: u32) -> LoyaltyResult {
    if roll <= loyalty {
        LoyaltyResult::Loyal
    } else if roll <= loyalty + 2 {
        LoyaltyResult::Wavering
    } else {
        LoyaltyResult::Disloyal
    }
}

/// Retainers receive half share of XP.
pub fn retainer_xp_share(total_xp: u64) -> u64 {
    total_xp / 2
}

/// Monthly wage by retainer level (standard rates).
pub fn standard_wage(level: u32) -> u32 {
    match level {
        0 => 25,       // Normal human
        1 => 25,
        2 => 50,
        3 => 100,
        4 => 200,
        _ => 200 * 2u32.pow(level.saturating_sub(4)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn retainer_creation() {
        let r = Retainer::new("Hrothgar", "Fighter", 1, 6, 7, 25);
        assert_eq!(r.name, "Hrothgar");
        assert!(r.is_alive());
        assert_eq!(r.loyalty, 7);
    }

    #[test]
    fn max_retainers_by_cha() {
        assert_eq!(max_retainers(3), 1);
        assert_eq!(max_retainers(10), 4);
        assert_eq!(max_retainers(18), 7);
    }

    #[test]
    fn base_loyalty_by_cha() {
        assert_eq!(base_loyalty(3), 4);
        assert_eq!(base_loyalty(10), 7);
        assert_eq!(base_loyalty(18), 10);
    }

    #[test]
    fn hiring_reaction_hostile() {
        assert_eq!(hiring_reaction_from_roll(2), HireReaction::Refused);
        assert_eq!(hiring_reaction_from_roll(3), HireReaction::Refused);
    }

    #[test]
    fn hiring_reaction_reluctant() {
        assert_eq!(hiring_reaction_from_roll(4), HireReaction::Reluctant);
        assert_eq!(hiring_reaction_from_roll(5), HireReaction::Reluctant);
    }

    #[test]
    fn hiring_reaction_uncertain() {
        assert_eq!(hiring_reaction_from_roll(6), HireReaction::Uncertain);
        assert_eq!(hiring_reaction_from_roll(8), HireReaction::Uncertain);
    }

    #[test]
    fn hiring_reaction_accepts() {
        assert_eq!(hiring_reaction_from_roll(9), HireReaction::Accepts);
        assert_eq!(hiring_reaction_from_roll(11), HireReaction::Accepts);
    }

    #[test]
    fn hiring_reaction_eager() {
        assert_eq!(hiring_reaction_from_roll(12), HireReaction::Eager);
        assert_eq!(hiring_reaction_from_roll(14), HireReaction::Eager);
    }

    #[test]
    fn hiring_with_rng() {
        let mut rng = StdRng::seed_from_u64(42);
        let result = hiring_reaction_with(&mut rng, 10);
        // Just verify it returns a valid result
        let _ = result.name();
    }

    #[test]
    fn loyalty_check_loyal() {
        assert_eq!(loyalty_check_from_roll(5, 7), LoyaltyResult::Loyal);
        assert_eq!(loyalty_check_from_roll(7, 7), LoyaltyResult::Loyal);
    }

    #[test]
    fn loyalty_check_wavering() {
        assert_eq!(loyalty_check_from_roll(8, 7), LoyaltyResult::Wavering);
        assert_eq!(loyalty_check_from_roll(9, 7), LoyaltyResult::Wavering);
    }

    #[test]
    fn loyalty_check_disloyal() {
        assert_eq!(loyalty_check_from_roll(10, 7), LoyaltyResult::Disloyal);
        assert_eq!(loyalty_check_from_roll(12, 7), LoyaltyResult::Disloyal);
    }

    #[test]
    fn retainer_xp_half_share() {
        assert_eq!(retainer_xp_share(1000), 500);
        assert_eq!(retainer_xp_share(0), 0);
        assert_eq!(retainer_xp_share(1), 0); // integer division
    }

    #[test]
    fn standard_wages() {
        assert_eq!(standard_wage(1), 25);
        assert_eq!(standard_wage(2), 50);
        assert_eq!(standard_wage(3), 100);
        assert_eq!(standard_wage(4), 200);
    }
}
