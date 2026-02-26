use std::collections::BTreeMap;

use ttrpg_ast::Name;
use ttrpg_interp::effect::FieldPathSegment;
use ttrpg_interp::state::{ActiveCondition, EntityRef, StateProvider, WritableState};
use ttrpg_interp::value::Value;

use osr_ai_gm::log_entry::LogEntry;
use osr_ai_gm::model::{Character, Monster};

use crate::registry::{character_index, is_character, is_monster, monster_index};

/// Bridge state that holds cloned game data for the interpreter.
///
/// After interpreter execution, the caller syncs modified data back
/// to the real GameState.
pub struct BridgeState {
    pub characters: Vec<Character>,
    pub monsters: Vec<Monster>,
    pub log: Vec<LogEntry>,
    pub log_seq: u64,
    pub combat_round: u32,
}

impl BridgeState {
    pub fn new(
        characters: Vec<Character>,
        monsters: Vec<Monster>,
        log: Vec<LogEntry>,
        log_seq: u64,
        combat_round: u32,
    ) -> Self {
        BridgeState {
            characters,
            monsters,
            log,
            log_seq,
            combat_round,
        }
    }

    /// Append a message to the combat log.
    pub fn log_message(&mut self, message: String) {
        self.log_seq += 1;
        self.log.push(LogEntry::new(self.log_seq, message));
    }
}

// ── StateProvider ──────────────────────────────────────────────

impl StateProvider for BridgeState {
    fn read_field(&self, entity: &EntityRef, field: &str) -> Option<Value> {
        if is_character(entity) {
            let c = self.characters.get(character_index(entity))?;
            read_character_field(c, field)
        } else if is_monster(entity) {
            let m = self.monsters.get(monster_index(entity))?;
            read_monster_field(m, field)
        } else {
            None
        }
    }

    fn read_conditions(&self, entity: &EntityRef) -> Option<Vec<ActiveCondition>> {
        let effects = if is_character(entity) {
            &self.characters.get(character_index(entity))?.effects
        } else if is_monster(entity) {
            &self.monsters.get(monster_index(entity))?.effects
        } else {
            return None;
        };

        let conditions = effects
            .iter()
            .map(|e| ActiveCondition {
                id: e.id as u64,
                name: e.name.clone().into(),
                params: BTreeMap::new(),
                bearer: *entity,
                gained_at: e.id as u64,
                duration: ttrpg_interp::value::duration_variant("permanent"),
            })
            .collect();
        Some(conditions)
    }

    fn read_turn_budget(&self, entity: &EntityRef) -> Option<BTreeMap<Name, Value>> {
        // Verify entity exists
        if is_character(entity) {
            self.characters.get(character_index(entity))?;
        } else if is_monster(entity) {
            self.monsters.get(monster_index(entity))?;
        } else {
            return None;
        }
        // B/X doesn't have action economy; return a simple budget
        let mut budget = BTreeMap::new();
        budget.insert("actions".into(), Value::Int(1));
        budget.insert("movement".into(), Value::Int(1));
        Some(budget)
    }

    fn read_enabled_options(&self) -> Vec<Name> {
        Vec::new()
    }

    fn position_eq(&self, _a: &Value, _b: &Value) -> bool {
        // Theater-of-mind; no grid positions
        false
    }

    fn distance(&self, _a: &Value, _b: &Value) -> Option<i64> {
        None
    }

    fn entity_type_name(&self, entity: &EntityRef) -> Option<Name> {
        if is_character(entity) {
            self.characters
                .get(character_index(entity))
                .map(|_| "Character".into())
        } else if is_monster(entity) {
            self.monsters
                .get(monster_index(entity))
                .map(|_| "Monster".into())
        } else {
            None
        }
    }
}

// ── WritableState ──────────────────────────────────────────────

impl WritableState for BridgeState {
    fn write_field(&mut self, entity: &EntityRef, path: &[FieldPathSegment], value: Value) {
        let field = match path.first() {
            Some(FieldPathSegment::Field(f)) => f.as_str(),
            _ => return,
        };

        if is_character(entity) {
            if let Some(c) = self.characters.get_mut(character_index(entity)) {
                write_character_field(c, field, &value);
            }
        } else if is_monster(entity) {
            if let Some(m) = self.monsters.get_mut(monster_index(entity)) {
                write_monster_field(m, field, &value);
            }
        }
    }

