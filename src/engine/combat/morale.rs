//! Morale check resolution.

use std::fmt;

use rand::Rng;

use crate::model::CombatState;

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
    combat.log.push(result.to_string());
    result
}
