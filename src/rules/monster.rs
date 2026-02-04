/// Monster stat blocks per OSE B/X monster manual.
/// 30 core monsters with complete stat blocks.

/// Static monster definition (template for spawning).
#[derive(Debug, Clone)]
pub struct MonsterDef {
    pub name: &'static str,
    pub hit_dice: &'static str,
    pub ac: i32,
    pub attacks: &'static [&'static str],
    pub damage: &'static str,
    pub movement: u32,       // feet per turn
    pub morale: u32,
    pub xp_value: u64,
    pub num_appearing: &'static str, // dice notation for dungeon encounters
    pub special: &'static str,
}

/// Look up a monster definition by name (case-insensitive).
pub fn find_monster(name: &str) -> Option<&'static MonsterDef> {
    MONSTERS.iter().find(|m| m.name.eq_ignore_ascii_case(name))
}

/// All monster definitions.
pub fn all_monsters() -> &'static [MonsterDef] {
    &MONSTERS
}

static MONSTERS: [MonsterDef; 30] = [
    MonsterDef {
        name: "Kobold",
        hit_dice: "1/2",
        ac: 7,
        attacks: &["Weapon"],
        damage: "1d4",
        movement: 60,
        morale: 6,
        xp_value: 5,
        num_appearing: "4d4",
        special: "Infravision 60'",
    },
    MonsterDef {
        name: "Giant Rat",
        hit_dice: "1/2",
        ac: 7,
        attacks: &["Bite"],
        damage: "1d3",
        movement: 120,
        morale: 8,
        xp_value: 5,
        num_appearing: "3d6",
        special: "Disease on bite (5% chance)",
    },
    MonsterDef {
        name: "Goblin",
        hit_dice: "1-1",
        ac: 6,
        attacks: &["Weapon"],
        damage: "1d6",
        movement: 60,
        morale: 7,
        xp_value: 5,
        num_appearing: "2d4",
        special: "Infravision 60', -1 attack in daylight",
    },
    MonsterDef {
        name: "Skeleton",
        hit_dice: "1",
        ac: 7,
        attacks: &["Weapon"],
        damage: "1d6",
        movement: 60,
        morale: 12,
        xp_value: 10,
        num_appearing: "3d4",
        special: "Undead: immune to sleep, charm, hold. Half damage from edged weapons",
    },
    MonsterDef {
        name: "Zombie",
        hit_dice: "2",
        ac: 8,
        attacks: &["Claw"],
        damage: "1d8",
        movement: 60,
        morale: 12,
        xp_value: 20,
        num_appearing: "2d4",
        special: "Undead: immune to sleep, charm, hold. Always lose initiative",
    },
    MonsterDef {
        name: "Orc",
        hit_dice: "1",
        ac: 6,
        attacks: &["Weapon"],
        damage: "1d6",
        movement: 120,
        morale: 8,
        xp_value: 10,
        num_appearing: "2d4",
        special: "Infravision 60', -1 attack in daylight",
    },
    MonsterDef {
        name: "Hobgoblin",
        hit_dice: "1+1",
        ac: 6,
        attacks: &["Weapon"],
        damage: "1d8",
        movement: 90,
        morale: 8,
        xp_value: 15,
        num_appearing: "1d6",
        special: "Infravision 60'",
    },
    MonsterDef {
        name: "Gnoll",
        hit_dice: "2",
        ac: 5,
        attacks: &["Weapon"],
        damage: "2d4",
        movement: 90,
        morale: 8,
        xp_value: 20,
        num_appearing: "1d6",
        special: "",
    },
    MonsterDef {
        name: "Ghoul",
        hit_dice: "2*",
        ac: 6,
        attacks: &["Claw", "Claw", "Bite"],
        damage: "1d3",
        movement: 90,
        morale: 9,
        xp_value: 25,
        num_appearing: "1d6",
        special: "Undead. Paralysis on hit (save vs Paralysis, elves immune)",
    },
    MonsterDef {
        name: "Bugbear",
        hit_dice: "3+1",
        ac: 5,
        attacks: &["Weapon"],
        damage: "2d4",
        movement: 90,
        morale: 9,
        xp_value: 50,
        num_appearing: "2d4",
        special: "Surprise on 1-3",
    },
    MonsterDef {
        name: "Gelatinous Cube",
        hit_dice: "4*",
        ac: 8,
        attacks: &["Touch"],
        damage: "2d4",
        movement: 60,
        morale: 12,
        xp_value: 125,
        num_appearing: "1",
        special: "Surprise on 1-4. Paralysis on hit (save vs Paralysis). Immune to lightning, cold",
    },
    MonsterDef {
        name: "Ogre",
        hit_dice: "4+1",
        ac: 5,
        attacks: &["Club"],
        damage: "1d10",
        movement: 90,
        morale: 10,
        xp_value: 125,
        num_appearing: "1d6",
        special: "",
    },
    MonsterDef {
        name: "Wight",
        hit_dice: "3*",
        ac: 5,
        attacks: &["Touch"],
        damage: "Energy drain",
        movement: 90,
        morale: 12,
        xp_value: 50,
        num_appearing: "1d6",
        special: "Undead. Energy drain: drains 1 level on hit. Immune to normal weapons",
    },
    MonsterDef {
        name: "Wraith",
        hit_dice: "4**",
        ac: 3,
        attacks: &["Touch"],
        damage: "1d6 + energy drain",
        movement: 120,
        morale: 12,
        xp_value: 175,
        num_appearing: "1d4",
        special: "Undead. Energy drain: drains 1 level. Only hit by silver or magic weapons",
    },
    MonsterDef {
        name: "Gargoyle",
        hit_dice: "4",
        ac: 5,
        attacks: &["Claw", "Claw", "Bite", "Horn"],
        damage: "1d4",
        movement: 90,
        morale: 11,
        xp_value: 75,
        num_appearing: "1d6",
        special: "Only hit by magic weapons. Fly 150'",
    },
    MonsterDef {
        name: "Owlbear",
        hit_dice: "5",
        ac: 5,
        attacks: &["Claw", "Claw", "Bite"],
        damage: "1d8",
        movement: 120,
        morale: 9,
        xp_value: 175,
        num_appearing: "1d4",
        special: "Hug: both claws hit = extra 2d8 damage",
    },
    MonsterDef {
        name: "Troll",
        hit_dice: "6+3*",
        ac: 4,
        attacks: &["Claw", "Claw", "Bite"],
        damage: "1d6",
        movement: 120,
        morale: 10,
        xp_value: 650,
        num_appearing: "1d8",
        special: "Regenerate 3 HP/round. Only killed by fire or acid",
    },
    MonsterDef {
        name: "Minotaur",
        hit_dice: "6",
        ac: 6,
        attacks: &["Gore", "Bite"],
        damage: "1d6",
        movement: 120,
        morale: 12,
        xp_value: 275,
        num_appearing: "1d6",
        special: "+2 damage on charge with gore",
    },
    MonsterDef {
        name: "Mummy",
        hit_dice: "5+1*",
        ac: 3,
        attacks: &["Touch"],
        damage: "1d12",
        movement: 60,
        morale: 12,
        xp_value: 400,
        num_appearing: "1d4",
        special: "Undead. Disease on hit. Only hit by magic weapons (half damage). Immune to sleep, charm, hold",
    },
    MonsterDef {
        name: "Spectre",
        hit_dice: "6**",
        ac: 2,
        attacks: &["Touch"],
        damage: "1d8 + energy drain",
        movement: 150,
        morale: 11,
        xp_value: 725,
        num_appearing: "1d4",
        special: "Undead. Energy drain: 2 levels per hit. Only hit by magic weapons",
    },
    MonsterDef {
        name: "Vampire",
        hit_dice: "7-9**",
        ac: 2,
        attacks: &["Touch or Bite"],
        damage: "1d10 + energy drain",
        movement: 120,
        morale: 11,
        xp_value: 1250,
        num_appearing: "1d4",
        special: "Undead. Energy drain: 2 levels. Charm gaze. Regenerate 3 HP/round. Shapechange",
    },
    MonsterDef {
        name: "Green Dragon",
        hit_dice: "8**",
        ac: 1,
        attacks: &["Claw", "Claw", "Bite"],
        damage: "1d6+1",
        movement: 90,
        morale: 9,
        xp_value: 1750,
        num_appearing: "1d4",
        special: "Breath weapon: chlorine gas cone 50'x40', damage = current HP. Fly 240'",
    },
    MonsterDef {
        name: "Red Dragon",
        hit_dice: "10**",
        ac: -1,
        attacks: &["Claw", "Claw", "Bite"],
        damage: "1d8",
        movement: 90,
        morale: 10,
        xp_value: 2300,
        num_appearing: "1d4",
        special: "Breath weapon: fire cone 90'x30', damage = current HP. Fly 240'",
    },
    MonsterDef {
        name: "Giant Spider",
        hit_dice: "3*",
        ac: 6,
        attacks: &["Bite"],
        damage: "2d6",
        movement: 120,
        morale: 8,
        xp_value: 50,
        num_appearing: "1d3",
        special: "Poison bite (save vs Poison or die). Web: 60' range",
    },
    MonsterDef {
        name: "Carrion Crawler",
        hit_dice: "3+1",
        ac: 7,
        attacks: &["Tentacle x8"],
        damage: "Paralysis",
        movement: 120,
        morale: 9,
        xp_value: 50,
        num_appearing: "1d3",
        special: "8 tentacle attacks per round. Paralysis on hit (save vs Paralysis)",
    },
    MonsterDef {
        name: "Rust Monster",
        hit_dice: "5",
        ac: 2,
        attacks: &["Touch"],
        damage: "Rust",
        movement: 120,
        morale: 7,
        xp_value: 175,
        num_appearing: "1d4",
        special: "Destroys metal on touch. Non-magical metal: instant. Magic items: save or destroyed",
    },
    MonsterDef {
        name: "Basilisk",
        hit_dice: "6+1**",
        ac: 4,
        attacks: &["Bite + Gaze"],
        damage: "1d10",
        movement: 60,
        morale: 9,
        xp_value: 950,
        num_appearing: "1d6",
        special: "Petrifying gaze (save vs Petrify or turned to stone). Petrifying touch",
    },
    MonsterDef {
        name: "Cockatrice",
        hit_dice: "5**",
        ac: 6,
        attacks: &["Beak"],
        damage: "1d6",
        movement: 90,
        morale: 7,
        xp_value: 425,
        num_appearing: "1d4",
        special: "Petrifying touch (save vs Petrify or turned to stone). Fly 180'",
    },
    MonsterDef {
        name: "Harpy",
        hit_dice: "3*",
        ac: 7,
        attacks: &["Claw", "Claw", "Weapon"],
        damage: "1d4",
        movement: 60,
        morale: 7,
        xp_value: 50,
        num_appearing: "1d6",
        special: "Charm song: all within 300' save vs Spells or charmed. Fly 150'",
    },
    MonsterDef {
        name: "Medusa",
        hit_dice: "4**",
        ac: 8,
        attacks: &["Snake bites"],
        damage: "1d6 + poison",
        movement: 90,
        morale: 8,
        xp_value: 175,
        num_appearing: "1d3",
        special: "Petrifying gaze (save vs Petrify). Poison snake hair (save vs Poison or die in 1 turn)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monster_count() {
        assert_eq!(all_monsters().len(), 30);
    }

    #[test]
    fn find_goblin() {
        let m = find_monster("Goblin").unwrap();
        assert_eq!(m.hit_dice, "1-1");
        assert_eq!(m.ac, 6);
        assert_eq!(m.morale, 7);
        assert_eq!(m.xp_value, 5);
    }

    #[test]
    fn find_case_insensitive() {
        assert!(find_monster("goblin").is_some());
        assert!(find_monster("GOBLIN").is_some());
        assert!(find_monster("Giant Rat").is_some());
        assert!(find_monster("giant rat").is_some());
    }

    #[test]
    fn find_nonexistent() {
        assert!(find_monster("Beholder").is_none());
    }

    #[test]
    fn skeleton_is_undead() {
        let m = find_monster("Skeleton").unwrap();
        assert!(m.special.contains("Undead"));
        assert_eq!(m.morale, 12); // undead never flee
    }

    #[test]
    fn orc_stats() {
        let m = find_monster("Orc").unwrap();
        assert_eq!(m.hit_dice, "1");
        assert_eq!(m.ac, 6);
        assert_eq!(m.damage, "1d6");
        assert_eq!(m.morale, 8);
        assert_eq!(m.xp_value, 10);
    }

    #[test]
    fn ogre_stats() {
        let m = find_monster("Ogre").unwrap();
        assert_eq!(m.hit_dice, "4+1");
        assert_eq!(m.ac, 5);
        assert_eq!(m.damage, "1d10");
        assert_eq!(m.xp_value, 125);
    }

    #[test]
    fn troll_regenerates() {
        let m = find_monster("Troll").unwrap();
        assert!(m.special.contains("Regenerate"));
        assert_eq!(m.hit_dice, "6+3*");
    }

    #[test]
    fn red_dragon_stats() {
        let m = find_monster("Red Dragon").unwrap();
        assert_eq!(m.hit_dice, "10**");
        assert_eq!(m.ac, -1);
        assert!(m.special.contains("fire"));
    }

    #[test]
    fn gelatinous_cube_stats() {
        let m = find_monster("Gelatinous Cube").unwrap();
        assert_eq!(m.hit_dice, "4*");
        assert_eq!(m.ac, 8);
        assert!(m.special.contains("Paralysis"));
    }

    #[test]
    fn kobold_weakest() {
        let m = find_monster("Kobold").unwrap();
        assert_eq!(m.hit_dice, "1/2");
        assert_eq!(m.xp_value, 5);
    }

    #[test]
    fn all_monsters_have_names() {
        for m in all_monsters() {
            assert!(!m.name.is_empty());
            assert!(!m.hit_dice.is_empty());
        }
    }

    #[test]
    fn xp_values_reasonable() {
        for m in all_monsters() {
            assert!(m.xp_value > 0, "{} has 0 XP", m.name);
        }
    }

    /// Table-driven test pinning HD and XP for every monster entry.
    #[test]
    fn pinned_hd_and_xp() {
        let expected: &[(&str, &str, u64)] = &[
            ("Kobold", "1/2", 5),
            ("Giant Rat", "1/2", 5),
            ("Goblin", "1-1", 5),
            ("Skeleton", "1", 10),
            ("Zombie", "2", 20),
            ("Orc", "1", 10),
            ("Hobgoblin", "1+1", 15),
            ("Gnoll", "2", 20),
            ("Ghoul", "2*", 25),
            ("Bugbear", "3+1", 50),
            ("Gelatinous Cube", "4*", 125),
            ("Ogre", "4+1", 125),
            ("Wight", "3*", 50),
            ("Wraith", "4**", 175),
            ("Gargoyle", "4", 75),
            ("Owlbear", "5", 175),
            ("Troll", "6+3*", 650),
            ("Minotaur", "6", 275),
            ("Mummy", "5+1*", 400),
            ("Spectre", "6**", 725),
            ("Vampire", "7-9**", 1250),
            ("Green Dragon", "8**", 1750),
            ("Red Dragon", "10**", 2300),
            ("Giant Spider", "3*", 50),
            ("Carrion Crawler", "3+1", 50),
            ("Rust Monster", "5", 175),
            ("Basilisk", "6+1**", 950),
            ("Cockatrice", "5**", 425),
            ("Harpy", "3*", 50),
            ("Medusa", "4**", 175),
        ];

        assert_eq!(
            expected.len(),
            all_monsters().len(),
            "expected table has {} entries but MONSTERS has {}",
            expected.len(),
            all_monsters().len(),
        );

        for &(name, hd, xp) in expected {
            let m = find_monster(name)
                .unwrap_or_else(|| panic!("monster '{}' not found", name));
            assert_eq!(m.hit_dice, hd, "{}: hit_dice", name);
            assert_eq!(m.xp_value, xp, "{}: xp_value", name);
        }
    }
}
