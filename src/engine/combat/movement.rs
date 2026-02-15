//! Combat movement: close, fighting withdrawal, and retreat.

use std::fmt;

use rand::Rng;

use crate::model::{Character, CombatState};

use super::attack::{monster_attack_modified_with, AttackResult};

/// Close distance toward the monsters.
///
/// Movement is capped by encounter movement rate (movement_rate / 3).
/// If `feet` is `None`, the character closes as far as possible (up to encounter move).
/// If `feet` is `Some(n)`, the character closes exactly `n` feet.
/// Returns an error if the requested distance exceeds the encounter movement rate.
/// Distance cannot go below 0.
pub fn close(combat: &mut CombatState, character: &Character, feet: Option<u32>) -> Result<String, String> {
    let encounter_move = character.movement_rate / 3;
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
    let encounter_move = character.movement_rate / 3;
    let half_move = encounter_move / 2;
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
    let encounter_move = character.movement_rate / 3;
    combat.distance = combat.distance.saturating_add(encounter_move);

    let retreat_msg = format!(
        "{} retreats at full speed ({}', distance now {}').",
        character.name, encounter_move, combat.distance);
    combat.log_event(retreat_msg);

    // Only monsters in melee range (≤10') before retreat get a free attack at +2
    let mut free_attacks = Vec::new();
    if pre_retreat_distance <= 10 {
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
            let atk = monster_attack_modified_with(combat, monster_idx, character, 2, None, rng);
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
