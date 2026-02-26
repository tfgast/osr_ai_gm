//! Spell slot progression tables per OSE Reference Booklet p16-17.
//!
//! When the `dsl-backend` feature is enabled and `OSR_BACKEND_SPELL=dsl`,
//! `spell_slots` delegates to the DSL table lookup. Native fallback preserved
//! when DSL is unavailable.

/// Spell slots available at a given level. Index 0 = 1st level spells, etc.
/// A value of 0 means no spells of that level available.
pub type SpellSlots = [u32; 6];

/// Which spell list a class uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SpellListType {
    None,
    Cleric,     // Cleric, Paladin
    Druid,      // Bard, Druid, Ranger
    Illusionist, // Gnome, Illusionist
    MagicUser,   // Elf, Half-Elf, Magic-User
    DrowArcaneAndDivine, // Drow: both arcane (magic-user) and divine (cleric)
}

/// Spell progression category — classes sharing a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SpellProgression {
    Bard,
    Cleric,
    Drow,
    Druid,
    ArcaneFullCaster, // Elf, Gnome, Illusionist, Magic-User
    HalfElf,
    Paladin,
    Ranger,
    NonCaster,
}

/// Returns spell slots for a progression type at a given character level.
/// Returns [0; 6] for non-casters or levels below casting threshold.
///
/// When `OSR_BACKEND_SPELL=dsl` and the `dsl-backend` feature is enabled,
/// delegates to the DSL `spell_slots` table lookup.
pub fn spell_slots(prog: SpellProgression, level: u32) -> SpellSlots {
    #[cfg(feature = "dsl-backend")]
    {
        if crate::backend::is_dsl(crate::backend::MechanicGroup::Spell) {
            if let Some(slots) = dsl_gate::dsl_spell_slots(prog, level) {
                return slots;
            }
        }
    }
    use SpellProgression::*;
    match prog {
        NonCaster => [0; 6],

        // Bard: druid spell list, up to 4th level spells, levels 1-14
        Bard => match level {
            1 => [0, 0, 0, 0, 0, 0],
            2 => [1, 0, 0, 0, 0, 0],
            3 => [2, 0, 0, 0, 0, 0],
            4 => [3, 0, 0, 0, 0, 0],
            5 => [3, 1, 0, 0, 0, 0],
            6 => [3, 2, 0, 0, 0, 0],
            7 => [3, 3, 0, 0, 0, 0],
            8 => [3, 3, 1, 0, 0, 0],
            9 => [3, 3, 2, 0, 0, 0],
            10 => [3, 3, 3, 0, 0, 0],
            11 => [3, 3, 3, 1, 0, 0],
            12 => [3, 3, 3, 2, 0, 0],
            13 => [3, 3, 3, 3, 0, 0],
            _ => [4, 4, 3, 3, 0, 0],
        },

        // Cleric: cleric spell list, up to 5th level spells, levels 1-14
        Cleric => match level {
            1 => [0, 0, 0, 0, 0, 0],
            2 => [1, 0, 0, 0, 0, 0],
            3 => [2, 0, 0, 0, 0, 0],
            4 => [2, 1, 0, 0, 0, 0],
            5 => [2, 2, 0, 0, 0, 0],
            6 => [2, 2, 1, 1, 0, 0],
            7 => [2, 2, 2, 1, 1, 0],
            8 => [3, 3, 2, 2, 1, 0],
            9 => [3, 3, 3, 2, 2, 0],
            10 => [4, 4, 3, 3, 2, 0],
            11 => [4, 4, 4, 3, 3, 0],
            12 => [5, 5, 4, 4, 3, 0],
            13 => [5, 5, 5, 4, 4, 0],
            _ => [6, 5, 5, 5, 4, 0],
        },

        // Drow: cleric spell list, up to 5th level spells, levels 1-10
        Drow => match level {
            1 => [1, 0, 0, 0, 0, 0], // only light (darkness) at 1st
            2 => [2, 0, 0, 0, 0, 0],
            3 => [2, 1, 0, 0, 0, 0],
            4 => [2, 2, 0, 0, 0, 0],
            5 => [2, 2, 1, 0, 0, 0],
            6 => [2, 2, 2, 1, 0, 0],
            7 => [3, 3, 2, 2, 1, 0],
            8 => [3, 3, 3, 2, 2, 0],
            9 => [4, 4, 3, 3, 2, 0],
            _ => [4, 4, 4, 3, 3, 0],
        },

        // Druid: druid spell list, up to 5th level spells, levels 1-14
        Druid => match level {
            1 => [1, 0, 0, 0, 0, 0],
            2 => [2, 0, 0, 0, 0, 0],
            3 => [2, 1, 0, 0, 0, 0],
            4 => [2, 2, 0, 0, 0, 0],
            5 => [2, 2, 1, 1, 0, 0],
            6 => [2, 2, 2, 1, 1, 0],
            7 => [3, 3, 2, 2, 1, 0],
            8 => [3, 3, 3, 2, 2, 0],
            9 => [4, 4, 3, 3, 2, 0],
            10 => [4, 4, 4, 3, 3, 0],
            11 => [5, 5, 4, 4, 3, 0],
            12 => [5, 5, 5, 4, 4, 0],
            13 => [6, 5, 5, 5, 4, 0],
            _ => [6, 6, 5, 5, 5, 0],
        },

        // Elf, Gnome, Illusionist, Magic-User: up to 6th level spells
        ArcaneFullCaster => match level {
            1 => [1, 0, 0, 0, 0, 0],
            2 => [2, 0, 0, 0, 0, 0],
            3 => [2, 1, 0, 0, 0, 0],
            4 => [2, 2, 0, 0, 0, 0],
            5 => [2, 2, 1, 0, 0, 0],
            6 => [2, 2, 2, 0, 0, 0],
            7 => [3, 2, 2, 1, 0, 0],
            8 => [3, 3, 2, 2, 0, 0],
            9 => [3, 3, 3, 2, 1, 0],
            10 => [3, 3, 3, 3, 2, 0],
            11 => [4, 3, 3, 3, 2, 1],
            12 => [4, 4, 3, 3, 3, 2],
            13 => [4, 4, 4, 3, 3, 3],
            _ => [4, 4, 4, 4, 3, 3],
        },

        // Half-Elf: magic-user spell list, up to 4th level spells, levels 1-12
        HalfElf => match level {
            1 => [0, 0, 0, 0, 0, 0],
            2 => [1, 0, 0, 0, 0, 0],
            3 => [2, 0, 0, 0, 0, 0],
            4 => [2, 0, 0, 0, 0, 0],
            5 => [2, 1, 0, 0, 0, 0],
            6 => [2, 2, 0, 0, 0, 0],
            7 => [2, 2, 0, 0, 0, 0],
            8 => [2, 2, 1, 0, 0, 0],
            9 => [3, 2, 1, 0, 0, 0],
            10 => [3, 2, 2, 0, 0, 0],
            11 => [3, 2, 2, 1, 0, 0],
            _ => [3, 3, 2, 1, 0, 0],
        },

        // Paladin: cleric spell list, up to 3rd level spells, levels 9-14
        Paladin => match level {
            1..=8 => [0, 0, 0, 0, 0, 0],
            9 => [1, 0, 0, 0, 0, 0],
            10 => [2, 0, 0, 0, 0, 0],
            11 => [2, 1, 0, 0, 0, 0],
            12 => [2, 2, 0, 0, 0, 0],
            13 => [2, 2, 1, 0, 0, 0],
            _ => [3, 2, 1, 0, 0, 0],
        },

        // Ranger: druid spell list, up to 3rd level spells, levels 8-14
        Ranger => match level {
            1..=7 => [0, 0, 0, 0, 0, 0],
            8 => [1, 0, 0, 0, 0, 0],
            9 => [2, 0, 0, 0, 0, 0],
            10 => [2, 1, 0, 0, 0, 0],
            11 => [2, 2, 0, 0, 0, 0],
            12 => [2, 2, 1, 0, 0, 0],
            13 => [3, 2, 1, 0, 0, 0],
            _ => [3, 2, 2, 0, 0, 0],
        },
    }
}

