//! Ability score modifier lookups per OSE Reference Booklet.
//!
//! When `dsl-backend` is enabled and `OSR_BACKEND_ABILITY=dsl`, these functions
//! delegate to the DSL interpreter instead of the hardcoded tables below.

// ── DSL evaluation helper (pure derives) ────────────────────────

#[cfg(feature = "dsl-backend")]
mod dsl_eval {
    use std::collections::BTreeMap;

    use ttrpg_interp::state::{ActiveCondition, EntityRef, StateProvider};
    use ttrpg_interp::value::Value;

    use crate::bridge::handler::BridgeHandler;

    /// Minimal state provider for pure derives that don't access entity state.
    struct NullState;

    impl StateProvider for NullState {
        fn read_field(&self, _: &EntityRef, _: &str) -> Option<Value> {
            None
        }
        fn read_conditions(&self, _: &EntityRef) -> Option<Vec<ActiveCondition>> {
            None
        }
        fn read_turn_budget(
            &self,
            _: &EntityRef,
        ) -> Option<BTreeMap<ttrpg_ast::Name, Value>> {
            None
        }
        fn read_enabled_options(&self) -> Vec<ttrpg_ast::Name> {
            Vec::new()
        }
        fn position_eq(&self, _: &Value, _: &Value) -> bool {
            false
        }
        fn distance(&self, _: &Value, _: &Value) -> Option<i64> {
            None
        }
        fn entity_type_name(&self, _: &EntityRef) -> Option<ttrpg_ast::Name> {
            None
        }
    }

    /// Evaluate a DSL ability derive returning i64 (caller casts).
    fn eval_derive(name: &str, score: i32) -> Option<i64> {
        let runtime = crate::backend::dsl()?;
        let state = NullState;
        let mut handler = BridgeHandler::new();
        let result = runtime
            .evaluate_derive(&state, &mut handler, name, vec![Value::Int(score as i64)])
            .ok()?;
        match result {
            Value::Int(v) => Some(v),
            _ => None,
        }
    }

    /// Evaluate a DSL ability derive, returning i32.
    pub(super) fn i32(name: &str, score: i32) -> Option<i32> {
        eval_derive(name, score).map(|v| v as i32)
    }

    /// Evaluate a DSL ability derive, returning u32.
    pub(super) fn u32(name: &str, score: i32) -> Option<u32> {
        eval_derive(name, score).map(|v| v as u32)
    }
}

#[cfg(feature = "dsl-backend")]
use crate::backend::{is_dsl, MechanicGroup};

/// Returns true when the Ability mechanic group is using the DSL backend.
#[cfg(feature = "dsl-backend")]
#[inline]
fn use_dsl() -> bool {
    is_dsl(MechanicGroup::Ability)
}

// ── Public API (gated) ──────────────────────────────────────────

/// STR melee modifier (attack and damage).
pub fn str_melee_mod(score: i32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::i32("str_melee_mod", score) {
            return v;
        }
    }
    match score {
        ..=3 => -3,
        4..=5 => -2,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 2,
        18.. => 3,
    }
}

/// STR open doors chance (X-in-6).
pub fn str_open_doors(score: i32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::u32("str_open_doors", score) {
            return v;
        }
    }
    match score {
        ..=8 => 1,
        9..=12 => 2,
        13..=15 => 3,
        16..=17 => 4,
        18.. => 5,
    }
}

/// DEX modifier (AC and missile attacks).
///
/// Returns a signed modifier: +3 for DEX 18, -3 for DEX 3.
/// For AC (descending): subtract this from AC, so positive = better (lower) AC.
/// For missile attacks: add directly as attack bonus.
///
/// Note: OSE publishes the AC column with inverted signs (-3 for DEX 18)
/// because descending AC treats negative as better. We use a single positive
/// convention here and subtract in `calculate_ac`.
pub fn dex_mod(score: i32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::i32("dex_mod", score) {
            return v;
        }
    }
    match score {
        ..=3 => -3,
        4..=5 => -2,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 2,
        18.. => 3,
    }
}

