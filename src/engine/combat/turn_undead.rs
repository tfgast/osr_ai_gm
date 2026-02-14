//! Turn undead resolution for clerics.

use std::fmt;

use rand::Rng;

use crate::model::{Character, CombatState};
use crate::rules::turn::{self, TurnResult};

/// Result of a turn undead attempt.
#[derive(Debug, Clone)]
pub struct TurnUndeadResult {
    pub cleric_name: String,
    pub cleric_level: u32,
    pub undead_type: String,
    pub undead_rank: u32,
    pub table_result: TurnResult,
    pub roll: Option<i32>,
    pub success: bool,
    pub hd_affected: u32,
    pub destroyed: bool,
}

impl fmt::Display for TurnUndeadResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} attempts to turn {} (rank {}): ",
            self.cleric_name, self.undead_type, self.undead_rank)?;
        match self.table_result {
            TurnResult::Impossible => {
                write!(f, "IMPOSSIBLE — cannot turn this undead type")
            }
            TurnResult::Roll(target) => {
                if let Some(roll) = self.roll {
                    if self.success {
                        write!(f, "2d6 = {} vs {} — TURNED! {} HD affected",
                            roll, target, self.hd_affected)
                    } else {
                        write!(f, "2d6 = {} vs {} — FAILED", roll, target)
                    }
                } else {
                    write!(f, "needs {} on 2d6", target)
                }
            }
            TurnResult::Turned => {
                write!(f, "AUTOMATIC TURN! {} HD affected", self.hd_affected)
            }
            TurnResult::Destroyed => {
                write!(f, "AUTOMATIC DESTRUCTION! {} HD affected", self.hd_affected)
            }
        }
    }
}

/// Resolve a cleric's turn undead attempt against a monster.
pub fn resolve_turn_undead(
    combat: &mut CombatState,
    cleric: &Character,
    cleric_level: u32,
    target_monster_idx: usize,
) -> TurnUndeadResult {
    resolve_turn_undead_with(combat, cleric, cleric_level, target_monster_idx,
        &mut rand::thread_rng())
}

pub fn resolve_turn_undead_with<R: Rng>(
    combat: &mut CombatState,
    cleric: &Character,
    cleric_level: u32,
    target_monster_idx: usize,
    rng: &mut R,
) -> TurnUndeadResult {
    let monster = &combat.monsters[target_monster_idx];
    let hd = monster.hit_dice.combat_hd();
    let rank = turn::undead_rank_from_hd(hd);
    let undead_type = monster.name.clone();

    let table_result = turn::turn_undead_result(cleric_level, rank);

    let (roll, success, hd_affected, destroyed) = match table_result {
        TurnResult::Impossible => (None, false, 0, false),
        TurnResult::Roll(target) => {
            let d1 = rng.gen_range(1..=6i32);
            let d2 = rng.gen_range(1..=6i32);
            let roll = d1 + d2;
            let success = roll >= target as i32;
            let hd_affected = if success {
                let h1 = rng.gen_range(1..=6u32);
                let h2 = rng.gen_range(1..=6u32);
                h1 + h2
            } else {
                0
            };
            (Some(roll), success, hd_affected, false)
        }
        TurnResult::Turned => {
            let h1 = rng.gen_range(1..=6u32);
            let h2 = rng.gen_range(1..=6u32);
            (None, true, h1 + h2, false)
        }
        TurnResult::Destroyed => {
            let h1 = rng.gen_range(1..=6u32);
            let h2 = rng.gen_range(1..=6u32);
            (None, true, h1 + h2, true)
        }
    };

    // Apply the turn/destroy effect to monsters
    if success && hd_affected > 0 {
        let mut remaining_hd = hd_affected;
        for m in combat.monsters.iter_mut() {
            if remaining_hd == 0 {
                break;
            }
            if !m.is_alive() || m.turned {
                continue;
            }
            let m_hd = m.hit_dice.combat_hd().max(1);
            if m_hd <= remaining_hd {
                remaining_hd -= m_hd;
                if destroyed {
                    m.hp = 0;
                } else {
                    m.turned = true;
                }
            }
        }
    }

    let result = TurnUndeadResult {
        cleric_name: cleric.name.clone(),
        cleric_level,
        undead_type,
        undead_rank: rank,
        table_result,
        roll,
        success,
        hd_affected,
        destroyed,
    };
    combat.log_event(result.to_string());
    result
}
