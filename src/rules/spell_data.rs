/// Spell definitions per OSE Rules Tome.
/// Covers Cleric, Magic-User, Druid, and Illusionist spell lists.

/// A spell definition with all relevant game data.
#[derive(Debug, Clone)]
pub struct SpellDef {
    pub name: &'static str,
    pub list: SpellList,
    pub level: u32,
    pub range: &'static str,
    pub duration: &'static str,
    pub description: &'static str,
}

/// Which spell list a spell belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellList {
    Cleric,
    MagicUser,
    Druid,
    Illusionist,
}

impl SpellList {
    pub fn name(self) -> &'static str {
        match self {
            SpellList::Cleric => "Cleric",
            SpellList::MagicUser => "Magic-User",
            SpellList::Druid => "Druid",
            SpellList::Illusionist => "Illusionist",
        }
    }
}

/// Get all spells for a given list and level.
pub fn spells_by_list_and_level(list: SpellList, level: u32) -> Vec<&'static SpellDef> {
    ALL_SPELLS.iter()
        .filter(|s| s.list == list && s.level == level)
        .collect()
}

/// Find a spell by name (case-insensitive) and optional list.
pub fn find_spell(name: &str, list: Option<SpellList>) -> Option<&'static SpellDef> {
    ALL_SPELLS.iter().find(|s| {
        s.name.eq_ignore_ascii_case(name) && list.map_or(true, |l| s.list == l)
    })
}

/// Get all spells.
pub fn all_spells() -> &'static [SpellDef] {
    &ALL_SPELLS
}