/// Check if a class has any spell slots at a given level.
pub fn can_cast(prog: SpellProgression, level: u32) -> bool {
    spell_slots(prog, level).iter().any(|&s| s > 0)
}

/// Total number of spell slots across all spell levels.
pub fn total_slots(prog: SpellProgression, level: u32) -> u32 {
    spell_slots(prog, level).iter().sum()
}

/// Returns the casting resource type for the active game system.
/// For OSE, this is always "vancian_slots".
pub fn casting_resource_type(prog: SpellProgression) -> String {
    #[cfg(feature = "dsl-backend")]
    {
        if crate::backend::is_dsl(crate::backend::MechanicGroup::Spell) {
            if let Some(s) = dsl_gate::dsl_casting_resource_type(prog) {
                return s;
            }
        }
    }
    let _ = prog;
    "vancian_slots".to_string()
}

/// Check if a character can cast a spell at the given spell level,
/// considering their current slot usage against maximum slots.
pub fn can_cast_spell(slots_used: &SpellSlots, max_slots: &SpellSlots, spell_level: u32) -> bool {
    #[cfg(feature = "dsl-backend")]
    {
        if crate::backend::is_dsl(crate::backend::MechanicGroup::Spell) {
            if let Some(result) = dsl_gate::dsl_can_cast_spell(slots_used, max_slots, spell_level) {
                return result;
            }
        }
    }
    // Native fallback: Vancian slot check
    let idx = (spell_level - 1) as usize;
    if idx < 6 {
        slots_used[idx] < max_slots[idx]
    } else {
        false
    }
}

