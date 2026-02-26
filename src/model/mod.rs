use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use crate::log_entry::LogEntry;
use crate::rules::alignment::AlignmentId;
use crate::rules::attack::HitDice;
use crate::rules::class::ClassId;
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
    pub class: ClassId,
    pub level: u32,
    pub abilities: AbilityScores,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub xp: u64,
    pub inventory: Vec<Item>,
    pub spells: Vec<Spell>,
    #[serde(default)]
    pub alignment: AlignmentId,
    #[serde(default)]
    pub gold_gp: u32,
    #[serde(default)]
    pub saving_throws: Option<SavingThrows>,
    #[serde(default)]
    pub thac0: u32,
    #[serde(default)]
    pub movement_rate: u32,
    /// Spell slots used since last rest (index 0 = 1st level, etc.). Resets on long rest.
    #[serde(default)]
    pub spell_slots_used: [u32; 6],
    /// Prepared (memorized) spells by level. Index 0 = 1st level spells.
    /// Each inner Vec contains spell names prepared at that level.
    /// For Vancian casting: length of each vec <= max_slots at that level.
    /// Empty if no spells prepared yet (or for non-Vancian systems).
    #[serde(default)]
    pub prepared_spells: Vec<Vec<String>>,
    /// Spell points used since last rest. For spell-point casting systems.
    #[serde(default)]
    pub spell_points_used: u32,
    /// Active effects on this character.
    #[serde(default)]
    pub effects: Vec<ActiveEffect>,
}

impl Character {
    pub fn new(name: &str, class: impl Into<ClassId>) -> Self {
        Character {
            name: name.to_string(),
            class: class.into(),
            level: 1,
            abilities: AbilityScores::default(),
            hp: 1,
            max_hp: 1,
            ac: 9,
            xp: 0,
            inventory: Vec::new(),
            spells: Vec::new(),
            alignment: AlignmentId::default(),
            gold_gp: 0,
            saving_throws: None,
            thac0: 19,
            movement_rate: 120,
            spell_slots_used: [0; 6],
            prepared_spells: Vec::new(),
            spell_points_used: 0,
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

/// OSE combat phase sequence — native fallback used when DSL backend is
/// unavailable. Prefer [`get_phase_sequence`] which tries the DSL first.
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

/// Get the combat phase sequence, trying the DSL `phase_sequence` derive first
/// and falling back to the native [`PHASE_SEQUENCE`] const.
pub fn get_phase_sequence() -> Vec<String> {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some(seq) = dsl_phase_sequence() {
            return seq;
        }
    }
    PHASE_SEQUENCE.iter().map(|s| s.to_string()).collect()
}

/// Evaluate the DSL `phase_sequence` derive.
#[cfg(feature = "dsl-backend")]
fn dsl_phase_sequence() -> Option<Vec<String>> {
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

    let result = runtime
        .evaluate_derive(&state, &mut handler, "phase_sequence", vec![])
        .ok()?;

    match result {
        Value::List(items) => {
            let mut phases = Vec::new();
            for item in items {
                match item {
                    Value::Str(s) => phases.push(s),
                    _ => return None,
                }
            }
            if phases.is_empty() {
                return None;
            }
            Some(phases)
        }
        _ => None,
    }
}

/// Get the initiative model, trying the DSL `initiative_model` derive first
/// and falling back to "group" (the OSE default).
pub fn get_initiative_model() -> String {
    #[cfg(feature = "dsl-backend")]
    if crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        if let Some(model) = dsl_initiative_model() {
            return model;
        }
    }
    "group".to_string()
}

/// Get the action budget for a combat phase from the DSL.
/// Returns a map of action-type → count (e.g. {"attack": 1}).
/// Falls back to None if the DSL is unavailable.
#[cfg(feature = "dsl-backend")]
pub fn dsl_action_budget(phase: &str) -> Option<HashMap<String, i32>> {
    use ttrpg_interp::effect::{Effect, EffectHandler, Response};
    use ttrpg_interp::reference_state::GameState;
    use ttrpg_interp::value::Value;

    struct NullHandler;
    impl EffectHandler for NullHandler {
        fn handle(&mut self, _: Effect) -> Response {
            Response::Acknowledged
        }
    }

    if !crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        return None;
    }

