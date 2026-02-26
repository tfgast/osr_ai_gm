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
///
/// When `OSR_BACKEND_TURN_UNDEAD=dsl` and the `dsl-backend` feature is enabled,
/// delegates to the DSL `turn_undead_result` derive. Falls back to native on error.
pub fn turn_undead_result(cleric_level: u32, undead_rank: u32) -> TurnResult {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::TurnUndead) {
        if let Some(result) = dsl_gate::dsl_turn_undead_result(cleric_level, undead_rank) {
            return result;
        }
    }

    turn_undead_result_native(cleric_level, undead_rank)
}

/// Native (Rust) implementation of the turn undead table lookup.
fn turn_undead_result_native(cleric_level: u32, undead_rank: u32) -> TurnResult {
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
///
/// When `OSR_BACKEND_TURN_UNDEAD=dsl` and the `dsl-backend` feature is enabled,
/// delegates to the DSL `undead_rank_from_hd` derive. Falls back to native on error.
pub fn undead_rank_from_hd(hd: u32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::TurnUndead) {
        if let Some(rank) = dsl_gate::dsl_undead_rank_from_hd(hd) {
            return rank;
        }
    }

    hd.clamp(1, 9)
}

/// Evaluate the DSL `turn_undead_attempt` mechanic (full rolling procedure).
///
/// Returns `(success, roll, table_result, hd_affected, destroyed)` on success,
/// or `None` if the DSL backend is unavailable or not configured for TurnUndead.
///
/// `undead_hd` is the hit dice of the target monster (not rank — the mechanic
/// converts HD→rank internally via `undead_rank_from_hd`).
#[cfg(feature = "dsl-backend")]
pub fn try_dsl_turn_undead_attempt(
    cleric_level: u32,
    undead_hd: u32,
) -> Option<(bool, Option<i32>, TurnResult, u32, bool)> {
    if !crate::backend::is_dsl(crate::backend::MechanicGroup::TurnUndead) {
        return None;
    }
    dsl_gate::dsl_turn_undead_attempt(cleric_level, undead_hd)
}

// ── DSL gate helpers ──────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
mod dsl_gate {
    use super::TurnResult;
    use crate::bridge::handler::BridgeHandler;
    use crate::bridge::state::BridgeState;
    use ttrpg_ast::Name;
    use ttrpg_interp::value::Value;

    /// Evaluate the DSL `turn_undead_result` derive.
    pub fn dsl_turn_undead_result(cleric_level: u32, undead_rank: u32) -> Option<TurnResult> {
        let runtime = crate::backend::dsl()?;
        let state = BridgeState::new(vec![], vec![], vec![], 0, 0);
        let mut handler = BridgeHandler::new();

        let result = runtime
            .evaluate_derive(
                &state,
                &mut handler,
                "turn_undead_result",
                vec![
                    Value::Int(cleric_level as i64),
                    Value::Int(undead_rank as i64),
                ],
            )
            .ok()?;

        value_to_turn_result(result)
    }

