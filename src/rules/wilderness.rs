//! Wilderness travel rules per OSE Rules Tome.
//!
//! Pure-rule functions for terrain mechanics, foraging, hunting, and
//! starvation. Each function has a native (Rust) fallback plus an
//! optional `#[cfg(feature = "dsl-backend")]` gate that delegates to
//! the DSL `ose_wilderness.ttrpg` derives when
//! `OSR_BACKEND_WILDERNESS=dsl` is set.
//!
//! The engine (`src/engine/wilderness_engine.rs`) and state
//! (`src/state/wilderness.rs`) currently contain the hardcoded
//! implementations; DSL gate wiring is handled in oag-mmg45.

use crate::state::wilderness::Terrain;

// ── Terrain movement cost ──────────────────────────────────────

/// Terrain movement cost as a fraction (numerator, denominator).
///
/// Effective miles/day = (base_miles * denominator) / numerator
/// where base_miles = movement_rate / 5.
///
/// Clear/City/Ocean: (1,1)   Hills/Forest/River/Barren: (3,2)
/// Desert/Swamp/Jungle: (2,1)   Mountains: (3,1)
pub fn terrain_movement_cost(terrain: Terrain) -> (u32, u32) {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Wilderness) {
        if let Some(val) = dsl_gate::dsl_terrain_movement_cost(terrain) {
            return val;
        }
    }
    terrain_movement_cost_native(terrain)
}

fn terrain_movement_cost_native(terrain: Terrain) -> (u32, u32) {
    match terrain {
        Terrain::Clear | Terrain::City | Terrain::Ocean => (1, 1),
        Terrain::Barren | Terrain::Hills | Terrain::Forest | Terrain::River => (3, 2),
        Terrain::Desert | Terrain::Jungle | Terrain::Swamp => (2, 1),
        Terrain::Mountains => (3, 1),
    }
}

// ── Daily hexes traveled ──────────────────────────────────────

/// Daily travel speed in hexes given party movement rate and terrain.
///
/// Formula: `(movement_rate / 5 * cost_den / cost_num) / 6`, min 1.
/// One hex = 6 miles; base overland travel = movement_rate / 5 miles/day.
pub fn hexes_per_day(movement_rate: u32, terrain: Terrain) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Wilderness) {
        if let Some(val) = dsl_gate::dsl_hexes_per_day(movement_rate, terrain) {
            return val;
        }
    }
    hexes_per_day_native(movement_rate, terrain)
}

fn hexes_per_day_native(movement_rate: u32, terrain: Terrain) -> u32 {
    let base_miles = movement_rate as u64 / 5;
    let (cost_num, cost_den) = terrain_movement_cost_native(terrain);
    let effective_miles = base_miles * cost_den as u64 / cost_num as u64;
    let hexes = effective_miles / 6;
    if hexes < 1 { 1 } else { hexes as u32 }
}

// ── Getting lost ──────────────────────────────────────────────

/// Getting lost chance per day of travel (X-in-6).
/// Roll 1d6; party is lost if roll <= this value.
///
/// Clear/City/Barren/Hills/Mountains: 1-in-6
/// Forest/River/Desert/Ocean/Swamp/Jungle: 2-in-6
pub fn terrain_lost_chance(terrain: Terrain) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Wilderness) {
        if let Some(val) = dsl_gate::dsl_terrain_lost_chance(terrain) {
            return val;
        }
    }
    terrain_lost_chance_native(terrain)
}

fn terrain_lost_chance_native(terrain: Terrain) -> u32 {
    match terrain {
        Terrain::Clear | Terrain::City | Terrain::Barren
        | Terrain::Hills | Terrain::Mountains => 1,
        Terrain::Forest | Terrain::River | Terrain::Desert
        | Terrain::Ocean | Terrain::Swamp | Terrain::Jungle => 2,
    }
}

// ── Foraging ──────────────────────────────────────────────────

/// Whether foraging is possible in this terrain.
/// Not possible in Ocean, City, or Barren.
pub fn terrain_can_forage(terrain: Terrain) -> bool {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Wilderness) {
        if let Some(val) = dsl_gate::dsl_terrain_can_forage(terrain) {
            return val;
        }
    }
    terrain_can_forage_native(terrain)
}

