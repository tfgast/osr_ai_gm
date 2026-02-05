//! Ability score modifier lookups per OSE Reference Booklet.

/// STR melee modifier (attack and damage).
pub fn str_melee_mod(score: i32) -> i32 {
    match score {
        3 => -3,
        4..=5 => -2,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 2,
        18 => 3,
        _ => 0,
    }
}

/// STR open doors chance (X-in-6).
pub fn str_open_doors(score: i32) -> u32 {
    match score {
        3..=8 => 1,
        9..=12 => 2,
        13..=15 => 3,
        16..=17 => 4,
        18 => 5,
        _ => 1,
    }
}

/// DEX modifier (AC and missile attacks).
///
/// Returns a signed modifier: +3 for DEX 18, -3 for DEX 3.
/// For AC (descending): subtract this from AC, so positive = better (lower) AC.
/// For missile attacks: add directly as attack bonus.
///
/// Note: OSE publishes the AC column with inverted signs (-3 for DEX 18)
/// because descending AC treats negative as better. We use a single positive
/// convention here and subtract in `calculate_ac`.
pub fn dex_mod(score: i32) -> i32 {
    match score {
        3 => -3,
        4..=5 => -2,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 2,
        18 => 3,
        _ => 0,
    }
}

/// Deprecated alias for [`dex_mod`]. Use `dex_mod` instead.
#[inline]
pub fn dex_ac_mod(score: i32) -> i32 { dex_mod(score) }

/// Deprecated alias for [`dex_mod`]. Use `dex_mod` instead.
#[inline]
pub fn dex_missile_mod(score: i32) -> i32 { dex_mod(score) }

/// DEX initiative modifier (optional individual initiative rule).
pub fn dex_init_mod(score: i32) -> i32 {
    match score {
        3 => -2,
        4..=5 => -1,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 1,
        18 => 2,
        _ => 0,
    }
}

/// CON hit point modifier (per HD).
pub fn con_hp_mod(score: i32) -> i32 {
    match score {
        3 => -3,
        4..=5 => -2,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 2,
        18 => 3,
        _ => 0,
    }
}

/// INT additional languages count.
pub fn int_extra_languages(score: i32) -> u32 {
    match score {
        3..=12 => 0,
        13..=15 => 1,
        16..=17 => 2,
        18 => 3,
        _ => 0,
    }
}

/// INT literacy level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Literacy {
    Illiterate,
    Basic,
    Literate,
}

pub fn int_literacy(score: i32) -> Literacy {
    match score {
        3..=5 => Literacy::Illiterate,
        6..=8 => Literacy::Basic,
        _ => Literacy::Literate,
    }
}

/// WIS magic save modifier.
pub fn wis_magic_save_mod(score: i32) -> i32 {
    match score {
        3 => -3,
        4..=5 => -2,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 2,
        18 => 3,
        _ => 0,
    }
}

/// CHA NPC reaction modifier.
pub fn cha_reaction_mod(score: i32) -> i32 {
    match score {
        3 => -2,
        4..=5 => -1,
        6..=8 => -1,
        9..=12 => 0,
        13..=15 => 1,
        16..=17 => 1,
        18 => 2,
        _ => 0,
    }
}

/// CHA max retainers.
pub fn cha_max_retainers(score: i32) -> u32 {
    match score {
        3 => 1,
        4..=5 => 2,
        6..=8 => 3,
        9..=12 => 4,
        13..=15 => 5,
        16..=17 => 6,
        18 => 7,
        _ => 4,
    }
}

/// CHA retainer loyalty (base).
pub fn cha_loyalty(score: i32) -> u32 {
    match score {
        3 => 4,
        4..=5 => 5,
        6..=8 => 6,
        9..=12 => 7,
        13..=15 => 8,
        16..=17 => 9,
        18 => 10,
        _ => 7,
    }
}

/// Prime requisite XP modifier.
pub fn prime_req_xp_mod(score: i32) -> i32 {
    match score {
        3..=5 => -20,
        6..=8 => -10,
        9..=12 => 0,
        13..=15 => 5,
        16..=18 => 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_modifiers() {
        assert_eq!(str_melee_mod(3), -3);
        assert_eq!(str_melee_mod(10), 0);
        assert_eq!(str_melee_mod(18), 3);
        assert_eq!(str_open_doors(3), 1);
        assert_eq!(str_open_doors(9), 2);
        assert_eq!(str_open_doors(18), 5);
    }

    #[test]
    fn dex_modifiers() {
        assert_eq!(dex_ac_mod(3), -3);
        assert_eq!(dex_ac_mod(10), 0);
        assert_eq!(dex_ac_mod(18), 3);
        assert_eq!(dex_missile_mod(4), -2);
        assert_eq!(dex_init_mod(3), -2);
        assert_eq!(dex_init_mod(18), 2);
    }

    #[test]
    fn con_modifiers() {
        assert_eq!(con_hp_mod(3), -3);
        assert_eq!(con_hp_mod(10), 0);
        assert_eq!(con_hp_mod(18), 3);
    }

    #[test]
    fn int_modifiers() {
        assert_eq!(int_extra_languages(3), 0);
        assert_eq!(int_extra_languages(13), 1);
        assert_eq!(int_extra_languages(18), 3);
        assert_eq!(int_literacy(3), Literacy::Illiterate);
        assert_eq!(int_literacy(7), Literacy::Basic);
        assert_eq!(int_literacy(10), Literacy::Literate);
    }

    #[test]
    fn wis_modifiers() {
        assert_eq!(wis_magic_save_mod(3), -3);
        assert_eq!(wis_magic_save_mod(12), 0);
        assert_eq!(wis_magic_save_mod(18), 3);
    }

    #[test]
    fn cha_modifiers() {
        assert_eq!(cha_reaction_mod(3), -2);
        assert_eq!(cha_reaction_mod(10), 0);
        assert_eq!(cha_reaction_mod(18), 2);
        assert_eq!(cha_max_retainers(3), 1);
        assert_eq!(cha_max_retainers(18), 7);
        assert_eq!(cha_loyalty(3), 4);
        assert_eq!(cha_loyalty(18), 10);
    }

    #[test]
    fn prime_requisite() {
        assert_eq!(prime_req_xp_mod(3), -20);
        assert_eq!(prime_req_xp_mod(7), -10);
        assert_eq!(prime_req_xp_mod(10), 0);
        assert_eq!(prime_req_xp_mod(14), 5);
        assert_eq!(prime_req_xp_mod(18), 10);
    }
}
