/// Equipment tables per OSE Reference Booklet p22-23.

/// Weapon quality flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponQualities {
    pub melee: bool,
    pub missile: bool,
    pub blunt: bool,
    pub two_handed: bool,
    pub slow: bool,
    pub brace: bool,
    pub charge: bool,
}

impl WeaponQualities {
    const fn melee() -> Self {
        WeaponQualities { melee: true, missile: false, blunt: false, two_handed: false, slow: false, brace: false, charge: false }
    }

    const fn melee_blunt() -> Self {
        WeaponQualities { melee: true, missile: false, blunt: true, two_handed: false, slow: false, brace: false, charge: false }
    }

    const fn melee_2h_slow() -> Self {
        WeaponQualities { melee: true, missile: false, blunt: false, two_handed: true, slow: true, brace: false, charge: false }
    }

    const fn melee_blunt_2h_slow() -> Self {
        WeaponQualities { melee: true, missile: false, blunt: true, two_handed: true, slow: true, brace: false, charge: false }
    }
}

/// Weapon definition.
#[derive(Debug, Clone)]
pub struct WeaponDef {
    pub name: &'static str,
    pub cost_gp: u32,
    pub weight_coins: u32,
    pub damage: &'static str,
    pub qualities: WeaponQualities,
    /// Missile range bands: (short, medium, long). All 0 for melee-only.
    pub range: (u32, u32, u32),
}

/// All weapons from OSE Reference Booklet p22-23.
pub fn weapons() -> Vec<WeaponDef> {
    vec![
        WeaponDef {
            name: "Battle axe", cost_gp: 7, weight_coins: 50, damage: "1d8",
            qualities: WeaponQualities::melee_2h_slow(),
            range: (0, 0, 0),
        },
        WeaponDef {
            name: "Club", cost_gp: 3, weight_coins: 50, damage: "1d4",
            qualities: WeaponQualities::melee_blunt(),
            range: (0, 0, 0),
        },
        WeaponDef {
            name: "Crossbow", cost_gp: 30, weight_coins: 50, damage: "1d6",
            qualities: WeaponQualities { melee: false, missile: true, blunt: false, two_handed: true, slow: true, brace: false, charge: false },
            range: (80, 160, 240),
        },
        WeaponDef {
            name: "Dagger", cost_gp: 3, weight_coins: 10, damage: "1d4",
            qualities: WeaponQualities { melee: true, missile: true, blunt: false, two_handed: false, slow: false, brace: false, charge: false },
            range: (10, 20, 30),
        },
        WeaponDef {
            name: "Hand axe", cost_gp: 4, weight_coins: 30, damage: "1d6",
            qualities: WeaponQualities { melee: true, missile: true, blunt: false, two_handed: false, slow: false, brace: false, charge: false },
            range: (10, 20, 30),
        },
        WeaponDef {
            name: "Javelin", cost_gp: 1, weight_coins: 20, damage: "1d4",
            qualities: WeaponQualities { melee: false, missile: true, blunt: false, two_handed: false, slow: false, brace: false, charge: false },
            range: (30, 60, 90),
        },
        WeaponDef {
            name: "Lance", cost_gp: 5, weight_coins: 120, damage: "1d6",
            qualities: WeaponQualities { melee: true, missile: false, blunt: false, two_handed: false, slow: false, brace: false, charge: true },
            range: (0, 0, 0),
        },
        WeaponDef {
            name: "Long bow", cost_gp: 40, weight_coins: 30, damage: "1d6",
            qualities: WeaponQualities { melee: false, missile: true, blunt: false, two_handed: true, slow: false, brace: false, charge: false },
            range: (70, 140, 210),
        },
        WeaponDef {
            name: "Mace", cost_gp: 5, weight_coins: 30, damage: "1d6",
            qualities: WeaponQualities::melee_blunt(),
            range: (0, 0, 0),
        },
        WeaponDef {
            name: "Pole arm", cost_gp: 7, weight_coins: 150, damage: "1d10",
            qualities: WeaponQualities { melee: true, missile: false, blunt: false, two_handed: true, slow: true, brace: true, charge: false },
            range: (0, 0, 0),
        },
        WeaponDef {
            name: "Short bow", cost_gp: 25, weight_coins: 30, damage: "1d6",
            qualities: WeaponQualities { melee: false, missile: true, blunt: false, two_handed: true, slow: false, brace: false, charge: false },
            range: (50, 100, 150),
        },
        WeaponDef {
            name: "Short sword", cost_gp: 7, weight_coins: 30, damage: "1d6",
            qualities: WeaponQualities::melee(),
            range: (0, 0, 0),
        },
        WeaponDef {
            name: "Silver dagger", cost_gp: 30, weight_coins: 10, damage: "1d4",
            qualities: WeaponQualities { melee: true, missile: true, blunt: false, two_handed: false, slow: false, brace: false, charge: false },
            range: (10, 20, 30),
        },
        WeaponDef {
            name: "Sling", cost_gp: 2, weight_coins: 20, damage: "1d4",
            qualities: WeaponQualities { melee: false, missile: true, blunt: true, two_handed: false, slow: false, brace: false, charge: false },
            range: (40, 80, 160),
        },
        WeaponDef {
            name: "Spear", cost_gp: 3, weight_coins: 30, damage: "1d6",
            qualities: WeaponQualities { melee: true, missile: true, blunt: false, two_handed: false, slow: false, brace: true, charge: false },
            range: (20, 40, 60),
        },
        WeaponDef {
            name: "Staff", cost_gp: 2, weight_coins: 40, damage: "1d4",
            qualities: WeaponQualities::melee_blunt_2h_slow(),
            range: (0, 0, 0),
        },
        WeaponDef {
            name: "Sword", cost_gp: 10, weight_coins: 60, damage: "1d8",
            qualities: WeaponQualities::melee(),
            range: (0, 0, 0),
        },
        WeaponDef {
            name: "Two-handed sword", cost_gp: 15, weight_coins: 150, damage: "1d10",
            qualities: WeaponQualities::melee_2h_slow(),
            range: (0, 0, 0),
        },
        WeaponDef {
            name: "War hammer", cost_gp: 5, weight_coins: 30, damage: "1d6",
            qualities: WeaponQualities::melee_blunt(),
            range: (0, 0, 0),
        },
    ]
}