fn terrain_can_forage_native(terrain: Terrain) -> bool {
    !matches!(terrain, Terrain::Ocean | Terrain::City | Terrain::Barren)
}

/// Foraging success chance (X-in-6). Returns 0 when foraging is impossible.
///
/// Forest/Jungle/River: 2-in-6   Clear/Hills/Swamp/Desert/Mountains: 1-in-6
pub fn terrain_forage_chance(terrain: Terrain) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Wilderness) {
        if let Some(val) = dsl_gate::dsl_terrain_forage_chance(terrain) {
            return val;
        }
    }
    terrain_forage_chance_native(terrain)
}

fn terrain_forage_chance_native(terrain: Terrain) -> u32 {
    match terrain {
        Terrain::Forest | Terrain::Jungle | Terrain::River => 2,
        Terrain::Clear | Terrain::Hills | Terrain::Swamp
        | Terrain::Desert | Terrain::Mountains => 1,
        _ => 0,
    }
}

// ── Hunting ───────────────────────────────────────────────────

/// Base hunting success chance (X-in-6).
/// Per OSE: always 1-in-6 regardless of terrain.
/// Hunting on the open ocean is rejected by the engine before this check.
pub fn hunt_chance() -> u32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Wilderness) {
        if let Some(val) = dsl_gate::dsl_hunt_chance() {
            return val;
        }
    }
    1
}

// ── Starvation ────────────────────────────────────────────────

/// Maximum starvation attack/save penalty.
/// Per OSE: -4 cap regardless of how many days without food.
pub fn starvation_penalty_cap() -> i32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Wilderness) {
        if let Some(val) = dsl_gate::dsl_starvation_penalty_cap() {
            return val;
        }
    }
    -4
}

/// Attack/save penalty for `days_without_food` days without food.
/// Scales as -1 per day, capped at [`starvation_penalty_cap`].
pub fn starvation_penalty(days_without_food: u32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Wilderness) {
        if let Some(val) = dsl_gate::dsl_starvation_penalty(days_without_food) {
            return val;
        }
    }
    starvation_penalty_native(days_without_food)
}

fn starvation_penalty_native(days_without_food: u32) -> i32 {
    -(days_without_food.min(4) as i32)
}

/// Die size for starvation HP damage (1d<N>). Returns the number of sides.
/// Per OSE: 1d4 HP damage per day after the damage threshold.
pub fn starvation_damage_die() -> u32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Wilderness) {
        if let Some(val) = dsl_gate::dsl_starvation_damage_die() {
            return val;
        }
    }
    4
}

/// Number of days without food before HP damage begins.
/// Per OSE: HP damage starts on the 3rd day without food.
pub fn starvation_damage_threshold() -> u32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Wilderness) {
        if let Some(val) = dsl_gate::dsl_starvation_damage_threshold() {
            return val;
        }
    }
    3
}

// ── DSL gate helpers ──────────────────────────────────────────

#[cfg(feature = "dsl-backend")]
mod dsl_gate {
    use std::collections::BTreeMap;

    use ttrpg_ast::Name;
    use ttrpg_interp::value::Value;

    use crate::bridge::handler::BridgeHandler;
    use crate::bridge::state::BridgeState;
    use crate::state::wilderness::Terrain;

    fn null_state() -> BridgeState {
        BridgeState::new(vec![], vec![], vec![], 0, 0)
    }

    fn terrain_to_value(terrain: Terrain) -> Value {
        let variant = match terrain {
            Terrain::Clear     => "t_clear",
            Terrain::Forest    => "t_forest",
            Terrain::Hills     => "t_hills",
            Terrain::Mountains => "t_mountains",
            Terrain::Desert    => "t_desert",
            Terrain::Swamp     => "t_swamp",
            Terrain::Jungle    => "t_jungle",
            Terrain::Ocean     => "t_ocean",
            Terrain::River     => "t_river",
            Terrain::Barren    => "t_barren",
            Terrain::City      => "t_city",
        };
        Value::EnumVariant {
            enum_name: "Terrain".into(),
            variant: Name::from(variant),
            fields: BTreeMap::new(),
        }
    }

