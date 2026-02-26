//! Spell preparation and resource management.
//!
//! Phase 3 of the spell system abstraction (oag-08wgc):
//! - Vancian memorize/prepare from spellbook
//! - Long rest to recharge spell slots/points
//! - Spell point variant support
//! - Spontaneous casting support
//!
//! The DSL defines the casting model (via `casting_resource_type`).
//! Rust tracks mutable state (slots used, points spent, prepared list).

use serde::Serialize;

use crate::engine::result::EngineError;
use crate::persist::GameState;
use crate::rules::class::class_def;
use crate::rules::spell::{self, SpellProgression};
use crate::rules::spell_data;

/// A single spell to prepare, identified by name and spell level.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct PreparedSpellEntry {
    pub name: String,
    pub level: u32,
}

/// Result of preparing spells for a character.
#[derive(Debug, Clone, Serialize)]
pub struct PrepareSpellsResult {
    pub message: String,
    pub character: String,
    pub prepared: Vec<Vec<String>>,
}

/// Result of a long rest for the party.
#[derive(Debug, Clone, Serialize)]
pub struct LongRestResult {
    pub message: String,
    pub characters_recharged: Vec<String>,
}

/// Prepare (memorize) spells for a character.
///
/// For Vancian casting: fills the character's prepared spell list from their
/// known spells, limited by available slots at each level.
///
/// Validates:
/// - Character exists and is a spell caster
/// - Each spell is in the character's known spells or the spell registry
/// - Number of spells per level does not exceed available slots
/// - Spell level is valid for the character's class/level
pub fn action_prepare_spells(
    state: &mut GameState,
    char_name: &str,
    spells: &[PreparedSpellEntry],
) -> Result<PrepareSpellsResult, EngineError> {
    // Find the character and get their class info
    let character = state
        .party
        .find_member(char_name)
        .ok_or_else(|| EngineError::InvalidInput(format!("no party member named '{}'.", char_name)))?;

    let def = class_def(character.class);

    if def.spell_progression == SpellProgression::NonCaster {
        return Err(EngineError::InvalidInput(format!(
            "{} is a {} and cannot cast spells.",
            char_name,
            character.class.name()
        )));
    }

    let max_slots = spell::spell_slots(def.spell_progression, character.level);
    let resource_type = spell::casting_resource_type(def.spell_progression);

    // Spell point systems don't use preparation
    if resource_type != "vancian_slots" {
        return Err(EngineError::InvalidInput(format!(
            "{} uses {} casting and does not prepare spells.",
            char_name, resource_type
        )));
    }

    // Build the new prepared list, validating each entry
    let mut new_prepared: Vec<Vec<String>> = vec![Vec::new(); 6];

    for entry in spells {
        if entry.level == 0 || entry.level > 6 {
            return Err(EngineError::InvalidInput(format!(
                "invalid spell level {}. Must be 1-6.",
                entry.level
            )));
        }

        let idx = (entry.level - 1) as usize;

        // Check that the character has slots at this level
        if max_slots[idx] == 0 {
            return Err(EngineError::InvalidInput(format!(
                "{} cannot cast level {} spells at character level {}.",
                char_name, entry.level, character.level
            )));
        }

        // Check slot limit
        if new_prepared[idx].len() >= max_slots[idx] as usize {
            return Err(EngineError::InvalidInput(format!(
                "cannot prepare more level {} spells. {} has {} slot(s) at that level.",
                entry.level, char_name, max_slots[idx]
            )));
        }

        // Verify spell exists in registry (case-insensitive)
        if spell_data::find_spell(&entry.name, None).is_none() {
            return Err(EngineError::InvalidInput(format!(
                "unknown spell '{}'.",
                entry.name
            )));
        }

        new_prepared[idx].push(entry.name.clone());
    }

    // Apply the prepared spell list
    let character = state
        .party
        .find_member_mut(char_name)
        .expect("character verified above");
    character.prepared_spells = new_prepared.clone();
    // Reset slots used when re-preparing
    character.spell_slots_used = [0; 6];

    let total: usize = new_prepared.iter().map(|v| v.len()).sum();
    let mut msg = format!("{} prepares {} spell(s).", char_name, total);
    for (i, level_spells) in new_prepared.iter().enumerate() {
        if !level_spells.is_empty() {
            msg.push_str(&format!(
                "\n  Level {}: {}",
                i + 1,
                level_spells.join(", ")
            ));
        }
    }

    Ok(PrepareSpellsResult {
        message: msg,
        character: char_name.to_string(),
        prepared: new_prepared,
    })
}

