//! Spell slot progression tables per OSE Reference Booklet p16-17.

/// Spell slots available at a given level. Index 0 = 1st level spells, etc.
/// A value of 0 means no spells of that level available.
pub type SpellSlots = [u32; 6];

/// Which spell list a class uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SpellListType {
    None,
    Cleric,     // Cleric, Paladin
    Druid,      // Bard, Druid, Ranger
    Illusionist, // Gnome, Illusionist
    MagicUser,   // Elf, Half-Elf, Magic-User
    DrowArcaneAndDivine, // Drow: both arcane (magic-user) and divine (cleric)
}

/// Spell progression category — classes sharing a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SpellProgression {
    Bard,
    Cleric,
    Drow,
    Druid,
    ArcaneFullCaster, // Elf, Gnome, Illusionist, Magic-User
    HalfElf,
    Paladin,
    Ranger,
    NonCaster,
}

/// Returns spell slots for a progression type at a given character level.
/// Returns [0; 6] for non-casters or levels below casting threshold.
pub fn spell_slots(prog: SpellProgression, level: u32) -> SpellSlots {
    use SpellProgression::*;
    match prog {
        NonCaster => [0; 6],

        // Bard: druid spell list, up to 4th level spells, levels 1-14
        Bard => match level {
            1 => [0, 0, 0, 0, 0, 0],
            2 => [1, 0, 0, 0, 0, 0],
            3 => [2, 0, 0, 0, 0, 0],
            4 => [3, 0, 0, 0, 0, 0],
            5 => [3, 1, 0, 0, 0, 0],
            6 => [3, 2, 0, 0, 0, 0],
            7 => [3, 3, 0, 0, 0, 0],
            8 => [3, 3, 1, 0, 0, 0],
            9 => [3, 3, 2, 0, 0, 0],
            10 => [3, 3, 3, 0, 0, 0],
            11 => [3, 3, 3, 1, 0, 0],
            12 => [3, 3, 3, 2, 0, 0],
            13 => [3, 3, 3, 3, 0, 0],
            _ => [4, 4, 3, 3, 0, 0],
        },

        // Cleric: cleric spell list, up to 5th level spells, levels 1-14
        Cleric => match level {
            1 => [0, 0, 0, 0, 0, 0],
            2 => [1, 0, 0, 0, 0, 0],
            3 => [2, 0, 0, 0, 0, 0],
            4 => [2, 1, 0, 0, 0, 0],
            5 => [2, 2, 0, 0, 0, 0],
            6 => [2, 2, 1, 1, 0, 0],
            7 => [2, 2, 2, 1, 1, 0],
            8 => [3, 3, 2, 2, 1, 0],
            9 => [3, 3, 3, 2, 2, 0],
            10 => [4, 4, 3, 3, 2, 0],
            11 => [4, 4, 4, 3, 3, 0],
            12 => [5, 5, 4, 4, 3, 0],
            13 => [5, 5, 5, 4, 4, 0],
            _ => [6, 5, 5, 5, 4, 0],
        },

        // Drow: cleric spell list, up to 5th level spells, levels 1-10
        Drow => match level {
            1 => [1, 0, 0, 0, 0, 0], // only light (darkness) at 1st
            2 => [2, 0, 0, 0, 0, 0],
            3 => [2, 1, 0, 0, 0, 0],
            4 => [2, 2, 0, 0, 0, 0],
            5 => [2, 2, 1, 0, 0, 0],
            6 => [2, 2, 2, 1, 0, 0],
            7 => [3, 3, 2, 2, 1, 0],
            8 => [3, 3, 3, 2, 2, 0],
            9 => [4, 4, 3, 3, 2, 0],
            _ => [4, 4, 4, 3, 3, 0],
        },

        // Druid: druid spell list, up to 5th level spells, levels 1-14
        Druid => match level {
            1 => [1, 0, 0, 0, 0, 0],
            2 => [2, 0, 0, 0, 0, 0],
            3 => [2, 1, 0, 0, 0, 0],
            4 => [2, 2, 0, 0, 0, 0],
            5 => [2, 2, 1, 1, 0, 0],
            6 => [2, 2, 2, 1, 1, 0],
            7 => [3, 3, 2, 2, 1, 0],
            8 => [3, 3, 3, 2, 2, 0],
            9 => [4, 4, 3, 3, 2, 0],
            10 => [4, 4, 4, 3, 3, 0],
            11 => [5, 5, 4, 4, 3, 0],
            12 => [5, 5, 5, 4, 4, 0],
            13 => [6, 5, 5, 5, 4, 0],
            _ => [6, 6, 5, 5, 5, 0],
        },

        // Elf, Gnome, Illusionist, Magic-User: up to 6th level spells
        ArcaneFullCaster => match level {
            1 => [1, 0, 0, 0, 0, 0],
            2 => [2, 0, 0, 0, 0, 0],
            3 => [2, 1, 0, 0, 0, 0],
            4 => [2, 2, 0, 0, 0, 0],
            5 => [2, 2, 1, 0, 0, 0],
            6 => [2, 2, 2, 0, 0, 0],
            7 => [3, 2, 2, 1, 0, 0],
            8 => [3, 3, 2, 2, 0, 0],
            9 => [3, 3, 3, 2, 1, 0],
            10 => [3, 3, 3, 3, 2, 0],
            11 => [4, 3, 3, 3, 2, 1],
            12 => [4, 4, 3, 3, 3, 2],
            13 => [4, 4, 4, 3, 3, 3],
            _ => [4, 4, 4, 4, 3, 3],
        },

        // Half-Elf: magic-user spell list, up to 4th level spells, levels 1-12
        HalfElf => match level {
            1 => [0, 0, 0, 0, 0, 0],
            2 => [1, 0, 0, 0, 0, 0],
            3 => [2, 0, 0, 0, 0, 0],
            4 => [2, 0, 0, 0, 0, 0],
            5 => [2, 1, 0, 0, 0, 0],
            6 => [2, 2, 0, 0, 0, 0],
            7 => [2, 2, 0, 0, 0, 0],
            8 => [2, 2, 1, 0, 0, 0],
            9 => [3, 2, 1, 0, 0, 0],
            10 => [3, 2, 2, 0, 0, 0],
            11 => [3, 2, 2, 1, 0, 0],
            _ => [3, 3, 2, 1, 0, 0],
        },

        // Paladin: cleric spell list, up to 3rd level spells, levels 9-14
        Paladin => match level {
            1..=8 => [0, 0, 0, 0, 0, 0],
            9 => [1, 0, 0, 0, 0, 0],
            10 => [2, 0, 0, 0, 0, 0],
            11 => [2, 1, 0, 0, 0, 0],
            12 => [2, 2, 0, 0, 0, 0],
            13 => [2, 2, 1, 0, 0, 0],
            _ => [3, 2, 1, 0, 0, 0],
        },

        // Ranger: druid spell list, up to 3rd level spells, levels 8-14
        Ranger => match level {
            1..=7 => [0, 0, 0, 0, 0, 0],
            8 => [1, 0, 0, 0, 0, 0],
            9 => [2, 0, 0, 0, 0, 0],
            10 => [2, 1, 0, 0, 0, 0],
            11 => [2, 2, 0, 0, 0, 0],
            12 => [2, 2, 1, 0, 0, 0],
            13 => [3, 2, 1, 0, 0, 0],
            _ => [3, 2, 2, 0, 0, 0],
        },
    }
}