/// Returns the cost (in resource units) to cast a spell of the given level.
/// For Vancian casting, this is always 1 slot.
pub fn cast_cost(spell_level: u32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    {
        if crate::backend::is_dsl(crate::backend::MechanicGroup::Spell) {
            if let Some(cost) = dsl_gate::dsl_cast_cost(spell_level) {
                return cost;
            }
        }
    }
    let _ = spell_level;
    1 // Vancian: always 1 slot per cast
}

/// Returns the spell point cost for a given spell level.
/// Used by spell-point casting systems where each spell level has a different cost.
pub fn spell_point_cost(spell_level: u32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    {
        if crate::backend::is_dsl(crate::backend::MechanicGroup::Spell) {
            if let Some(cost) = dsl_gate::dsl_spell_point_cost(spell_level) {
                return cost;
            }
        }
    }
    // Native fallback: standard OSR spell point conversion
    match spell_level {
        1 => 2,
        2 => 3,
        3 => 5,
        4 => 6,
        5 => 7,
        6 => 9,
        _ => 0,
    }
}

/// Returns true if a disrupted (failed) spell still consumes its Vancian slot.
/// Per B/X OSE: an attempted casting always expends the slot, even if disrupted.
/// Game systems that use a different model (e.g. prepared-without-expend) can
/// override the DSL `disruption_consumes_slot` derive to return 0.
pub fn disruption_consumes_slot() -> bool {
    #[cfg(feature = "dsl-backend")]
    {
        if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
            if let Some(v) = dsl_gate::dsl_disruption_consumes_slot() {
                return v;
            }
        }
    }
    true // native fallback: B/X OSE always consumes the slot
}

/// Check whether all casting resources recharge on long rest.
/// Returns true if the casting model fully recharges on rest (all known models do).
pub fn rest_recovery(prog: SpellProgression) -> bool {
    #[cfg(feature = "dsl-backend")]
    {
        if crate::backend::is_dsl(crate::backend::MechanicGroup::Spell) {
            if let Some(result) = dsl_gate::dsl_rest_recovery(prog) {
                return result;
            }
        }
    }
    let _ = prog;
    true // All resource types fully recharge on rest
}

// ── DSL gate helpers ──────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
mod dsl_gate {
    use std::collections::BTreeMap;

    use ttrpg_ast::Name;
    use ttrpg_interp::effect::{Effect, EffectHandler, Response};
    use ttrpg_interp::state::{ActiveCondition, EntityRef, StateProvider};
    use ttrpg_interp::value::Value;

    use super::{SpellProgression, SpellSlots};
    use crate::backend;

