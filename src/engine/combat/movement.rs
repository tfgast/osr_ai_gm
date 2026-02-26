//! Combat movement: close, fighting withdrawal, and retreat.

use std::fmt;

use rand::Rng;

use crate::model::{Character, CombatState};

use super::attack::{monster_attack_modified_with, AttackResult};

// ── DSL movement derives ──────────────────────────────────────

/// Compute encounter movement rate via DSL or native fallback.
/// Returns base_movement / 3.
fn encounter_move_rate(base_movement: u32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some(v) = dsl_encounter_movement(base_movement) {
            return v;
        }
    }
    #[cfg(feature = "legacy-native")]
    return base_movement / 3;
    #[cfg(not(feature = "legacy-native"))]
    panic!("Native fallback unavailable: enable the 'legacy-native' feature");
}

/// Compute fighting withdrawal distance via DSL or native fallback.
/// Returns encounter_movement / 2.
fn fighting_withdrawal_move(base_movement: u32) -> u32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some(v) = dsl_fighting_withdrawal_distance(base_movement) {
            return v;
        }
    }
    #[cfg(feature = "legacy-native")]
    return (base_movement / 3) / 2;
    #[cfg(not(feature = "legacy-native"))]
    panic!("Native fallback unavailable: enable the 'legacy-native' feature");
}

/// Return the melee engagement distance via DSL or native fallback (10 feet).
fn melee_engagement_dist() -> u32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some(v) = dsl_melee_engagement_distance() {
            return v;
        }
    }
    #[cfg(feature = "legacy-native")]
    return 10;
    #[cfg(not(feature = "legacy-native"))]
    panic!("Native fallback unavailable: enable the 'legacy-native' feature");
}

/// Return the retreat free attack bonus via DSL or native fallback (+2).
fn retreat_attack_bonus() -> i32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some(v) = dsl_retreat_free_attack_bonus() {
            return v;
        }
    }
    #[cfg(feature = "legacy-native")]
    return 2;
    #[cfg(not(feature = "legacy-native"))]
    panic!("Native fallback unavailable: enable the 'legacy-native' feature");
}

#[cfg(feature = "dsl-backend")]
fn dsl_encounter_movement(base_movement: u32) -> Option<u32> {
    use ttrpg_interp::value::Value;
    let runtime = crate::backend::dsl()?;
    let mut handler = crate::backend::SimpleDiceHandler::new();
    match runtime.evaluate_derive(
        &crate::backend::NullState,
        &mut handler,
        "encounter_movement",
        vec![Value::Int(base_movement as i64)],
    ) {
        Ok(Value::Int(v)) if v >= 0 => Some(v as u32),
        _ => None,
    }
}

#[cfg(feature = "dsl-backend")]
fn dsl_fighting_withdrawal_distance(base_movement: u32) -> Option<u32> {
    use ttrpg_interp::value::Value;
    let runtime = crate::backend::dsl()?;
    let mut handler = crate::backend::SimpleDiceHandler::new();
    match runtime.evaluate_derive(
        &crate::backend::NullState,
        &mut handler,
        "fighting_withdrawal_distance",
        vec![Value::Int(base_movement as i64)],
    ) {
        Ok(Value::Int(v)) if v >= 0 => Some(v as u32),
        _ => None,
    }
}

#[cfg(feature = "dsl-backend")]
fn dsl_melee_engagement_distance() -> Option<u32> {
    use ttrpg_interp::value::Value;
    let runtime = crate::backend::dsl()?;
    let mut handler = crate::backend::SimpleDiceHandler::new();
    match runtime.evaluate_derive(
        &crate::backend::NullState,
        &mut handler,
        "melee_engagement_distance",
        vec![],
    ) {
        Ok(Value::Int(v)) if v >= 0 => Some(v as u32),
        _ => None,
    }
}

#[cfg(feature = "dsl-backend")]
fn dsl_retreat_free_attack_bonus() -> Option<i32> {
    use ttrpg_interp::value::Value;
    let runtime = crate::backend::dsl()?;
    let mut handler = crate::backend::SimpleDiceHandler::new();
    match runtime.evaluate_derive(
        &crate::backend::NullState,
        &mut handler,
        "retreat_free_attack_bonus",
        vec![],
    ) {
        Ok(Value::Int(v)) => Some(v as i32),
        _ => None,
    }
}