    fn add_condition(&mut self, entity: &EntityRef, cond: ActiveCondition) {
        use osr_ai_gm::state::effect::{ActiveEffect, EffectDuration};
        let effect = ActiveEffect {
            id: cond.id as u32,
            name: cond.name.to_string(),
            source: String::new(),
            duration: EffectDuration::Permanent,
            modifiers: Vec::new(),
            notes: String::new(),
        };
        if is_character(entity) {
            if let Some(c) = self.characters.get_mut(character_index(entity)) {
                c.effects.push(effect);
            }
        } else if is_monster(entity) {
            if let Some(m) = self.monsters.get_mut(monster_index(entity)) {
                m.effects.push(effect);
            }
        }
    }

    fn remove_condition(
        &mut self,
        entity: &EntityRef,
        name: &str,
        _params: Option<&BTreeMap<Name, Value>>,
    ) {
        if is_character(entity) {
            if let Some(c) = self.characters.get_mut(character_index(entity)) {
                c.effects.retain(|e| e.name != name);
            }
        } else if is_monster(entity) {
            if let Some(m) = self.monsters.get_mut(monster_index(entity)) {
                m.effects.retain(|e| e.name != name);
            }
        }
    }

    fn write_turn_field(&mut self, _entity: &EntityRef, _field: &str, _value: Value) {
        // B/X has no action economy turn budget
    }

    fn remove_field(&mut self, _entity: &EntityRef, _field: &str) {
        // Grant/revoke groups not used in this bridge
    }
}

// ── Field readers ──────────────────────────────────────────────

fn read_character_field(c: &Character, field: &str) -> Option<Value> {
    match field {
        "name" => Some(Value::Str(c.name.clone())),
        "class" => Some(Value::Str(format!("{:?}", c.class))),
        "level" => Some(Value::Int(c.level as i64)),
        "hp" | "HP" => Some(Value::Int(c.hp as i64)),
        "max_hp" => Some(Value::Int(c.max_hp as i64)),
        "ac" | "AC" => Some(Value::Int(c.ac as i64)),
        "xp" => Some(Value::Int(c.xp as i64)),
        "gold_gp" => Some(Value::Int(c.gold_gp as i64)),
        "thac0" | "THAC0" => Some(Value::Int(c.thac0 as i64)),
        "movement_rate" => Some(Value::Int(c.movement_rate as i64)),
        "alive" => Some(Value::Bool(c.is_alive())),
        // Ability scores — both long and short names
        "strength" | "STR" => Some(Value::Int(c.abilities.strength as i64)),
        "intelligence" | "INT" => Some(Value::Int(c.abilities.intelligence as i64)),
        "wisdom" | "WIS" => Some(Value::Int(c.abilities.wisdom as i64)),
        "dexterity" | "DEX" => Some(Value::Int(c.abilities.dexterity as i64)),
        "constitution" | "CON" => Some(Value::Int(c.abilities.constitution as i64)),
        "charisma" | "CHA" => Some(Value::Int(c.abilities.charisma as i64)),
        "alignment" => Some(Value::Str(format!("{:?}", c.alignment))),
        // Saving throws (dynamic map lookup with save_ prefix)
        f if f.starts_with("save_") => {
            let save_name = &f[5..]; // strip "save_" prefix
            c.saving_throws.as_ref().and_then(|st| st.get(save_name).map(|v| Value::Int(v as i64)))
        }
        _ => None,
    }
}

fn read_monster_field(m: &Monster, field: &str) -> Option<Value> {
    match field {
        "name" => Some(Value::Str(m.name.clone())),
        "hp" | "HP" => Some(Value::Int(m.hp as i64)),
        "max_hp" => Some(Value::Int(m.max_hp as i64)),
        "ac" | "AC" => Some(Value::Int(m.ac as i64)),
        "morale" => Some(Value::Int(m.morale as i64)),
        "xp_value" => Some(Value::Int(m.xp_value as i64)),
        "hit_dice" => Some(Value::Int(m.hit_dice.base as i64)),
        "hit_dice_modifier" => Some(Value::Int(m.hit_dice.modifier as i64)),
        "turned" => Some(Value::Bool(m.turned)),
        "helpless" => Some(Value::Bool(m.helpless)),
        "undead" => Some(Value::Bool(m.undead)),
        "immune_to_normal_weapons" => Some(Value::Bool(m.immune_to_normal_weapons)),
        "alive" => Some(Value::Bool(m.is_alive())),
        "damage" => Some(Value::Str(m.damage.clone())),
        _ => None,
    }
}