    /// Null state provider for pure table evaluation (no entity access needed).
    struct NullState;

    impl StateProvider for NullState {
        fn read_field(&self, _: &EntityRef, _: &str) -> Option<Value> { None }
        fn read_conditions(&self, _: &EntityRef) -> Option<Vec<ActiveCondition>> { None }
        fn read_turn_budget(&self, _: &EntityRef) -> Option<BTreeMap<Name, Value>> { None }
        fn read_enabled_options(&self) -> Vec<Name> { Vec::new() }
        fn position_eq(&self, _: &Value, _: &Value) -> bool { false }
        fn distance(&self, _: &Value, _: &Value) -> Option<i64> { None }
        fn entity_type_name(&self, _: &EntityRef) -> Option<Name> { None }
    }

    /// Null effect handler for pure table evaluation (no effects fired).
    struct NullHandler;

    impl EffectHandler for NullHandler {
        fn handle(&mut self, _: Effect) -> Response { Response::Acknowledged }
    }

    /// Map Rust SpellProgression to DSL enum variant name.
    fn progression_to_dsl(prog: SpellProgression) -> Value {
        let variant = match prog {
            SpellProgression::NonCaster => "non_caster",
            SpellProgression::Bard => "prog_bard",
            SpellProgression::Cleric => "prog_cleric",
            SpellProgression::Drow => "prog_drow",
            SpellProgression::Druid => "prog_druid",
            SpellProgression::ArcaneFullCaster => "prog_arcane_full",
            SpellProgression::HalfElf => "prog_half_elf",
            SpellProgression::Paladin => "prog_paladin",
            SpellProgression::Ranger => "prog_ranger",
        };
        Value::EnumVariant {
            enum_name: "SpellProgression".into(),
            variant: Name::from(variant),
            fields: BTreeMap::new(),
        }
    }

    /// Convert a DSL list<int> result to a SpellSlots [u32; 6].
    fn value_to_spell_slots(value: Value) -> Option<SpellSlots> {
        match value {
            Value::List(items) if items.len() == 6 => {
                let mut slots = [0u32; 6];
                for (i, item) in items.iter().enumerate() {
                    match item {
                        Value::Int(v) => slots[i] = *v as u32,
                        _ => return None,
                    }
                }
                Some(slots)
            }
            _ => None,
        }
    }

