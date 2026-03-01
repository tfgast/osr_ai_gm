use osr_ai_gm::dice as oag_dice;
use ttrpg_interp::value::{DiceExpr as InterpDice, RollResult as InterpRoll};

/// Convert a ttrpg_interp DiceExpr to an oag DiceExpr.
///
/// Returns `None` if the expression uses features oag doesn't support
/// (e.g. dice filters, or multi-group expressions like `1d20 + 1d6`).
pub fn interp_to_oag(expr: &InterpDice) -> Option<oag_dice::DiceExpr> {
    // oag only supports single-group dice expressions
    if expr.groups.len() != 1 {
        return None;
    }
    let group = &expr.groups[0];
    if group.filter.is_some() {
        return None;
    }
    if group.count == 1 && group.sides == 100 && expr.modifier == 0 {
        return Some(oag_dice::DiceExpr::Percentile);
    }
    Some(oag_dice::DiceExpr::Standard {
        count: group.count,
        sides: group.sides,
        modifier: expr.modifier as i32,
    })
}

/// Convert an oag RollResult back to a ttrpg_interp RollResult,
/// preserving the original ttrpg_interp DiceExpr.
pub fn oag_to_interp(oag: &oag_dice::RollResult, original_expr: &InterpDice) -> InterpRoll {
    let dice: Vec<i64> = oag.rolls.iter().map(|&r| r as i64).collect();
    let unmodified: i64 = dice.iter().sum();
    InterpRoll {
        expr: original_expr.clone(),
        dice: dice.clone(),
        kept: dice,
        modifier: oag.modifier as i64,
        total: oag.total as i64,
        unmodified,
    }
}

/// Roll a ttrpg_interp DiceExpr using oag's dice system.
///
/// Falls back to direct rolling for expressions oag can't handle
/// (e.g. dice filters or multi-group expressions).
pub fn roll_interp_dice(expr: &InterpDice) -> InterpRoll {
    if let Some(oag_expr) = interp_to_oag(expr) {
        let oag_result = oag_dice::roll(&oag_expr);
        oag_to_interp(&oag_result, expr)
    } else {
        // Fallback: roll directly for filtered/multi-group dice
        roll_directly(expr)
    }
}

/// Roll dice directly, supporting filters and multi-group expressions.
fn roll_directly(expr: &InterpDice) -> InterpRoll {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let mut all_dice = Vec::new();
    let mut all_kept = Vec::new();

    for group in &expr.groups {
        let mut dice: Vec<i64> = (0..group.count)
            .map(|_| rng.gen_range(1..=group.sides as i64))
            .collect();

        let kept = if let Some(filter) = &group.filter {
            let mut sorted = dice.clone();
            sorted.sort();
            match *filter {
                ttrpg_ast::DiceFilter::KeepHighest(n) => {
                    sorted.into_iter().rev().take(n as usize).collect()
                }
                ttrpg_ast::DiceFilter::KeepLowest(n) => {
                    sorted.into_iter().take(n as usize).collect()
                }
                ttrpg_ast::DiceFilter::DropHighest(n) => {
                    let len = sorted.len();
                    sorted.into_iter().take(len.saturating_sub(n as usize)).collect()
                }
                ttrpg_ast::DiceFilter::DropLowest(n) => {
                    sorted.into_iter().skip(n as usize).collect()
                }
            }
        } else {
            dice.clone()
        };

        // Sort original dice for consistent display
        dice.sort();
        all_dice.extend(dice);
        all_kept.extend(kept);
    }

    let unmodified: i64 = all_kept.iter().sum();
    let total = unmodified + expr.modifier;

    InterpRoll {
        expr: expr.clone(),
        dice: all_dice,
        kept: all_kept,
        modifier: expr.modifier,
        total,
        unmodified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttrpg_interp::value::DiceGroup;

    fn single_group(count: u32, sides: u32, filter: Option<ttrpg_ast::DiceFilter>, modifier: i64) -> InterpDice {
        InterpDice::single(count, sides, filter, modifier)
    }

    #[test]
    fn convert_standard_dice() {
        let interp = single_group(2, 6, None, 3);
        let oag = interp_to_oag(&interp).unwrap();
        assert_eq!(
            oag,
            oag_dice::DiceExpr::Standard {
                count: 2,
                sides: 6,
                modifier: 3
            }
        );
    }

    #[test]
    fn convert_percentile() {
        let interp = single_group(1, 100, None, 0);
        let oag = interp_to_oag(&interp).unwrap();
        assert_eq!(oag, oag_dice::DiceExpr::Percentile);
    }

    #[test]
    fn filtered_dice_not_convertible() {
        let interp = single_group(4, 6, Some(ttrpg_ast::DiceFilter::KeepHighest(3)), 0);
        assert!(interp_to_oag(&interp).is_none());
    }

    #[test]
    fn multi_group_not_convertible() {
        let interp = InterpDice {
            groups: vec![
                DiceGroup { count: 1, sides: 20, filter: None },
                DiceGroup { count: 1, sides: 6, filter: None },
            ],
            modifier: 5,
        };
        assert!(interp_to_oag(&interp).is_none());
    }

    #[test]
    fn roll_standard_dice_bounds() {
        let expr = single_group(3, 6, None, 0);
        for _ in 0..100 {
            let result = roll_interp_dice(&expr);
            assert!(result.total >= 3 && result.total <= 18);
            assert_eq!(result.dice.len(), 3);
            assert_eq!(result.kept.len(), 3);
            assert_eq!(result.modifier, 0);
        }
    }

    #[test]
    fn roll_filtered_dice() {
        let expr = single_group(4, 6, Some(ttrpg_ast::DiceFilter::KeepHighest(3)), 0);
        for _ in 0..100 {
            let result = roll_interp_dice(&expr);
            assert_eq!(result.dice.len(), 4);
            assert_eq!(result.kept.len(), 3);
            assert!(result.total >= 3 && result.total <= 18);
        }
    }

    #[test]
    fn roll_multi_group() {
        let expr = InterpDice {
            groups: vec![
                DiceGroup { count: 1, sides: 20, filter: None },
                DiceGroup { count: 1, sides: 6, filter: None },
            ],
            modifier: 2,
        };
        for _ in 0..100 {
            let result = roll_interp_dice(&expr);
            assert_eq!(result.dice.len(), 2);
            assert_eq!(result.kept.len(), 2);
            // 1d20 (1-20) + 1d6 (1-6) + 2 = 4..28
            assert!(result.total >= 4 && result.total <= 28);
            assert_eq!(result.modifier, 2);
        }
    }

    #[test]
    fn roundtrip_preserves_expr() {
        let interp_expr = single_group(1, 20, None, 5);
        let result = roll_interp_dice(&interp_expr);
        assert_eq!(result.expr, interp_expr);
        assert_eq!(result.modifier, 5);
    }
}