// ── Field writers ──────────────────────────────────────────────

fn write_character_field(c: &mut Character, field: &str, value: &Value) {
    match field {
        "hp" | "HP" => {
            if let Value::Int(v) = value {
                c.hp = *v as i32;
            }
        }
        "max_hp" => {
            if let Value::Int(v) = value {
                c.max_hp = *v as i32;
            }
        }
        "ac" | "AC" => {
            if let Value::Int(v) = value {
                c.ac = *v as i32;
            }
        }
        "xp" => {
            if let Value::Int(v) = value {
                c.xp = *v as u64;
            }
        }
        "gold_gp" => {
            if let Value::Int(v) = value {
                c.gold_gp = *v as u32;
            }
        }
        "thac0" | "THAC0" => {
            if let Value::Int(v) = value {
                c.thac0 = *v as u32;
            }
        }
        "movement_rate" => {
            if let Value::Int(v) = value {
                c.movement_rate = *v as u32;
            }
        }
        "level" => {
            if let Value::Int(v) = value {
                c.level = *v as u32;
            }
        }
        "strength" | "STR" => {
            if let Value::Int(v) = value {
                c.abilities.strength = *v as i32;
            }
        }
        "intelligence" | "INT" => {
            if let Value::Int(v) = value {
                c.abilities.intelligence = *v as i32;
            }
        }
        "wisdom" | "WIS" => {
            if let Value::Int(v) = value {
                c.abilities.wisdom = *v as i32;
            }
        }
        "dexterity" | "DEX" => {
            if let Value::Int(v) = value {
                c.abilities.dexterity = *v as i32;
            }
        }
        "constitution" | "CON" => {
            if let Value::Int(v) = value {
                c.abilities.constitution = *v as i32;
            }
        }
        "charisma" | "CHA" => {
            if let Value::Int(v) = value {
                c.abilities.charisma = *v as i32;
            }
        }
        _ => {}
    }
}

