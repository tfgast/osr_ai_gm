use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use crate::log_entry::LogEntry;
use crate::rules::alignment::Alignment;
use crate::rules::attack::HitDice;
use crate::rules::class::Class;
use crate::rules::save::SavingThrows;
use crate::state::effect::ActiveEffect;

/// A single attack routine a monster can perform each round.
/// For a bear with "2 claws (1d3), 1 bite (1d6)", this expands to three entries:
/// claw/1d3, claw/1d3, bite/1d6.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterAttackRoutine {
    pub name: String,
    pub damage: String,
}

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
    pub class: Class,
    pub level: u32,
    pub abilities: AbilityScores,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub xp: u64,
    pub inventory: Vec<Item>,
    pub spells: Vec<Spell>,
    #[serde(default)]
    pub alignment: Alignment,
    #[serde(default)]
    pub gold_gp: u32,
    #[serde(default)]
    pub saving_throws: Option<SavingThrows>,
    #[serde(default)]
    pub thac0: u32,
    #[serde(default)]
    pub movement_rate: u32,
    /// Spell slots used since last rest (index 0 = 1st level, etc.). Resets on rest.
    #[serde(default)]
    pub spell_slots_used: [u32; 6],
    /// Active effects on this character.
    #[serde(default)]
    pub effects: Vec<ActiveEffect>,
}

