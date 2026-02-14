use serde::{Deserialize, Serialize};
use std::fmt;

/// Target of an active effect.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "name")]
pub enum EffectTarget {
    Character(String),
    Monster(usize),
    Global,
}

/// How an effect's duration is measured.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum EffectDuration {
    Rounds(u32),
    Turns(u32),
    Permanent,
    Concentration,
}

impl EffectDuration {
    pub fn is_expired(&self) -> bool {
        matches!(self, EffectDuration::Rounds(0) | EffectDuration::Turns(0))
    }
}

impl fmt::Display for EffectDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectDuration::Rounds(n) => write!(f, "{} round{}", n, if *n == 1 { "" } else { "s" }),
            EffectDuration::Turns(n) => write!(f, "{} turn{}", n, if *n == 1 { "" } else { "s" }),
            EffectDuration::Permanent => write!(f, "permanent"),
            EffectDuration::Concentration => write!(f, "concentration"),
        }
    }
}

/// Which stat a modifier affects (informational — the AI GM applies manually).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModifierStat {
    AttackRoll,
    DamageRoll,
    ArmorClass,
    SavingThrows,
    MovementRate,
    Morale,
    Custom(String),
}

impl fmt::Display for ModifierStat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModifierStat::AttackRoll => write!(f, "atk"),
            ModifierStat::DamageRoll => write!(f, "dmg"),
            ModifierStat::ArmorClass => write!(f, "AC"),
            ModifierStat::SavingThrows => write!(f, "saves"),
            ModifierStat::MovementRate => write!(f, "mv"),
            ModifierStat::Morale => write!(f, "morale"),
            ModifierStat::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// A single stat modifier within an effect.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Modifier {
    pub stat: ModifierStat,
    pub value: i32,
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.value >= 0 {
            write!(f, "+{} {}", self.value, self.stat)
        } else {
            write!(f, "{} {}", self.value, self.stat)
        }
    }
}

/// An active effect on a character, monster, or the global game state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveEffect {
    pub id: u32,
    pub name: String,
    pub source: String,
    pub target: EffectTarget,
    pub duration: EffectDuration,
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
    #[serde(default)]
    pub notes: String,
}

impl ActiveEffect {
    pub fn is_expired(&self) -> bool {
        self.duration.is_expired()
    }

    /// Format as a one-line summary: `[1] Bless (4 rounds, +1 atk, +1 dmg)`
    pub fn summary_line(&self) -> String {
        let mut parts = vec![format!("{}", self.duration)];
        for m in &self.modifiers {
            parts.push(format!("{}", m));
        }
        format!("[{}] {} ({})", self.id, self.name, parts.join(", "))
    }

    /// Format as multi-line detail for QueryParty display.
    pub fn detail_lines(&self) -> String {
        let mut out = format!(
            "[{}] {} ({}, source: {})",
            self.id, self.name, self.duration, self.source
        );
        if !self.modifiers.is_empty() {
            let mods: Vec<String> = self.modifiers.iter().map(|m| format!("{}", m)).collect();
            out.push_str(&format!("\n    {}", mods.join(", ")));
        }
        if !self.notes.is_empty() {
            out.push_str(&format!("\n    Note: {}", self.notes));
        }
        out
    }
}

/// Manages a collection of active effects with auto-incrementing IDs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EffectList {
    next_id: u32,
    pub effects: Vec<ActiveEffect>,
}

impl EffectList {
    pub fn new() -> Self {
        EffectList {
            next_id: 1,
            effects: Vec::new(),
        }
    }

