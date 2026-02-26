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
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Morale) {
        if let Some(result) = check_morale_dsl(morale_score) {
            combat.log_event(result.to_string());
            return result;
        }
        // DSL evaluation failed — fall through to native
    }

    #[cfg(feature = "legacy-native")]
    return check_morale_with(combat, morale_score, &mut rand::thread_rng());
    #[cfg(not(feature = "legacy-native"))]
    panic!("Native fallback unavailable: enable the 'legacy-native' feature");
}

#[cfg(feature = "legacy-native")]
pub fn check_morale_with<R: Rng>(combat: &mut CombatState, morale_score: u32, rng: &mut R) -> MoraleResult {
    let d1 = rng.gen_range(1..=6i32);
    let d2 = rng.gen_range(1..=6i32);
    let roll = d1 + d2;
    let passed = roll <= morale_score as i32;

    let result = MoraleResult { roll, morale_score, passed };
    combat.log_event(result.to_string());
    result
}

// ── DSL backend ──────────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
fn check_morale_dsl(morale_score: u32) -> Option<MoraleResult> {
    use std::collections::BTreeMap;

    use ttrpg_ast::Name;
    use ttrpg_interp::effect::{Effect, EffectHandler, Response};
    use ttrpg_interp::state::{ActiveCondition, EntityRef, StateProvider};
    use ttrpg_interp::value::Value;

    use crate::bridge::handler::BridgeHandler;

    // Morale doesn't read entity state — provide a null provider.
    struct NullState;
    impl StateProvider for NullState {
        fn read_field(&self, _: &EntityRef, _: &str) -> Option<Value> { None }
        fn read_conditions(&self, _: &EntityRef) -> Option<Vec<ActiveCondition>> { None }
        fn read_turn_budget(&self, _: &EntityRef) -> Option<BTreeMap<Name, Value>> { None }
        fn read_enabled_options(&self) -> Vec<Name> { Vec::new() }
        fn position_eq(&self, _: &Value, _: &Value) -> bool { false }
        fn distance(&self, _: &Value, _: &Value) -> Option<i64> { None }
    }

    // Wraps BridgeHandler to capture the 2d6 roll total.
    struct MoraleHandler {
        inner: BridgeHandler,
        roll_total: Option<i64>,
    }

    impl EffectHandler for MoraleHandler {
        fn handle(&mut self, effect: Effect) -> Response {
            let response = self.inner.handle(effect);
            if let Response::Rolled(ref result) = response {
                self.roll_total = Some(result.total);
            }
            response
        }
    }

    let runtime = crate::backend::dsl()?;
    let mut handler = MoraleHandler {
        inner: BridgeHandler::new(),
        roll_total: None,
    };

    let result = runtime
        .evaluate_mechanic(
            &NullState,
            &mut handler,
            "morale_check",
            vec![Value::Int(morale_score as i64)],
        )
        .ok()?;

    let passed = match &result {
        Value::EnumVariant { variant, .. } => variant.as_str() == "morale_hold",
        _ => return None,
    };

    let roll = handler.roll_total.unwrap_or(0) as i32;
    Some(MoraleResult { roll, morale_score, passed })
}

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_tests {
    use super::*;

    #[test]
    fn dsl_morale_check_returns_valid_result() {
        let result = check_morale_dsl(7).expect("DSL morale_check should succeed");
        assert_eq!(result.morale_score, 7);
        assert!(result.roll >= 2 && result.roll <= 12, "2d6 must be 2..=12, got {}", result.roll);
        assert_eq!(result.passed, result.roll <= 7);
    }

    #[test]
    fn dsl_morale_check_bounds() {
        for _ in 0..100 {
            let result = check_morale_dsl(7).expect("DSL morale_check should succeed");
            assert!(result.roll >= 2 && result.roll <= 12);
        }
    }

    #[test]
    fn dsl_morale_score_12_always_holds() {
        for _ in 0..50 {
            let result = check_morale_dsl(12).expect("DSL morale_check should succeed");
            assert!(result.passed, "morale 12 should always hold, rolled {}", result.roll);
        }
    }

    #[test]
    fn dsl_morale_score_1_always_flees() {
        for _ in 0..50 {
            let result = check_morale_dsl(1).expect("DSL morale_check should succeed");
            assert!(!result.passed, "morale 1 should always flee, rolled {}", result.roll);
        }
    }
}
