//! Thief skill tables per OSE Rules Tome.
//! Most skills are d% (percentile), Hear Noise is d6.
//!
//! When the `dsl-backend` feature is enabled and `OSR_BACKEND_THIEF=dsl`,
//! derive functions delegate to DSL evaluations. Table lookups (skill_chance)
//! and the check_skill function remain native; the DSL mechanic handles
//! the full check flow at the engine action level.

/// Thief skills available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThiefSkill {
    ClimbWalls,
    FindTraps,
    HearNoise,
    HideShadows,
    MoveSilently,
    OpenLocks,
    PickPockets,
    ReadLanguages,
}

impl ThiefSkill {
    pub const ALL: [ThiefSkill; 8] = [
        ThiefSkill::ClimbWalls,
        ThiefSkill::FindTraps,
        ThiefSkill::HearNoise,
        ThiefSkill::HideShadows,
        ThiefSkill::MoveSilently,
        ThiefSkill::OpenLocks,
        ThiefSkill::PickPockets,
        ThiefSkill::ReadLanguages,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ThiefSkill::ClimbWalls => "Climb Walls",
            ThiefSkill::FindTraps => "Find Traps",
            ThiefSkill::HearNoise => "Hear Noise",
            ThiefSkill::HideShadows => "Hide in Shadows",
            ThiefSkill::MoveSilently => "Move Silently",
            ThiefSkill::OpenLocks => "Open Locks",
            ThiefSkill::PickPockets => "Pick Pockets",
            ThiefSkill::ReadLanguages => "Read Languages",
        }
    }

    /// Whether this skill uses d6 (true) or d% (false).
    pub fn is_d6(self) -> bool {
        matches!(self, ThiefSkill::HearNoise)
    }
}

/// Get the thief skill chance at a given level.
/// For d% skills: returns the percentage chance (1-99).
/// For Hear Noise: returns the X-in-6 chance.
pub fn skill_chance(skill: ThiefSkill, level: u32) -> u32 {
    let lvl = level.clamp(1, 14) as usize - 1;
    match skill {
        //                       1    2    3    4    5    6    7    8    9   10   11   12   13   14
        ThiefSkill::ClimbWalls => [
            87,  88,  89,  90,  91,  92,  93,  94,  95,  96,  97,  98,  99,  99
        ][lvl],
        ThiefSkill::FindTraps => [
            10,  15,  20,  25,  30,  35,  40,  45,  50,  55,  60,  65,  70,  75
        ][lvl],
        ThiefSkill::HearNoise => [
            2,   2,   3,   3,   3,   3,   4,   4,   4,   4,   5,   5,   5,   5
        ][lvl],
        ThiefSkill::HideShadows => [
            10,  15,  20,  25,  30,  35,  40,  45,  50,  55,  60,  65,  70,  75
        ][lvl],
        ThiefSkill::MoveSilently => [
            20,  25,  30,  35,  40,  45,  50,  55,  60,  65,  70,  75,  80,  85
        ][lvl],
        ThiefSkill::OpenLocks => [
            15,  20,  25,  30,  35,  40,  45,  50,  55,  60,  65,  70,  75,  80
        ][lvl],
        ThiefSkill::PickPockets => [
            20,  25,  30,  35,  40,  45,  50,  55,  60,  65,  70,  75,  80,  85
        ][lvl],
        ThiefSkill::ReadLanguages => [
            0,   0,   0,   20,  25,  30,  35,  40,  45,  50,  55,  60,  65,  70
        ][lvl],
    }
}

/// Result of a thief skill check.
#[derive(Debug, Clone, PartialEq)]
pub struct ThiefSkillResult {
    pub skill: ThiefSkill,
    pub target: u32,
    pub roll: u32,
    pub success: bool,
}

/// Perform a thief skill check with a given roll.
/// For d% skills: roll is 1-100, success if roll <= target.
/// For Hear Noise (d6): roll is 1-6, success if roll <= target.
pub fn check_skill(skill: ThiefSkill, level: u32, roll: u32) -> ThiefSkillResult {
    let target = skill_chance(skill, level);
    let success = roll <= target;
    ThiefSkillResult { skill, target, roll, success }
}

