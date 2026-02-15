use serde::{Deserialize, Serialize};
use std::fmt;

/// How long an effect lasts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectDuration {
    /// Lasts a number of combat rounds (10 seconds each).
    Rounds(u32),
    /// Lasts a number of dungeon turns (10 minutes each).
    Turns(u32),
    /// Permanent until explicitly removed.
    Permanent,
    /// Lasts as long as the caster concentrates. GM removes manually.
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

/// What stat an effect modifier targets (informational only — the AI GM applies these).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// A single modifier applied by an effect (informational).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// An active effect on a character, monster, or the game world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveEffect {
    pub id: u32,
    pub name: String,
    pub source: String,
    pub duration: EffectDuration,
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
    #[serde(default)]
    pub notes: String,
}

impl ActiveEffect {
    /// Check if this effect has expired (Rounds(0) or Turns(0)).
    pub fn is_expired(&self) -> bool {
        matches!(self.duration, EffectDuration::Rounds(0) | EffectDuration::Turns(0))
    }

    /// Tick one combat round. Returns true if the effect expired.
    pub fn tick_round(&mut self) -> bool {
        if let EffectDuration::Rounds(ref mut n) = self.duration {
            *n = n.saturating_sub(1);
            return *n == 0;
        }
        false
    }

    /// Tick one dungeon turn. Returns true if the effect expired.
    pub fn tick_turn(&mut self) -> bool {
        if let EffectDuration::Turns(ref mut n) = self.duration {
            *n = n.saturating_sub(1);
            return *n == 0;
        }
        false
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

impl fmt::Display for ActiveEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.duration)?;
        if !self.modifiers.is_empty() {
            let mods: Vec<String> = self.modifiers.iter().map(|m| m.to_string()).collect();
            write!(f, " [{}]", mods.join(", "))?;
        }
        if !self.notes.is_empty() {
            write!(f, " — {}", self.notes)?;
        }
        Ok(())
    }
}

/// Helper for managing a collection of effects.
pub fn next_effect_id(effects: &[ActiveEffect]) -> u32 {
    effects.iter().map(|e| e.id).max().unwrap_or(0) + 1
}

/// Tick all Rounds-based effects, removing expired ones.
/// Returns messages describing what expired.
pub fn tick_round_effects(effects: &mut Vec<ActiveEffect>, target_label: &str) -> Vec<String> {
    let mut messages = Vec::new();

    for effect in effects.iter_mut() {
        if effect.tick_round() {
            let mut msg = format!(
                "Effect expired on {}: {} has worn off.",
                target_label, effect.name
            );
            if !effect.notes.is_empty() {
                msg.push_str(&format!(" ({})", effect.notes));
            }
            messages.push(msg);
        }
    }

    effects.retain(|e| !e.is_expired());
    messages
}

