use osr_ai_gm::dice as oag_dice;
use ttrpg_interp::value::{DiceExpr as InterpDice, RollResult as InterpRoll};

/// Convert a ttrpg_interp DiceExpr to an oag DiceExpr.
///
/// Returns `None` if the expression uses features oag doesn't support
/// (e.g. dice filters like keep-highest).
pub fn interp_to_oag(expr: &InterpDice) -> Option<oag_dice::DiceExpr> {
    if expr.filter.is_some() {
        return None;
    }
    if expr.count == 1 && expr.sides == 100 && expr.modifier == 0 {
        return Some(oag_dice::DiceExpr::Percentile);
    }
    Some(oag_dice::DiceExpr::Standard {
        count: expr.count,
        sides: expr.sides,
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
/// (e.g. dice filters).
pub fn roll_interp_dice(expr: &InterpDice) -> InterpRoll {
    if let Some(oag_expr) = interp_to_oag(expr) {
        let oag_result = oag_dice::roll(&oag_expr);
        oag_to_interp(&oag_result, expr)
    } else {
        // Fallback: roll directly for filtered dice
        roll_with_filter(expr)
    }
}

/// Roll dice with filter support (keep highest/lowest, etc.).
fn roll_with_filter(expr: &InterpDice) -> InterpRoll {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let mut dice: Vec<i64> = (0..expr.count)
        .map(|_| rng.gen_range(1..=expr.sides as i64))
        .collect();

    let kept = if let Some(filter) = &expr.filter {
        let mut sorted = dice.clone();
        sorted.sort();
        match filter {
            ttrpg_ast::DiceFilter::KeepHighest(n) => {
                let n = *n as usize;
                sorted.into_iter().rev().take(n).collect()
            }
            ttrpg_ast::DiceFilter::KeepLowest(n) => {
                let n = *n as usize;
                sorted.into_iter().take(n).collect()
            }
            ttrpg_ast::DiceFilter::DropHighest(n) => {
                let n = *n as usize;
                let len = sorted.len();
                sorted.into_iter().take(len.saturating_sub(n)).collect()
            }
            ttrpg_ast::DiceFilter::DropLowest(n) => {
                let n = *n as usize;
                sorted.into_iter().skip(n).collect()
            }
        }
    } else {
        dice.clone()
    };

    let unmodified: i64 = kept.iter().sum();
    let total = unmodified + expr.modifier;

    // Sort original dice for consistent display
    dice.sort();

    InterpRoll {
        expr: expr.clone(),
        dice,
        kept,
        modifier: expr.modifier,
        total,
        unmodified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_standard_dice() {
        let interp = InterpDice {
            count: 2,
            sides: 6,
            filter: None,
            modifier: 3,
        };
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
        let interp = InterpDice {
            count: 1,
            sides: 100,
            filter: None,
            modifier: 0,
        };
        let oag = interp_to_oag(&interp).unwrap();
        assert_eq!(oag, oag_dice::DiceExpr::Percentile);
    }

    #[test]
    fn filtered_dice_not_convertible() {
        let interp = InterpDice {
            count: 4,
            sides: 6,
            filter: Some(ttrpg_ast::DiceFilter::KeepHighest(3)),
            modifier: 0,
        };
        assert!(interp_to_oag(&interp).is_none());
    }

    #[test]
    fn roll_standard_dice_bounds() {
        let expr = InterpDice {
            count: 3,
            sides: 6,
            filter: None,
            modifier: 0,
        };
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
        let expr = InterpDice {
            count: 4,
            sides: 6,
            filter: Some(ttrpg_ast::DiceFilter::KeepHighest(3)),
            modifier: 0,
        };
        for _ in 0..100 {
            let result = roll_interp_dice(&expr);
            assert_eq!(result.dice.len(), 4);
            assert_eq!(result.kept.len(), 3);
            assert!(result.total >= 3 && result.total <= 18);
        }
    }

    #[test]
    fn roundtrip_preserves_expr() {
        let interp_expr = InterpDice {
            count: 1,
            sides: 20,
            filter: None,
            modifier: 5,
        };
        let result = roll_interp_dice(&interp_expr);
        assert_eq!(result.expr, interp_expr);
        assert_eq!(result.modifier, 5);
    }
}