impl Character {
    pub fn new(name: &str, class: Class) -> Self {
        Character {
            name: name.to_string(),
            class,
            level: 1,
            abilities: AbilityScores::default(),
            hp: 1,
            max_hp: 1,
            ac: 9,
            xp: 0,
            inventory: Vec::new(),
            spells: Vec::new(),
            alignment: Alignment::default(),
            gold_gp: 0,
            saving_throws: None,
            thac0: 19,
            movement_rate: 120,
            spell_slots_used: [0; 6],
            effects: Vec::new(),
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
    pub hit_dice: HitDice,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub attacks: Vec<String>,
    pub damage: String,
    pub morale: u32,
    pub xp_value: u64,
    /// Whether this monster has been turned by a cleric.
    #[serde(default)]
    pub turned: bool,
    /// Whether this monster is helpless (sleeping, paralyzed, held, etc.).
    /// Helpless creatures can be auto-killed without an attack roll.
    #[serde(default)]
    pub helpless: bool,
    /// Whether this monster is undead (eligible for turn undead).
    #[serde(default)]
    pub undead: bool,
    /// Whether this monster is immune to non-magical weapons.
    #[serde(default)]
    pub immune_to_normal_weapons: bool,
    /// Individual attack routines (e.g., claw/1d3, claw/1d3, bite/1d6).
    /// Each entry is one attack roll the monster makes per round.
    #[serde(default)]
    pub attack_routines: Vec<MonsterAttackRoutine>,
    /// Active effects on this monster.
    #[serde(default)]
    pub effects: Vec<ActiveEffect>,
}

impl Monster {
    pub fn new(name: &str, hit_dice: HitDice) -> Self {
        Monster {
            name: name.to_string(),
            hit_dice,
            hp: 1,
            max_hp: 1,
            ac: 9,
            attacks: Vec::new(),
            damage: String::new(),
            morale: 7,
            xp_value: 0,
            turned: false,
            helpless: false,
            undead: false,
            immune_to_normal_weapons: false,
            attack_routines: Vec::new(),
            effects: Vec::new(),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Check if this monster is helpless (sleeping, paralyzed, held, etc.).
    pub fn is_helpless(&self) -> bool {
        self.helpless
    }
}

/// A party of characters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub members: Vec<Character>,
    pub gold: u64,
    pub marching_order: Vec<String>,
    /// Rations in person-days of food.
    #[serde(default)]
    pub rations: u32,
    /// Consecutive days without adequate food.
    /// Per OSE rules: after 1+ days, penalties apply.
    #[serde(default)]
    pub days_without_food: u32,
}

impl Party {
    pub fn new() -> Self {
        Party {
            members: Vec::new(),
            gold: 0,
            marching_order: Vec::new(),
            rations: 0,
            days_without_food: 0,
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
    pub value_gp: u64,
    pub equipped: bool,
}

impl Item {
    pub fn new(name: &str, weight: f32, value_gp: u64) -> Self {
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

/// OSE combat phase sequence. Temporary const array — will become a DSL derive
/// once list literals land (tdsl-jd5).
pub const PHASE_SEQUENCE: &[&str] = &[
    "Declaration",
    "Initiative",
    "Morale",
    "Movement",
    "Missile",
    "Magic",
    "Melee",
    "EndOfRound",
];

/// Display name for a phase ID. Returns the ID itself for most phases,
/// but formats "EndOfRound" as "End of Round" for human display.
pub fn phase_display_name(phase: &str) -> &str {
    match phase {
        "EndOfRound" => "End of Round",
        other => other,
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
    pub log: Vec<LogEntry>,
    /// Monotonic sequence counter for log entry ordering.
    #[serde(default)]
    pub log_seq: u64,
    /// Characters who declared spells this round (for disruption tracking).
    #[serde(default)]
    pub spell_declarations: Vec<String>,
    /// Pending spells: (character_name, spell_name) for resolution during magic phase.
    #[serde(default)]
    pub pending_spells: Vec<(String, String)>,
    /// Characters whose spells were disrupted this round.
    #[serde(default)]
    pub disrupted: Vec<String>,
    /// Current combat phase (string-based phase ID from PHASE_SEQUENCE).
    #[serde(default = "CombatState::default_phase")]
    pub phase: String,
    /// Whether the first-death morale check has been triggered.
    #[serde(default)]
    pub first_death_checked: bool,
    /// Whether the half-killed morale check has been triggered.
    #[serde(default)]
    pub half_killed_checked: bool,
    /// Initial monster count for morale trigger tracking.
    #[serde(default)]
    pub initial_monster_count: usize,
    /// Combat log length after the last initiative roll (to detect repeated rolls).
    #[serde(default)]
    pub log_len_at_initiative: usize,
    /// Attacks used by each monster this round (monster_idx → attacks_used).
    #[serde(default)]
    pub monsters_attacked_this_round: HashMap<usize, usize>,
    /// Characters who have already acted (attacked/backstabbed) this round.
    #[serde(default)]
    pub characters_acted: Vec<String>,
}

impl CombatState {
    pub fn new(monsters: Vec<Monster>, distance: u32) -> Self {
        let initial_count = monsters.len();
        CombatState {
            round: 0,
            monsters,
            party_initiative: 0,
            monster_initiative: 0,
            distance,
            log: Vec::new(),
            log_seq: 0,
            spell_declarations: Vec::new(),
            pending_spells: Vec::new(),
            disrupted: Vec::new(),
            phase: PHASE_SEQUENCE[0].to_string(),
            first_death_checked: false,
            half_killed_checked: false,
            initial_monster_count: initial_count,
            log_len_at_initiative: 0,
            monsters_attacked_this_round: HashMap::new(),
            characters_acted: Vec::new(),
        }
    }

    /// Append a message to the combat log with a monotonic sequence number.
    pub fn log_event(&mut self, message: String) {
        self.log_seq += 1;
        self.log.push(LogEntry::new(self.log_seq, message));
    }

    fn default_phase() -> String {
        PHASE_SEQUENCE[0].to_string()
    }

    /// Advance to the next combat phase by walking PHASE_SEQUENCE.
    pub fn advance_phase(&mut self) {
        let idx = PHASE_SEQUENCE.iter().position(|&p| p == self.phase);
        let next_idx = match idx {
            Some(i) if i + 1 < PHASE_SEQUENCE.len() => i + 1,
            _ => {
                // Wrap: last phase (EndOfRound) → first phase (Declaration).
                // Clear stale spell state from the completed round so the
                // next declaration phase starts fresh.
                self.spell_declarations.clear();
                self.pending_spells.clear();
                0
            }
        };
        self.phase = PHASE_SEQUENCE[next_idx].to_string();
    }

    pub fn living_monsters(&self) -> Vec<(usize, &Monster)> {
        self.monsters.iter().enumerate()
            .filter(|(_, m)| m.is_alive())
            .collect()
    }

    pub fn living_monster_count(&self) -> usize {
        self.monsters.iter().filter(|m| m.is_alive()).count()
    }

    /// Check whether a morale trigger condition has been newly met.
    /// Returns true if morale should be checked (first death, or half killed).
    ///
    /// When the DSL backend is enabled, delegates to the `should_check_morale`
    /// derive. Falls back to native logic on DSL failure.
    pub fn should_check_morale(&mut self) -> bool {
        let dead = self.initial_monster_count - self.living_monster_count();

        #[cfg(feature = "dsl-backend")]
        if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
            if let Some(result) = dsl_should_check_morale(
                dead,
                self.initial_monster_count,
                self.first_death_checked,
                self.half_killed_checked,
            ) {
                if result {
                    // Update flags the same way the native code would
                    if dead >= 1 && !self.first_death_checked {
                        self.first_death_checked = true;
                    } else if self.initial_monster_count > 0
                        && dead * 2 >= self.initial_monster_count
                        && !self.half_killed_checked
                    {
                        self.half_killed_checked = true;
                    }
                }
                return result;
            }
            // DSL failed — fall through to native
        }

        if dead >= 1 && !self.first_death_checked {
            self.first_death_checked = true;
            return true;
        }
        if self.initial_monster_count > 0
            && dead * 2 >= self.initial_monster_count
            && !self.half_killed_checked
        {
            self.half_killed_checked = true;
            return true;
        }
        false
    }
}

/// DSL evaluation of the should_check_morale derive.
#[cfg(feature = "dsl-backend")]
fn dsl_should_check_morale(
    deaths: usize,
    initial: usize,
    first_death_checked: bool,
    half_killed_checked: bool,
) -> Option<bool> {
    use ttrpg_interp::effect::{Effect, EffectHandler, Response};
    use ttrpg_interp::reference_state::GameState;
    use ttrpg_interp::value::Value;

    struct NullHandler;
    impl EffectHandler for NullHandler {
        fn handle(&mut self, _: Effect) -> Response {
            Response::Acknowledged
        }
    }

    let runtime = crate::backend::dsl()?;
    let state = GameState::new();
    let mut handler = NullHandler;

    let result = runtime.evaluate_derive(
        &state,
        &mut handler,
        "should_check_morale",
        vec![
            Value::Int(deaths as i64),
            Value::Int(initial as i64),
            Value::Int(if first_death_checked { 1 } else { 0 }),
            Value::Int(if half_killed_checked { 1 } else { 0 }),
        ],
    ).ok()?;

    match result {
        Value::Int(n) => Some(n != 0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_creation() {
        let c = Character::new("Theron", Class::Fighter);
        assert_eq!(c.name, "Theron");
        assert_eq!(c.class, Class::Fighter);
        assert_eq!(c.level, 1);
        assert!(c.is_alive());
    }

    #[test]
    fn party_operations() {
        let mut party = Party::new();
        party.add_member(Character::new("Arden", Class::Cleric));
        party.add_member(Character::new("Brin", Class::Thief));
        assert_eq!(party.members.len(), 2);
        assert!(party.find_member("arden").is_some());
        assert!(party.find_member("nobody").is_none());
    }

    #[test]
    fn serialization_roundtrip() {
        let c = Character::new("Test", Class::MagicUser);
        let json = serde_json::to_string(&c).unwrap();
        let c2: Character = serde_json::from_str(&json).unwrap();
        assert_eq!(c.name, c2.name);
        assert_eq!(c.class, c2.class);
    }

    #[test]
    fn backward_compat_class_string() {
        // Old saves stored class as a string like "Magic-User"
        let old_json = r#"{
            "name": "OldChar",
            "class": "Magic-User",
            "level": 1,
            "abilities": {"strength":10,"intelligence":10,"wisdom":10,"dexterity":10,"constitution":10,"charisma":10},
            "hp": 5, "max_hp": 5, "ac": 9, "xp": 0,
            "inventory": [], "spells": []
        }"#;
        let c: Character = serde_json::from_str(old_json).unwrap();
        assert_eq!(c.class, Class::MagicUser);
    }
}