/// Long rest: recharge all spell resources for the entire party.
///
/// For Vancian casting: resets spell_slots_used to [0; 6].
/// For spell points: resets spell_points_used to 0.
/// Prepared spells are retained (character still has them memorized).
///
/// This represents an overnight rest with 8 hours of sleep.
pub fn action_long_rest(state: &mut GameState) -> Result<LongRestResult, EngineError> {
    let mut recharged = Vec::new();

    for member in &mut state.party.members {
        if !member.is_alive() {
            continue;
        }

        let def = class_def(member.class);
        if def.spell_progression == SpellProgression::NonCaster {
            continue;
        }

        let had_used_slots = member.spell_slots_used.iter().any(|&s| s > 0);
        let had_used_points = member.spell_points_used > 0;

        if had_used_slots || had_used_points {
            member.spell_slots_used = [0; 6];
            member.spell_points_used = 0;
            recharged.push(member.name.clone());
        }
    }

    let msg = if recharged.is_empty() {
        "party completes a long rest. no spell resources to recharge.".to_string()
    } else {
        format!(
            "party completes a long rest. spell resources recharged for: {}.",
            recharged.join(", ")
        )
    };

    Ok(LongRestResult {
        message: msg,
        characters_recharged: recharged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Character, Spell};
    use crate::rules::class::Class;

    fn setup_magic_user() -> GameState {
        let mut state = GameState::new();
        let mut mu = Character::new("Zara", Class::MagicUser);
        mu.level = 3; // Gets [2, 1, 0, 0, 0, 0] slots
        mu.spells = vec![
            Spell::new("Sleep", 1),
            Spell::new("Magic Missile", 1),
            Spell::new("Shield", 1),
            Spell::new("Web", 2),
        ];
        state.party.add_member(mu);
        state
    }

    #[test]
    fn prepare_spells_happy_path() {
        let mut state = setup_magic_user();
        let spells = vec![
            PreparedSpellEntry { name: "Sleep".to_string(), level: 1 },
            PreparedSpellEntry { name: "Magic Missile".to_string(), level: 1 },
            PreparedSpellEntry { name: "Web".to_string(), level: 2 },
        ];
        let result = action_prepare_spells(&mut state, "Zara", &spells).unwrap();
        assert_eq!(result.prepared[0].len(), 2); // 2 first-level spells
        assert_eq!(result.prepared[1].len(), 1); // 1 second-level spell

        let zara = state.party.find_member("Zara").unwrap();
        assert_eq!(zara.prepared_spells[0], vec!["Sleep", "Magic Missile"]);
        assert_eq!(zara.prepared_spells[1], vec!["Web"]);
    }

    #[test]
    fn prepare_spells_rejects_non_caster() {
        let mut state = GameState::new();
        state.party.add_member(Character::new("Grond", Class::Fighter));
        let spells = vec![
            PreparedSpellEntry { name: "Sleep".to_string(), level: 1 },
        ];
        let result = action_prepare_spells(&mut state, "Grond", &spells);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot cast spells"));
    }

    #[test]
    fn prepare_spells_rejects_excess_slots() {
        let mut state = setup_magic_user();
        // Level 3 MU has 2 first-level slots, try to prepare 3
        let spells = vec![
            PreparedSpellEntry { name: "Sleep".to_string(), level: 1 },
            PreparedSpellEntry { name: "Magic Missile".to_string(), level: 1 },
            PreparedSpellEntry { name: "Shield".to_string(), level: 1 },
        ];
        let result = action_prepare_spells(&mut state, "Zara", &spells);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot prepare more"));
    }

    #[test]
    fn prepare_spells_rejects_unknown_spell() {
        let mut state = setup_magic_user();
        let spells = vec![
            PreparedSpellEntry { name: "Nonexistent Spell".to_string(), level: 1 },
        ];
        let result = action_prepare_spells(&mut state, "Zara", &spells);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown spell"));
    }

    #[test]
    fn prepare_spells_rejects_too_high_level() {
        let mut state = setup_magic_user();
        // Level 3 MU has no 3rd level slots
        let spells = vec![
            PreparedSpellEntry { name: "Fire Ball".to_string(), level: 3 },
        ];
        let result = action_prepare_spells(&mut state, "Zara", &spells);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot cast level 3"));
    }

    #[test]
    fn prepare_spells_resets_slots_used() {
        let mut state = setup_magic_user();
        // Simulate some casting
        state.party.find_member_mut("Zara").unwrap().spell_slots_used = [1, 0, 0, 0, 0, 0];

        let spells = vec![
            PreparedSpellEntry { name: "Sleep".to_string(), level: 1 },
        ];
        action_prepare_spells(&mut state, "Zara", &spells).unwrap();

        let zara = state.party.find_member("Zara").unwrap();
        assert_eq!(zara.spell_slots_used, [0; 6]); // reset on re-preparation
    }

    #[test]
    fn long_rest_recharges_slots() {
        let mut state = setup_magic_user();
        let zara = state.party.find_member_mut("Zara").unwrap();
        zara.spell_slots_used = [2, 1, 0, 0, 0, 0];
        zara.prepared_spells = vec![
            vec!["Sleep".to_string(), "Magic Missile".to_string()],
            vec!["Web".to_string()],
        ];

        let result = action_long_rest(&mut state).unwrap();
        assert!(result.characters_recharged.contains(&"Zara".to_string()));

        let zara = state.party.find_member("Zara").unwrap();
        assert_eq!(zara.spell_slots_used, [0; 6]);
        // Prepared spells retained after rest
        assert_eq!(zara.prepared_spells[0].len(), 2);
    }

    #[test]
    fn long_rest_skips_non_casters() {
        let mut state = GameState::new();
        state.party.add_member(Character::new("Grond", Class::Fighter));
        let result = action_long_rest(&mut state).unwrap();
        assert!(result.characters_recharged.is_empty());
        assert!(result.message.contains("no spell resources"));
    }

    #[test]
    fn long_rest_skips_dead_characters() {
        let mut state = setup_magic_user();
        let zara = state.party.find_member_mut("Zara").unwrap();
        zara.spell_slots_used = [1, 0, 0, 0, 0, 0];
        zara.hp = 0; // dead

        let result = action_long_rest(&mut state).unwrap();
        assert!(result.characters_recharged.is_empty());
    }

    #[test]
    fn long_rest_recharges_spell_points() {
        let mut state = GameState::new();
        let mut mu = Character::new("Pointy", Class::MagicUser);
        mu.level = 3;
        mu.spell_points_used = 5;
        state.party.add_member(mu);

        let result = action_long_rest(&mut state).unwrap();
        assert!(result.characters_recharged.contains(&"Pointy".to_string()));

        let pointy = state.party.find_member("Pointy").unwrap();
        assert_eq!(pointy.spell_points_used, 0);
    }
}