/// Armour definition.
#[derive(Debug, Clone)]
pub struct ArmourDef {
    pub name: &'static str,
    pub ac: i32,        // descending AC (9=unarmoured, 3=plate)
    pub cost_gp: u32,
    pub weight_coins: u32,
    pub is_shield: bool,
}

/// All armour types from OSE Reference Booklet p22.
pub fn armour() -> Vec<ArmourDef> {
    vec![
        ArmourDef { name: "None",       ac: 9, cost_gp: 0,  weight_coins: 0,   is_shield: false },
        ArmourDef { name: "Leather",    ac: 7, cost_gp: 20, weight_coins: 200, is_shield: false },
        ArmourDef { name: "Chain mail", ac: 5, cost_gp: 40, weight_coins: 400, is_shield: false },
        ArmourDef { name: "Plate mail", ac: 3, cost_gp: 60, weight_coins: 500, is_shield: false },
        ArmourDef { name: "Shield",     ac: 0, cost_gp: 10, weight_coins: 100, is_shield: true  },
    ]
}

/// Adventuring gear item.
#[derive(Debug, Clone)]
pub struct GearDef {
    pub name: &'static str,
    pub cost_gp: u32,
}

/// All adventuring gear from OSE Reference Booklet p22.
pub fn gear() -> Vec<GearDef> {
    vec![
        GearDef { name: "Backpack",                cost_gp: 5 },
        GearDef { name: "Crowbar",                 cost_gp: 10 },
        GearDef { name: "Garlic",                  cost_gp: 5 },
        GearDef { name: "Grappling hook",          cost_gp: 25 },
        GearDef { name: "Hammer (small)",          cost_gp: 2 },
        GearDef { name: "Holy symbol",             cost_gp: 25 },
        GearDef { name: "Holy water (vial)",       cost_gp: 25 },
        GearDef { name: "Iron spikes (12)",        cost_gp: 1 },
        GearDef { name: "Lantern",                 cost_gp: 10 },
        GearDef { name: "Mirror (hand-sized)",     cost_gp: 5 },
        GearDef { name: "Oil (1 flask)",           cost_gp: 2 },
        GearDef { name: "Pole (10')",              cost_gp: 1 },
        GearDef { name: "Rations (iron, 7 days)",  cost_gp: 15 },
        GearDef { name: "Rations (standard, 7 days)", cost_gp: 5 },
        GearDef { name: "Rope (50')",              cost_gp: 1 },
        GearDef { name: "Sack (large)",            cost_gp: 2 },
        GearDef { name: "Sack (small)",            cost_gp: 1 },
        GearDef { name: "Stakes (3) and mallet",   cost_gp: 3 },
        GearDef { name: "Thieves' tools",          cost_gp: 25 },
        GearDef { name: "Tinder box (flint & steel)", cost_gp: 3 },
        GearDef { name: "Torches (6)",             cost_gp: 1 },
        GearDef { name: "Waterskin",               cost_gp: 1 },
        GearDef { name: "Wine (2 pints)",          cost_gp: 1 },
        GearDef { name: "Wolfsbane (1 bunch)",     cost_gp: 10 },
    ]
}