/// Deprecated alias for [`dex_mod`]. Use `dex_mod` instead.
#[inline]
pub fn dex_ac_mod(score: i32) -> i32 { dex_mod(score) }

/// Deprecated alias for [`dex_mod`]. Use `dex_mod` instead.
#[inline]
pub fn dex_missile_mod(score: i32) -> i32 { dex_mod(score) }

/// DEX initiative modifier (optional individual initiative rule).
pub fn dex_init_mod(score: i32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::i32("dex_init_mod", score) {
            return v;
        }
    }
    match score {
        ..=3 => -2,
        4..=5 => -1,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 1,
        18.. => 2,
    }
}

/// CON hit point modifier (per HD).
pub fn con_hp_mod(score: i32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::i32("con_hp_mod", score) {
            return v;
        }
    }
    match score {
        ..=3 => -3,
        4..=5 => -2,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 2,
        18.. => 3,
    }
}

/// INT additional languages count.
pub fn int_extra_languages(score: i32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::u32("int_extra_languages", score) {
            return v;
        }
    }
    match score {
        ..=12 => 0,
        13..=15 => 1,
        16..=17 => 2,
        18.. => 3,
    }
}

/// INT literacy level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Literacy {
    Illiterate,
    Basic,
    Literate,
}

/// INT literacy (no DSL equivalent — always native).
pub fn int_literacy(score: i32) -> Literacy {
    match score {
        3..=5 => Literacy::Illiterate,
        6..=8 => Literacy::Basic,
        _ => Literacy::Literate,
    }
}

/// WIS magic save modifier.
pub fn wis_magic_save_mod(score: i32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::i32("wis_magic_save_mod", score) {
            return v;
        }
    }
    match score {
        ..=3 => -3,
        4..=5 => -2,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 2,
        18.. => 3,
    }
}

/// CHA NPC reaction modifier.
pub fn cha_reaction_mod(score: i32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::i32("cha_reaction_mod", score) {
            return v;
        }
    }
    match score {
        ..=3 => -2,
        4..=5 => -1,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 1,
        18.. => 2,
    }
}

/// CHA max retainers.
pub fn cha_max_retainers(score: i32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::u32("cha_max_retainers", score) {
            return v;
        }
    }
    match score {
        ..=3 => 1,
        4..=5 => 2,
        6..=8 => 3,
        9..=12 => 4,
        13..=15 => 5,
        16..=17 => 6,
        18.. => 7,
    }
}

/// CHA retainer loyalty (base).
pub fn cha_loyalty(score: i32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::u32("cha_loyalty", score) {
            return v;
        }
    }
    match score {
        ..=3 => 4,
        4..=5 => 5,
        6..=8 => 6,
        9..=12 => 7,
        13..=15 => 8,
        16..=17 => 9,
        18.. => 10,
    }
}