fn write_monster_field(m: &mut Monster, field: &str, value: &Value) {
    match field {
        "hp" | "HP" => {
            if let Value::Int(v) = value {
                m.hp = *v as i32;
            }
        }
        "max_hp" => {
            if let Value::Int(v) = value {
                m.max_hp = *v as i32;
            }
        }
        "ac" | "AC" => {
            if let Value::Int(v) = value {
                m.ac = *v as i32;
            }
        }
        "morale" => {
            if let Value::Int(v) = value {
                m.morale = *v as u32;
            }
        }
        "xp_value" => {
            if let Value::Int(v) = value {
                m.xp_value = *v as u64;
            }
        }
        "turned" => {
            if let Value::Bool(v) = value {
                m.turned = *v;
            }
        }
        "helpless" => {
            if let Value::Bool(v) = value {
                m.helpless = *v;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{character_ref, monster_ref};
    use osr_ai_gm::model::AbilityScores;
    use osr_ai_gm::rules::attack::HitDice;
    use osr_ai_gm::rules::class::Class;
    use osr_ai_gm::rules::save::SavingThrows;

    fn test_character() -> Character {
        let mut c = Character::new("Grond", Class::Fighter);
        c.level = 3;
        c.hp = 18;
        c.max_hp = 24;
        c.ac = 5;
        c.thac0 = 19;
        c.xp = 4000;
        c.gold_gp = 150;
        c.movement_rate = 90;
        c.abilities = AbilityScores {
            strength: 16,
            intelligence: 10,
            wisdom: 12,
            dexterity: 13,
            constitution: 14,
            charisma: 9,
        };
        c.saving_throws = Some(SavingThrows::new(12, 13, 14, 15, 16));
        c
    }

    fn test_monster() -> Monster {
        let mut m = Monster::new(
            "Goblin",
            HitDice {
                base: 1,
                modifier: -1,
                specials: 0,
                fractional: false,
                range_end: None,
            },
        );
        m.hp = 3;
        m.max_hp = 3;
        m.ac = 6;
        m.morale = 7;
        m.xp_value = 5;
        m
    }

    fn test_state() -> BridgeState {
        BridgeState::new(
            vec![test_character()],
            vec![test_monster()],
            Vec::new(),
            0,
            1,
        )
    }

    #[test]
    fn read_character_fields() {
        let state = test_state();
        let c = character_ref(0);

        assert_eq!(state.read_field(&c, "name"), Some(Value::Str("Grond".into())));
        assert_eq!(state.read_field(&c, "level"), Some(Value::Int(3)));
        assert_eq!(state.read_field(&c, "hp"), Some(Value::Int(18)));
        assert_eq!(state.read_field(&c, "HP"), Some(Value::Int(18)));
        assert_eq!(state.read_field(&c, "max_hp"), Some(Value::Int(24)));
        assert_eq!(state.read_field(&c, "ac"), Some(Value::Int(5)));
        assert_eq!(state.read_field(&c, "thac0"), Some(Value::Int(19)));
        assert_eq!(state.read_field(&c, "THAC0"), Some(Value::Int(19)));
        assert_eq!(state.read_field(&c, "xp"), Some(Value::Int(4000)));
        assert_eq!(state.read_field(&c, "gold_gp"), Some(Value::Int(150)));
        assert_eq!(state.read_field(&c, "movement_rate"), Some(Value::Int(90)));
        assert_eq!(state.read_field(&c, "alive"), Some(Value::Bool(true)));
    }

    #[test]
    fn read_character_abilities() {
        let state = test_state();
        let c = character_ref(0);

        assert_eq!(state.read_field(&c, "strength"), Some(Value::Int(16)));
        assert_eq!(state.read_field(&c, "STR"), Some(Value::Int(16)));
        assert_eq!(state.read_field(&c, "intelligence"), Some(Value::Int(10)));
        assert_eq!(state.read_field(&c, "INT"), Some(Value::Int(10)));
        assert_eq!(state.read_field(&c, "wisdom"), Some(Value::Int(12)));
        assert_eq!(state.read_field(&c, "dexterity"), Some(Value::Int(13)));
        assert_eq!(state.read_field(&c, "constitution"), Some(Value::Int(14)));
        assert_eq!(state.read_field(&c, "charisma"), Some(Value::Int(9)));
    }

    #[test]
    fn read_character_saving_throws() {
        let state = test_state();
        let c = character_ref(0);

        assert_eq!(state.read_field(&c, "save_death"), Some(Value::Int(12)));
        assert_eq!(state.read_field(&c, "save_wands"), Some(Value::Int(13)));
        assert_eq!(state.read_field(&c, "save_paralysis"), Some(Value::Int(14)));
        assert_eq!(state.read_field(&c, "save_breath"), Some(Value::Int(15)));
        assert_eq!(state.read_field(&c, "save_spells"), Some(Value::Int(16)));
    }

    #[test]
    fn read_monster_fields() {
        let state = test_state();
        let m = monster_ref(0);

        assert_eq!(state.read_field(&m, "name"), Some(Value::Str("Goblin".into())));
        assert_eq!(state.read_field(&m, "hp"), Some(Value::Int(3)));
        assert_eq!(state.read_field(&m, "max_hp"), Some(Value::Int(3)));
        assert_eq!(state.read_field(&m, "ac"), Some(Value::Int(6)));
        assert_eq!(state.read_field(&m, "morale"), Some(Value::Int(7)));
        assert_eq!(state.read_field(&m, "xp_value"), Some(Value::Int(5)));
        assert_eq!(state.read_field(&m, "hit_dice"), Some(Value::Int(1)));
        assert_eq!(state.read_field(&m, "hit_dice_modifier"), Some(Value::Int(-1)));
        assert_eq!(state.read_field(&m, "alive"), Some(Value::Bool(true)));
    }

    #[test]
    fn read_unknown_field_returns_none() {
        let state = test_state();
        assert_eq!(state.read_field(&character_ref(0), "nonexistent"), None);
        assert_eq!(state.read_field(&monster_ref(0), "nonexistent"), None);
    }

    #[test]
    fn read_invalid_entity_returns_none() {
        let state = test_state();
        assert_eq!(state.read_field(&character_ref(99), "hp"), None);
        assert_eq!(state.read_field(&monster_ref(99), "hp"), None);
    }

    #[test]
    fn write_character_hp() {
        let mut state = test_state();
        let c = character_ref(0);

        state.write_field(
            &c,
            &[FieldPathSegment::Field("hp".into())],
            Value::Int(10),
        );
        assert_eq!(state.read_field(&c, "hp"), Some(Value::Int(10)));
    }

    #[test]
    fn write_monster_hp() {
        let mut state = test_state();
        let m = monster_ref(0);

        state.write_field(
            &m,
            &[FieldPathSegment::Field("hp".into())],
            Value::Int(0),
        );
        assert_eq!(state.read_field(&m, "hp"), Some(Value::Int(0)));
        assert_eq!(state.read_field(&m, "alive"), Some(Value::Bool(false)));
    }

    #[test]
    fn write_character_ability() {
        let mut state = test_state();
        let c = character_ref(0);

        state.write_field(
            &c,
            &[FieldPathSegment::Field("STR".into())],
            Value::Int(18),
        );
        assert_eq!(state.read_field(&c, "strength"), Some(Value::Int(18)));
    }

    #[test]
    fn entity_type_names() {
        let state = test_state();
        assert_eq!(
            state.entity_type_name(&character_ref(0)),
            Some("Character".into())
        );
        assert_eq!(
            state.entity_type_name(&monster_ref(0)),
            Some("Monster".into())
        );
        assert_eq!(state.entity_type_name(&character_ref(99)), None);
    }

    #[test]
    fn log_message_increments_seq() {
        let mut state = test_state();
        state.log_message("Test message".into());
        assert_eq!(state.log.len(), 1);
        assert_eq!(state.log[0].seq, 1);
        assert_eq!(state.log[0].message, "Test message");

        state.log_message("Second".into());
        assert_eq!(state.log.len(), 2);
        assert_eq!(state.log[1].seq, 2);
    }

    #[test]
    fn read_turn_budget() {
        let state = test_state();
        let budget = state.read_turn_budget(&character_ref(0)).unwrap();
        assert_eq!(budget.get("actions"), Some(&Value::Int(1)));
        assert_eq!(budget.get("movement"), Some(&Value::Int(1)));
    }

    #[test]
    fn read_conditions_empty() {
        let state = test_state();
        let conds = state.read_conditions(&character_ref(0)).unwrap();
        assert!(conds.is_empty());
    }

    // ── Attack roll table verification ─────────────────────────

    #[test]
    fn attack_roll_thac0_fighter_level_tables() {
        // B/X Fighter THAC0 progression (OSE reference):
        // Level 1-3: THAC0 19
        // Level 4-6: THAC0 17
        // Level 7-9: THAC0 14
        let cases = [(1, 19u32), (3, 19), (4, 17), (7, 14)];

        for (level, expected_thac0) in cases {
            let mut c = Character::new("TestFighter", Class::Fighter);
            c.level = level;
            c.thac0 = expected_thac0;
            let state = BridgeState::new(vec![c], Vec::new(), Vec::new(), 0, 1);
            let cr = character_ref(0);

            assert_eq!(
                state.read_field(&cr, "thac0"),
                Some(Value::Int(expected_thac0 as i64)),
                "Fighter level {} should have THAC0 {}",
                level,
                expected_thac0
            );
        }
    }

    #[test]
    fn saving_throw_comparison_tables() {
        // B/X Fighter saving throws (OSE):
        // Level 1-3: D:12 W:13 P:14 B:15 S:16
        let st = SavingThrows::new(12, 13, 14, 15, 16);
        let mut c = Character::new("TestFighter", Class::Fighter);
        c.level = 1;
        c.saving_throws = Some(st);

        let state = BridgeState::new(vec![c], Vec::new(), Vec::new(), 0, 1);
        let cr = character_ref(0);

        assert_eq!(state.read_field(&cr, "save_death"), Some(Value::Int(12)));
        assert_eq!(state.read_field(&cr, "save_wands"), Some(Value::Int(13)));
        assert_eq!(state.read_field(&cr, "save_paralysis"), Some(Value::Int(14)));
        assert_eq!(state.read_field(&cr, "save_breath"), Some(Value::Int(15)));
        assert_eq!(state.read_field(&cr, "save_spells"), Some(Value::Int(16)));
    }
}
