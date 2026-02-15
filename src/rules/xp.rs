//! XP thresholds and level advancement per OSE Rules Tome.
//! Each class has its own XP progression table.

use super::class::Class;

/// XP required to reach each level (index 0 = level 1 = 0 XP).
/// Tables go up to level 14 for most human classes, less for demihumans.
pub fn xp_for_level(class: Class, level: u32) -> u64 {
    let table = xp_table(class);
    if level == 0 { return 0; }
    let idx = (level as usize).saturating_sub(1);
    if idx < table.len() {
        table[idx]
    } else {
        // Beyond max level: return a very high value (effectively unreachable)
        u64::MAX
    }
}

/// Check if a character has enough XP to advance to the next level.
/// Returns the new level if advancement is possible, None otherwise.
pub fn check_level_up(class: Class, current_level: u32, xp: u64) -> Option<u32> {
    let max = super::class::class_def(class).max_level;
    if current_level >= max {
        return None;
    }
    let next = current_level + 1;
    let needed = xp_for_level(class, next);
    if needed != u64::MAX && xp >= needed {
        Some(next)
    } else {
        None
    }
}

/// Calculate XP bonus/penalty from prime requisite score.
/// Returns a percentage modifier: -20, -10, 0, +5, or +10.
pub fn prime_req_xp_modifier(class: Class, abilities: &[i32; 6]) -> i32 {
    use super::class::class_def;
    use super::ability::prime_req_xp_mod;

    let def = class_def(class);
    if def.prime_requisites.is_empty() {
        return 0;
    }
    // For classes with multiple prime requisites, use the lowest score
    let min_score = def.prime_requisites.iter()
        .map(|&idx| abilities[idx])
        .min()
        .unwrap_or(10);
    prime_req_xp_mod(min_score)
}

/// Apply XP modifier and return adjusted XP amount.
/// modifier is a percentage: -20, -10, 0, 5, 10
pub fn adjust_xp(base_xp: u64, modifier_pct: i32) -> u64 {
    if modifier_pct == 0 {
        return base_xp;
    }
    let adjusted = base_xp as f64 * (1.0 + modifier_pct as f64 / 100.0);
    adjusted.round().max(0.0) as u64
}

