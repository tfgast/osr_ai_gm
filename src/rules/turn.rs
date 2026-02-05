//! Turn undead tables per OSE Reference Booklet p18.
//!
//! Clerics can attempt to turn undead by presenting their holy symbol.
//! The result depends on cleric level vs undead rank (based on HD).
//!
//! Undead ranks:
//!   1 = Skeleton (1 HD)     5 = Wraith (5 HD)     9 = Infernal (9+ HD)
//!   2 = Zombie (2 HD)       6 = Mummy (6 HD)
//!   3 = Ghoul (3 HD)        7 = Spectre (7 HD)
//!   4 = Wight (4 HD)        8 = Vampire (8 HD)
//!
//! The table follows a diagonal pattern based on (cleric_level - undead_rank):
//!   diff <= -3:  Impossible (cannot turn)
//!   diff == -2:  Need 11+ on 2d6
//!   diff == -1:  Need 9+ on 2d6
//!   diff ==  0:  Need 7+ on 2d6
//!   diff 1..=2:  Automatic Turn (T)
//!   diff >= 3:   Automatic Destroy (D)

use serde::{Deserialize, Serialize};

/// Result of a turn undead table lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnResult {
    /// Cannot turn this type of undead at this level.
    Impossible,
    /// Must roll 2d6 and meet or exceed this target number.
    Roll(u32),
    /// Automatically turned — 2d6 HD of undead are affected.
    Turned,
    /// Automatically destroyed — 2d6 HD of undead are destroyed.
    Destroyed,
}

/// Look up the turn undead result for a given cleric level and undead rank.
///
/// - `cleric_level`: the cleric's character level (1+)
/// - `undead_rank`: the undead type rank (1-9, clamped)
pub fn turn_undead_result(cleric_level: u32, undead_rank: u32) -> TurnResult {
    let rank = undead_rank.clamp(1, 9);
    let diff = cleric_level as i32 - rank as i32;

    if diff <= -3 {
        TurnResult::Impossible
    } else if diff == -2 {
        TurnResult::Roll(11)
    } else if diff == -1 {
        TurnResult::Roll(9)
    } else if diff == 0 {
        TurnResult::Roll(7)
    } else if diff <= 2 {
        TurnResult::Turned
    } else {
        TurnResult::Destroyed
    }
}

