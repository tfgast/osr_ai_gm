//! Saving throw tables per OSE Reference Booklet p13.

/// Five saving throw categories (D, W, P, B, S).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SavingThrows {
    pub death: u32,
    pub wands: u32,
    pub paralysis: u32,
    pub breath: u32,
    pub spells: u32,
}

impl SavingThrows {
    pub const fn new(d: u32, w: u32, p: u32, b: u32, s: u32) -> Self {
        SavingThrows { death: d, wands: w, paralysis: p, breath: b, spells: s }
    }
}

/// Each class maps to one of these save table groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SaveCategory {
    Thief,       // Acrobat, Assassin, Bard, Thief
    Barbarian,
    Cleric,      // Cleric, Druid
    Drow,
    Dwarf,       // Duergar, Dwarf, Halfling
    Elf,
    Fighter,     // Fighter, Knight, Ranger
    Gnome,
    HalfElf,
    HalfOrc,
    MagicUser,   // Illusionist, Magic-User
    Paladin,
    Svirfneblin,
}

/// Look up saving throws by category and character level.
pub fn saving_throws(cat: SaveCategory, level: u32) -> SavingThrows {
    use SaveCategory::*;
    match cat {
        Thief => match level {
            0..=4 => SavingThrows::new(13, 14, 13, 16, 15),
            5..=8 => SavingThrows::new(12, 13, 11, 14, 13),
            9..=12 => SavingThrows::new(10, 11, 9, 12, 10),
            _ => SavingThrows::new(8, 9, 7, 10, 8),
        },
        Barbarian => match level {
            0..=3 => SavingThrows::new(10, 13, 12, 15, 16),
            4..=6 => SavingThrows::new(8, 11, 10, 13, 13),
            7..=9 => SavingThrows::new(6, 9, 8, 10, 10),
            10..=12 => SavingThrows::new(4, 7, 6, 8, 7),
            _ => SavingThrows::new(3, 5, 4, 5, 5),
        },
        Cleric => match level {
            0..=4 => SavingThrows::new(11, 12, 14, 16, 15),
            5..=8 => SavingThrows::new(9, 10, 12, 14, 12),
            9..=12 => SavingThrows::new(6, 7, 9, 11, 9),
            _ => SavingThrows::new(3, 5, 7, 8, 7),
        },
        Drow => match level {
            0..=3 => SavingThrows::new(12, 13, 13, 15, 12),
            4..=6 => SavingThrows::new(10, 11, 11, 13, 10),
            7..=9 => SavingThrows::new(8, 9, 9, 10, 8),
            _ => SavingThrows::new(6, 7, 8, 8, 6),
        },
        Dwarf => match level {
            0..=3 => SavingThrows::new(8, 9, 10, 13, 12),
            4..=6 => SavingThrows::new(6, 7, 8, 10, 10),
            7..=9 => SavingThrows::new(4, 5, 6, 7, 8),
            _ => SavingThrows::new(2, 3, 4, 4, 6),
        },
        Elf => match level {
            0..=3 => SavingThrows::new(12, 13, 13, 15, 15),
            4..=6 => SavingThrows::new(10, 11, 11, 13, 12),
            7..=9 => SavingThrows::new(8, 9, 9, 10, 10),
            _ => SavingThrows::new(6, 7, 8, 8, 8),
        },
        Fighter => match level {
            0..=3 => SavingThrows::new(12, 13, 14, 15, 16),
            4..=6 => SavingThrows::new(10, 11, 12, 13, 14),
            7..=9 => SavingThrows::new(8, 9, 10, 10, 12),
            10..=12 => SavingThrows::new(6, 7, 8, 8, 10),
            _ => SavingThrows::new(4, 5, 6, 5, 8),
        },
        Gnome => match level {
            0..=5 => SavingThrows::new(8, 9, 10, 14, 11),
            _ => SavingThrows::new(6, 7, 8, 11, 9),
        },
        HalfElf => match level {
            0..=3 => SavingThrows::new(12, 13, 13, 15, 15),
            4..=6 => SavingThrows::new(10, 11, 11, 13, 12),
            7..=9 => SavingThrows::new(8, 9, 9, 10, 10),
            _ => SavingThrows::new(6, 7, 8, 8, 8),
        },
        HalfOrc => match level {
            0..=4 => SavingThrows::new(13, 14, 13, 16, 15),
            _ => SavingThrows::new(12, 13, 11, 14, 13),
        },
        MagicUser => match level {
            0..=5 => SavingThrows::new(13, 14, 13, 16, 15),
            6..=10 => SavingThrows::new(11, 12, 11, 14, 12),
            _ => SavingThrows::new(8, 9, 8, 11, 8),
        },
        Paladin => match level {
            0..=3 => SavingThrows::new(10, 11, 12, 13, 14),
            4..=6 => SavingThrows::new(8, 9, 10, 11, 12),
            7..=9 => SavingThrows::new(6, 7, 8, 8, 10),
            10..=12 => SavingThrows::new(4, 5, 6, 6, 8),
            _ => SavingThrows::new(2, 3, 4, 3, 6),
        },
        Svirfneblin => match level {
            0..=3 => SavingThrows::new(8, 9, 10, 14, 11),
            4..=6 => SavingThrows::new(6, 7, 8, 11, 9),
            _ => SavingThrows::new(4, 5, 6, 9, 7),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_0_gets_worst_saves() {
        // Level 0 should use the first (worst) tier, not the last (best)
        let s = saving_throws(SaveCategory::Fighter, 0);
        assert_eq!(s, SavingThrows::new(12, 13, 14, 15, 16)); // same as level 1-3
        let s = saving_throws(SaveCategory::Thief, 0);
        assert_eq!(s, SavingThrows::new(13, 14, 13, 16, 15)); // same as level 1-4
    }

    #[test]
    fn thief_saves_level_1() {
        let s = saving_throws(SaveCategory::Thief, 1);
        assert_eq!(s, SavingThrows::new(13, 14, 13, 16, 15));
    }

    #[test]
    fn thief_saves_level_5() {
        let s = saving_throws(SaveCategory::Thief, 5);
        assert_eq!(s, SavingThrows::new(12, 13, 11, 14, 13));
    }

    #[test]
    fn fighter_saves_level_1() {
        let s = saving_throws(SaveCategory::Fighter, 1);
        assert_eq!(s, SavingThrows::new(12, 13, 14, 15, 16));
    }

    #[test]
    fn fighter_saves_level_13() {
        let s = saving_throws(SaveCategory::Fighter, 13);
        assert_eq!(s, SavingThrows::new(4, 5, 6, 5, 8));
    }

    #[test]
    fn cleric_saves_level_4() {
        let s = saving_throws(SaveCategory::Cleric, 4);
        assert_eq!(s, SavingThrows::new(11, 12, 14, 16, 15));
    }

    #[test]
    fn cleric_saves_level_5() {
        let s = saving_throws(SaveCategory::Cleric, 5);
        assert_eq!(s, SavingThrows::new(9, 10, 12, 14, 12));
    }

    #[test]
    fn dwarf_saves_level_1() {
        let s = saving_throws(SaveCategory::Dwarf, 1);
        assert_eq!(s, SavingThrows::new(8, 9, 10, 13, 12));
    }

    #[test]
    fn magic_user_saves_level_1() {
        let s = saving_throws(SaveCategory::MagicUser, 1);
        assert_eq!(s, SavingThrows::new(13, 14, 13, 16, 15));
    }

    #[test]
    fn magic_user_saves_level_6() {
        let s = saving_throws(SaveCategory::MagicUser, 6);
        assert_eq!(s, SavingThrows::new(11, 12, 11, 14, 12));
    }

    #[test]
    fn paladin_saves_level_13() {
        let s = saving_throws(SaveCategory::Paladin, 13);
        assert_eq!(s, SavingThrows::new(2, 3, 4, 3, 6));
    }

    #[test]
    fn barbarian_saves_level_7() {
        let s = saving_throws(SaveCategory::Barbarian, 7);
        assert_eq!(s, SavingThrows::new(6, 9, 8, 10, 10));
    }

    #[test]
    fn drow_saves_level_10() {
        let s = saving_throws(SaveCategory::Drow, 10);
        assert_eq!(s, SavingThrows::new(6, 7, 8, 8, 6));
    }

    #[test]
    fn svirfneblin_saves_level_7() {
        let s = saving_throws(SaveCategory::Svirfneblin, 7);
        assert_eq!(s, SavingThrows::new(4, 5, 6, 9, 7));
    }
}