/// Prime requisite XP modifier.
pub fn prime_req_xp_mod(score: i32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if use_dsl() {
        if let Some(v) = dsl_eval::i32("prime_req_xp_mod", score) {
            return v;
        }
    }
    match score {
        ..=5 => -20,
        6..=8 => -10,
        9..=12 => 0,
        13..=15 => 5,
        16.. => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_modifiers() {
        assert_eq!(str_melee_mod(3), -3);
        assert_eq!(str_melee_mod(10), 0);
        assert_eq!(str_melee_mod(18), 3);
        assert_eq!(str_open_doors(3), 1);
        assert_eq!(str_open_doors(9), 2);
        assert_eq!(str_open_doors(18), 5);
    }

    #[test]
    fn dex_modifiers() {
        assert_eq!(dex_ac_mod(3), -3);
        assert_eq!(dex_ac_mod(10), 0);
        assert_eq!(dex_ac_mod(18), 3);
        assert_eq!(dex_missile_mod(4), -2);
        assert_eq!(dex_init_mod(3), -2);
        assert_eq!(dex_init_mod(18), 2);
    }

    #[test]
    fn con_modifiers() {
        assert_eq!(con_hp_mod(3), -3);
        assert_eq!(con_hp_mod(10), 0);
        assert_eq!(con_hp_mod(18), 3);
    }

    #[test]
    fn int_modifiers() {
        assert_eq!(int_extra_languages(3), 0);
        assert_eq!(int_extra_languages(13), 1);
        assert_eq!(int_extra_languages(18), 3);
        assert_eq!(int_literacy(3), Literacy::Illiterate);
        assert_eq!(int_literacy(7), Literacy::Basic);
        assert_eq!(int_literacy(10), Literacy::Literate);
    }

    #[test]
    fn wis_modifiers() {
        assert_eq!(wis_magic_save_mod(3), -3);
        assert_eq!(wis_magic_save_mod(12), 0);
        assert_eq!(wis_magic_save_mod(18), 3);
    }

    #[test]
    fn cha_modifiers() {
        assert_eq!(cha_reaction_mod(3), -2);
        assert_eq!(cha_reaction_mod(10), 0);
        assert_eq!(cha_reaction_mod(18), 2);
        assert_eq!(cha_max_retainers(3), 1);
        assert_eq!(cha_max_retainers(18), 7);
        assert_eq!(cha_loyalty(3), 4);
        assert_eq!(cha_loyalty(18), 10);
    }

    #[test]
    fn prime_requisite() {
        assert_eq!(prime_req_xp_mod(3), -20);
        assert_eq!(prime_req_xp_mod(7), -10);
        assert_eq!(prime_req_xp_mod(10), 0);
        assert_eq!(prime_req_xp_mod(14), 5);
        assert_eq!(prime_req_xp_mod(18), 10);
    }
}

// ── DSL parity tests ────────────────────────────────────────────

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_tests {
    use super::*;

    /// Verify every DSL ability derive matches the native table for scores 3–18.
    #[test]
    fn dsl_matches_native_for_all_scores() {
        // These are the native expected values for scores 3..=18.
        for score in 3..=18i32 {
            let dsl_val = dsl_eval::i32("str_melee_mod", score)
                .unwrap_or_else(|| panic!("DSL str_melee_mod({}) failed", score));
            assert_eq!(
                dsl_val,
                str_melee_mod_native(score),
                "str_melee_mod mismatch at score {}",
                score
            );

            let dsl_val = dsl_eval::u32("str_open_doors", score)
                .unwrap_or_else(|| panic!("DSL str_open_doors({}) failed", score));
            assert_eq!(
                dsl_val,
                str_open_doors_native(score),
                "str_open_doors mismatch at score {}",
                score
            );

            let dsl_val = dsl_eval::i32("dex_mod", score)
                .unwrap_or_else(|| panic!("DSL dex_mod({}) failed", score));
            assert_eq!(
                dsl_val,
                dex_mod_native(score),
                "dex_mod mismatch at score {}",
                score
            );

            let dsl_val = dsl_eval::i32("dex_init_mod", score)
                .unwrap_or_else(|| panic!("DSL dex_init_mod({}) failed", score));
            assert_eq!(
                dsl_val,
                dex_init_mod_native(score),
                "dex_init_mod mismatch at score {}",
                score
            );

            let dsl_val = dsl_eval::i32("con_hp_mod", score)
                .unwrap_or_else(|| panic!("DSL con_hp_mod({}) failed", score));
            assert_eq!(
                dsl_val,
                con_hp_mod_native(score),
                "con_hp_mod mismatch at score {}",
                score
            );

            let dsl_val = dsl_eval::u32("int_extra_languages", score)
                .unwrap_or_else(|| panic!("DSL int_extra_languages({}) failed", score));
            assert_eq!(
                dsl_val,
                int_extra_languages_native(score),
                "int_extra_languages mismatch at score {}",
                score
            );

            let dsl_val = dsl_eval::i32("wis_magic_save_mod", score)
                .unwrap_or_else(|| panic!("DSL wis_magic_save_mod({}) failed", score));
            assert_eq!(
                dsl_val,
                wis_magic_save_mod_native(score),
                "wis_magic_save_mod mismatch at score {}",
                score
            );

            let dsl_val = dsl_eval::i32("cha_reaction_mod", score)
                .unwrap_or_else(|| panic!("DSL cha_reaction_mod({}) failed", score));
            assert_eq!(
                dsl_val,
                cha_reaction_mod_native(score),
                "cha_reaction_mod mismatch at score {}",
                score
            );

            let dsl_val = dsl_eval::u32("cha_max_retainers", score)
                .unwrap_or_else(|| panic!("DSL cha_max_retainers({}) failed", score));
            assert_eq!(
                dsl_val,
                cha_max_retainers_native(score),
                "cha_max_retainers mismatch at score {}",
                score
            );

            let dsl_val = dsl_eval::u32("cha_loyalty", score)
                .unwrap_or_else(|| panic!("DSL cha_loyalty({}) failed", score));
            assert_eq!(
                dsl_val,
                cha_loyalty_native(score),
                "cha_loyalty mismatch at score {}",
                score
            );

            let dsl_val = dsl_eval::i32("prime_req_xp_mod", score)
                .unwrap_or_else(|| panic!("DSL prime_req_xp_mod({}) failed", score));
            assert_eq!(
                dsl_val,
                prime_req_xp_mod_native(score),
                "prime_req_xp_mod mismatch at score {}",
                score
            );
        }
    }

    // ── Native-only copies for parity comparison ────────────────

    fn str_melee_mod_native(score: i32) -> i32 {
        match score {
            3 => -3, 4..=5 => -2, 6..=8 => -1, 9..=12 => 0,
            13..=15 => 1, 16..=17 => 2, 18 => 3, _ => 0,
        }
    }

    fn str_open_doors_native(score: i32) -> u32 {
        match score {
            3..=8 => 1, 9..=12 => 2, 13..=15 => 3, 16..=17 => 4, 18 => 5, _ => 1,
        }
    }

    fn dex_mod_native(score: i32) -> i32 {
        match score {
            3 => -3, 4..=5 => -2, 6..=8 => -1, 9..=12 => 0,
            13..=15 => 1, 16..=17 => 2, 18 => 3, _ => 0,
        }
    }

    fn dex_init_mod_native(score: i32) -> i32 {
        match score {
            3 => -2, 4..=5 => -1, 6..=8 => -1, 9..=12 => 0,
            13..=15 => 1, 16..=17 => 1, 18 => 2, _ => 0,
        }
    }

    fn con_hp_mod_native(score: i32) -> i32 {
        match score {
            3 => -3, 4..=5 => -2, 6..=8 => -1, 9..=12 => 0,
            13..=15 => 1, 16..=17 => 2, 18 => 3, _ => 0,
        }
    }

    fn int_extra_languages_native(score: i32) -> u32 {
        match score {
            3..=12 => 0, 13..=15 => 1, 16..=17 => 2, 18 => 3, _ => 0,
        }
    }

    fn wis_magic_save_mod_native(score: i32) -> i32 {
        match score {
            3 => -3, 4..=5 => -2, 6..=8 => -1, 9..=12 => 0,
            13..=15 => 1, 16..=17 => 2, 18 => 3, _ => 0,
        }
    }

    fn cha_reaction_mod_native(score: i32) -> i32 {
        match score {
            3 => -2, 4..=5 => -1, 6..=8 => -1, 9..=12 => 0,
            13..=15 => 1, 16..=17 => 1, 18 => 2, _ => 0,
        }
    }

    fn cha_max_retainers_native(score: i32) -> u32 {
        match score {
            3 => 1, 4..=5 => 2, 6..=8 => 3, 9..=12 => 4,
            13..=15 => 5, 16..=17 => 6, 18 => 7, _ => 4,
        }
    }

    fn cha_loyalty_native(score: i32) -> u32 {
        match score {
            3 => 4, 4..=5 => 5, 6..=8 => 6, 9..=12 => 7,
            13..=15 => 8, 16..=17 => 9, 18 => 10, _ => 7,
        }
    }

    fn prime_req_xp_mod_native(score: i32) -> i32 {
        match score {
            3..=5 => -20, 6..=8 => -10, 9..=12 => 0,
            13..=15 => 5, 16..=18 => 10, _ => 0,
        }
    }
}
