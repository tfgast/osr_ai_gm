//! Exploration mechanics per OSE Rules Tome.
//!
//! Wandering monster check interval and chance, search room, listen at door.
//!
//! When the `dsl-backend` feature is enabled and `OSR_BACKEND_EXPLORATION=dsl`,
//! derive functions delegate to DSL evaluations. The engine still performs the
//! actual dice rolls; these functions return the thresholds and intervals only.

#[cfg(feature = "dsl-backend")]
use crate::backend::{is_dsl, MechanicGroup};

/// How many dungeon turns between wandering monster checks.
/// OSE: check every 2 turns.
pub fn wandering_monster_interval() -> u32 {
    #[cfg(feature = "dsl-backend")]
    {
        if is_dsl(MechanicGroup::Exploration) {
            if let Some(val) = dsl_gate::dsl_wandering_monster_interval() {
                return val;
            }
        }
    }
    2
}

/// Wandering monster die size (1..=N); a roll of 1 triggers an encounter.
/// OSE: 1-in-6 (die size 6).
pub fn wandering_monster_die() -> u32 {
    #[cfg(feature = "dsl-backend")]
    {
        if is_dsl(MechanicGroup::Exploration) {
            if let Some(val) = dsl_gate::dsl_wandering_monster_die() {
                return val;
            }
        }
    }
    6
}

/// Search room success threshold (X-in-6).
/// OSE: base 1-in-6; elves get 2-in-6.
pub fn search_room_chance(is_elf: bool) -> u32 {
    #[cfg(feature = "dsl-backend")]
    {
        if is_dsl(MechanicGroup::Exploration) {
            if let Some(val) = dsl_gate::dsl_search_room_chance(is_elf) {
                return val;
            }
        }
    }
    if is_elf { 2 } else { 1 }
}

/// Listen at door success threshold (X-in-6).
/// OSE: base 1-in-6; demihumans (elves, halflings) get 2-in-6.
pub fn listen_at_door_chance(is_demihuman: bool) -> u32 {
    #[cfg(feature = "dsl-backend")]
    {
        if is_dsl(MechanicGroup::Exploration) {
            if let Some(val) = dsl_gate::dsl_listen_at_door_chance(is_demihuman) {
                return val;
            }
        }
    }
    if is_demihuman { 2 } else { 1 }
}

// ── DSL gate helpers ──────────────────────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
mod dsl_gate {
    use ttrpg_interp::value::Value;

    use crate::backend::{self, NullState, SimpleDiceHandler};

    pub fn dsl_wandering_monster_interval() -> Option<u32> {
        let rt = backend::dsl()?;
        let result = rt
            .evaluate_derive(&NullState, &mut SimpleDiceHandler::new(), "wandering_monster_interval", vec![])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_wandering_monster_die() -> Option<u32> {
        let rt = backend::dsl()?;
        let result = rt
            .evaluate_derive(&NullState, &mut SimpleDiceHandler::new(), "wandering_monster_die", vec![])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_search_room_chance(is_elf: bool) -> Option<u32> {
        let rt = backend::dsl()?;
        let result = rt
            .evaluate_derive(
                &NullState,
                &mut SimpleDiceHandler::new(),
                "search_room_chance",
                vec![Value::Bool(is_elf)],
            )
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_listen_at_door_chance(is_demihuman: bool) -> Option<u32> {
        let rt = backend::dsl()?;
        let result = rt
            .evaluate_derive(
                &NullState,
                &mut SimpleDiceHandler::new(),
                "listen_at_door_chance",
                vec![Value::Bool(is_demihuman)],
            )
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wandering_monster_interval_is_2() {
        assert_eq!(wandering_monster_interval(), 2);
    }

    #[test]
    fn wandering_monster_die_is_6() {
        assert_eq!(wandering_monster_die(), 6);
    }

    #[test]
    fn search_room_chance_base() {
        assert_eq!(search_room_chance(false), 1);
    }

    #[test]
    fn search_room_chance_elf() {
        assert_eq!(search_room_chance(true), 2);
    }

    #[test]
    fn listen_at_door_chance_base() {
        assert_eq!(listen_at_door_chance(false), 1);
    }

    #[test]
    fn listen_at_door_chance_demihuman() {
        assert_eq!(listen_at_door_chance(true), 2);
    }
}
