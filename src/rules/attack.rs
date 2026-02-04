/// Attack resolution per OSE combat rules (Reference Booklet p19-20).
///
/// Uses THAC0 (To Hit AC 0) system with descending AC.
/// Attack succeeds when d20 + modifiers >= THAC0 - target_AC.
/// Natural 1 always misses, natural 20 always hits.

/// Calculate the target number needed on d20 to hit a given AC.
/// target_number = THAC0 - target_AC
///
/// Example: THAC0 19 vs AC 5 (chain mail) = need 14 on d20.
pub fn target_number(thac0: u32, target_ac: i32) -> i32 {
    thac0 as i32 - target_ac
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
    if distance == 0 || short == 0 {
        None // not a missile weapon or invalid range
    } else if distance <= short {
        Some(1) // +1 at short range
    } else if distance <= medium {
        Some(0) // no modifier at medium range
    } else if distance <= long {
        Some(-1) // -1 at long range
    } else {
        None // out of range
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
        _ => 10,
    }
}

/// Parse a monster's hit dice string to get the HD number.
///
/// Handles common formats:
/// - `"2"` → 2
/// - `"1+1"` → 1 (bonus HP ignored for THAC0/turning)
/// - `"3*"` → 3 (asterisk for special abilities ignored)
/// - `"1/2"` → 1 (fractional HD treated as 1)
/// - `"1-1"` → 1 (negative bonus ignored)
pub fn parse_monster_hd(hd_str: &str) -> u32 {
    let s = hd_str.trim();
    // Handle fractional HD like "1/2"
    if s.contains('/') {
        return 1;
    }
    // Take leading digits before any +, -, or * modifier
    let num_str: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    let n = num_str.parse().unwrap_or(1);
    if n == 0 { 1 } else { n }
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

    // --- parse_monster_hd ---

    #[test]
    fn parse_hd_simple() {
        assert_eq!(parse_monster_hd("1"), 1);
        assert_eq!(parse_monster_hd("2"), 2);
        assert_eq!(parse_monster_hd("5"), 5);
        assert_eq!(parse_monster_hd("10"), 10);
    }

    #[test]
    fn parse_hd_with_bonus() {
        assert_eq!(parse_monster_hd("1+1"), 1);
        assert_eq!(parse_monster_hd("3+1"), 3);
        assert_eq!(parse_monster_hd("2+2"), 2);
    }

    #[test]
    fn parse_hd_with_penalty() {
        assert_eq!(parse_monster_hd("1-1"), 1);
    }

    #[test]
    fn parse_hd_with_asterisk() {
        assert_eq!(parse_monster_hd("3*"), 3);
        assert_eq!(parse_monster_hd("6**"), 6);
    }

    #[test]
    fn parse_hd_fractional() {
        assert_eq!(parse_monster_hd("1/2"), 1);
    }

    #[test]
    fn parse_hd_whitespace() {
        assert_eq!(parse_monster_hd("  3  "), 3);
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
    fn monster_thac0_20hd() {
        assert_eq!(monster_thac0(20), 10);
    }
}