/// Tick all Turns-based effects, removing expired ones.
/// Returns messages describing what expired.
pub fn tick_turn_effects(effects: &mut Vec<ActiveEffect>, target_label: &str) -> Vec<String> {
    let mut messages = Vec::new();

    for effect in effects.iter_mut() {
        if effect.tick_turn() {
            let mut msg = format!(
                "Effect expired on {}: {} has worn off.",
                target_label, effect.name
            );
            if !effect.notes.is_empty() {
                msg.push_str(&format!(" ({})", effect.notes));
            }
            messages.push(msg);
        }
    }

    effects.retain(|e| !e.is_expired());
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_effect(name: &str, duration: EffectDuration) -> ActiveEffect {
        ActiveEffect {
            id: 1,
            name: name.to_string(),
            source: "test".to_string(),
            duration,
            modifiers: Vec::new(),
            notes: String::new(),
        }
    }

    // --- Serialization roundtrip ---

    #[test]
    fn serialization_roundtrip() {
        let effect = ActiveEffect {
            id: 1,
            name: "Bless".to_string(),
            source: "Cleric".to_string(),
            duration: EffectDuration::Rounds(6),
            modifiers: vec![
                Modifier { stat: ModifierStat::AttackRoll, value: 1 },
                Modifier { stat: ModifierStat::Morale, value: 1 },
            ],
            notes: "Party-wide".to_string(),
        };

        let json = serde_json::to_string(&effect).unwrap();
        let loaded: ActiveEffect = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.name, "Bless");
        assert_eq!(loaded.duration, EffectDuration::Rounds(6));
        assert_eq!(loaded.modifiers.len(), 2);
        assert_eq!(loaded.notes, "Party-wide");
    }

    #[test]
    fn backward_compat_no_modifiers_no_notes() {
        // Old saves without modifiers/notes fields
        let json = r#"{
            "id": 1,
            "name": "Shield",
            "source": "MU",
            "duration": {"Rounds": 10}
        }"#;
        let effect: ActiveEffect = serde_json::from_str(json).unwrap();
        assert_eq!(effect.name, "Shield");
        assert!(effect.modifiers.is_empty());
        assert!(effect.notes.is_empty());
    }

    // --- tick_round ---

    #[test]
    fn tick_round_decrements_rounds() {
        let mut effect = make_effect("Bless", EffectDuration::Rounds(3));
        assert!(!effect.tick_round());
        assert_eq!(effect.duration, EffectDuration::Rounds(2));
        assert!(!effect.tick_round());
        assert_eq!(effect.duration, EffectDuration::Rounds(1));
        assert!(effect.tick_round()); // expired
        assert_eq!(effect.duration, EffectDuration::Rounds(0));
        assert!(effect.is_expired());
    }

    #[test]
    fn tick_round_does_not_affect_turns() {
        let mut effect = make_effect("Light", EffectDuration::Turns(6));
        assert!(!effect.tick_round());
        assert_eq!(effect.duration, EffectDuration::Turns(6));
    }

    #[test]
    fn tick_round_does_not_affect_permanent() {
        let mut effect = make_effect("Curse", EffectDuration::Permanent);
        assert!(!effect.tick_round());
        assert_eq!(effect.duration, EffectDuration::Permanent);
    }

    #[test]
    fn tick_round_does_not_affect_concentration() {
        let mut effect = make_effect("Detect Magic", EffectDuration::Concentration);
        assert!(!effect.tick_round());
        assert_eq!(effect.duration, EffectDuration::Concentration);
    }

    // --- tick_turn ---

    #[test]
    fn tick_turn_decrements_turns() {
        let mut effect = make_effect("Light", EffectDuration::Turns(2));
        assert!(!effect.tick_turn());
        assert_eq!(effect.duration, EffectDuration::Turns(1));
        assert!(effect.tick_turn()); // expired
        assert!(effect.is_expired());
    }

    #[test]
    fn tick_turn_does_not_affect_rounds() {
        let mut effect = make_effect("Bless", EffectDuration::Rounds(3));
        assert!(!effect.tick_turn());
        assert_eq!(effect.duration, EffectDuration::Rounds(3));
    }

    // --- tick_round_effects (batch) ---

    #[test]
    fn tick_round_effects_removes_expired() {
        let mut effects = vec![
            make_effect("Bless", EffectDuration::Rounds(1)),
            make_effect("Shield", EffectDuration::Rounds(3)),
            make_effect("Light", EffectDuration::Turns(6)),
        ];

        let messages = tick_round_effects(&mut effects, "Fighter");
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("Bless"));
        assert!(messages[0].contains("Fighter"));
        assert_eq!(effects.len(), 2); // Bless removed
        assert_eq!(effects[0].name, "Shield");
        assert_eq!(effects[0].duration, EffectDuration::Rounds(2));
        assert_eq!(effects[1].name, "Light"); // unchanged
    }

    #[test]
    fn tick_round_effects_includes_notes_in_expiry_message() {
        let mut effect = make_effect("Hold Person", EffectDuration::Rounds(1));
        effect.notes = "Target can act again".to_string();
        let mut effects = vec![effect];

        let messages = tick_round_effects(&mut effects, "Goblin");
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("Target can act again"));
    }

    // --- Display ---

    #[test]
    fn display_formatting() {
        let effect = ActiveEffect {
            id: 1,
            name: "Bless".to_string(),
            source: "Cleric".to_string(),
            duration: EffectDuration::Rounds(6),
            modifiers: vec![Modifier { stat: ModifierStat::AttackRoll, value: 1 }],
            notes: "Allies only".to_string(),
        };
        let display = format!("{}", effect);
        assert!(display.contains("Bless"));
        assert!(display.contains("6 rounds"));
        assert!(display.contains("+1 atk"));
        assert!(display.contains("Allies only"));
    }

    #[test]
    fn duration_display() {
        assert_eq!(format!("{}", EffectDuration::Rounds(1)), "1 round");
        assert_eq!(format!("{}", EffectDuration::Rounds(3)), "3 rounds");
        assert_eq!(format!("{}", EffectDuration::Turns(1)), "1 turn");
        assert_eq!(format!("{}", EffectDuration::Turns(10)), "10 turns");
        assert_eq!(format!("{}", EffectDuration::Permanent), "permanent");
        assert_eq!(format!("{}", EffectDuration::Concentration), "concentration");
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

    // --- summary_line / detail_lines ---

    #[test]
    fn effect_summary_line() {
        let e = ActiveEffect {
            id: 1,
            name: "Bless".into(),
            source: "Brin".into(),
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

    // --- next_effect_id ---

    #[test]
    fn next_effect_id_empty() {
        assert_eq!(next_effect_id(&[]), 1);
    }

    #[test]
    fn next_effect_id_increments() {
        let effects = vec![
            make_effect("A", EffectDuration::Rounds(1)),
            ActiveEffect { id: 5, ..make_effect("B", EffectDuration::Rounds(2)) },
        ];
        assert_eq!(next_effect_id(&effects), 6);
    }
}