/// XP table for each class. Index 0 = level 1 (0 XP), index 1 = level 2, etc.
fn xp_table(class: Class) -> &'static [u64] {
    match class {
        // Fighter: 0, 2000, 4000, 8000, 16000, 32000, 64000, 120000, 240000, 360000, 480000, 600000, 720000, 840000
        Class::Fighter | Class::Knight => &[
            0, 2_000, 4_000, 8_000, 16_000, 32_000, 64_000,
            120_000, 240_000, 360_000, 480_000, 600_000, 720_000, 840_000,
        ],
        // Thief: 0, 1200, 2400, 4800, 9600, 20000, 40000, 80000, 160000, 280000, 400000, 520000, 640000, 760000
        Class::Thief | Class::Acrobat | Class::Assassin => &[
            0, 1_200, 2_400, 4_800, 9_600, 20_000, 40_000,
            80_000, 160_000, 280_000, 400_000, 520_000, 640_000, 760_000,
        ],
        // Cleric: 0, 1500, 3000, 6000, 12000, 25000, 50000, 100000, 200000, 300000, 400000, 500000, 600000, 700000
        Class::Cleric | Class::Druid => &[
            0, 1_500, 3_000, 6_000, 12_000, 25_000, 50_000,
            100_000, 200_000, 300_000, 400_000, 500_000, 600_000, 700_000,
        ],
        // Magic-User: 0, 2500, 5000, 10000, 20000, 40000, 80000, 150000, 300000, 450000, 600000, 750000, 900000, 1050000
        Class::MagicUser | Class::Illusionist => &[
            0, 2_500, 5_000, 10_000, 20_000, 40_000, 80_000,
            150_000, 300_000, 450_000, 600_000, 750_000, 900_000, 1_050_000,
        ],
        // Dwarf: 0, 2200, 4400, 8800, 17000, 35000, 70000, 140000, 270000, 400000, 530000, 660000
        Class::Dwarf | Class::Duergar => &[
            0, 2_200, 4_400, 8_800, 17_000, 35_000, 70_000,
            140_000, 270_000, 400_000, 530_000, 660_000,
        ],
        // Elf: 0, 4000, 8000, 16000, 32000, 64000, 120000, 250000, 400000, 600000
        Class::Elf | Class::Drow => &[
            0, 4_000, 8_000, 16_000, 32_000, 64_000, 120_000,
            250_000, 400_000, 600_000,
        ],
        // Halfling: 0, 2000, 4000, 8000, 16000, 32000, 64000, 120000
        Class::Halfling => &[
            0, 2_000, 4_000, 8_000, 16_000, 32_000, 64_000, 120_000,
        ],
        // Half-Elf: 0, 2500, 5000, 10000, 20000, 40000, 80000, 150000, 300000, 400000, 500000, 600000
        Class::HalfElf => &[
            0, 2_500, 5_000, 10_000, 20_000, 40_000, 80_000,
            150_000, 300_000, 400_000, 500_000, 600_000,
        ],
        // Bard: 0, 2000, 4000, 8000, 16000, 32000, 64000, 120000, 240000, 360000, 480000, 600000, 720000, 840000
        Class::Bard => &[
            0, 2_000, 4_000, 8_000, 16_000, 32_000, 64_000,
            120_000, 240_000, 360_000, 480_000, 600_000, 720_000, 840_000,
        ],
        // Barbarian: 0, 2000, 4000, 8000, 16000, 32000, 64000, 120000, 240000, 360000, 480000, 600000, 720000, 840000
        Class::Barbarian => &[
            0, 2_000, 4_000, 8_000, 16_000, 32_000, 64_000,
            120_000, 240_000, 360_000, 480_000, 600_000, 720_000, 840_000,
        ],
        // Paladin: 0, 2750, 5500, 12000, 24000, 45000, 95000, 175000, 350000, 500000, 650000, 800000, 950000, 1100000
        Class::Paladin => &[
            0, 2_750, 5_500, 12_000, 24_000, 45_000, 95_000,
            175_000, 350_000, 500_000, 650_000, 800_000, 950_000, 1_100_000,
        ],
        // Ranger: 0, 2250, 4500, 10000, 20000, 40000, 90000, 150000, 325000, 475000, 625000, 775000, 925000, 1075000
        Class::Ranger => &[
            0, 2_250, 4_500, 10_000, 20_000, 40_000, 90_000,
            150_000, 325_000, 475_000, 625_000, 775_000, 925_000, 1_075_000,
        ],
        // Gnome: 0, 3000, 6000, 12000, 25000, 50000, 100000, 200000
        Class::Gnome => &[
            0, 3_000, 6_000, 12_000, 25_000, 50_000, 100_000, 200_000,
        ],
        // Half-Orc: 0, 1500, 3000, 6000, 12000, 25000, 50000, 100000
        Class::HalfOrc => &[
            0, 1_500, 3_000, 6_000, 12_000, 25_000, 50_000, 100_000,
        ],
        // Svirfneblin: 0, 3000, 6000, 12000, 24000, 48000, 100000, 200000
        Class::Svirfneblin => &[
            0, 3_000, 6_000, 12_000, 24_000, 48_000, 100_000, 200_000,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fighter_xp_table() {
        assert_eq!(xp_for_level(Class::Fighter, 1), 0);
        assert_eq!(xp_for_level(Class::Fighter, 2), 2_000);
        assert_eq!(xp_for_level(Class::Fighter, 3), 4_000);
        assert_eq!(xp_for_level(Class::Fighter, 9), 240_000);
        assert_eq!(xp_for_level(Class::Fighter, 14), 840_000);
    }

    #[test]
    fn thief_xp_table() {
        assert_eq!(xp_for_level(Class::Thief, 1), 0);
        assert_eq!(xp_for_level(Class::Thief, 2), 1_200);
        assert_eq!(xp_for_level(Class::Thief, 5), 9_600);
    }

    #[test]
    fn cleric_xp_table() {
        assert_eq!(xp_for_level(Class::Cleric, 1), 0);
        assert_eq!(xp_for_level(Class::Cleric, 2), 1_500);
        assert_eq!(xp_for_level(Class::Cleric, 14), 700_000);
    }

    #[test]
    fn magic_user_xp_table() {
        assert_eq!(xp_for_level(Class::MagicUser, 1), 0);
        assert_eq!(xp_for_level(Class::MagicUser, 2), 2_500);
    }

    #[test]
    fn dwarf_xp_table() {
        assert_eq!(xp_for_level(Class::Dwarf, 1), 0);
        assert_eq!(xp_for_level(Class::Dwarf, 2), 2_200);
        assert_eq!(xp_for_level(Class::Dwarf, 12), 660_000);
    }

    #[test]
    fn beyond_max_level() {
        // Halfling max level 8, level 9 should return MAX
        assert_eq!(xp_for_level(Class::Halfling, 9), u64::MAX);
    }

    #[test]
    fn check_level_up_ready() {
        assert_eq!(check_level_up(Class::Fighter, 1, 2_000), Some(2));
        assert_eq!(check_level_up(Class::Fighter, 1, 3_000), Some(2));
    }

    #[test]
    fn check_level_up_not_ready() {
        assert_eq!(check_level_up(Class::Fighter, 1, 1_999), None);
    }

    #[test]
    fn check_level_up_at_max() {
        assert_eq!(check_level_up(Class::Halfling, 8, 999_999), None);
    }

    #[test]
    fn duergar_cannot_exceed_max_level() {
        // Duergar max_level=10 but shares Dwarf XP table (12 levels).
        // Should not level past 10 even with enough XP.
        assert_eq!(check_level_up(Class::Duergar, 10, 999_999), None);
    }

    #[test]
    fn prime_req_modifier_fighter_high_str() {
        // Fighter prime req is STR. STR 16 = +10%
        let abilities = [16, 10, 10, 10, 10, 10];
        assert_eq!(prime_req_xp_modifier(Class::Fighter, &abilities), 10);
    }

    #[test]
    fn prime_req_modifier_fighter_low_str() {
        let abilities = [5, 10, 10, 10, 10, 10];
        assert_eq!(prime_req_xp_modifier(Class::Fighter, &abilities), -20);
    }

    #[test]
    fn prime_req_modifier_elf_dual() {
        // Elf has STR + INT prime requisites, uses lowest
        let abilities = [16, 10, 10, 10, 10, 10]; // INT 10 is the low one
        assert_eq!(prime_req_xp_modifier(Class::Elf, &abilities), 0);
    }

    #[test]
    fn adjust_xp_with_bonus() {
        assert_eq!(adjust_xp(1000, 10), 1100);
        assert_eq!(adjust_xp(1000, 5), 1050);
    }

    #[test]
    fn adjust_xp_with_penalty() {
        assert_eq!(adjust_xp(1000, -10), 900);
        assert_eq!(adjust_xp(1000, -20), 800);
    }

    #[test]
    fn adjust_xp_zero_mod() {
        assert_eq!(adjust_xp(1000, 0), 1000);
    }
}