/// Close distance toward the monsters.
///
/// Movement is capped by encounter movement rate (movement_rate / 3).
/// If `feet` is `None`, the character closes as far as possible (up to encounter move).
/// If `feet` is `Some(n)`, the character closes exactly `n` feet.
/// Returns an error if the requested distance exceeds the encounter movement rate.
/// Distance cannot go below 0.
pub fn close(combat: &mut CombatState, character: &Character, feet: Option<u32>) -> Result<String, String> {
    let encounter_move = encounter_move_rate(character.movement_rate);
    if encounter_move == 0 {
        return Err(format!("{} cannot move (movement rate 0).", character.name));
    }

    if combat.distance == 0 {
        return Err("already in melee range.".to_string());
    }

    let actual = match feet {
        Some(f) => {
            if f == 0 {
                return Err("distance to close must be positive.".to_string());
            }
            if f > encounter_move {
                return Err(format!(
                    "{} can only move {}' per round (encounter movement rate). Requested {}' is too far.",
                    character.name, encounter_move, f));
            }
            f.min(combat.distance)
        }
        None => encounter_move.min(combat.distance), // close as far as possible this round
    };

    combat.distance = combat.distance.saturating_sub(actual);

    let msg = format!("{} closes {}' (distance now {}')",
        character.name, actual, combat.distance);
    combat.log_event(msg.clone());
    Ok(msg)
}

/// Resolve fighting withdrawal for a character.
/// Half encounter movement speed backward; no free attacks from enemies.
/// Character can still defend but cannot attack this round.
pub fn fighting_withdrawal(combat: &mut CombatState, character: &Character) -> String {
    let half_move = fighting_withdrawal_move(character.movement_rate);
    combat.distance = combat.distance.saturating_add(half_move);
    let msg = format!("{} performs a fighting withdrawal ({}' backward, distance now {}')",
        character.name, half_move, combat.distance);
    combat.log_event(msg.clone());
    msg
}

/// Result of a retreat attempt with auto-resolved free attacks.
#[derive(Debug, Clone)]
pub struct RetreatResult {
    pub retreater: String,
    pub distance_moved: u32,
    pub new_distance: u32,
    pub free_attacks: Vec<AttackResult>,
}

impl fmt::Display for RetreatResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} retreats at full speed ({}', distance now {}').",
            self.retreater, self.distance_moved, self.new_distance)?;
        if self.free_attacks.is_empty() {
            write!(f, "No enemies in melee range for free attacks.")
        } else {
            writeln!(f, "Free attacks at +2:")?;
            for (i, atk) in self.free_attacks.iter().enumerate() {
                if i > 0 { writeln!(f)?; }
                write!(f, "  {}", atk)?;
            }
            Ok(())
        }
    }
}

/// Resolve retreat for a character.
/// Full encounter movement speed; enemies in melee get a free attack at +2.
/// All living monsters in melee range automatically execute free attacks.
pub fn retreat(combat: &mut CombatState, character: &mut Character) -> RetreatResult {
    retreat_with(combat, character, &mut rand::thread_rng())
}

/// Retreat with deterministic RNG for testing.
pub fn retreat_with<R: Rng>(
    combat: &mut CombatState,
    character: &mut Character,
    rng: &mut R,
) -> RetreatResult {
    let pre_retreat_distance = combat.distance;
    let encounter_move = encounter_move_rate(character.movement_rate);
    combat.distance = combat.distance.saturating_add(encounter_move);

    let retreat_msg = format!(
        "{} retreats at full speed ({}', distance now {}').",
        character.name, encounter_move, combat.distance);
    combat.log_event(retreat_msg);

    // Only monsters in melee range before retreat get a free attack at +retreat_attack_bonus
    let melee_dist = melee_engagement_dist();
    let free_atk_bonus = retreat_attack_bonus();
    let mut free_attacks = Vec::new();
    if pre_retreat_distance <= melee_dist {
        let alive_monster_indices: Vec<usize> = combat.monsters.iter()
            .enumerate()
            .filter(|(_, m)| m.is_alive() && !m.turned)
            .map(|(i, _)| i)
            .collect();

        for monster_idx in alive_monster_indices {
            // Character may have died from a previous free attack
            if !character.is_alive() {
                break;
            }
            let atk = monster_attack_modified_with(combat, monster_idx, character, free_atk_bonus, None, rng);
            free_attacks.push(atk);
        }
    }

    RetreatResult {
        retreater: character.name.clone(),
        distance_moved: encounter_move,
        new_distance: combat.distance,
        free_attacks,
    }
}

#[cfg(all(test, feature = "dsl-backend"))]
mod dsl_tests {
    use super::*;

    #[test]
    fn dsl_encounter_movement_120_gives_40() {
        let result = encounter_move_rate(120);
        assert_eq!(result, 40, "120' base movement should give 40' encounter movement");
    }

    #[test]
    fn dsl_encounter_movement_90_gives_30() {
        let result = encounter_move_rate(90);
        assert_eq!(result, 30);
    }

    #[test]
    fn dsl_fighting_withdrawal_120_gives_20() {
        // 120 / 3 = 40, 40 / 2 = 20
        let result = fighting_withdrawal_move(120);
        assert_eq!(result, 20, "fighting withdrawal for 120' base should be 20'");
    }

    #[test]
    fn dsl_melee_engagement_distance_is_10() {
        assert_eq!(melee_engagement_dist(), 10);
    }

    #[test]
    fn dsl_retreat_free_attack_bonus_is_2() {
        assert_eq!(retreat_attack_bonus(), 2);
    }
}