/// Backstab damage multiplier by thief level per OSE Rules Tome.
/// Level 1-4: x2, Level 5-8: x3, Level 9-12: x4, Level 13+: x5
pub fn backstab_multiplier(level: u32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    {
        if crate::backend::is_dsl(crate::backend::MechanicGroup::Thief) {
            if let Some(val) = dsl_gate::dsl_backstab_multiplier(level) {
                return val;
            }
        }
    }
    #[cfg(feature = "legacy-native")]
    return match level {
        1..=4 => 2,
        5..=8 => 3,
        9..=12 => 4,
        _ => 5,
    };
    #[cfg(not(feature = "legacy-native"))]
    panic!("Native fallback unavailable: enable the 'legacy-native' feature");
}

/// Backstab attack bonus: +4 to hit.
pub const BACKSTAB_ATTACK_BONUS: i32 = 4;

/// Whether a class has thief skills. Uses ClassDef capability field via registry.
pub fn has_thief_skills(id: &crate::rules::class::ClassId) -> bool {
    crate::rules::class::class_registry()
        .get_by_id(id)
        .is_some_and(|d| d.has_thief_skills)
}

/// Whether a class can backstab. Uses ClassDef capability field via registry.
pub fn can_backstab(id: &crate::rules::class::ClassId) -> bool {
    crate::rules::class::class_registry()
        .get_by_id(id)
        .is_some_and(|d| d.can_backstab)
}

// ── DSL gate helpers ──────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
mod dsl_gate {
    use std::collections::BTreeMap;

    use ttrpg_ast::Name;
    use ttrpg_interp::effect::{Effect, EffectHandler, Response};
    use ttrpg_interp::state::{ActiveCondition, EntityRef, StateProvider};
    use ttrpg_interp::value::Value;

    use crate::backend;

    /// Null state provider for pure derive evaluation (no entity access needed).
    struct NullState;

    impl StateProvider for NullState {
        fn read_field(&self, _: &EntityRef, _: &str) -> Option<Value> { None }
        fn read_conditions(&self, _: &EntityRef) -> Option<Vec<ActiveCondition>> { None }
        fn read_turn_budget(&self, _: &EntityRef) -> Option<BTreeMap<Name, Value>> { None }
        fn read_enabled_options(&self) -> Vec<Name> { Vec::new() }
        fn position_eq(&self, _: &Value, _: &Value) -> bool { false }
        fn distance(&self, _: &Value, _: &Value) -> Option<i64> { None }
    }

    /// Null effect handler for pure derive evaluation (no effects fired).
    struct NullHandler;

    impl EffectHandler for NullHandler {
        fn handle(&mut self, _: Effect) -> Response { Response::Acknowledged }
    }

    fn class_to_dsl(id: &crate::rules::class::ClassId) -> Value {
        use crate::rules::class::canonical_to_dsl_variant;
        Value::EnumVariant {
            enum_name: "Class".into(),
            variant: Name::from(canonical_to_dsl_variant(id.as_str())),
            fields: BTreeMap::new(),
        }
    }