static ALL_SPELLS: [SpellDef; 72] = [
    // =========================================================================
    // CLERIC SPELLS
    // =========================================================================
    // Level 1
    SpellDef { name: "Cure Light Wounds", list: SpellList::Cleric, level: 1,
        range: "Touch", duration: "Instant",
        description: "Heals 1d6+1 HP or cures paralysis" },
    SpellDef { name: "Detect Evil", list: SpellList::Cleric, level: 1,
        range: "120'", duration: "6 turns",
        description: "Reveals enchanted or evil creatures/objects within range" },
    SpellDef { name: "Detect Magic", list: SpellList::Cleric, level: 1,
        range: "60'", duration: "2 turns",
        description: "Enchanted objects, areas, or creatures glow" },
    SpellDef { name: "Light", list: SpellList::Cleric, level: 1,
        range: "120'", duration: "12 turns",
        description: "Magical light in 15' radius. Reversed: Darkness" },
    SpellDef { name: "Protection from Evil", list: SpellList::Cleric, level: 1,
        range: "Self", duration: "12 turns",
        description: "+1 AC and saves vs evil creatures. Evil creatures -1 to attack" },
    SpellDef { name: "Purify Food and Water", list: SpellList::Cleric, level: 1,
        range: "10'", duration: "Instant",
        description: "Makes spoiled or poisoned food and water safe" },
    SpellDef { name: "Resist Cold", list: SpellList::Cleric, level: 1,
        range: "30'", duration: "6 turns",
        description: "Target unharmed by non-magical cold. +2 save vs magical cold, damage halved" },
    // Level 2
    SpellDef { name: "Bless", list: SpellList::Cleric, level: 2,
        range: "60'", duration: "6 turns",
        description: "Allies in 20' area gain +1 to attack and morale. Reversed: Blight" },
    SpellDef { name: "Find Traps", list: SpellList::Cleric, level: 2,
        range: "30'", duration: "2 turns",
        description: "Traps in range glow with magical light" },
    SpellDef { name: "Hold Person", list: SpellList::Cleric, level: 2,
        range: "180'", duration: "9 turns",
        description: "Paralyses 1d4 humanoids (save vs Spells negates)" },
    SpellDef { name: "Know Alignment", list: SpellList::Cleric, level: 2,
        range: "10'", duration: "1 round",
        description: "Reveals alignment of one creature or object" },
    SpellDef { name: "Resist Fire", list: SpellList::Cleric, level: 2,
        range: "30'", duration: "6 turns",
        description: "Target unharmed by non-magical fire. +2 save vs magical fire, damage halved" },
    SpellDef { name: "Silence 15' Radius", list: SpellList::Cleric, level: 2,
        range: "180'", duration: "12 turns",
        description: "Complete silence in 15' radius. Prevents spellcasting" },
    SpellDef { name: "Snake Charm", list: SpellList::Cleric, level: 2,
        range: "60'", duration: "1d4+1 rounds",
        description: "Charms snakes with total HD up to caster's level" },
    SpellDef { name: "Speak with Animals", list: SpellList::Cleric, level: 2,
        range: "Self", duration: "6 turns",
        description: "Can communicate with normal animals" },
    // Level 3
    SpellDef { name: "Continual Light", list: SpellList::Cleric, level: 3,
        range: "120'", duration: "Permanent",
        description: "Brilliant light in 30' radius, permanent until dispelled" },
    SpellDef { name: "Cure Disease", list: SpellList::Cleric, level: 3,
        range: "30'", duration: "Instant",
        description: "Cures any disease. Reversed: Cause Disease" },
    SpellDef { name: "Growth of Animal", list: SpellList::Cleric, level: 3,
        range: "120'", duration: "12 turns",
        description: "One normal animal doubles in size. Double damage, can carry more" },
    SpellDef { name: "Locate Object", list: SpellList::Cleric, level: 3,
        range: "120'", duration: "6 turns",
        description: "Senses direction of a known object within range" },
    SpellDef { name: "Remove Curse", list: SpellList::Cleric, level: 3,
        range: "Touch", duration: "Instant",
        description: "Removes one curse from a person or object. Reversed: Curse" },
    SpellDef { name: "Striking", list: SpellList::Cleric, level: 3,
        range: "30'", duration: "1 turn",
        description: "One weapon deals +1d6 damage and can hit creatures requiring magic weapons" },

    // =========================================================================
    // MAGIC-USER SPELLS
    // =========================================================================
    // Level 1
    SpellDef { name: "Charm Person", list: SpellList::MagicUser, level: 1,
        range: "120'", duration: "Special",
        description: "Target humanoid regards caster as friend (save vs Spells negates)" },
    SpellDef { name: "Detect Magic", list: SpellList::MagicUser, level: 1,
        range: "60'", duration: "2 turns",
        description: "Enchanted objects, areas, or creatures glow" },
    SpellDef { name: "Floating Disc", list: SpellList::MagicUser, level: 1,
        range: "Self", duration: "6 turns",
        description: "Invisible disc carries 500 cn, follows caster at 60'" },
    SpellDef { name: "Hold Portal", list: SpellList::MagicUser, level: 1,
        range: "10'", duration: "2d6 turns",
        description: "Magically holds shut a door or gate" },
    SpellDef { name: "Magic Missile", list: SpellList::MagicUser, level: 1,
        range: "150'", duration: "1 round",
        description: "Unerring bolt deals 1d6+1 damage. Always hits, no save" },
    SpellDef { name: "Protection from Evil", list: SpellList::MagicUser, level: 1,
        range: "Self", duration: "6 turns",
        description: "+1 AC and saves vs evil creatures. Evil creatures -1 to attack" },
    SpellDef { name: "Read Languages", list: SpellList::MagicUser, level: 1,
        range: "Self", duration: "2 turns",
        description: "Can read any language, inscription, map, code" },
    SpellDef { name: "Read Magic", list: SpellList::MagicUser, level: 1,
        range: "Self", duration: "1 turn",
        description: "Can read magical text (scrolls, spell books)" },
    SpellDef { name: "Shield", list: SpellList::MagicUser, level: 1,
        range: "Self", duration: "2 turns",
        description: "AC 2 vs missiles, AC 4 vs other attacks" },
    SpellDef { name: "Sleep", list: SpellList::MagicUser, level: 1,
        range: "240'", duration: "4d4 turns",
        description: "Causes 2d8 HD of creatures to fall asleep (no save). Affects weakest first" },
    SpellDef { name: "Ventriloquism", list: SpellList::MagicUser, level: 1,
        range: "60'", duration: "2 turns",
        description: "Caster's voice seems to come from another location" },
    // Level 2
    SpellDef { name: "Continual Light", list: SpellList::MagicUser, level: 2,
        range: "120'", duration: "Permanent",
        description: "Brilliant light in 30' radius, permanent" },
    SpellDef { name: "Detect Evil", list: SpellList::MagicUser, level: 2,
        range: "60'", duration: "2 turns",
        description: "Reveals enchanted or evil creatures/objects" },
    SpellDef { name: "Detect Invisible", list: SpellList::MagicUser, level: 2,
        range: "Self", duration: "6 turns",
        description: "Caster can see invisible creatures and objects" },
    SpellDef { name: "ESP", list: SpellList::MagicUser, level: 2,
        range: "60'", duration: "12 turns",
        description: "Read surface thoughts of one creature. Blocked by 2' rock, thin lead" },
    SpellDef { name: "Invisibility", list: SpellList::MagicUser, level: 2,
        range: "240'", duration: "Until broken",
        description: "Target becomes invisible. Ends if target attacks or casts spell" },
    SpellDef { name: "Knock", list: SpellList::MagicUser, level: 2,
        range: "60'", duration: "1 round",
        description: "Opens stuck, barred, locked, held, or wizard-locked door" },
    SpellDef { name: "Levitate", list: SpellList::MagicUser, level: 2,
        range: "Self", duration: "6 turns + level",
        description: "Caster moves vertically at 20'/round. No horizontal movement" },
    SpellDef { name: "Mirror Image", list: SpellList::MagicUser, level: 2,
        range: "Self", duration: "6 turns",
        description: "1d4 illusory duplicates appear. Each hit destroys one duplicate" },
    SpellDef { name: "Phantasmal Force", list: SpellList::MagicUser, level: 2,
        range: "240'", duration: "Concentration",
        description: "Visual illusion in 20' cube. Deals illusory damage until disbelieved" },
    SpellDef { name: "Web", list: SpellList::MagicUser, level: 2,
        range: "10'", duration: "48 turns",
        description: "Fills 10'x10'x10' area with sticky webs. STR check to break free" },
    // Level 3
    SpellDef { name: "Clairvoyance", list: SpellList::MagicUser, level: 3,
        range: "60'", duration: "12 turns",
        description: "See through eyes of another creature within range" },
    SpellDef { name: "Dispel Magic", list: SpellList::MagicUser, level: 3,
        range: "120'", duration: "Instant",
        description: "Ends spells in 20' cube. Success based on caster level comparison" },
    SpellDef { name: "Fireball", list: SpellList::MagicUser, level: 3,
        range: "240'", duration: "Instant",
        description: "Explodes in 20' radius for 1d6 per caster level (save for half)" },
    SpellDef { name: "Fly", list: SpellList::MagicUser, level: 3,
        range: "Self", duration: "1d6 turns + level",
        description: "Caster can fly at 120'/turn" },
    SpellDef { name: "Haste", list: SpellList::MagicUser, level: 3,
        range: "240'", duration: "3 turns",
        description: "24 creatures in 60' area double movement and attacks. Reversed: Slow" },
    SpellDef { name: "Hold Person", list: SpellList::MagicUser, level: 3,
        range: "120'", duration: "1 turn/level",
        description: "Paralyses 1d4 humanoids (save vs Spells negates)" },
    SpellDef { name: "Infravision", list: SpellList::MagicUser, level: 3,
        range: "Self", duration: "1 day",
        description: "Caster can see in darkness to 60'" },
    SpellDef { name: "Invisibility 10' Radius", list: SpellList::MagicUser, level: 3,
        range: "120'", duration: "Until broken",
        description: "All creatures within 10' of target become invisible" },
    SpellDef { name: "Lightning Bolt", list: SpellList::MagicUser, level: 3,
        range: "180'", duration: "Instant",
        description: "60' long, 5' wide bolt deals 1d6 per caster level (save for half)" },
    SpellDef { name: "Protection from Evil 10' Radius", list: SpellList::MagicUser, level: 3,
        range: "Self", duration: "12 turns",
        description: "As Protection from Evil but affects all within 10' of caster" },
    SpellDef { name: "Protection from Normal Missiles", list: SpellList::MagicUser, level: 3,
        range: "30'", duration: "12 turns",
        description: "Target immune to non-magical missiles" },
    SpellDef { name: "Water Breathing", list: SpellList::MagicUser, level: 3,
        range: "30'", duration: "1 day",
        description: "Target can breathe underwater" },

    // =========================================================================
    // DRUID SPELLS
    // =========================================================================
    // Level 1
    SpellDef { name: "Animal Friendship", list: SpellList::Druid, level: 1,
        range: "120'", duration: "Permanent",
        description: "Charms a normal animal (save vs Spells negates). Max HD 2+1" },
    SpellDef { name: "Detect Danger", list: SpellList::Druid, level: 1,
        range: "120'", duration: "6 turns",
        description: "Detects natural dangers (quicksand, sinkholes, unsafe areas)" },
    SpellDef { name: "Entangle", list: SpellList::Druid, level: 1,
        range: "60'", duration: "1 turn",
        description: "Plants in 20' area grasp and hold creatures (save vs Spells to move at half)" },
    SpellDef { name: "Faerie Fire", list: SpellList::Druid, level: 1,
        range: "60'", duration: "1 round/level",
        description: "Outlines creatures in 10' radius with glow. -2 AC to affected targets" },
    SpellDef { name: "Predict Weather", list: SpellList::Druid, level: 1,
        range: "Self", duration: "Instant",
        description: "Reveals weather conditions for the next 12 hours in 1 mile radius" },
    SpellDef { name: "Speak with Animals", list: SpellList::Druid, level: 1,
        range: "Self", duration: "6 turns",
        description: "Can communicate with normal animals" },
    // Level 2
    SpellDef { name: "Barkskin", list: SpellList::Druid, level: 2,
        range: "Touch", duration: "4 rounds + 1/level",
        description: "Target's skin becomes bark-like, AC 6 (or +1 if already better)" },
    SpellDef { name: "Create Water", list: SpellList::Druid, level: 2,
        range: "10'", duration: "Instant",
        description: "Creates enough water for 12 creatures and mounts for 1 day" },
    SpellDef { name: "Heat Metal", list: SpellList::Druid, level: 2,
        range: "30'", duration: "7 rounds",
        description: "Heats metal worn by target. Escalating damage over 7 rounds" },
    SpellDef { name: "Obscurement", list: SpellList::Druid, level: 2,
        range: "Self", duration: "1 turn/level",
        description: "Misty vapour around druid. Missile attacks -2 in area" },
    SpellDef { name: "Produce Flame", list: SpellList::Druid, level: 2,
        range: "Self/40'", duration: "2 rounds/level",
        description: "Flame in hand: light, melee 1d4+1, or throw 40' for 1d4+1" },
    SpellDef { name: "Warp Wood", list: SpellList::Druid, level: 2,
        range: "240'", duration: "Permanent",
        description: "Warps wooden objects. Can ruin weapons, open doors, etc." },

    // =========================================================================
    // ILLUSIONIST SPELLS
    // =========================================================================
    // Level 1
    SpellDef { name: "Auditory Illusion", list: SpellList::Illusionist, level: 1,
        range: "240'", duration: "2 rounds/level",
        description: "Creates illusory sounds. Volume as 4 humans per caster level" },
    SpellDef { name: "Chromatic Orb", list: SpellList::Illusionist, level: 1,
        range: "60'", duration: "1 round",
        description: "Hurls orb of colour. 1d6 damage, effect varies by colour" },
    SpellDef { name: "Colour Spray", list: SpellList::Illusionist, level: 1,
        range: "Self", duration: "Instant",
        description: "Cone of colours. 1-2 HD: unconscious. 3-4 HD: blinded. 5+ HD: stunned" },
    SpellDef { name: "Dancing Lights", list: SpellList::Illusionist, level: 1,
        range: "120'", duration: "2 rounds/level",
        description: "Creates 1-4 lights or a glowing humanoid shape" },
    SpellDef { name: "Phantasmal Force", list: SpellList::Illusionist, level: 1,
        range: "240'", duration: "Concentration",
        description: "Visual illusion in 20' cube. Deals illusory damage until disbelieved" },
    SpellDef { name: "Wall of Fog", list: SpellList::Illusionist, level: 1,
        range: "30'", duration: "1 turn + 1 round/level",
        description: "Creates 20'x20'x20' fog bank. Blocks sight, missile attacks -2" },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_spell_count() {
        assert_eq!(all_spells().len(), 72);
    }

    #[test]
    fn cleric_level_1_spells() {
        let spells = spells_by_list_and_level(SpellList::Cleric, 1);
        assert_eq!(spells.len(), 7);
        assert!(spells.iter().any(|s| s.name == "Cure Light Wounds"));
    }

    #[test]
    fn cleric_level_2_spells() {
        let spells = spells_by_list_and_level(SpellList::Cleric, 2);
        assert_eq!(spells.len(), 8);
        assert!(spells.iter().any(|s| s.name == "Hold Person"));
    }

    #[test]
    fn cleric_level_3_spells() {
        let spells = spells_by_list_and_level(SpellList::Cleric, 3);
        assert_eq!(spells.len(), 6);
    }

    #[test]
    fn magic_user_level_1_spells() {
        let spells = spells_by_list_and_level(SpellList::MagicUser, 1);
        assert_eq!(spells.len(), 11);
        assert!(spells.iter().any(|s| s.name == "Magic Missile"));
        assert!(spells.iter().any(|s| s.name == "Sleep"));
    }

    #[test]
    fn magic_user_level_2_spells() {
        let spells = spells_by_list_and_level(SpellList::MagicUser, 2);
        assert_eq!(spells.len(), 10);
        assert!(spells.iter().any(|s| s.name == "Invisibility"));
    }

    #[test]
    fn magic_user_level_3_spells() {
        let spells = spells_by_list_and_level(SpellList::MagicUser, 3);
        assert_eq!(spells.len(), 12);
        assert!(spells.iter().any(|s| s.name == "Fireball"));
        assert!(spells.iter().any(|s| s.name == "Lightning Bolt"));
    }

    #[test]
    fn druid_level_1_spells() {
        let spells = spells_by_list_and_level(SpellList::Druid, 1);
        assert_eq!(spells.len(), 6);
    }

    #[test]
    fn druid_level_2_spells() {
        let spells = spells_by_list_and_level(SpellList::Druid, 2);
        assert_eq!(spells.len(), 6);
    }

    #[test]
    fn illusionist_level_1_spells() {
        let spells = spells_by_list_and_level(SpellList::Illusionist, 1);
        assert_eq!(spells.len(), 6);
    }

    #[test]
    fn find_magic_missile() {
        let spell = find_spell("Magic Missile", None).unwrap();
        assert_eq!(spell.level, 1);
        assert_eq!(spell.list, SpellList::MagicUser);
        assert_eq!(spell.range, "150'");
    }

    #[test]
    fn find_spell_case_insensitive() {
        assert!(find_spell("magic missile", None).is_some());
        assert!(find_spell("FIREBALL", None).is_some());
    }

    #[test]
    fn find_spell_with_list_filter() {
        // Both cleric and MU have "Detect Magic" — filter by list
        let cleric = find_spell("Detect Magic", Some(SpellList::Cleric)).unwrap();
        assert_eq!(cleric.list, SpellList::Cleric);
        let mu = find_spell("Detect Magic", Some(SpellList::MagicUser)).unwrap();
        assert_eq!(mu.list, SpellList::MagicUser);
    }

    #[test]
    fn sleep_description() {
        let spell = find_spell("Sleep", None).unwrap();
        assert!(spell.description.contains("2d8 HD"));
    }

    #[test]
    fn cure_light_wounds() {
        let spell = find_spell("Cure Light Wounds", None).unwrap();
        assert_eq!(spell.range, "Touch");
        assert_eq!(spell.list, SpellList::Cleric);
    }
}
