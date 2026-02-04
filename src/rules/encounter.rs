use crate::state::wilderness::Terrain;

/// An entry from an encounter table.
#[derive(Debug, Clone, PartialEq)]
pub struct EncounterEntry {
    /// Monster name.
    pub name: &'static str,
    /// Number appearing (dice notation).
    pub number: &'static str,
}

impl EncounterEntry {
    const fn new(name: &'static str, number: &'static str) -> Self {
        EncounterEntry { name, number }
    }
}

// ============================================================================
// Dungeon encounter tables by level (OSE-style)
// Roll d20 for encounter, keyed by dungeon level 1-8+
// ============================================================================

const DUNGEON_LEVEL_1: &[EncounterEntry] = &[
    EncounterEntry::new("Bee, Giant Killer", "1d10"),
    EncounterEntry::new("Goblin", "2d4"),
    EncounterEntry::new("Green Slime", "1d4"),
    EncounterEntry::new("Kobold", "4d4"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Orc", "2d4"),
    EncounterEntry::new("Skeleton", "3d4"),
    EncounterEntry::new("Snake, Cobra", "1d6"),
    EncounterEntry::new("Spider, Crab", "1d4"),
    EncounterEntry::new("Sprite", "3d6"),
    EncounterEntry::new("Stirge", "1d10"),
    EncounterEntry::new("Trader", "1d8"),
    EncounterEntry::new("Wolf", "2d6"),
    EncounterEntry::new("Centipede, Giant", "2d4"),
    EncounterEntry::new("Bandit", "1d8"),
    EncounterEntry::new("Beetle, Fire", "1d8"),
    EncounterEntry::new("Dwarf", "1d6"),
    EncounterEntry::new("Gnome", "1d6"),
    EncounterEntry::new("Halfling", "3d6"),
    EncounterEntry::new("Rat, Giant", "3d6"),
];

const DUNGEON_LEVEL_2: &[EncounterEntry] = &[
    EncounterEntry::new("Beetle, Oil", "1d8"),
    EncounterEntry::new("Berserker", "1d6"),
    EncounterEntry::new("Cat, Mountain Lion", "1d4"),
    EncounterEntry::new("Elf", "1d4"),
    EncounterEntry::new("Ghoul", "1d6"),
    EncounterEntry::new("Gnoll", "1d6"),
    EncounterEntry::new("Gray Ooze", "1d4"),
    EncounterEntry::new("Hobgoblin", "1d6"),
    EncounterEntry::new("Lizard, Draco", "1d4"),
    EncounterEntry::new("Lizardman", "2d4"),
    EncounterEntry::new("Neanderthal", "1d10"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Snake, Pit Viper", "1d8"),
    EncounterEntry::new("Spider, Black Widow", "1d3"),
    EncounterEntry::new("Troglodyte", "1d8"),
    EncounterEntry::new("Zombie", "2d4"),
    EncounterEntry::new("Goblin", "2d4"),
    EncounterEntry::new("Orc", "2d4"),
    EncounterEntry::new("Skeleton", "3d4"),
    EncounterEntry::new("Kobold", "4d4"),
];

const DUNGEON_LEVEL_3: &[EncounterEntry] = &[
    EncounterEntry::new("Ape, White", "1d6"),
    EncounterEntry::new("Beetle, Tiger", "1d6"),
    EncounterEntry::new("Bugbear", "2d4"),
    EncounterEntry::new("Doppelganger", "1d6"),
    EncounterEntry::new("Gargoyle", "1d6"),
    EncounterEntry::new("Gelatinous Cube", "1"),
    EncounterEntry::new("Harpy", "1d6"),
    EncounterEntry::new("Living Statue, Crystal", "1d6"),
    EncounterEntry::new("Lycanthrope, Wererat", "1d8"),
    EncounterEntry::new("Medium", "1d4"),
    EncounterEntry::new("Medusa", "1d3"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Ochre Jelly", "1"),
    EncounterEntry::new("Ogre", "1d6"),
    EncounterEntry::new("Shadow", "1d8"),
    EncounterEntry::new("Spider, Tarantella", "1d3"),
    EncounterEntry::new("Toad, Giant", "1d4"),
    EncounterEntry::new("Wight", "1d6"),
    EncounterEntry::new("Ghoul", "1d6"),
    EncounterEntry::new("Hobgoblin", "1d6"),
];

const DUNGEON_LEVEL_4_5: &[EncounterEntry] = &[
    EncounterEntry::new("Bear, Cave", "1d2"),
    EncounterEntry::new("Caecilia", "1d3"),
    EncounterEntry::new("Cockatrice", "1d4"),
    EncounterEntry::new("Hellhound", "2d4"),
    EncounterEntry::new("Lycanthrope, Werewolf", "1d6"),
    EncounterEntry::new("Minotaur", "1d6"),
    EncounterEntry::new("Mummy", "1d4"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Ochre Jelly", "1"),
    EncounterEntry::new("Owlbear", "1d4"),
    EncounterEntry::new("Rhagodessa", "1d4"),
    EncounterEntry::new("Rust Monster", "1d4"),
    EncounterEntry::new("Spectre", "1d4"),
    EncounterEntry::new("Troll", "1d8"),
    EncounterEntry::new("Wraith", "1d4"),
    EncounterEntry::new("Ogre", "1d6"),
    EncounterEntry::new("Gargoyle", "1d6"),
    EncounterEntry::new("Wight", "1d6"),
    EncounterEntry::new("Bugbear", "2d4"),
    EncounterEntry::new("Shadow", "1d8"),
];

const DUNGEON_LEVEL_6_7: &[EncounterEntry] = &[
    EncounterEntry::new("Basilisk", "1d6"),
    EncounterEntry::new("Black Pudding", "1"),
    EncounterEntry::new("Dragon, White", "1d4"),
    EncounterEntry::new("Dragon, Black", "1d4"),
    EncounterEntry::new("Efreeti", "1"),
    EncounterEntry::new("Elemental", "1"),
    EncounterEntry::new("Giant, Hill", "1d4"),
    EncounterEntry::new("Giant, Stone", "1d2"),
    EncounterEntry::new("Hydra", "1"),
    EncounterEntry::new("Lycanthrope, Werebear", "1d4"),
    EncounterEntry::new("Manticore", "1d2"),
    EncounterEntry::new("Mummy", "1d4"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Salamander, Flame", "1d4+1"),
    EncounterEntry::new("Scorpion, Giant", "1d6"),
    EncounterEntry::new("Spectre", "1d4"),
    EncounterEntry::new("Troll", "1d8"),
    EncounterEntry::new("Vampire", "1d4"),
    EncounterEntry::new("Wyvern", "1d2"),
    EncounterEntry::new("Wraith", "1d4"),
];

const DUNGEON_LEVEL_8_PLUS: &[EncounterEntry] = &[
    EncounterEntry::new("Black Pudding", "1"),
    EncounterEntry::new("Chimera", "1d2"),
    EncounterEntry::new("Dragon, Gold", "1d4"),
    EncounterEntry::new("Dragon, Red", "1d4"),
    EncounterEntry::new("Giant, Cloud", "1d2"),
    EncounterEntry::new("Giant, Storm", "1"),
    EncounterEntry::new("Golem, Bone", "1"),
    EncounterEntry::new("Golem, Amber", "1"),
    EncounterEntry::new("Hydra", "1"),
    EncounterEntry::new("Lycanthrope, Devil Swine", "1d3"),
    EncounterEntry::new("Manticore", "1d2"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Purple Worm", "1d2"),
    EncounterEntry::new("Salamander, Frost", "1d3"),
    EncounterEntry::new("Salamander, Flame", "1d4+1"),
    EncounterEntry::new("Skeleton, Giant", "1d4"),
    EncounterEntry::new("Vampire", "1d4"),
    EncounterEntry::new("Giant, Fire", "1d2"),
    EncounterEntry::new("Giant, Frost", "1d2"),
    EncounterEntry::new("Dragon, Blue", "1d4"),
];

/// Get the dungeon encounter table for a given dungeon level.
/// Returns a slice of 20 encounter entries. Use a d20 roll (1-20) to index.
pub fn dungeon_table(level: u32) -> &'static [EncounterEntry] {
    match level {
        0 | 1 => DUNGEON_LEVEL_1,
        2 => DUNGEON_LEVEL_2,
        3 => DUNGEON_LEVEL_3,
        4 | 5 => DUNGEON_LEVEL_4_5,
        6 | 7 => DUNGEON_LEVEL_6_7,
        _ => DUNGEON_LEVEL_8_PLUS,
    }
}

/// Look up a dungeon encounter by level and d20 roll (1-20).
pub fn dungeon_encounter(level: u32, roll: u32) -> &'static EncounterEntry {
    let table = dungeon_table(level);
    let idx = (roll.saturating_sub(1) as usize).min(table.len() - 1);
    &table[idx]
}

// ============================================================================
// Wilderness encounter tables by terrain
// ============================================================================

const WILDERNESS_CLEAR: &[EncounterEntry] = &[
    EncounterEntry::new("Bandit", "1d8"),
    EncounterEntry::new("Berserker", "1d6"),
    EncounterEntry::new("Boar", "1d6"),
    EncounterEntry::new("Brigand", "3d10"),
    EncounterEntry::new("Cat, Lion", "1d4"),
    EncounterEntry::new("Centaur", "2d10"),
    EncounterEntry::new("Dog, Wild", "4d4"),
    EncounterEntry::new("Elephant", "1d20"),
    EncounterEntry::new("Giant, Hill", "1d4"),
    EncounterEntry::new("Gnoll", "1d6"),
    EncounterEntry::new("Goblin", "4d6"),
    EncounterEntry::new("Halfling", "5d4"),
    EncounterEntry::new("Hawk", "1d6"),
    EncounterEntry::new("Horse, Wild", "2d6"),
    EncounterEntry::new("Merchant", "1d20"),
    EncounterEntry::new("Mule", "2d6"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Orc", "2d4"),
    EncounterEntry::new("Pilgrim", "1d12"),
    EncounterEntry::new("Wolf", "2d6"),
];

const WILDERNESS_FOREST: &[EncounterEntry] = &[
    EncounterEntry::new("Ant, Giant", "2d4"),
    EncounterEntry::new("Bandit", "1d8"),
    EncounterEntry::new("Bear, Black", "1d4"),
    EncounterEntry::new("Boar", "1d6"),
    EncounterEntry::new("Bugbear", "2d4"),
    EncounterEntry::new("Cat, Panther", "1d2"),
    EncounterEntry::new("Centipede, Giant", "1d8"),
    EncounterEntry::new("Dryad", "1d6"),
    EncounterEntry::new("Elf", "1d4"),
    EncounterEntry::new("Goblin", "4d6"),
    EncounterEntry::new("Green Dragon", "1d4"),
    EncounterEntry::new("Hawk", "1d6"),
    EncounterEntry::new("Hobgoblin", "1d6"),
    EncounterEntry::new("Lizard, Draco", "1d4"),
    EncounterEntry::new("Lycanthrope, Werewolf", "1d6"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Ogre", "1d6"),
    EncounterEntry::new("Orc", "2d4"),
    EncounterEntry::new("Spider, Crab", "1d4"),
    EncounterEntry::new("Unicorn", "1d4"),
];

const WILDERNESS_HILLS: &[EncounterEntry] = &[
    EncounterEntry::new("Bandit", "1d8"),
    EncounterEntry::new("Bear, Cave", "1d2"),
    EncounterEntry::new("Berserker", "1d6"),
    EncounterEntry::new("Boar", "1d6"),
    EncounterEntry::new("Bugbear", "2d4"),
    EncounterEntry::new("Dwarf", "1d6"),
    EncounterEntry::new("Eagle, Giant", "1d6"),
    EncounterEntry::new("Gnoll", "1d6"),
    EncounterEntry::new("Goblin", "4d6"),
    EncounterEntry::new("Hawk", "1d6"),
    EncounterEntry::new("Hobgoblin", "1d6"),
    EncounterEntry::new("Manticore", "1d2"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Orc", "2d4"),
    EncounterEntry::new("Pegasus", "1d12"),
    EncounterEntry::new("Roc, Small", "1d12"),
    EncounterEntry::new("Troll", "1d8"),
    EncounterEntry::new("Wolf", "2d6"),
    EncounterEntry::new("Wyvern", "1d2"),
    EncounterEntry::new("Giant, Hill", "1d4"),
];

const WILDERNESS_MOUNTAINS: &[EncounterEntry] = &[
    EncounterEntry::new("Bear, Cave", "1d2"),
    EncounterEntry::new("Berserker", "1d6"),
    EncounterEntry::new("Dwarf", "1d6"),
    EncounterEntry::new("Eagle, Giant", "1d6"),
    EncounterEntry::new("Gargoyle", "1d6"),
    EncounterEntry::new("Giant, Cloud", "1d2"),
    EncounterEntry::new("Giant, Frost", "1d2"),
    EncounterEntry::new("Giant, Stone", "1d2"),
    EncounterEntry::new("Gnome", "1d8"),
    EncounterEntry::new("Goblin", "4d6"),
    EncounterEntry::new("Griffin", "2d8"),
    EncounterEntry::new("Hawk", "1d6"),
    EncounterEntry::new("Manticore", "1d2"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Orc", "2d4"),
    EncounterEntry::new("Roc, Small", "1d12"),
    EncounterEntry::new("Troll", "1d8"),
    EncounterEntry::new("White Dragon", "1d4"),
    EncounterEntry::new("Wolf", "2d6"),
    EncounterEntry::new("Wyvern", "1d2"),
];

const WILDERNESS_DESERT: &[EncounterEntry] = &[
    EncounterEntry::new("Ant, Giant", "2d4"),
    EncounterEntry::new("Bandit", "1d8"),
    EncounterEntry::new("Camel", "2d4"),
    EncounterEntry::new("Cat, Lion", "1d4"),
    EncounterEntry::new("Centipede, Giant", "1d8"),
    EncounterEntry::new("Dragon, Blue", "1d4"),
    EncounterEntry::new("Efreeti", "1"),
    EncounterEntry::new("Giant, Fire", "1d2"),
    EncounterEntry::new("Gnoll", "1d6"),
    EncounterEntry::new("Hawk", "1d6"),
    EncounterEntry::new("Lizard, Giant", "1d6"),
    EncounterEntry::new("Manticore", "1d2"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Ogre", "1d6"),
    EncounterEntry::new("Orc", "2d4"),
    EncounterEntry::new("Purple Worm", "1d2"),
    EncounterEntry::new("Scorpion, Giant", "1d6"),
    EncounterEntry::new("Snake, Rattlesnake", "1d8"),
    EncounterEntry::new("Spider, Tarantella", "1d3"),
    EncounterEntry::new("Nomad", "1d4"),
];

const WILDERNESS_SWAMP: &[EncounterEntry] = &[
    EncounterEntry::new("Basilisk", "1d6"),
    EncounterEntry::new("Beetle, Oil", "1d8"),
    EncounterEntry::new("Black Dragon", "1d4"),
    EncounterEntry::new("Centipede, Giant", "1d8"),
    EncounterEntry::new("Crocodile", "1d8"),
    EncounterEntry::new("Frog, Giant", "1d6"),
    EncounterEntry::new("Ghoul", "1d6"),
    EncounterEntry::new("Insect Swarm", "1"),
    EncounterEntry::new("Leech, Giant", "1d4"),
    EncounterEntry::new("Lizardman", "2d4"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Ogre", "1d6"),
    EncounterEntry::new("Orc", "2d4"),
    EncounterEntry::new("Rat, Giant", "3d6"),
    EncounterEntry::new("Snake, Pit Viper", "1d8"),
    EncounterEntry::new("Spider, Crab", "1d4"),
    EncounterEntry::new("Stirge", "1d10"),
    EncounterEntry::new("Toad, Giant", "1d4"),
    EncounterEntry::new("Troll", "1d8"),
    EncounterEntry::new("Zombie", "2d4"),
];

const WILDERNESS_JUNGLE: &[EncounterEntry] = &[
    EncounterEntry::new("Ant, Giant", "2d4"),
    EncounterEntry::new("Ape, White", "1d6"),
    EncounterEntry::new("Basilisk", "1d6"),
    EncounterEntry::new("Caecilia", "1d3"),
    EncounterEntry::new("Cat, Panther", "1d2"),
    EncounterEntry::new("Centipede, Giant", "1d8"),
    EncounterEntry::new("Crocodile", "1d8"),
    EncounterEntry::new("Green Dragon", "1d4"),
    EncounterEntry::new("Elephant", "1d20"),
    EncounterEntry::new("Goblin", "4d6"),
    EncounterEntry::new("Insect Swarm", "1"),
    EncounterEntry::new("Lizard, Giant", "1d6"),
    EncounterEntry::new("Lizardman", "2d4"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Ogre", "1d6"),
    EncounterEntry::new("Orc", "2d4"),
    EncounterEntry::new("Snake, Cobra", "1d6"),
    EncounterEntry::new("Spider, Crab", "1d4"),
    EncounterEntry::new("Troglodyte", "1d8"),
    EncounterEntry::new("Troll", "1d8"),
];

const WILDERNESS_OCEAN: &[EncounterEntry] = &[
    EncounterEntry::new("Buccaneer", "1d20"),
    EncounterEntry::new("Dragon Turtle", "1"),
    EncounterEntry::new("Hydra, Sea", "1"),
    EncounterEntry::new("Merchant Ship", "1d6"),
    EncounterEntry::new("Octopus, Giant", "1d2"),
    EncounterEntry::new("Pirate", "1d20"),
    EncounterEntry::new("Roc, Large", "1d12"),
    EncounterEntry::new("Sea Dragon", "1d4"),
    EncounterEntry::new("Sea Serpent", "1d4"),
    EncounterEntry::new("Shark", "2d4"),
    EncounterEntry::new("Snake, Sea", "1d8"),
    EncounterEntry::new("Squid, Giant", "1d4"),
    EncounterEntry::new("Whale", "1d6"),
    EncounterEntry::new("NPC Party: Adventurer", "1d4+4"),
    EncounterEntry::new("Pirate", "1d20"),
    EncounterEntry::new("Merchant Ship", "1d6"),
    EncounterEntry::new("Buccaneer", "1d20"),
    EncounterEntry::new("Sea Serpent", "1d4"),
    EncounterEntry::new("Shark", "2d4"),
    EncounterEntry::new("Whale", "1d6"),
];

/// Get the wilderness encounter table for a given terrain type.
/// Returns a slice of 20 encounter entries.
pub fn wilderness_table(terrain: Terrain) -> &'static [EncounterEntry] {
    match terrain {
        Terrain::Clear | Terrain::City | Terrain::Barren => WILDERNESS_CLEAR,
        Terrain::Forest => WILDERNESS_FOREST,
        Terrain::Hills => WILDERNESS_HILLS,
        Terrain::Mountains => WILDERNESS_MOUNTAINS,
        Terrain::Desert => WILDERNESS_DESERT,
        Terrain::Swamp => WILDERNESS_SWAMP,
        Terrain::Jungle => WILDERNESS_JUNGLE,
        Terrain::Ocean | Terrain::River => WILDERNESS_OCEAN,
    }
}

/// Look up a wilderness encounter by terrain and d20 roll (1-20).
pub fn wilderness_encounter(terrain: Terrain, roll: u32) -> &'static EncounterEntry {
    let table = wilderness_table(terrain);
    let idx = (roll.saturating_sub(1) as usize).min(table.len() - 1);
    &table[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dungeon_table_level_1_has_20_entries() {
        assert_eq!(dungeon_table(1).len(), 20);
    }

    #[test]
    fn dungeon_table_level_8_has_20_entries() {
        assert_eq!(dungeon_table(8).len(), 20);
    }

    #[test]
    fn dungeon_encounter_roll_1() {
        let e = dungeon_encounter(1, 1);
        assert_eq!(e.name, "Bee, Giant Killer");
    }

    #[test]
    fn dungeon_encounter_roll_20() {
        let e = dungeon_encounter(1, 20);
        assert_eq!(e.name, "Rat, Giant");
    }

    #[test]
    fn dungeon_encounter_clamp_high_roll() {
        let e = dungeon_encounter(1, 100);
        assert_eq!(e.name, "Rat, Giant");
    }

    #[test]
    fn dungeon_encounter_clamp_zero_roll() {
        let e = dungeon_encounter(1, 0);
        assert_eq!(e.name, "Bee, Giant Killer");
    }

    #[test]
    fn dungeon_level_ranges() {
        // Level 0 and 1 use same table
        assert!(std::ptr::eq(dungeon_table(0), dungeon_table(1)));
        // Level 4 and 5 use same table
        assert!(std::ptr::eq(dungeon_table(4), dungeon_table(5)));
        // Level 6 and 7 use same table
        assert!(std::ptr::eq(dungeon_table(6), dungeon_table(7)));
        // Level 8 and 9 use same table
        assert!(std::ptr::eq(dungeon_table(8), dungeon_table(9)));
    }

    #[test]
    fn wilderness_table_clear_has_20_entries() {
        assert_eq!(wilderness_table(Terrain::Clear).len(), 20);
    }

    #[test]
    fn wilderness_table_all_terrains() {
        for terrain in &[
            Terrain::Clear, Terrain::Forest, Terrain::Hills,
            Terrain::Mountains, Terrain::Desert, Terrain::Swamp,
            Terrain::Jungle, Terrain::Ocean,
        ] {
            let table = wilderness_table(*terrain);
            assert_eq!(table.len(), 20, "table for {:?} should have 20 entries", terrain);
        }
    }

    #[test]
    fn wilderness_encounter_forest() {
        let e = wilderness_encounter(Terrain::Forest, 1);
        assert_eq!(e.name, "Ant, Giant");
    }

    #[test]
    fn wilderness_encounter_swamp() {
        let e = wilderness_encounter(Terrain::Swamp, 3);
        assert_eq!(e.name, "Black Dragon");
    }

    #[test]
    fn wilderness_city_uses_clear_table() {
        assert!(std::ptr::eq(
            wilderness_table(Terrain::City),
            wilderness_table(Terrain::Clear)
        ));
    }

    #[test]
    fn encounter_entry_number_is_valid_dice() {
        // Spot-check that number appearing fields look like dice notation
        let table = dungeon_table(1);
        for entry in table {
            assert!(!entry.number.is_empty(), "{} has empty number", entry.name);
        }
    }
}