    let runtime = crate::backend::dsl()?;
    let state = GameState::new();
    let mut handler = NullHandler;

    let result = runtime
        .evaluate_derive(
            &state,
            &mut handler,
            "action_budget",
            vec![Value::Str(phase.to_string())],
        )
        .ok()?;

    match result {
        Value::Map(entries) => {
            let mut budget = HashMap::new();
            for (k, v) in entries {
                if let (Value::Str(key), Value::Int(val)) = (k, v) {
                    budget.insert(key, val as i32);
                }
            }
            Some(budget)
        }
        _ => None,
    }
}

/// Get the initiative model from the DSL (`initiative_model` derive).
/// Returns "group" or "individual". Falls back to None if DSL unavailable.
#[cfg(feature = "dsl-backend")]
pub fn dsl_initiative_model() -> Option<String> {
    use ttrpg_interp::effect::{Effect, EffectHandler, Response};
    use ttrpg_interp::reference_state::GameState;
    use ttrpg_interp::value::Value;

    struct NullHandler;
    impl EffectHandler for NullHandler {
        fn handle(&mut self, _: Effect) -> Response {
            Response::Acknowledged
        }
    }

    if !crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        return None;
    }

    let runtime = crate::backend::dsl()?;
    let state = GameState::new();
    let mut handler = NullHandler;

    let result = runtime
        .evaluate_derive(
            &state,
            &mut handler,
            "initiative_model",
            vec![],
        )
        .ok()?;

    match result {
        Value::Str(s) => Some(s),
        _ => None,
    }
}

/// Check whether a phase should be skipped via the DSL `skip_phase` derive.
/// Returns Some(true) to skip, Some(false) to run, None if DSL unavailable.
#[cfg(feature = "dsl-backend")]
fn dsl_skip_phase(phase: &str, round: u32, has_spells_declared: bool, has_living_monsters: bool) -> Option<bool> {
    use ttrpg_interp::effect::{Effect, EffectHandler, Response};
    use ttrpg_interp::reference_state::GameState;
    use ttrpg_interp::value::Value;

    struct NullHandler;
    impl EffectHandler for NullHandler {
        fn handle(&mut self, _: Effect) -> Response {
            Response::Acknowledged
        }
    }

    if !crate::backend::is_dsl(crate::backend::MechanicGroup::Combat) {
        return None;
    }

    let runtime = crate::backend::dsl()?;
    let state = GameState::new();
    let mut handler = NullHandler;

    let result = runtime
        .evaluate_derive(
            &state,
            &mut handler,
            "skip_phase",
            vec![
                Value::Str(phase.to_string()),
                Value::Int(round as i64),
                Value::Int(if has_spells_declared { 1 } else { 0 }),
                Value::Int(if has_living_monsters { 1 } else { 0 }),
            ],
        )
        .ok()?;

    match result {
        Value::Int(n) => Some(n != 0),
        _ => None,
    }
}

/// Display name for a phase ID. Returns the ID itself for most phases,
/// but formats "EndOfRound" as "End of Round" for human display.
pub fn phase_display_name(phase: &str) -> &str {
    match phase {
        "EndOfRound" => "End of Round",
        other => other,
    }
}

/// A single entry in the individual initiative order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiativeEntry {
    /// Display name of the combatant.
    pub name: String,
    /// "character" or "monster".
    pub side: String,
    /// Index into the party members or monsters list.
    pub index: usize,
    /// Initiative roll result (e.g. 1d6 + DEX mod).
    pub roll: i32,
}

