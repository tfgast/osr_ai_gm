use rand::Rng;
use std::fmt;

/// A parsed dice expression: XdY+Z, d%, or X-in-6.
#[derive(Debug, Clone, PartialEq)]
pub enum DiceExpr {
    /// Standard dice: count, sides, modifier (e.g., 2d6+3)
    Standard { count: u32, sides: u32, modifier: i32 },
    /// Percentile dice: d%
    Percentile,
    /// X-in-6 chance roll
    XIn6(u32),
}

/// Result of rolling dice.
#[derive(Debug, Clone)]
pub struct RollResult {
    pub expr: DiceExpr,
    pub rolls: Vec<u32>,
    pub modifier: i32,
    pub total: i32,
}

impl fmt::Display for RollResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.expr {
            DiceExpr::Standard { count, sides, modifier } => {
                write!(f, "{}d{}", count, sides)?;
                if *modifier > 0 {
                    write!(f, "+{}", modifier)?;
                } else if *modifier < 0 {
                    write!(f, "{}", modifier)?;
                }
                write!(f, ": {:?} = {}", self.rolls, self.total)
            }
            DiceExpr::Percentile => {
                write!(f, "d%: {}", self.total)
            }
            DiceExpr::XIn6(x) => {
                let success = self.total as u32 <= *x;
                write!(f, "{}-in-6: rolled {} — {}", x, self.total,
                    if success { "SUCCESS" } else { "FAILURE" })
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    InvalidFormat(String),
    InvalidNumber(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidFormat(s) => write!(f, "invalid dice format: {}", s),
            ParseError::InvalidNumber(s) => write!(f, "invalid number: {}", s),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a dice notation string into a DiceExpr.
///
/// Supported formats:
/// - `XdY` or `XdY+Z` or `XdY-Z` (standard dice)
/// - `dY` (shorthand for 1dY)
/// - `d%` (percentile)
/// - `X-in-6` (X-in-6 chance)
pub fn parse(input: &str) -> Result<DiceExpr, ParseError> {
    let s = input.trim().to_lowercase();

    // Check for X-in-6
    if let Some(x_str) = s.strip_suffix("-in-6") {
        let x: u32 = x_str.parse()
            .map_err(|_| ParseError::InvalidNumber(x_str.to_string()))?;
        if x == 0 || x > 6 {
            return Err(ParseError::InvalidFormat(
                format!("{}-in-6: x must be 1-6", x)));
        }
        return Ok(DiceExpr::XIn6(x));
    }

    // Check for d%
    if s == "d%" {
        return Ok(DiceExpr::Percentile);
    }

    // Standard dice: XdY+Z
    let d_pos = s.find('d')
        .ok_or_else(|| ParseError::InvalidFormat(s.clone()))?;

    let count: u32 = if d_pos == 0 {
        1
    } else {
        s[..d_pos].parse()
            .map_err(|_| ParseError::InvalidNumber(s[..d_pos].to_string()))?
    };

    let after_d = &s[d_pos + 1..];

    // Find modifier (+/-)
    let (sides_str, modifier) = if let Some(pos) = after_d.rfind('+') {
        let mod_str = &after_d[pos + 1..];
        let m: i32 = mod_str.parse()
            .map_err(|_| ParseError::InvalidNumber(mod_str.to_string()))?;
        (&after_d[..pos], m)
    } else if let Some(pos) = after_d.rfind('-') {
        let mod_str = &after_d[pos..];
        let m: i32 = mod_str.parse()
            .map_err(|_| ParseError::InvalidNumber(mod_str.to_string()))?;
        (&after_d[..pos], m)
    } else {
        (after_d, 0)
    };

    let sides: u32 = sides_str.parse()
        .map_err(|_| ParseError::InvalidNumber(sides_str.to_string()))?;

    if count == 0 || sides == 0 {
        return Err(ParseError::InvalidFormat(
            "count and sides must be > 0".to_string()));
    }

    Ok(DiceExpr::Standard { count, sides, modifier })
}

/// Roll a dice expression using the provided RNG.
pub fn roll_with<R: Rng>(expr: &DiceExpr, rng: &mut R) -> RollResult {
    match expr {
        DiceExpr::Standard { count, sides, modifier } => {
            let rolls: Vec<u32> = (0..*count)
                .map(|_| rng.gen_range(1..=*sides))
                .collect();
            let sum: u32 = rolls.iter().sum();
            let total = sum as i32 + modifier;
            RollResult {
                expr: expr.clone(),
                rolls,
                modifier: *modifier,
                total,
            }
        }
        DiceExpr::Percentile => {
            let val = rng.gen_range(1..=100);
            RollResult {
                expr: expr.clone(),
                rolls: vec![val],
                modifier: 0,
                total: val as i32,
            }
        }
        DiceExpr::XIn6(x) => {
            let val = rng.gen_range(1..=6u32);
            RollResult {
                expr: DiceExpr::XIn6(*x),
                rolls: vec![val],
                modifier: 0,
                total: val as i32,
            }
        }
    }
}

/// Roll a dice expression using thread_rng.
pub fn roll(expr: &DiceExpr) -> RollResult {
    roll_with(expr, &mut rand::thread_rng())
}

/// Parse and roll a dice notation string.
pub fn roll_str(input: &str) -> Result<RollResult, ParseError> {
    let expr = parse(input)?;
    Ok(roll(&expr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_dice() {
        assert_eq!(parse("2d6").unwrap(), DiceExpr::Standard { count: 2, sides: 6, modifier: 0 });
        assert_eq!(parse("1d20").unwrap(), DiceExpr::Standard { count: 1, sides: 20, modifier: 0 });
        assert_eq!(parse("d8").unwrap(), DiceExpr::Standard { count: 1, sides: 8, modifier: 0 });
    }

    #[test]
    fn parse_with_modifier() {
        assert_eq!(parse("2d6+3").unwrap(), DiceExpr::Standard { count: 2, sides: 6, modifier: 3 });
        assert_eq!(parse("1d20-2").unwrap(), DiceExpr::Standard { count: 1, sides: 20, modifier: -2 });
        assert_eq!(parse("3d8+10").unwrap(), DiceExpr::Standard { count: 3, sides: 8, modifier: 10 });
    }

    #[test]
    fn parse_percentile() {
        assert_eq!(parse("d%").unwrap(), DiceExpr::Percentile);
    }

    #[test]
    fn parse_x_in_6() {
        assert_eq!(parse("2-in-6").unwrap(), DiceExpr::XIn6(2));
        assert_eq!(parse("5-in-6").unwrap(), DiceExpr::XIn6(5));
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(parse("2D6").unwrap(), DiceExpr::Standard { count: 2, sides: 6, modifier: 0 });
        assert_eq!(parse("D%").unwrap(), DiceExpr::Percentile);
    }

    #[test]
    fn parse_errors() {
        assert!(parse("").is_err());
        assert!(parse("abc").is_err());
        assert!(parse("0d6").is_err());
        assert!(parse("2d0").is_err());
        assert!(parse("7-in-6").is_err());
        assert!(parse("0-in-6").is_err());
    }

    #[test]
    fn roll_standard_bounds() {
        let expr = parse("3d6").unwrap();
        for _ in 0..100 {
            let result = roll(&expr);
            assert!(result.total >= 3 && result.total <= 18);
            assert_eq!(result.rolls.len(), 3);
        }
    }

    #[test]
    fn roll_with_modifier_bounds() {
        let expr = parse("2d6+5").unwrap();
        for _ in 0..100 {
            let result = roll(&expr);
            assert!(result.total >= 7 && result.total <= 17);
        }
    }

    #[test]
    fn roll_percentile_bounds() {
        let expr = parse("d%").unwrap();
        for _ in 0..100 {
            let result = roll(&expr);
            assert!(result.total >= 1 && result.total <= 100);
        }
    }

    #[test]
    fn roll_x_in_6_bounds() {
        let expr = parse("3-in-6").unwrap();
        for _ in 0..100 {
            let result = roll(&expr);
            assert!(result.total >= 1 && result.total <= 6);
        }
    }

    #[test]
    fn roll_display() {
        let result = RollResult {
            expr: DiceExpr::Standard { count: 2, sides: 6, modifier: 3 },
            rolls: vec![4, 5],
            modifier: 3,
            total: 12,
        };
        let display = format!("{}", result);
        assert!(display.contains("2d6+3"));
        assert!(display.contains("12"));
    }
}
