// Attack resolution per OSE combat rules (Reference Booklet p19-20).
//
// Uses THAC0 (To Hit AC 0) system with descending AC.
// Attack succeeds when d20 + modifiers >= THAC0 - target_AC.
// Natural 1 always misses, natural 20 always hits.

use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize, Deserializer, Serializer};

/// Structured representation of monster Hit Dice.
///
/// Parses B/X-style HD notation:
/// - `"2"` — 2 HD
/// - `"1+1"` — 1 HD with +1 HP bonus
/// - `"1-1"` — less than 1 HD (attacks as Normal Human)
/// - `"3*"` — 3 HD with one special ability (affects XP)
/// - `"6**"` — 6 HD with two special abilities
/// - `"1/2"` or `"0.5"` — half a hit die
/// - `"7-9**"` — HD range (e.g., vampire), with special abilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitDice {
    /// Base HD number (leading digits).
    pub base: u32,
    /// HP modifier: positive for bonus (+1), negative for penalty (-1).
    /// For ranges like "7-9", this is 0 and `range_end` is set instead.
    pub modifier: i32,
    /// Number of special ability asterisks (0, 1, or 2).
    pub specials: u8,
    /// True if fractional HD (e.g., "1/2").
    pub fractional: bool,
    /// End of HD range for monsters like vampire ("7-9" → range_end = Some(9)).
    pub range_end: Option<u32>,
}

impl HitDice {
    /// The effective HD for combat (THAC0 and turning).
    ///
    /// - Fractional HD counts as 1
    /// - Bonuses are ignored (THAC0 based on base HD only)
    /// - Penalties subtract from base
    /// - Ranges return the midpoint
    pub fn combat_hd(&self) -> u32 {
        if self.fractional {
            return 1;
        }
        if let Some(end) = self.range_end {
            return (self.base + end) / 2;
        }
        if self.modifier < 0 {
            self.base.saturating_sub(self.modifier.unsigned_abs())
        } else {
            self.base
        }
    }

    /// Number of dice to roll for HP (0 for fractional — roll 1d4 instead).
    /// For range HD ("1 to 3"), randomly picks a value in [base, range_end].
    pub fn hp_dice_count(&self) -> u32 {
        if self.fractional {
            0
        } else if let Some(end) = self.range_end {
            use rand::Rng;
            rand::thread_rng().gen_range(self.base..=end)
        } else {
            self.base
        }
    }

    /// HP modifier applied after rolling dice (e.g. +1 for "3+1").
    pub fn hp_modifier(&self) -> i32 {
        self.modifier
    }
}

impl Default for HitDice {
    fn default() -> Self {
        HitDice {
            base: 1,
            modifier: 0,
            specials: 0,
            fractional: false,
            range_end: None,
        }
    }
}

impl FromStr for HitDice {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty hit dice string".to_string());
        }

        // Handle "X to Y" range notation (e.g. "1 to 3")
        if let Some(pos) = s.find(" to ") {
            let base: u32 = s[..pos].trim().parse()
                .map_err(|_| format!("invalid hit dice range: '{}'", s))?;
            let rest = &s[pos + 4..];
            let specials = rest.chars().rev().take_while(|c| *c == '*').count() as u8;
            let end_str = &rest[..rest.len() - specials as usize];
            let end: u32 = end_str.trim().parse()
                .map_err(|_| format!("invalid hit dice range end: '{}'", s))?;
            return Ok(HitDice { base, modifier: 0, specials, fractional: false, range_end: Some(end) });
        }

        // Handle fractional HD like "1/2", "½", or "0.5"
        if s.contains('/') || s.starts_with('½') || s.starts_with("0.5") {
            let specials = s.chars().rev().take_while(|c| *c == '*').count() as u8;
            return Ok(HitDice {
                base: 1,
                modifier: 0,
                specials,
                fractional: true,
                range_end: None,
            });
        }

        // Count and strip trailing asterisks
        let specials = s.chars().rev().take_while(|c| *c == '*').count() as u8;
        let s = &s[..s.len() - specials as usize];

        // Parse leading digits
        let num_str: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
        let base: u32 = num_str.parse().map_err(|_| format!("invalid hit dice: no base number in '{}'", s))?;

        let rest = &s[num_str.len()..];

        if rest.is_empty() {
            return Ok(HitDice { base, modifier: 0, specials, fractional: false, range_end: None });
        }

        if let Some(after_plus) = rest.strip_prefix('+') {
            let bonus: i32 = after_plus.trim().parse()
                .map_err(|_| format!("invalid hit dice modifier: '{}'", after_plus))?;
            return Ok(HitDice { base, modifier: bonus, specials, fractional: false, range_end: None });
        }

        if let Some(after_minus) = rest.strip_prefix('-') {
            let val: u32 = after_minus.trim().parse()
                .map_err(|_| format!("invalid hit dice modifier: '{}'", after_minus))?;
            if val > base {
                // Range like "7-9"
                return Ok(HitDice { base, modifier: 0, specials, fractional: false, range_end: Some(val) });
            } else {
                // Penalty like "1-1"
                return Ok(HitDice { base, modifier: -(val as i32), specials, fractional: false, range_end: None });
            }
        }

        Err(format!("invalid hit dice notation: '{}'", rest))
    }
}