/// Per-entity action usage tracking for DSL-driven action budgets.
/// Maps entity name → action_type → uses remaining this phase.
pub type ActionBudgetTracker = HashMap<String, HashMap<String, i32>>;

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
    /// Individual initiative order (populated when initiative_model is "individual").
    /// Sorted from highest to lowest roll. Empty when using group initiative.
    #[serde(default)]
    pub initiative_order: Vec<InitiativeEntry>,
    /// Per-entity action budget tracking for the current phase.
    /// Maps entity name → action_type → uses consumed.
    #[serde(default)]
    pub action_budget_used: ActionBudgetTracker,
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
            phase: get_phase_sequence().first().cloned().unwrap_or_else(|| PHASE_SEQUENCE[0].to_string()),
            first_death_checked: false,
            half_killed_checked: false,
            initial_monster_count: initial_count,
            log_len_at_initiative: 0,
            monsters_attacked_this_round: HashMap::new(),
            characters_acted: Vec::new(),
            initiative_order: Vec::new(),
            action_budget_used: HashMap::new(),
        }
    }

    /// Append a message to the combat log with a monotonic sequence number.
    pub fn log_event(&mut self, message: String) {
        self.log_seq += 1;
        self.log.push(LogEntry::new(self.log_seq, message));
    }

    fn default_phase() -> String {
        get_phase_sequence().first().cloned().unwrap_or_else(|| PHASE_SEQUENCE[0].to_string())
    }

    /// Advance to the next combat phase using the DSL-sourced (or fallback)
    /// phase sequence. Phases flagged by `skip_phase` are automatically skipped.
    pub fn advance_phase(&mut self) {
        let sequence = get_phase_sequence();
        let idx = sequence.iter().position(|p| p == &self.phase);
        let len = sequence.len();

        // Walk forward, skipping phases the DSL says to skip.
        // Guard: at most `len` iterations to avoid infinite loops if all phases skip.
        let mut steps = 0;
        let mut next_idx = match idx {
            Some(i) if i + 1 < len => i + 1,
            _ => {
                // Wrap: last phase → first phase.
                // Clear stale spell state from the completed round.
                self.spell_declarations.clear();
                self.pending_spells.clear();
                0
            }
        };

        loop {
            // Check DSL skip_phase condition
            let should_skip = {
                #[cfg(feature = "dsl-backend")]
                {
                    dsl_skip_phase(
                        &sequence[next_idx],
                        self.round,
                        !self.spell_declarations.is_empty(),
                        self.living_monster_count() > 0,
                    )
                    .unwrap_or(false)
                }
                #[cfg(not(feature = "dsl-backend"))]
                {
                    let _ = &sequence[next_idx];
                    false
                }
            };

            steps += 1;
            if !should_skip || steps >= len {
                break;
            }

            // Advance past skipped phase
            if next_idx + 1 < len {
                next_idx += 1;
            } else {
                self.spell_declarations.clear();
                self.pending_spells.clear();
                next_idx = 0;
            }
        }

        self.phase = sequence[next_idx].clone();
        // Reset per-entity action budgets for the new phase
        self.action_budget_used.clear();
    }

    /// Check whether an entity has budget remaining for the given action type
    /// in the current phase. Uses DSL `action_budget` if available, otherwise
    /// falls back to allowing the action (preserving existing behavior).
    pub fn has_action_budget(&self, _entity_name: &str, _action_type: &str) -> bool {
        #[cfg(feature = "dsl-backend")]
        {
            if let Some(budget) = dsl_action_budget(&self.phase) {
                let max = budget.get(_action_type).copied().unwrap_or(0);
                if max <= 0 {
                    return false;
                }
                let used = self.action_budget_used
                    .get(_entity_name)
                    .and_then(|m| m.get(_action_type))
                    .copied()
                    .unwrap_or(0);
                return used < max;
            }
        }
        // No DSL budget available — allow the action (legacy behavior)
        true
    }

    /// Record that an entity consumed one use of the given action type.
    pub fn consume_action(&mut self, entity_name: &str, action_type: &str) {
        *self.action_budget_used
            .entry(entity_name.to_string())
            .or_default()
            .entry(action_type.to_string())
            .or_insert(0) += 1;
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
    use crate::rules::class::Class;

    #[test]
    fn character_creation() {
        let c = Character::new("Theron", Class::Fighter);
        assert_eq!(c.name, "Theron");
        assert_eq!(c.class, ClassId::from_enum(Class::Fighter));
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
        assert_eq!(c.class, ClassId::from_enum(Class::MagicUser));
    }
}