    pub fn dsl_terrain_movement_cost(terrain: Terrain) -> Option<(u32, u32)> {
        let rt = crate::backend::dsl()?;
        let state = null_state();
        let terrain_val = terrain_to_value(terrain);

        let num = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "terrain_movement_cost_num",
                vec![terrain_val.clone()])
            .ok()?;
        let den = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "terrain_movement_cost_den",
                vec![terrain_val])
            .ok()?;

        match (num, den) {
            (Value::Int(n), Value::Int(d)) => Some((n as u32, d as u32)),
            _ => None,
        }
    }

    pub fn dsl_hexes_per_day(movement_rate: u32, terrain: Terrain) -> Option<u32> {
        let rt = crate::backend::dsl()?;
        let state = null_state();
        let result = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "hexes_per_day",
                vec![Value::Int(movement_rate as i64), terrain_to_value(terrain)])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_terrain_lost_chance(terrain: Terrain) -> Option<u32> {
        let rt = crate::backend::dsl()?;
        let state = null_state();
        let result = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "terrain_lost_chance",
                vec![terrain_to_value(terrain)])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_terrain_can_forage(terrain: Terrain) -> Option<bool> {
        let rt = crate::backend::dsl()?;
        let state = null_state();
        let result = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "terrain_can_forage",
                vec![terrain_to_value(terrain)])
            .ok()?;
        match result {
            Value::Bool(v) => Some(v),
            _ => None,
        }
    }

    pub fn dsl_terrain_forage_chance(terrain: Terrain) -> Option<u32> {
        let rt = crate::backend::dsl()?;
        let state = null_state();
        let result = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "terrain_forage_chance",
                vec![terrain_to_value(terrain)])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_hunt_chance() -> Option<u32> {
        let rt = crate::backend::dsl()?;
        let state = null_state();
        let result = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "hunt_chance", vec![])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_starvation_penalty_cap() -> Option<i32> {
        let rt = crate::backend::dsl()?;
        let state = null_state();
        let result = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "starvation_penalty_cap", vec![])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as i32),
            _ => None,
        }
    }

    pub fn dsl_starvation_penalty(days_without_food: u32) -> Option<i32> {
        let rt = crate::backend::dsl()?;
        let state = null_state();
        let result = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "starvation_penalty",
                vec![Value::Int(days_without_food as i64)])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as i32),
            _ => None,
        }
    }

    pub fn dsl_starvation_damage_die() -> Option<u32> {
        let rt = crate::backend::dsl()?;
        let state = null_state();
        let result = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "starvation_damage_die", vec![])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }

    pub fn dsl_starvation_damage_threshold() -> Option<u32> {
        let rt = crate::backend::dsl()?;
        let state = null_state();
        let result = rt
            .evaluate_derive(&state, &mut BridgeHandler::new(), "starvation_damage_threshold",
                vec![])
            .ok()?;
        match result {
            Value::Int(v) => Some(v as u32),
            _ => None,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_cost_clear() {
        assert_eq!(terrain_movement_cost(Terrain::Clear), (1, 1));
        assert_eq!(terrain_movement_cost(Terrain::City), (1, 1));
        assert_eq!(terrain_movement_cost(Terrain::Ocean), (1, 1));
    }

    #[test]
    fn movement_cost_medium() {
        assert_eq!(terrain_movement_cost(Terrain::Hills), (3, 2));
        assert_eq!(terrain_movement_cost(Terrain::Forest), (3, 2));
        assert_eq!(terrain_movement_cost(Terrain::River), (3, 2));
        assert_eq!(terrain_movement_cost(Terrain::Barren), (3, 2));
    }

    #[test]
    fn movement_cost_hard() {
        assert_eq!(terrain_movement_cost(Terrain::Desert), (2, 1));
        assert_eq!(terrain_movement_cost(Terrain::Swamp), (2, 1));
        assert_eq!(terrain_movement_cost(Terrain::Jungle), (2, 1));
    }

    #[test]
    fn movement_cost_mountains() {
        assert_eq!(terrain_movement_cost(Terrain::Mountains), (3, 1));
    }

    #[test]
    fn hexes_per_day_clear_120() {
        // 120' on clear: 24 miles/day / 6 = 4 hexes
        assert_eq!(hexes_per_day(120, Terrain::Clear), 4);
    }

    #[test]
    fn hexes_per_day_forest_120() {
        // 120' on forest (3/2 cost): 24 * 2/3 = 16 / 6 = 2 hexes
        assert_eq!(hexes_per_day(120, Terrain::Forest), 2);
    }

    #[test]
    fn hexes_per_day_mountains_120() {
        // 120' on mountains (3/1 cost): 24 / 3 = 8 / 6 = 1 hex
        assert_eq!(hexes_per_day(120, Terrain::Mountains), 1);
    }

    #[test]
    fn hexes_per_day_minimum_one() {
        // Even slowest terrain and slowest movement gives ≥ 1 hex
        assert!(hexes_per_day(30, Terrain::Mountains) >= 1);
    }

    #[test]
    fn lost_chance_easy_terrain() {
        assert_eq!(terrain_lost_chance(Terrain::Clear), 1);
        assert_eq!(terrain_lost_chance(Terrain::City), 1);
        assert_eq!(terrain_lost_chance(Terrain::Barren), 1);
        assert_eq!(terrain_lost_chance(Terrain::Hills), 1);
        assert_eq!(terrain_lost_chance(Terrain::Mountains), 1);
    }

    #[test]
    fn lost_chance_hard_terrain() {
        assert_eq!(terrain_lost_chance(Terrain::Forest), 2);
        assert_eq!(terrain_lost_chance(Terrain::River), 2);
        assert_eq!(terrain_lost_chance(Terrain::Desert), 2);
        assert_eq!(terrain_lost_chance(Terrain::Ocean), 2);
        assert_eq!(terrain_lost_chance(Terrain::Swamp), 2);
        assert_eq!(terrain_lost_chance(Terrain::Jungle), 2);
    }

    #[test]
    fn forage_possible_terrain() {
        assert!(terrain_can_forage(Terrain::Clear));
        assert!(terrain_can_forage(Terrain::Forest));
        assert!(terrain_can_forage(Terrain::Hills));
        assert!(terrain_can_forage(Terrain::Mountains));
        assert!(terrain_can_forage(Terrain::Desert));
        assert!(terrain_can_forage(Terrain::Swamp));
        assert!(terrain_can_forage(Terrain::Jungle));
        assert!(terrain_can_forage(Terrain::River));
    }

    #[test]
    fn forage_impossible_terrain() {
        assert!(!terrain_can_forage(Terrain::Ocean));
        assert!(!terrain_can_forage(Terrain::City));
        assert!(!terrain_can_forage(Terrain::Barren));
    }

    #[test]
    fn forage_chance_rich() {
        assert_eq!(terrain_forage_chance(Terrain::Forest), 2);
        assert_eq!(terrain_forage_chance(Terrain::Jungle), 2);
        assert_eq!(terrain_forage_chance(Terrain::River), 2);
    }

    #[test]
    fn forage_chance_sparse() {
        assert_eq!(terrain_forage_chance(Terrain::Clear), 1);
        assert_eq!(terrain_forage_chance(Terrain::Hills), 1);
        assert_eq!(terrain_forage_chance(Terrain::Swamp), 1);
        assert_eq!(terrain_forage_chance(Terrain::Desert), 1);
        assert_eq!(terrain_forage_chance(Terrain::Mountains), 1);
    }

    #[test]
    fn forage_chance_zero() {
        assert_eq!(terrain_forage_chance(Terrain::Ocean), 0);
        assert_eq!(terrain_forage_chance(Terrain::City), 0);
        assert_eq!(terrain_forage_chance(Terrain::Barren), 0);
    }

    #[test]
    fn hunt_chance_is_one() {
        assert_eq!(hunt_chance(), 1);
    }

    #[test]
    fn starvation_penalty_cap_is_minus_four() {
        assert_eq!(starvation_penalty_cap(), -4);
    }

    #[test]
    fn starvation_penalty_progression() {
        assert_eq!(starvation_penalty(0), 0);
        assert_eq!(starvation_penalty(1), -1);
        assert_eq!(starvation_penalty(2), -2);
        assert_eq!(starvation_penalty(3), -3);
        assert_eq!(starvation_penalty(4), -4);
        assert_eq!(starvation_penalty(5), -4); // capped
        assert_eq!(starvation_penalty(10), -4);
    }

    #[test]
    fn starvation_damage_die_is_four() {
        assert_eq!(starvation_damage_die(), 4);
    }

    #[test]
    fn starvation_damage_threshold_is_three() {
        assert_eq!(starvation_damage_threshold(), 3);
    }
}

// ── DSL backend tests ─────────────────────────────────────────
//
// Verify that DSL derives produce identical results to the native
// Rust implementations. Run with:
//   OSR_BACKEND_WILDERNESS=dsl cargo test --features dsl-backend

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_tests {
    use super::dsl_gate;
    use super::*;

    #[test]
    fn dsl_movement_cost_matches_native() {
        for terrain in all_terrains() {
            let native = terrain_movement_cost_native(terrain);
            let dsl = dsl_gate::dsl_terrain_movement_cost(terrain)
                .unwrap_or_else(|| panic!("DSL returned None for {:?}", terrain));
            assert_eq!(native, dsl, "movement_cost mismatch for {:?}", terrain);
        }
    }

    #[test]
    fn dsl_hexes_per_day_matches_native() {
        for terrain in all_terrains() {
            for rate in [30, 60, 90, 120, 150] {
                let native = hexes_per_day_native(rate, terrain);
                let dsl = dsl_gate::dsl_hexes_per_day(rate, terrain)
                    .unwrap_or_else(|| panic!("DSL returned None for {:?} rate={}", terrain, rate));
                assert_eq!(native, dsl,
                    "hexes_per_day mismatch for {:?} rate={}", terrain, rate);
            }
        }
    }

    #[test]
    fn dsl_lost_chance_matches_native() {
        for terrain in all_terrains() {
            let native = terrain_lost_chance_native(terrain);
            let dsl = dsl_gate::dsl_terrain_lost_chance(terrain)
                .unwrap_or_else(|| panic!("DSL returned None for {:?}", terrain));
            assert_eq!(native, dsl, "lost_chance mismatch for {:?}", terrain);
        }
    }

    #[test]
    fn dsl_can_forage_matches_native() {
        for terrain in all_terrains() {
            let native = terrain_can_forage_native(terrain);
            let dsl = dsl_gate::dsl_terrain_can_forage(terrain)
                .unwrap_or_else(|| panic!("DSL returned None for {:?}", terrain));
            assert_eq!(native, dsl, "can_forage mismatch for {:?}", terrain);
        }
    }

    #[test]
    fn dsl_forage_chance_matches_native() {
        for terrain in all_terrains() {
            let native = terrain_forage_chance_native(terrain);
            let dsl = dsl_gate::dsl_terrain_forage_chance(terrain)
                .unwrap_or_else(|| panic!("DSL returned None for {:?}", terrain));
            assert_eq!(native, dsl, "forage_chance mismatch for {:?}", terrain);
        }
    }

    #[test]
    fn dsl_hunt_chance_is_one() {
        let val = dsl_gate::dsl_hunt_chance().expect("DSL hunt_chance returned None");
        assert_eq!(val, 1);
    }

    #[test]
    fn dsl_starvation_penalty_matches_native() {
        for days in 0u32..=10 {
            let native = starvation_penalty_native(days);
            let dsl = dsl_gate::dsl_starvation_penalty(days)
                .unwrap_or_else(|| panic!("DSL returned None for days={}", days));
            assert_eq!(native, dsl, "starvation_penalty mismatch for days={}", days);
        }
    }

    #[test]
    fn dsl_starvation_constants() {
        assert_eq!(dsl_gate::dsl_starvation_penalty_cap().unwrap(), -4);
        assert_eq!(dsl_gate::dsl_starvation_damage_die().unwrap(), 4);
        assert_eq!(dsl_gate::dsl_starvation_damage_threshold().unwrap(), 3);
    }

    fn all_terrains() -> [Terrain; 11] {
        [
            Terrain::Clear, Terrain::Forest, Terrain::Hills,
            Terrain::Mountains, Terrain::Desert, Terrain::Swamp,
            Terrain::Jungle, Terrain::Ocean, Terrain::River,
            Terrain::Barren, Terrain::City,
        ]
    }
}