    pub fn dsl_spell_slots(prog: SpellProgression, level: u32) -> Option<SpellSlots> {
        let rt = backend::dsl()?;
        let args = vec![progression_to_dsl(prog), Value::Int(level as i64)];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "spell_slots", args).ok()?;
        value_to_spell_slots(result)
    }

    /// Build a CastingResource enum variant Value from its string name.
    fn casting_resource_to_dsl(name: &str) -> Value {
        Value::EnumVariant {
            enum_name: "CastingResource".into(),
            variant: Name::from(name),
            fields: BTreeMap::new(),
        }
    }

    /// Convert a SpellSlots [u32; 6] to a DSL list<int> Value.
    fn spell_slots_to_dsl(slots: &super::SpellSlots) -> Value {
        Value::List(slots.iter().map(|&s| Value::Int(s as i64)).collect())
    }

    pub fn dsl_casting_resource_type(prog: SpellProgression) -> Option<String> {
        let rt = backend::dsl()?;
        let args = vec![progression_to_dsl(prog)];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "casting_resource_type", args).ok()?;
        match result {
            Value::EnumVariant { variant, .. } => Some(variant.to_string()),
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn dsl_can_cast_spell(
        slots_used: &super::SpellSlots,
        max_slots: &super::SpellSlots,
        spell_level: u32,
    ) -> Option<bool> {
        let rt = backend::dsl()?;
        let resource = dsl_casting_resource_type(SpellProgression::Cleric)?;
        let args = vec![
            casting_resource_to_dsl(&resource),
            spell_slots_to_dsl(slots_used),
            spell_slots_to_dsl(max_slots),
            Value::Int(spell_level as i64),
        ];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "can_cast_spell", args).ok()?;
        match result {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub fn dsl_cast_cost(spell_level: u32) -> Option<u32> {
        let rt = backend::dsl()?;
        let resource = dsl_casting_resource_type(SpellProgression::Cleric)?;
        let args = vec![
            casting_resource_to_dsl(&resource),
            Value::Int(spell_level as i64),
        ];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "cast_cost", args).ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_spell_point_cost(spell_level: u32) -> Option<u32> {
        let rt = backend::dsl()?;
        let args = vec![Value::Int(spell_level as i64)];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "spell_point_cost", args).ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_rest_recovery(prog: SpellProgression) -> Option<bool> {
        let rt = backend::dsl()?;
        let resource = dsl_casting_resource_type(prog)?;
        let args = vec![casting_resource_to_dsl(&resource)];
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "rest_recovery", args).ok()?;
        match result {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// Check DSL `disruption_consumes_slot` derive (Combat group).
    /// Returns Some(true) if the derive says slot is consumed, Some(false) if not, None on error.
    pub fn dsl_disruption_consumes_slot() -> Option<bool> {
        if !backend::is_dsl(crate::backend::MechanicGroup::Combat) {
            return None;
        }
        let rt = backend::dsl()?;
        let result = rt.evaluate_derive(&NullState, &mut NullHandler, "disruption_consumes_slot", vec![]).ok()?;
        match result {
            Value::Int(v) => Some(v != 0),
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_caster() {
        assert_eq!(spell_slots(SpellProgression::NonCaster, 14), [0; 6]);
        assert!(!can_cast(SpellProgression::NonCaster, 14));
    }

    #[test]
    fn cleric_no_spells_level_1() {
        assert!(!can_cast(SpellProgression::Cleric, 1));
    }

    #[test]
    fn cleric_gets_spells_level_2() {
        let s = spell_slots(SpellProgression::Cleric, 2);
        assert_eq!(s[0], 1);
        assert!(can_cast(SpellProgression::Cleric, 2));
    }

    #[test]
    fn cleric_level_14() {
        let s = spell_slots(SpellProgression::Cleric, 14);
        assert_eq!(s, [6, 5, 5, 5, 4, 0]);
    }

    #[test]
    fn magic_user_level_1() {
        let s = spell_slots(SpellProgression::ArcaneFullCaster, 1);
        assert_eq!(s, [1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn magic_user_level_14() {
        let s = spell_slots(SpellProgression::ArcaneFullCaster, 14);
        assert_eq!(s, [4, 4, 4, 4, 3, 3]);
    }

    #[test]
    fn druid_level_7() {
        let s = spell_slots(SpellProgression::Druid, 7);
        assert_eq!(s, [3, 3, 2, 2, 1, 0]);
    }

    #[test]
    fn bard_level_2() {
        let s = spell_slots(SpellProgression::Bard, 2);
        assert_eq!(s, [1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn bard_level_14() {
        let s = spell_slots(SpellProgression::Bard, 14);
        assert_eq!(s, [4, 4, 3, 3, 0, 0]);
    }

    #[test]
    fn half_elf_level_8() {
        let s = spell_slots(SpellProgression::HalfElf, 8);
        assert_eq!(s, [2, 2, 1, 0, 0, 0]);
    }

    #[test]
    fn paladin_no_spells_level_8() {
        assert!(!can_cast(SpellProgression::Paladin, 8));
    }

    #[test]
    fn paladin_gets_spells_level_9() {
        let s = spell_slots(SpellProgression::Paladin, 9);
        assert_eq!(s[0], 1);
    }

    #[test]
    fn ranger_level_10() {
        let s = spell_slots(SpellProgression::Ranger, 10);
        assert_eq!(s, [2, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn drow_level_1() {
        let s = spell_slots(SpellProgression::Drow, 1);
        assert_eq!(s[0], 1); // only light (darkness)
    }

    #[test]
    fn drow_level_10() {
        let s = spell_slots(SpellProgression::Drow, 10);
        assert_eq!(s, [4, 4, 4, 3, 3, 0]);
    }

    #[test]
    fn total_slots_magic_user_5() {
        assert_eq!(total_slots(SpellProgression::ArcaneFullCaster, 5), 5);
    }

    #[test]
    fn casting_resource_type_is_vancian() {
        assert_eq!(
            casting_resource_type(SpellProgression::Cleric),
            "vancian_slots"
        );
    }

    #[test]
    fn can_cast_spell_with_slots_available() {
        let used = [0, 0, 0, 0, 0, 0];
        let max = [2, 1, 0, 0, 0, 0];
        assert!(can_cast_spell(&used, &max, 1));
        assert!(can_cast_spell(&used, &max, 2));
        assert!(!can_cast_spell(&used, &max, 3)); // max is 0
    }

    #[test]
    fn can_cast_spell_all_slots_used() {
        let used = [2, 1, 0, 0, 0, 0];
        let max = [2, 1, 0, 0, 0, 0];
        assert!(!can_cast_spell(&used, &max, 1));
        assert!(!can_cast_spell(&used, &max, 2));
    }

    #[test]
    fn can_cast_spell_partial_usage() {
        let used = [1, 0, 0, 0, 0, 0];
        let max = [2, 1, 0, 0, 0, 0];
        assert!(can_cast_spell(&used, &max, 1)); // 1 of 2 used
        assert!(can_cast_spell(&used, &max, 2)); // 0 of 1 used
    }

    #[test]
    fn cast_cost_is_one() {
        assert_eq!(cast_cost(1), 1);
        assert_eq!(cast_cost(3), 1);
        assert_eq!(cast_cost(6), 1);
    }

    #[test]
    fn spell_point_costs() {
        assert_eq!(spell_point_cost(1), 2);
        assert_eq!(spell_point_cost(2), 3);
        assert_eq!(spell_point_cost(3), 5);
        assert_eq!(spell_point_cost(4), 6);
        assert_eq!(spell_point_cost(5), 7);
        assert_eq!(spell_point_cost(6), 9);
        assert_eq!(spell_point_cost(7), 0);
    }

    #[test]
    fn rest_recovery_always_true() {
        assert!(rest_recovery(SpellProgression::Cleric));
        assert!(rest_recovery(SpellProgression::ArcaneFullCaster));
        assert!(rest_recovery(SpellProgression::NonCaster));
    }
}

// ── DSL parity tests ────────────────────────────────────────────

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_tests {
    use super::*;

    /// Native-only spell_slots (bypass DSL gate).
    fn native_spell_slots(prog: SpellProgression, level: u32) -> SpellSlots {
        use SpellProgression::*;
        match prog {
            NonCaster => [0; 6],
            Bard => match level {
                1 => [0, 0, 0, 0, 0, 0], 2 => [1, 0, 0, 0, 0, 0],
                3 => [2, 0, 0, 0, 0, 0], 4 => [3, 0, 0, 0, 0, 0],
                5 => [3, 1, 0, 0, 0, 0], 6 => [3, 2, 0, 0, 0, 0],
                7 => [3, 3, 0, 0, 0, 0], 8 => [3, 3, 1, 0, 0, 0],
                9 => [3, 3, 2, 0, 0, 0], 10 => [3, 3, 3, 0, 0, 0],
                11 => [3, 3, 3, 1, 0, 0], 12 => [3, 3, 3, 2, 0, 0],
                13 => [3, 3, 3, 3, 0, 0], _ => [4, 4, 3, 3, 0, 0],
            },
            Cleric => match level {
                1 => [0, 0, 0, 0, 0, 0], 2 => [1, 0, 0, 0, 0, 0],
                3 => [2, 0, 0, 0, 0, 0], 4 => [2, 1, 0, 0, 0, 0],
                5 => [2, 2, 0, 0, 0, 0], 6 => [2, 2, 1, 1, 0, 0],
                7 => [2, 2, 2, 1, 1, 0], 8 => [3, 3, 2, 2, 1, 0],
                9 => [3, 3, 3, 2, 2, 0], 10 => [4, 4, 3, 3, 2, 0],
                11 => [4, 4, 4, 3, 3, 0], 12 => [5, 5, 4, 4, 3, 0],
                13 => [5, 5, 5, 4, 4, 0], _ => [6, 5, 5, 5, 4, 0],
            },
            Drow => match level {
                1 => [1, 0, 0, 0, 0, 0], 2 => [2, 0, 0, 0, 0, 0],
                3 => [2, 1, 0, 0, 0, 0], 4 => [2, 2, 0, 0, 0, 0],
                5 => [2, 2, 1, 0, 0, 0], 6 => [2, 2, 2, 1, 0, 0],
                7 => [3, 3, 2, 2, 1, 0], 8 => [3, 3, 3, 2, 2, 0],
                9 => [4, 4, 3, 3, 2, 0], _ => [4, 4, 4, 3, 3, 0],
            },
            Druid => match level {
                1 => [1, 0, 0, 0, 0, 0], 2 => [2, 0, 0, 0, 0, 0],
                3 => [2, 1, 0, 0, 0, 0], 4 => [2, 2, 0, 0, 0, 0],
                5 => [2, 2, 1, 1, 0, 0], 6 => [2, 2, 2, 1, 1, 0],
                7 => [3, 3, 2, 2, 1, 0], 8 => [3, 3, 3, 2, 2, 0],
                9 => [4, 4, 3, 3, 2, 0], 10 => [4, 4, 4, 3, 3, 0],
                11 => [5, 5, 4, 4, 3, 0], 12 => [5, 5, 5, 4, 4, 0],
                13 => [6, 5, 5, 5, 4, 0], _ => [6, 6, 5, 5, 5, 0],
            },
            ArcaneFullCaster => match level {
                1 => [1, 0, 0, 0, 0, 0], 2 => [2, 0, 0, 0, 0, 0],
                3 => [2, 1, 0, 0, 0, 0], 4 => [2, 2, 0, 0, 0, 0],
                5 => [2, 2, 1, 0, 0, 0], 6 => [2, 2, 2, 0, 0, 0],
                7 => [3, 2, 2, 1, 0, 0], 8 => [3, 3, 2, 2, 0, 0],
                9 => [3, 3, 3, 2, 1, 0], 10 => [3, 3, 3, 3, 2, 0],
                11 => [4, 3, 3, 3, 2, 1], 12 => [4, 4, 3, 3, 3, 2],
                13 => [4, 4, 4, 3, 3, 3], _ => [4, 4, 4, 4, 3, 3],
            },
            HalfElf => match level {
                1 => [0, 0, 0, 0, 0, 0], 2 => [1, 0, 0, 0, 0, 0],
                3 => [2, 0, 0, 0, 0, 0], 4 => [2, 0, 0, 0, 0, 0],
                5 => [2, 1, 0, 0, 0, 0], 6 => [2, 2, 0, 0, 0, 0],
                7 => [2, 2, 0, 0, 0, 0], 8 => [2, 2, 1, 0, 0, 0],
                9 => [3, 2, 1, 0, 0, 0], 10 => [3, 2, 2, 0, 0, 0],
                11 => [3, 2, 2, 1, 0, 0], _ => [3, 3, 2, 1, 0, 0],
            },
            Paladin => match level {
                1..=8 => [0, 0, 0, 0, 0, 0],
                9 => [1, 0, 0, 0, 0, 0], 10 => [2, 0, 0, 0, 0, 0],
                11 => [2, 1, 0, 0, 0, 0], 12 => [2, 2, 0, 0, 0, 0],
                13 => [2, 2, 1, 0, 0, 0], _ => [3, 2, 1, 0, 0, 0],
            },
            Ranger => match level {
                1..=7 => [0, 0, 0, 0, 0, 0],
                8 => [1, 0, 0, 0, 0, 0], 9 => [2, 0, 0, 0, 0, 0],
                10 => [2, 1, 0, 0, 0, 0], 11 => [2, 2, 0, 0, 0, 0],
                12 => [2, 2, 1, 0, 0, 0], 13 => [3, 2, 1, 0, 0, 0],
                _ => [3, 2, 2, 0, 0, 0],
            },
        }
    }

    /// All 9 progressions to test.
    const ALL_PROGS: [SpellProgression; 9] = [
        SpellProgression::NonCaster,
        SpellProgression::Bard,
        SpellProgression::Cleric,
        SpellProgression::Drow,
        SpellProgression::Druid,
        SpellProgression::ArcaneFullCaster,
        SpellProgression::HalfElf,
        SpellProgression::Paladin,
        SpellProgression::Ranger,
    ];

    /// Max level defined in the DSL table for each progression.
    /// Out-of-range levels fall back to native (correct behavior).
    fn dsl_max_level(prog: SpellProgression) -> u32 {
        match prog {
            SpellProgression::Drow => 10,
            SpellProgression::HalfElf => 12,
            _ => 14,
        }
    }

    /// Verify DSL spell_slots table matches native for all defined progression/level combos.
    #[test]
    fn dsl_matches_native_all_progressions() {
        for &prog in &ALL_PROGS {
            let max = dsl_max_level(prog);
            for level in 1..=max {
                let dsl = dsl_gate::dsl_spell_slots(prog, level)
                    .unwrap_or_else(|| panic!("DSL spell_slots({:?}, {}) failed", prog, level));
                let native = native_spell_slots(prog, level);
                assert_eq!(
                    dsl, native,
                    "spell_slots({:?}, {}) mismatch: DSL={:?} native={:?}",
                    prog, level, dsl, native
                );
            }
        }
    }

    /// Verify that out-of-range levels gracefully fall back (DSL returns None).
    #[test]
    fn dsl_out_of_range_returns_none() {
        // Drow level 11 is not in the DSL table
        assert!(dsl_gate::dsl_spell_slots(SpellProgression::Drow, 11).is_none());
        // HalfElf level 13 is not in the DSL table
        assert!(dsl_gate::dsl_spell_slots(SpellProgression::HalfElf, 13).is_none());
    }

    #[test]
    fn dsl_casting_resource_type_is_vancian() {
        let result = dsl_gate::dsl_casting_resource_type(SpellProgression::Cleric);
        assert_eq!(result, Some("vancian_slots".to_string()));
    }

    #[test]
    fn dsl_can_cast_spell_with_slots() {
        let used = [0, 0, 0, 0, 0, 0];
        let max = [2, 1, 0, 0, 0, 0];
        assert_eq!(dsl_gate::dsl_can_cast_spell(&used, &max, 1), Some(true));
        assert_eq!(dsl_gate::dsl_can_cast_spell(&used, &max, 2), Some(true));
        assert_eq!(dsl_gate::dsl_can_cast_spell(&used, &max, 3), Some(false));
    }

    #[test]
    fn dsl_can_cast_spell_exhausted() {
        let used = [2, 1, 0, 0, 0, 0];
        let max = [2, 1, 0, 0, 0, 0];
        assert_eq!(dsl_gate::dsl_can_cast_spell(&used, &max, 1), Some(false));
        assert_eq!(dsl_gate::dsl_can_cast_spell(&used, &max, 2), Some(false));
    }

    #[test]
    fn dsl_cast_cost_is_one() {
        for level in 1..=6 {
            assert_eq!(
                dsl_gate::dsl_cast_cost(level),
                Some(1),
                "cast_cost for spell level {} should be 1",
                level
            );
        }
    }

    #[test]
    fn dsl_spell_point_costs() {
        assert_eq!(dsl_gate::dsl_spell_point_cost(1), Some(2));
        assert_eq!(dsl_gate::dsl_spell_point_cost(2), Some(3));
        assert_eq!(dsl_gate::dsl_spell_point_cost(3), Some(5));
        assert_eq!(dsl_gate::dsl_spell_point_cost(4), Some(6));
        assert_eq!(dsl_gate::dsl_spell_point_cost(5), Some(7));
        assert_eq!(dsl_gate::dsl_spell_point_cost(6), Some(9));
    }

    #[test]
    fn dsl_rest_recovery_true() {
        assert_eq!(
            dsl_gate::dsl_rest_recovery(SpellProgression::Cleric),
            Some(true)
        );
    }

    #[test]
    fn dsl_disruption_consumes_slot_is_true() {
        // B/X OSE: disrupted spells still consume the slot.
        assert_eq!(
            dsl_gate::dsl_disruption_consumes_slot(),
            Some(true)
        );
    }
}