    /// Evaluate the DSL `undead_rank_from_hd` derive.
    pub fn dsl_undead_rank_from_hd(hd: u32) -> Option<u32> {
        let runtime = crate::backend::dsl()?;
        let state = BridgeState::new(vec![], vec![], vec![], 0, 0);
        let mut handler = BridgeHandler::new();

        let result = runtime
            .evaluate_derive(
                &state,
                &mut handler,
                "undead_rank_from_hd",
                vec![Value::Int(hd as i64)],
            )
            .ok()?;

        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    /// Evaluate the DSL `turn_undead_attempt` mechanic.
    ///
    /// Returns `(success, roll, table_result, hd_affected, destroyed)`:
    /// - `success`: whether the turn succeeded
    /// - `roll`: `Some(n)` if a 2d6 was rolled, `None` for auto-turn/destroy
    /// - `table_result`: the table lookup result (for display)
    /// - `hd_affected`: HD of undead affected (0 on failure/impossible)
    /// - `destroyed`: true if undead are destroyed (vs. merely turned)
    pub fn dsl_turn_undead_attempt(
        cleric_level: u32,
        undead_hd: u32,
    ) -> Option<(bool, Option<i32>, TurnResult, u32, bool)> {
        let runtime = crate::backend::dsl()?;
        let state = BridgeState::new(vec![], vec![], vec![], 0, 0);
        let mut handler = BridgeHandler::new();

        let result = runtime
            .evaluate_mechanic(
                &state,
                &mut handler,
                "turn_undead_attempt",
                vec![
                    Value::Int(cleric_level as i64),
                    Value::Int(undead_hd as i64),
                ],
            )
            .ok()?;

        value_to_turn_attempt(result)
    }

    /// Convert a DSL `TurnAttempt` enum variant to the Rust turn outcome tuple.
    fn value_to_turn_attempt(
        value: Value,
    ) -> Option<(bool, Option<i32>, TurnResult, u32, bool)> {
        match value {
            Value::EnumVariant {
                variant, fields, ..
            } => match variant.as_str() {
                "turn_impossible" => Some((false, None, TurnResult::Impossible, 0, false)),
                "turn_failed" => {
                    let roll = fields.get(&Name::from("roll")).and_then(|v| {
                        if let Value::Int(n) = v { Some(*n as i32) } else { None }
                    })?;
                    let target = fields.get(&Name::from("target")).and_then(|v| {
                        if let Value::Int(n) = v { Some(*n as u32) } else { None }
                    })?;
                    Some((false, Some(roll), TurnResult::Roll(target), 0, false))
                }
                "turn_rolled" => {
                    let roll = fields.get(&Name::from("roll")).and_then(|v| {
                        if let Value::Int(n) = v { Some(*n as i32) } else { None }
                    })?;
                    let target = fields.get(&Name::from("target")).and_then(|v| {
                        if let Value::Int(n) = v { Some(*n as u32) } else { None }
                    })?;
                    let hd = fields.get(&Name::from("hd")).and_then(|v| {
                        if let Value::Int(n) = v { Some(*n as u32) } else { None }
                    })?;
                    Some((true, Some(roll), TurnResult::Roll(target), hd, false))
                }
                "turn_auto" => {
                    let hd = fields.get(&Name::from("hd")).and_then(|v| {
                        if let Value::Int(n) = v { Some(*n as u32) } else { None }
                    })?;
                    Some((true, None, TurnResult::Turned, hd, false))
                }
                "turn_destroy" => {
                    let hd = fields.get(&Name::from("hd")).and_then(|v| {
                        if let Value::Int(n) = v { Some(*n as u32) } else { None }
                    })?;
                    Some((true, None, TurnResult::Destroyed, hd, true))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Convert a DSL `TurnResult` enum variant to the Rust `TurnResult`.
    fn value_to_turn_result(value: Value) -> Option<TurnResult> {
        match value {
            Value::EnumVariant {
                variant, fields, ..
            } => match variant.as_str() {
                "Impossible" => Some(TurnResult::Impossible),
                "Roll" => {
                    let target = fields.get(&Name::from("target"))?;
                    if let Value::Int(v) = target {
                        Some(TurnResult::Roll(*v as u32))
                    } else {
                        None
                    }
                }
                "Turned" => Some(TurnResult::Turned),
                "Destroyed" => Some(TurnResult::Destroyed),
                _ => None,
            },
            _ => None,
        }
    }
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

// ── DSL backend tests ─────────────────────────────────────────
//
// These tests verify that the DSL derives produce identical results
// to the native Rust implementation. Run with:
//   OSR_BACKEND_TURN_UNDEAD=dsl cargo test --features dsl-backend

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_tests {
    use super::dsl_gate;
    use super::*;

    // --- DSL turn_undead_result matches native for all rank/level combos ---

    #[test]
    fn dsl_turn_undead_result_matches_native() {
        for level in 1..=14 {
            for rank in 1..=9 {
                let native = turn_undead_result_native(level, rank);
                let dsl = dsl_gate::dsl_turn_undead_result(level, rank)
                    .unwrap_or_else(|| panic!(
                        "DSL returned None for level={}, rank={}", level, rank
                    ));
                assert_eq!(
                    native, dsl,
                    "Mismatch at level={}, rank={}: native={:?}, dsl={:?}",
                    level, rank, native, dsl
                );
            }
        }
    }

    // --- DSL undead_rank_from_hd matches native ---

    #[test]
    fn dsl_undead_rank_matches_native() {
        for hd in 0..=15 {
            let native = hd.clamp(1, 9);
            let dsl = dsl_gate::dsl_undead_rank_from_hd(hd)
                .unwrap_or_else(|| panic!("DSL returned None for hd={}", hd));
            assert_eq!(
                native, dsl,
                "Mismatch at hd={}: native={}, dsl={}",
                hd, native, dsl
            );
        }
    }

    // --- Spot-check specific DSL results ---

    #[test]
    fn dsl_skeleton_level_1() {
        let result = dsl_gate::dsl_turn_undead_result(1, 1).unwrap();
        assert_eq!(result, TurnResult::Roll(7));
    }

    #[test]
    fn dsl_vampire_level_11_destroys() {
        let result = dsl_gate::dsl_turn_undead_result(11, 8).unwrap();
        assert_eq!(result, TurnResult::Destroyed);
    }

    #[test]
    fn dsl_wight_level_1_impossible() {
        let result = dsl_gate::dsl_turn_undead_result(1, 4).unwrap();
        assert_eq!(result, TurnResult::Impossible);
    }

    #[test]
    fn dsl_skeleton_level_2_auto_turn() {
        let result = dsl_gate::dsl_turn_undead_result(2, 1).unwrap();
        assert_eq!(result, TurnResult::Turned);
    }

    // --- DSL turn_undead_attempt mechanic ---
    //
    // These tests verify the new orchestration mechanic that combines
    // rank lookup, table lookup, and conditional dice rolling.

    #[test]
    fn dsl_attempt_impossible_returns_failure() {
        // Wight (HD=4, rank=4) vs level 1 cleric: diff=-3 → Impossible
        let result = dsl_gate::dsl_turn_undead_attempt(1, 4).unwrap();
        let (success, roll, table, hd, destroyed) = result;
        assert!(!success);
        assert_eq!(roll, None);
        assert_eq!(table, TurnResult::Impossible);
        assert_eq!(hd, 0);
        assert!(!destroyed);
    }

    #[test]
    fn dsl_attempt_auto_turn_sets_hd_affected() {
        // Skeleton (HD=1, rank=1) vs level 2 cleric: diff=1 → Turned (auto)
        let result = dsl_gate::dsl_turn_undead_attempt(2, 1).unwrap();
        let (success, roll, table, hd, destroyed) = result;
        assert!(success);
        assert_eq!(roll, None);
        assert_eq!(table, TurnResult::Turned);
        assert!(hd >= 2 && hd <= 12, "hd={} should be 2d6 range 2-12", hd);
        assert!(!destroyed);
    }

    #[test]
    fn dsl_attempt_auto_destroy_sets_destroyed_flag() {
        // Skeleton (HD=1, rank=1) vs level 4 cleric: diff=3 → Destroyed (auto)
        let result = dsl_gate::dsl_turn_undead_attempt(4, 1).unwrap();
        let (success, roll, table, hd, destroyed) = result;
        assert!(success);
        assert_eq!(roll, None);
        assert_eq!(table, TurnResult::Destroyed);
        assert!(hd >= 2 && hd <= 12, "hd={} should be 2d6 range 2-12", hd);
        assert!(destroyed);
    }

    #[test]
    fn dsl_attempt_roll_case_returns_roll_value() {
        // Ghoul (HD=3, rank=3) vs level 1 cleric: diff=-2 → Roll(11)
        // We can't control the dice, but we can verify the structure.
        let result = dsl_gate::dsl_turn_undead_attempt(1, 3).unwrap();
        let (_, roll, table, _, _) = result;
        assert_eq!(table, TurnResult::Roll(11));
        let r = roll.expect("roll case must have a roll value");
        assert!(r >= 2 && r <= 12, "roll={} should be 2d6 range 2-12", r);
    }

    #[test]
    fn dsl_attempt_roll_success_has_hd() {
        // Use level=9 vs skeleton (HD=1, rank=1): diff=8 → Destroyed.
        // Use level=1 vs skeleton (HD=1, rank=1): diff=0 → Roll(7).
        // With 100 iterations, we'll see at least some successes (E[roll]=7, ~58% succeed).
        let mut saw_success_with_hd = false;
        for _ in 0..100 {
            let result = dsl_gate::dsl_turn_undead_attempt(1, 1).unwrap();
            let (success, roll, table, hd, destroyed) = result;
            assert_eq!(table, TurnResult::Roll(7));
            assert!(!destroyed);
            if success {
                let r = roll.unwrap();
                assert!(r >= 7, "success roll must be >= 7, got {}", r);
                assert!(hd >= 2 && hd <= 12, "hd={} should be 2d6 on success", hd);
                saw_success_with_hd = true;
            } else {
                let r = roll.unwrap();
                assert!(r < 7, "failure roll must be < 7, got {}", r);
                assert_eq!(hd, 0);
            }
        }
        assert!(saw_success_with_hd, "expected at least one success in 100 trials");
    }
}