/// Ammunition definition.
#[derive(Debug, Clone)]
pub struct AmmoDef {
    pub name: &'static str,
    pub cost_gp: u32,
}

/// All ammunition types from OSE Reference Booklet p22.
pub fn ammunition() -> Vec<AmmoDef> {
    vec![
        AmmoDef { name: "Arrows (quiver of 20)",     cost_gp: 5 },
        AmmoDef { name: "Crossbow bolts (case of 30)", cost_gp: 10 },
        AmmoDef { name: "Silver tipped arrow (1)",    cost_gp: 5 },
        AmmoDef { name: "Sling stones",               cost_gp: 0 },
    ]
}

/// Look up a weapon by name (case-insensitive).
pub fn find_weapon(name: &str) -> Option<WeaponDef> {
    weapons().into_iter().find(|w| w.name.eq_ignore_ascii_case(name))
}

/// Look up armour by name (case-insensitive).
pub fn find_armour(name: &str) -> Option<ArmourDef> {
    armour().into_iter().find(|a| a.name.eq_ignore_ascii_case(name))
}

/// Look up gear by name (case-insensitive).
pub fn find_gear(name: &str) -> Option<GearDef> {
    gear().into_iter().find(|g| g.name.eq_ignore_ascii_case(name))
}

/// Calculate AC from armour and shield, plus DEX modifier.
/// AC is descending: lower = better.
pub fn calculate_ac(armour_ac: i32, has_shield: bool, dex_mod: i32) -> i32 {
    let mut ac = armour_ac;
    if has_shield {
        ac -= 1; // shield improves AC by 1 (descending)
    }
    ac - dex_mod // positive DEX mod improves (lowers) AC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_count() {
        assert_eq!(weapons().len(), 19);
    }

    #[test]
    fn armour_count() {
        assert_eq!(armour().len(), 5); // none + 3 types + shield
    }

    #[test]
    fn gear_count() {
        assert_eq!(gear().len(), 24);
    }

    #[test]
    fn find_sword() {
        let w = find_weapon("Sword").unwrap();
        assert_eq!(w.cost_gp, 10);
        assert_eq!(w.damage, "1d8");
        assert!(w.qualities.melee);
        assert!(!w.qualities.missile);
    }

    #[test]
    fn find_dagger() {
        let w = find_weapon("dagger").unwrap();
        assert_eq!(w.cost_gp, 3);
        assert!(w.qualities.melee);
        assert!(w.qualities.missile);
        assert_eq!(w.range, (10, 20, 30));
    }

    #[test]
    fn find_leather() {
        let a = find_armour("leather").unwrap();
        assert_eq!(a.ac, 7);
        assert_eq!(a.cost_gp, 20);
    }

    #[test]
    fn find_plate() {
        let a = find_armour("Plate mail").unwrap();
        assert_eq!(a.ac, 3);
    }

    #[test]
    fn ac_unarmoured() {
        assert_eq!(calculate_ac(9, false, 0), 9);
    }

    #[test]
    fn ac_plate_shield_dex() {
        // Plate (3) + shield (-1) + DEX +2 mod (-2) = 0
        assert_eq!(calculate_ac(3, true, 2), 0);
    }

    #[test]
    fn ac_leather_no_shield_low_dex() {
        // Leather (7) + no shield + DEX -1 = 8
        assert_eq!(calculate_ac(7, false, -1), 8);
    }

    #[test]
    fn blunt_weapons() {
        let blunt: Vec<_> = weapons().into_iter().filter(|w| w.qualities.blunt).collect();
        let names: Vec<&str> = blunt.iter().map(|w| w.name).collect();
        assert!(names.contains(&"Club"));
        assert!(names.contains(&"Mace"));
        assert!(names.contains(&"Sling"));
        assert!(names.contains(&"Staff"));
        assert!(names.contains(&"War hammer"));
    }
}