    pub fn dsl_backstab_multiplier(level: u32) -> Option<u32> {
        let rt = backend::dsl()?;
        let args = vec![Value::Int(level as i64)];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "backstab_multiplier", args).ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_has_thief_skills(id: &crate::rules::class::ClassId) -> Option<bool> {
        let rt = backend::dsl()?;
        let args = vec![class_to_dsl(id)];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "has_thief_skills", args).ok()?;
        match result {
            Value::Bool(v) => Some(v),
            _ => None,
        }
    }

    pub fn dsl_can_backstab(id: &crate::rules::class::ClassId) -> Option<bool> {
        let rt = backend::dsl()?;
        let args = vec![class_to_dsl(id)];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "can_backstab", args).ok()?;
        match result {
            Value::Bool(v) => Some(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn climb_walls_progression() {
        assert_eq!(skill_chance(ThiefSkill::ClimbWalls, 1), 87);
        assert_eq!(skill_chance(ThiefSkill::ClimbWalls, 7), 93);
        assert_eq!(skill_chance(ThiefSkill::ClimbWalls, 14), 99);
    }

    #[test]
    fn find_traps_progression() {
        assert_eq!(skill_chance(ThiefSkill::FindTraps, 1), 10);
        assert_eq!(skill_chance(ThiefSkill::FindTraps, 5), 30);
        assert_eq!(skill_chance(ThiefSkill::FindTraps, 14), 75);
    }

    #[test]
    fn hear_noise_d6() {
        assert!(ThiefSkill::HearNoise.is_d6());
        assert!(!ThiefSkill::ClimbWalls.is_d6());
        assert_eq!(skill_chance(ThiefSkill::HearNoise, 1), 2);
        assert_eq!(skill_chance(ThiefSkill::HearNoise, 4), 3);
        assert_eq!(skill_chance(ThiefSkill::HearNoise, 10), 4);
        assert_eq!(skill_chance(ThiefSkill::HearNoise, 14), 5);
    }

    #[test]
    fn open_locks_progression() {
        assert_eq!(skill_chance(ThiefSkill::OpenLocks, 1), 15);
        assert_eq!(skill_chance(ThiefSkill::OpenLocks, 14), 80);
    }

    #[test]
    fn read_languages_unavailable_low_level() {
        assert_eq!(skill_chance(ThiefSkill::ReadLanguages, 1), 0);
        assert_eq!(skill_chance(ThiefSkill::ReadLanguages, 3), 0);
        assert_eq!(skill_chance(ThiefSkill::ReadLanguages, 4), 20);
    }

    #[test]
    fn skill_check_success() {
        let result = check_skill(ThiefSkill::OpenLocks, 1, 10); // target 15, roll 10
        assert!(result.success);
        assert_eq!(result.target, 15);
    }

    #[test]
    fn skill_check_failure() {
        let result = check_skill(ThiefSkill::OpenLocks, 1, 20); // target 15, roll 20
        assert!(!result.success);
    }

    #[test]
    fn skill_check_exact() {
        let result = check_skill(ThiefSkill::OpenLocks, 1, 15); // target 15, roll 15
        assert!(result.success);
    }

    #[test]
    fn backstab_multiplier_level_1() {
        assert_eq!(backstab_multiplier(1), 2);
    }

    #[test]
    fn backstab_multiplier_level_5() {
        assert_eq!(backstab_multiplier(5), 3);
    }

    #[test]
    fn backstab_multiplier_level_9() {
        assert_eq!(backstab_multiplier(9), 4);
    }

    #[test]
    fn backstab_multiplier_level_13() {
        assert_eq!(backstab_multiplier(13), 5);
    }

    #[test]
    fn thief_has_skills() {
        use crate::rules::class::ClassId;
        assert!(has_thief_skills(&ClassId::new("Thief")));
        assert!(has_thief_skills(&ClassId::new("Assassin")));
        assert!(has_thief_skills(&ClassId::new("Acrobat")));
        assert!(!has_thief_skills(&ClassId::new("Fighter")));
        assert!(!has_thief_skills(&ClassId::new("Cleric")));
    }

    #[test]
    fn thief_can_backstab() {
        use crate::rules::class::ClassId;
        assert!(can_backstab(&ClassId::new("Thief")));
        assert!(can_backstab(&ClassId::new("Assassin")));
        assert!(!can_backstab(&ClassId::new("Acrobat")));
        assert!(!can_backstab(&ClassId::new("Fighter")));
    }

    #[test]
    fn level_clamping() {
        // Level 0 should clamp to 1
        assert_eq!(skill_chance(ThiefSkill::ClimbWalls, 0), 87);
        // Level 20 should clamp to 14
        assert_eq!(skill_chance(ThiefSkill::ClimbWalls, 20), 99);
    }

    #[test]
    fn hear_noise_d6_check() {
        let result = check_skill(ThiefSkill::HearNoise, 1, 2); // target 2, roll 2
        assert!(result.success);
        let result = check_skill(ThiefSkill::HearNoise, 1, 3); // target 2, roll 3
        assert!(!result.success);
    }
}