/// Convert undead Hit Dice to a turn undead rank (1-9).
/// HD 1-8 map directly to ranks 1-8; HD 9+ all map to rank 9.
pub fn undead_rank_from_hd(hd: u32) -> u32 {
    hd.clamp(1, 9)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Skeleton (rank 1) ---

    #[test]
    fn skeleton_level_1() {
        // diff = 0, need 7
        assert_eq!(turn_undead_result(1, 1), TurnResult::Roll(7));
    }

    #[test]
    fn skeleton_level_2() {
        // diff = 1, auto turn
        assert_eq!(turn_undead_result(2, 1), TurnResult::Turned);
    }

    #[test]
    fn skeleton_level_3() {
        // diff = 2, auto turn
        assert_eq!(turn_undead_result(3, 1), TurnResult::Turned);
    }

    #[test]
    fn skeleton_level_4() {
        // diff = 3, destroy
        assert_eq!(turn_undead_result(4, 1), TurnResult::Destroyed);
    }

    #[test]
    fn skeleton_level_14() {
        // diff = 13, destroy
        assert_eq!(turn_undead_result(14, 1), TurnResult::Destroyed);
    }

    // --- Zombie (rank 2) ---

    #[test]
    fn zombie_level_1() {
        // diff = -1, need 9
        assert_eq!(turn_undead_result(1, 2), TurnResult::Roll(9));
    }

    #[test]
    fn zombie_level_2() {
        // diff = 0, need 7
        assert_eq!(turn_undead_result(2, 2), TurnResult::Roll(7));
    }

    #[test]
    fn zombie_level_3() {
        // diff = 1, auto turn
        assert_eq!(turn_undead_result(3, 2), TurnResult::Turned);
    }

    #[test]
    fn zombie_level_5() {
        // diff = 3, destroy
        assert_eq!(turn_undead_result(5, 2), TurnResult::Destroyed);
    }

    // --- Ghoul (rank 3) ---

    #[test]
    fn ghoul_level_1() {
        // diff = -2, need 11
        assert_eq!(turn_undead_result(1, 3), TurnResult::Roll(11));
    }

    #[test]
    fn ghoul_level_2() {
        // diff = -1, need 9
        assert_eq!(turn_undead_result(2, 3), TurnResult::Roll(9));
    }

    #[test]
    fn ghoul_level_3() {
        // diff = 0, need 7
        assert_eq!(turn_undead_result(3, 3), TurnResult::Roll(7));
    }

    // --- Wight (rank 4) ---

    #[test]
    fn wight_level_1() {
        // diff = -3, impossible
        assert_eq!(turn_undead_result(1, 4), TurnResult::Impossible);
    }

    #[test]
    fn wight_level_2() {
        // diff = -2, need 11
        assert_eq!(turn_undead_result(2, 4), TurnResult::Roll(11));
    }

    // --- Vampire (rank 8) ---

    #[test]
    fn vampire_level_5() {
        // diff = -3, impossible
        assert_eq!(turn_undead_result(5, 8), TurnResult::Impossible);
    }

    #[test]
    fn vampire_level_6() {
        // diff = -2, need 11
        assert_eq!(turn_undead_result(6, 8), TurnResult::Roll(11));
    }

    #[test]
    fn vampire_level_7() {
        // diff = -1, need 9
        assert_eq!(turn_undead_result(7, 8), TurnResult::Roll(9));
    }

    #[test]
    fn vampire_level_8() {
        // diff = 0, need 7
        assert_eq!(turn_undead_result(8, 8), TurnResult::Roll(7));
    }

    #[test]
    fn vampire_level_9() {
        // diff = 1, auto turn
        assert_eq!(turn_undead_result(9, 8), TurnResult::Turned);
    }

    #[test]
    fn vampire_level_10() {
        // diff = 2, auto turn
        assert_eq!(turn_undead_result(10, 8), TurnResult::Turned);
    }

    #[test]
    fn vampire_level_11() {
        // diff = 3, destroy
        assert_eq!(turn_undead_result(11, 8), TurnResult::Destroyed);
    }

    // --- Infernal (rank 9) ---

    #[test]
    fn infernal_level_6() {
        // diff = -3, impossible
        assert_eq!(turn_undead_result(6, 9), TurnResult::Impossible);
    }

    #[test]
    fn infernal_level_7() {
        // diff = -2, need 11
        assert_eq!(turn_undead_result(7, 9), TurnResult::Roll(11));
    }

    #[test]
    fn infernal_level_9() {
        // diff = 0, need 7
        assert_eq!(turn_undead_result(9, 9), TurnResult::Roll(7));
    }

    #[test]
    fn infernal_level_11() {
        // diff = 2, auto turn
        assert_eq!(turn_undead_result(11, 9), TurnResult::Turned);
    }

    #[test]
    fn infernal_level_12() {
        // diff = 3, destroy
        assert_eq!(turn_undead_result(12, 9), TurnResult::Destroyed);
    }

    // --- undead_rank_from_hd ---

    #[test]
    fn rank_clamps_low() {
        assert_eq!(undead_rank_from_hd(0), 1);
    }

    #[test]
    fn rank_direct_mapping() {
        for hd in 1..=8 {
            assert_eq!(undead_rank_from_hd(hd), hd);
        }
    }

    #[test]
    fn rank_clamps_high() {
        assert_eq!(undead_rank_from_hd(9), 9);
        assert_eq!(undead_rank_from_hd(15), 9);
        assert_eq!(undead_rank_from_hd(100), 9);
    }
}