impl fmt::Display for HitDice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.fractional {
            write!(f, "1/2")?;
        } else if let Some(end) = self.range_end {
            write!(f, "{}-{}", self.base, end)?;
        } else if self.modifier > 0 {
            write!(f, "{}+{}", self.base, self.modifier)?;
        } else if self.modifier < 0 {
            write!(f, "{}{}", self.base, self.modifier)?;
        } else {
            write!(f, "{}", self.base)?;
        }
        for _ in 0..self.specials {
            write!(f, "*")?;
        }
        Ok(())
    }
}

impl Serialize for HitDice {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for HitDice {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        HitDice::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Calculate the target number needed on d20 to hit a given AC.
/// target_number = THAC0 - target_AC
///
/// Example: THAC0 19 vs AC 5 (chain mail) = need 14 on d20.
pub fn target_number(thac0: u32, target_ac: i32) -> i32 {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some(v) = dsl_target_number(thac0, target_ac) {
            return v;
        }
    }
    native_target_number(thac0, target_ac)
}

fn native_target_number(thac0: u32, target_ac: i32) -> i32 {
    thac0 as i32 - target_ac
}

#[cfg(feature = "dsl-backend")]
fn dsl_target_number(thac0: u32, target_ac: i32) -> Option<i32> {
    use ttrpg_interp::value::Value;
    let runtime = crate::backend::dsl()?;
    let mut handler = crate::backend::SimpleDiceHandler::new();
    match runtime.evaluate_derive(
        &crate::backend::NullState,
        &mut handler,
        "target_number",
        vec![Value::Int(thac0 as i64), Value::Int(target_ac as i64)],
    ) {
        Ok(Value::Int(v)) => Some(v as i32),
        _ => None,
    }
}

/// Determine if an attack roll hits.
///
/// - `thac0`: attacker's THAC0 value
/// - `target_ac`: defender's AC (descending — lower is better)
/// - `modifiers`: sum of all attack modifiers (STR for melee, DEX for missile, etc.)
/// - `roll`: natural d20 result (before modifiers)
pub fn hits(thac0: u32, target_ac: i32, modifiers: i32, roll: u32) -> bool {
    if roll == 1 {
        return false; // Natural 1 always misses
    }
    if roll == 20 {
        return true; // Natural 20 always hits
    }
    (roll as i32 + modifiers) >= target_number(thac0, target_ac)
}

/// Determine missile attack range modifier per OSE rules.
///
/// - Short range: +1 to hit
/// - Medium range: no modifier
/// - Long range: -1 to hit
/// - Beyond long range: out of range (returns None)
///
/// Returns `None` if the target is out of range or weapon has no range (melee only).
pub fn missile_range_modifier(distance: u32, short: u32, medium: u32, long: u32) -> Option<i32> {
    // Out-of-range and invalid checks are always native (DSL doesn't handle None)
    if distance == 0 || short == 0 {
        return None;
    }
    if distance > long {
        return None;
    }
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some(v) = dsl_missile_range_mod(distance, short, medium, long) {
            return Some(v);
        }
    }
    Some(native_missile_range_modifier(distance, short, medium, long))
}

fn native_missile_range_modifier(distance: u32, short: u32, medium: u32, _long: u32) -> i32 {
    if distance <= short {
        1
    } else if distance <= medium {
        0
    } else {
        -1
    }
}

#[cfg(feature = "dsl-backend")]
fn dsl_missile_range_mod(distance: u32, short: u32, medium: u32, long: u32) -> Option<i32> {
    use ttrpg_interp::value::Value;
    let runtime = crate::backend::dsl()?;
    let mut handler = crate::backend::SimpleDiceHandler::new();
    match runtime.evaluate_derive(
        &crate::backend::NullState,
        &mut handler,
        "missile_range_mod",
        vec![
            Value::Int(distance as i64),
            Value::Int(short as i64),
            Value::Int(medium as i64),
            Value::Int(long as i64),
        ],
    ) {
        Ok(Value::Int(v)) => Some(v as i32),
        _ => None,
    }
}