    pub fn add(&mut self, name: String, source: String, target: EffectTarget, duration: EffectDuration, modifiers: Vec<Modifier>, notes: String) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.effects.push(ActiveEffect {
            id,
            name,
            source,
            target,
            duration,
            modifiers,
            notes,
        });
        id
    }

    pub fn remove(&mut self, id: u32) -> Option<ActiveEffect> {
        if let Some(pos) = self.effects.iter().position(|e| e.id == id) {
            Some(self.effects.remove(pos))
        } else {
            None
        }
    }

    pub fn find(&self, id: u32) -> Option<&ActiveEffect> {
        self.effects.iter().find(|e| e.id == id)
    }

    /// Tick all round-based durations, returning expired effects.
    pub fn tick_round(&mut self) -> Vec<ActiveEffect> {
        for e in &mut self.effects {
            if let EffectDuration::Rounds(ref mut n) = e.duration {
                *n = n.saturating_sub(1);
            }
        }
        self.drain_expired()
    }

    /// Tick all turn-based durations, returning expired effects.
    pub fn tick_turn(&mut self) -> Vec<ActiveEffect> {
        for e in &mut self.effects {
            if let EffectDuration::Turns(ref mut n) = e.duration {
                *n = n.saturating_sub(1);
            }
        }
        self.drain_expired()
    }

    fn drain_expired(&mut self) -> Vec<ActiveEffect> {
        let mut expired = Vec::new();
        self.effects.retain(|e| {
            if e.is_expired() {
                expired.push(e.clone());
                false
            } else {
                true
            }
        });
        expired
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Get effects targeting a specific character.
    pub fn for_character(&self, name: &str) -> Vec<&ActiveEffect> {
        self.effects
            .iter()
            .filter(|e| matches!(&e.target, EffectTarget::Character(n) if n.eq_ignore_ascii_case(name)))
            .collect()
    }

    /// Get effects targeting a specific monster by index.
    pub fn for_monster(&self, idx: usize) -> Vec<&ActiveEffect> {
        self.effects
            .iter()
            .filter(|e| matches!(&e.target, EffectTarget::Monster(i) if *i == idx))
            .collect()
    }

    /// Get global/area effects.
    pub fn global_effects(&self) -> Vec<&ActiveEffect> {
        self.effects
            .iter()
            .filter(|e| matches!(&e.target, EffectTarget::Global))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_display() {
        assert_eq!(EffectDuration::Rounds(1).to_string(), "1 round");
        assert_eq!(EffectDuration::Rounds(4).to_string(), "4 rounds");
        assert_eq!(EffectDuration::Turns(1).to_string(), "1 turn");
        assert_eq!(EffectDuration::Turns(8).to_string(), "8 turns");
        assert_eq!(EffectDuration::Permanent.to_string(), "permanent");
        assert_eq!(EffectDuration::Concentration.to_string(), "concentration");
    }

    #[test]
    fn duration_expired() {
        assert!(EffectDuration::Rounds(0).is_expired());
        assert!(EffectDuration::Turns(0).is_expired());
        assert!(!EffectDuration::Rounds(1).is_expired());
        assert!(!EffectDuration::Turns(1).is_expired());
        assert!(!EffectDuration::Permanent.is_expired());
        assert!(!EffectDuration::Concentration.is_expired());
    }

    #[test]
    fn modifier_display() {
        let m = Modifier { stat: ModifierStat::AttackRoll, value: 1 };
        assert_eq!(m.to_string(), "+1 atk");
        let m = Modifier { stat: ModifierStat::ArmorClass, value: -2 };
        assert_eq!(m.to_string(), "-2 AC");
    }

    #[test]
    fn effect_summary_line() {
        let e = ActiveEffect {
            id: 1,
            name: "Bless".into(),
            source: "Brin".into(),
            target: EffectTarget::Character("Aldric".into()),
            duration: EffectDuration::Rounds(4),
            modifiers: vec![
                Modifier { stat: ModifierStat::AttackRoll, value: 1 },
                Modifier { stat: ModifierStat::DamageRoll, value: 1 },
            ],
            notes: String::new(),
        };
        assert_eq!(e.summary_line(), "[1] Bless (4 rounds, +1 atk, +1 dmg)");
    }

    #[test]
    fn effect_detail_lines() {
        let e = ActiveEffect {
            id: 1,
            name: "Protection from Evil".into(),
            source: "Brin".into(),
            target: EffectTarget::Character("Aldric".into()),
            duration: EffectDuration::Turns(8),
            modifiers: vec![
                Modifier { stat: ModifierStat::SavingThrows, value: 1 },
            ],
            notes: "Enchanted/summoned creatures cannot melee; broken if caster initiates melee.".into(),
        };
        let detail = e.detail_lines();
        assert!(detail.contains("[1] Protection from Evil (8 turns, source: Brin)"));
        assert!(detail.contains("+1 saves"));
        assert!(detail.contains("Note: Enchanted/summoned"));
    }

    #[test]
    fn effect_list_add_and_find() {
        let mut list = EffectList::new();
        let id = list.add(
            "Bless".into(), "Brin".into(),
            EffectTarget::Character("Aldric".into()),
            EffectDuration::Rounds(4),
            vec![], String::new(),
        );
        assert_eq!(id, 1);
        assert_eq!(list.len(), 1);
        assert!(list.find(1).is_some());
        assert!(list.find(99).is_none());
    }

    #[test]
    fn effect_list_remove() {
        let mut list = EffectList::new();
        list.add("A".into(), "S".into(), EffectTarget::Global, EffectDuration::Permanent, vec![], String::new());
        let removed = list.remove(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "A");
        assert!(list.is_empty());
        assert!(list.remove(1).is_none());
    }

    #[test]
    fn tick_round_decrements_and_expires() {
        let mut list = EffectList::new();
        list.add("Short".into(), "S".into(), EffectTarget::Global, EffectDuration::Rounds(1), vec![], String::new());
        list.add("Long".into(), "S".into(), EffectTarget::Global, EffectDuration::Rounds(3), vec![], String::new());
        list.add("Perm".into(), "S".into(), EffectTarget::Global, EffectDuration::Permanent, vec![], String::new());
        list.add("TurnBased".into(), "S".into(), EffectTarget::Global, EffectDuration::Turns(5), vec![], String::new());

        let expired = list.tick_round();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].name, "Short");
        assert_eq!(list.len(), 3);

        // Long should now have 2 rounds
        let long = list.find(2).unwrap();
        assert_eq!(long.duration, EffectDuration::Rounds(2));

        // Perm unchanged
        let perm = list.find(3).unwrap();
        assert_eq!(perm.duration, EffectDuration::Permanent);

        // Turn-based unchanged by tick_round
        let turn_based = list.find(4).unwrap();
        assert_eq!(turn_based.duration, EffectDuration::Turns(5));
    }

    #[test]
    fn tick_turn_decrements_and_expires() {
        let mut list = EffectList::new();
        list.add("TShort".into(), "S".into(), EffectTarget::Global, EffectDuration::Turns(1), vec![], String::new());
        list.add("TLong".into(), "S".into(), EffectTarget::Global, EffectDuration::Turns(3), vec![], String::new());
        list.add("Conc".into(), "S".into(), EffectTarget::Global, EffectDuration::Concentration, vec![], String::new());
        list.add("RoundBased".into(), "S".into(), EffectTarget::Global, EffectDuration::Rounds(5), vec![], String::new());

        let expired = list.tick_turn();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].name, "TShort");
        assert_eq!(list.len(), 3);

        // Concentration unchanged
        let conc = list.find(3).unwrap();
        assert_eq!(conc.duration, EffectDuration::Concentration);

        // Round-based unchanged by tick_turn
        let round_based = list.find(4).unwrap();
        assert_eq!(round_based.duration, EffectDuration::Rounds(5));
    }

    #[test]
    fn filter_by_target() {
        let mut list = EffectList::new();
        list.add("A".into(), "S".into(), EffectTarget::Character("Aldric".into()), EffectDuration::Permanent, vec![], String::new());
        list.add("B".into(), "S".into(), EffectTarget::Character("Brin".into()), EffectDuration::Permanent, vec![], String::new());
        list.add("C".into(), "S".into(), EffectTarget::Monster(0), EffectDuration::Permanent, vec![], String::new());
        list.add("D".into(), "S".into(), EffectTarget::Global, EffectDuration::Permanent, vec![], String::new());

        assert_eq!(list.for_character("Aldric").len(), 1);
        assert_eq!(list.for_character("aldric").len(), 1); // case insensitive
        assert_eq!(list.for_character("Brin").len(), 1);
        assert_eq!(list.for_monster(0).len(), 1);
        assert_eq!(list.for_monster(1).len(), 0);
        assert_eq!(list.global_effects().len(), 1);
    }

    #[test]
    fn serialization_roundtrip() {
        let mut list = EffectList::new();
        list.add(
            "Protection from Evil".into(),
            "Brin".into(),
            EffectTarget::Character("Aldric".into()),
            EffectDuration::Turns(8),
            vec![Modifier { stat: ModifierStat::SavingThrows, value: 1 }],
            "Cannot melee enchanted creatures.".into(),
        );
        let json = serde_json::to_string(&list).unwrap();
        let loaded: EffectList = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.effects[0].name, "Protection from Evil");
        assert_eq!(loaded.effects[0].modifiers.len(), 1);
    }

    #[test]
    fn backward_compat_empty_effects() {
        // Old saves won't have effects at all — default should work
        let json = r#"{"next_id":1,"effects":[]}"#;
        let list: EffectList = serde_json::from_str(json).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn auto_increment_ids() {
        let mut list = EffectList::new();
        let id1 = list.add("A".into(), "S".into(), EffectTarget::Global, EffectDuration::Permanent, vec![], String::new());
        let id2 = list.add("B".into(), "S".into(), EffectTarget::Global, EffectDuration::Permanent, vec![], String::new());
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);

        // Remove first, add another — should get id 3, not 1
        list.remove(1);
        let id3 = list.add("C".into(), "S".into(), EffectTarget::Global, EffectDuration::Permanent, vec![], String::new());
        assert_eq!(id3, 3);
    }
}
