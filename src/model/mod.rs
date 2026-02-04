use serde::{Deserialize, Serialize};
use crate::rules::save::SavingThrows;

/// Character ability scores.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbilityScores {
    pub strength: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub charisma: i32,
}

impl AbilityScores {
    /// Convert to array: [STR, INT, WIS, DEX, CON, CHA].
    pub fn to_array(&self) -> [i32; 6] {
        [self.strength, self.intelligence, self.wisdom,
         self.dexterity, self.constitution, self.charisma]
    }

    /// Create from array: [STR, INT, WIS, DEX, CON, CHA].
    pub fn from_array(a: &[i32; 6]) -> Self {
        AbilityScores {
            strength: a[0], intelligence: a[1], wisdom: a[2],
            dexterity: a[3], constitution: a[4], charisma: a[5],
        }
    }
}

/// A player or non-player character.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub class: String,
    pub level: u32,
    pub abilities: AbilityScores,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub xp: u64,
    pub inventory: Vec<Item>,
    pub spells: Vec<Spell>,
    #[serde(default)]
    pub alignment: String,
    #[serde(default)]
    pub gold_gp: u32,
    #[serde(default)]
    pub saving_throws: Option<SavingThrows>,
    #[serde(default)]
    pub thac0: u32,
    #[serde(default)]
    pub movement_rate: u32,
}

impl Character {
    pub fn new(name: &str, class: &str) -> Self {
        Character {
            name: name.to_string(),
            class: class.to_string(),
            level: 1,
            abilities: AbilityScores::default(),
            hp: 1,
            max_hp: 1,
            ac: 9,
            xp: 0,
            inventory: Vec::new(),
            spells: Vec::new(),
            alignment: String::new(),
            gold_gp: 0,
            saving_throws: None,
            thac0: 19,
            movement_rate: 120,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

/// A monster or creature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monster {
    pub name: String,
    pub hit_dice: String,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub attacks: Vec<String>,
    pub damage: String,
    pub morale: u32,
    pub xp_value: u64,
}

impl Monster {
    pub fn new(name: &str, hit_dice: &str) -> Self {
        Monster {
            name: name.to_string(),
            hit_dice: hit_dice.to_string(),
            hp: 1,
            max_hp: 1,
            ac: 9,
            attacks: Vec::new(),
            damage: String::new(),
            morale: 7,
            xp_value: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

/// A party of characters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub members: Vec<Character>,
    pub gold: u64,
    pub marching_order: Vec<String>,
}

impl Party {
    pub fn new() -> Self {
        Party {
            members: Vec::new(),
            gold: 0,
            marching_order: Vec::new(),
        }
    }

    pub fn add_member(&mut self, character: Character) {
        self.marching_order.push(character.name.clone());
        self.members.push(character);
    }

    pub fn find_member(&self, name: &str) -> Option<&Character> {
        self.members.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    pub fn find_member_mut(&mut self, name: &str) -> Option<&mut Character> {
        self.members.iter_mut().find(|c| c.name.eq_ignore_ascii_case(name))
    }
}

impl Default for Party {
    fn default() -> Self {
        Self::new()
    }
}

/// An item or piece of equipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub weight: f32,
    pub value_gp: u32,
    pub equipped: bool,
}

impl Item {
    pub fn new(name: &str, weight: f32, value_gp: u32) -> Self {
        Item {
            name: name.to_string(),
            weight,
            value_gp,
            equipped: false,
        }
    }
}

/// A spell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spell {
    pub name: String,
    pub level: u32,
    pub range: String,
    pub duration: String,
    pub description: String,
}

impl Spell {
    pub fn new(name: &str, level: u32) -> Self {
        Spell {
            name: name.to_string(),
            level,
            range: String::new(),
            duration: String::new(),
            description: String::new(),
        }
    }
}

/// State of an active combat encounter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatState {
    pub round: u32,
    pub monsters: Vec<Monster>,
    pub party_initiative: i32,
    pub monster_initiative: i32,
    pub distance: u32,
    pub log: Vec<String>,
    /// Characters who declared spells this round (for disruption tracking).
    #[serde(default)]
    pub spell_declarations: Vec<String>,
    /// Characters whose spells were disrupted this round.
    #[serde(default)]
    pub disrupted: Vec<String>,
}

impl CombatState {
    pub fn new(monsters: Vec<Monster>, distance: u32) -> Self {
        CombatState {
            round: 0,
            monsters,
            party_initiative: 0,
            monster_initiative: 0,
            distance,
            log: Vec::new(),
            spell_declarations: Vec::new(),
            disrupted: Vec::new(),
        }
    }

    pub fn living_monsters(&self) -> Vec<(usize, &Monster)> {
        self.monsters.iter().enumerate()
            .filter(|(_, m)| m.is_alive())
            .collect()
    }

    pub fn living_monster_count(&self) -> usize {
        self.monsters.iter().filter(|m| m.is_alive()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_creation() {
        let c = Character::new("Theron", "Fighter");
        assert_eq!(c.name, "Theron");
        assert_eq!(c.class, "Fighter");
        assert_eq!(c.level, 1);
        assert!(c.is_alive());
    }

    #[test]
    fn party_operations() {
        let mut party = Party::new();
        party.add_member(Character::new("Arden", "Cleric"));
        party.add_member(Character::new("Brin", "Thief"));
        assert_eq!(party.members.len(), 2);
        assert!(party.find_member("arden").is_some());
        assert!(party.find_member("nobody").is_none());
    }

    #[test]
    fn serialization_roundtrip() {
        let c = Character::new("Test", "Magic-User");
        let json = serde_json::to_string(&c).unwrap();
        let c2: Character = serde_json::from_str(&json).unwrap();
        assert_eq!(c.name, c2.name);
        assert_eq!(c.class, c2.class);
    }
}