/// THAC0 for monsters based on Hit Dice.
/// Monsters fight as martial (Fighter) combatants of equivalent HD level.
/// 0 HD (normal humans) use THAC0 20.
pub fn monster_thac0(hd: u32) -> u32 {
    if hd == 0 {
        return 20;
    }
    // Martial (Fighter) attack progression per OSE Reference Booklet p19
    match hd {
        1..=3 => 19,
        4..=6 => 17,
        7..=9 => 14,
        10..=12 => 12,
        13..=15 => 10,
        16..=18 => 8,
        _ => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- target_number ---

    #[test]
    fn target_number_unarmoured() {
        // THAC0 19 vs AC 9 = need 10
        assert_eq!(target_number(19, 9), 10);
    }

    #[test]
    fn target_number_chain() {
        // THAC0 19 vs AC 5 = need 14
        assert_eq!(target_number(19, 5), 14);
    }

    #[test]
    fn target_number_ac_zero() {
        assert_eq!(target_number(19, 0), 19);
    }

    #[test]
    fn target_number_negative_ac() {
        // THAC0 19 vs AC -3 = need 22 (very hard)
        assert_eq!(target_number(19, -3), 22);
    }

    #[test]
    fn target_number_better_thac0() {
        // THAC0 14 vs AC 5 = need 9
        assert_eq!(target_number(14, 5), 9);
    }

    // --- hits ---

    #[test]
    fn natural_1_always_misses() {
        // Even with huge bonus and easy target
        assert!(!hits(19, 9, 10, 1));
    }

    #[test]
    fn natural_20_always_hits() {
        // Even with terrible odds
        assert!(hits(19, -10, -5, 20));
    }

    #[test]
    fn standard_hit_exact() {
        // THAC0 19 vs AC 9, need 10, roll exactly 10
        assert!(hits(19, 9, 0, 10));
    }

    #[test]
    fn standard_hit_exceeds() {
        assert!(hits(19, 9, 0, 15));
    }

    #[test]
    fn standard_miss() {
        // THAC0 19 vs AC 9, need 10, roll 9
        assert!(!hits(19, 9, 0, 9));
    }

    #[test]
    fn hit_with_str_modifier() {
        // THAC0 19 vs AC 5, +2 STR mod, need 14
        // Roll 12 + 2 = 14, hits
        assert!(hits(19, 5, 2, 12));
        // Roll 11 + 2 = 13, misses
        assert!(!hits(19, 5, 2, 11));
    }

    #[test]
    fn hit_with_negative_modifier() {
        // THAC0 19 vs AC 9, -2 mod, need 10
        // Roll 12 - 2 = 10, hits
        assert!(hits(19, 9, -2, 12));
        // Roll 11 - 2 = 9, misses
        assert!(!hits(19, 9, -2, 11));
    }

    // --- missile_range_modifier ---

    #[test]
    fn missile_short_range() {
        // Short bow: 50/100/150
        assert_eq!(missile_range_modifier(30, 50, 100, 150), Some(1));
    }

    #[test]
    fn missile_short_range_boundary() {
        assert_eq!(missile_range_modifier(50, 50, 100, 150), Some(1));
    }

    #[test]
    fn missile_medium_range() {
        assert_eq!(missile_range_modifier(75, 50, 100, 150), Some(0));
    }

    #[test]
    fn missile_medium_range_boundary() {
        assert_eq!(missile_range_modifier(100, 50, 100, 150), Some(0));
    }

    #[test]
    fn missile_long_range() {
        assert_eq!(missile_range_modifier(120, 50, 100, 150), Some(-1));
    }

    #[test]
    fn missile_long_range_boundary() {
        assert_eq!(missile_range_modifier(150, 50, 100, 150), Some(-1));
    }

    #[test]
    fn missile_out_of_range() {
        assert_eq!(missile_range_modifier(200, 50, 100, 150), None);
    }

    #[test]
    fn missile_melee_only_weapon() {
        // Range (0,0,0) = melee only
        assert_eq!(missile_range_modifier(10, 0, 0, 0), None);
    }

    #[test]
    fn missile_zero_distance() {
        assert_eq!(missile_range_modifier(0, 50, 100, 150), None);
    }

    // --- monster_thac0 ---

    #[test]
    fn monster_thac0_normal_human() {
        assert_eq!(monster_thac0(0), 20);
    }

    #[test]
    fn monster_thac0_1hd() {
        assert_eq!(monster_thac0(1), 19);
    }

    #[test]
    fn monster_thac0_3hd() {
        assert_eq!(monster_thac0(3), 19);
    }

    #[test]
    fn monster_thac0_4hd() {
        assert_eq!(monster_thac0(4), 17);
    }

    #[test]
    fn monster_thac0_7hd() {
        assert_eq!(monster_thac0(7), 14);
    }

    #[test]
    fn monster_thac0_10hd() {
        assert_eq!(monster_thac0(10), 12);
    }

    #[test]
    fn monster_thac0_13hd() {
        assert_eq!(monster_thac0(13), 10);
    }

    #[test]
    fn monster_thac0_16hd() {
        assert_eq!(monster_thac0(16), 8);
    }

    #[test]
    fn monster_thac0_20hd() {
        assert_eq!(monster_thac0(20), 6);
    }

    // --- HitDice ---

    #[test]
    fn hit_dice_simple() {
        let hd: HitDice = "2".parse().unwrap();
        assert_eq!(hd.base, 2);
        assert_eq!(hd.modifier, 0);
        assert_eq!(hd.specials, 0);
        assert_eq!(hd.combat_hd(), 2);
        assert_eq!(hd.to_string(), "2");
    }

    #[test]
    fn hit_dice_with_bonus() {
        let hd: HitDice = "3+1".parse().unwrap();
        assert_eq!(hd.base, 3);
        assert_eq!(hd.modifier, 1);
        assert_eq!(hd.combat_hd(), 3);
        assert_eq!(hd.to_string(), "3+1");
    }

    #[test]
    fn hit_dice_with_penalty() {
        let hd: HitDice = "1-1".parse().unwrap();
        assert_eq!(hd.base, 1);
        assert_eq!(hd.modifier, -1);
        assert_eq!(hd.combat_hd(), 0);
        assert_eq!(hd.to_string(), "1-1");
    }

    #[test]
    fn hit_dice_with_special() {
        let hd: HitDice = "4*".parse().unwrap();
        assert_eq!(hd.specials, 1);
        assert_eq!(hd.combat_hd(), 4);
        assert_eq!(hd.to_string(), "4*");
    }

    #[test]
    fn hit_dice_double_special() {
        let hd: HitDice = "6**".parse().unwrap();
        assert_eq!(hd.specials, 2);
        assert_eq!(hd.combat_hd(), 6);
        assert_eq!(hd.to_string(), "6**");
    }

    #[test]
    fn hit_dice_bonus_with_special() {
        let hd: HitDice = "6+3*".parse().unwrap();
        assert_eq!(hd.base, 6);
        assert_eq!(hd.modifier, 3);
        assert_eq!(hd.specials, 1);
        assert_eq!(hd.to_string(), "6+3*");
    }

    #[test]
    fn hit_dice_range() {
        let hd: HitDice = "7-9**".parse().unwrap();
        assert_eq!(hd.base, 7);
        assert_eq!(hd.range_end, Some(9));
        assert_eq!(hd.specials, 2);
        assert_eq!(hd.combat_hd(), 8);
        assert_eq!(hd.to_string(), "7-9**");
    }

    #[test]
    fn hp_dice_count_respects_range() {
        let hd: HitDice = "1 to 3".parse().unwrap();
        let mut saw_above_base = false;
        for _ in 0..100 {
            let count = hd.hp_dice_count();
            assert!(count >= 1 && count <= 3, "hp_dice_count {} out of range 1-3", count);
            if count > 1 {
                saw_above_base = true;
            }
        }
        assert!(saw_above_base, "100 rolls should produce at least one value above base HD");
    }

    #[test]
    fn hit_dice_fractional() {
        let hd: HitDice = "1/2".parse().unwrap();
        assert!(hd.fractional);
        assert_eq!(hd.combat_hd(), 1);
        assert_eq!(hd.to_string(), "1/2");
    }

    #[test]
    fn hit_dice_fractional_decimal() {
        let hd: HitDice = "0.5".parse().unwrap();
        assert!(hd.fractional);
        assert_eq!(hd.combat_hd(), 1);
        assert_eq!(hd.hp_dice_count(), 0);
        assert_eq!(hd.to_string(), "1/2");
    }

    #[test]
    fn hit_dice_fractional_decimal_with_special() {
        let hd: HitDice = "0.5*".parse().unwrap();
        assert!(hd.fractional);
        assert_eq!(hd.specials, 1);
        assert_eq!(hd.to_string(), "1/2*");
    }

    #[test]
    fn hit_dice_serde_roundtrip() {
        let hd: HitDice = "5+1*".parse().unwrap();
        let json = serde_json::to_string(&hd).unwrap();
        assert_eq!(json, "\"5+1*\"");
        let parsed: HitDice = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, hd);
    }
}