/// Check if a class has any spell slots at a given level.
pub fn can_cast(prog: SpellProgression, level: u32) -> bool {
    spell_slots(prog, level).iter().any(|&s| s > 0)
}

/// Total number of spell slots across all spell levels.
pub fn total_slots(prog: SpellProgression, level: u32) -> u32 {
    spell_slots(prog, level).iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_caster() {
        assert_eq!(spell_slots(SpellProgression::NonCaster, 14), [0; 6]);
        assert!(!can_cast(SpellProgression::NonCaster, 14));
    }

    #[test]
    fn cleric_no_spells_level_1() {
        assert!(!can_cast(SpellProgression::Cleric, 1));
    }

    #[test]
    fn cleric_gets_spells_level_2() {
        let s = spell_slots(SpellProgression::Cleric, 2);
        assert_eq!(s[0], 1);
        assert!(can_cast(SpellProgression::Cleric, 2));
    }

    #[test]
    fn cleric_level_14() {
        let s = spell_slots(SpellProgression::Cleric, 14);
        assert_eq!(s, [6, 5, 5, 5, 4, 0]);
    }

    #[test]
    fn magic_user_level_1() {
        let s = spell_slots(SpellProgression::ArcaneFullCaster, 1);
        assert_eq!(s, [1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn magic_user_level_14() {
        let s = spell_slots(SpellProgression::ArcaneFullCaster, 14);
        assert_eq!(s, [4, 4, 4, 4, 3, 3]);
    }

    #[test]
    fn druid_level_7() {
        let s = spell_slots(SpellProgression::Druid, 7);
        assert_eq!(s, [3, 3, 2, 2, 1, 0]);
    }

    #[test]
    fn bard_level_2() {
        let s = spell_slots(SpellProgression::Bard, 2);
        assert_eq!(s, [1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn bard_level_14() {
        let s = spell_slots(SpellProgression::Bard, 14);
        assert_eq!(s, [4, 4, 3, 3, 0, 0]);
    }

    #[test]
    fn half_elf_level_8() {
        let s = spell_slots(SpellProgression::HalfElf, 8);
        assert_eq!(s, [2, 2, 1, 0, 0, 0]);
    }

    #[test]
    fn paladin_no_spells_level_8() {
        assert!(!can_cast(SpellProgression::Paladin, 8));
    }

    #[test]
    fn paladin_gets_spells_level_9() {
        let s = spell_slots(SpellProgression::Paladin, 9);
        assert_eq!(s[0], 1);
    }

    #[test]
    fn ranger_level_10() {
        let s = spell_slots(SpellProgression::Ranger, 10);
        assert_eq!(s, [2, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn drow_level_1() {
        let s = spell_slots(SpellProgression::Drow, 1);
        assert_eq!(s[0], 1); // only light (darkness)
    }

    #[test]
    fn drow_level_10() {
        let s = spell_slots(SpellProgression::Drow, 10);
        assert_eq!(s, [4, 4, 4, 3, 3, 0]);
    }

    #[test]
    fn total_slots_magic_user_5() {
        assert_eq!(total_slots(SpellProgression::ArcaneFullCaster, 5), 5);
    }
}
