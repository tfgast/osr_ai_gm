use crate::engine::result::EngineError;
use crate::engine::xp;
use crate::persist::GameState;
use crate::rules::class::class_def;
use crate::rules::thief;
use crate::rules::xp::{check_level_up, xp_for_level};
use crate::rules::spell;
use crate::state::game::GameMode;

use super::results::{
    AddNoteResult, AddRationsResult, AwardTreasureXpResult, AwardXpResult, DamageResult,
    DeleteNoteResult, DismissRetainerResult, HealResult, ListNotesResult, ListRetainersResult,
    NoteEntry, RetainerSummary, RulingResult, SetHpResult, SetRationsResult, ThiefSkillCheckResult,
    TrainResult,
};

fn no_party_member_err(char_name: &str) -> EngineError {
    EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
}

pub fn action_award_xp(
    state: &mut GameState,
    char_name: &str,
    amount: u64,
    apply_prime_req_modifier: bool,
) -> Result<AwardXpResult, EngineError> {
    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;

    if !character.is_alive() {
        return Err(EngineError::InvalidInput(format!(
            "{} is dead and cannot receive XP.",
            character.name
        )));
    }

    let (adjusted_xp, modifier_pct, total_xp, ready_to_train) = if apply_prime_req_modifier {
        let result = xp::award_xp(character, amount, 0);
        (
            result.adjusted_xp,
            result.modifier_pct,
            result.new_total,
            result.ready_to_train,
        )
    } else {
        character.xp += amount;
        let ready = check_level_up(character.class, character.level, character.xp).is_some();
        (amount, 0, character.xp, ready)
    };

    let next_xp = xp_for_level(character.class, character.level + 1);
    let next_level_xp = if next_xp == u64::MAX {
        None
    } else {
        Some(next_xp)
    };

    Ok(AwardXpResult {
        character: character.name.clone(),
        base_xp: amount,
        adjusted_xp,
        modifier_pct,
        total_xp,
        next_level_xp,
        ready_to_train,
    })
}

pub fn action_ruling(state: &mut GameState, text: &str) -> Result<RulingResult, EngineError> {
    let note = format!("[RULING] {}", text);
    state.notes.push(note.clone());
    Ok(RulingResult {
        text: text.to_string(),
        note,
    })
}

pub fn action_list_notes(state: &GameState) -> Result<ListNotesResult, EngineError> {
    let notes = state
        .notes
        .iter()
        .enumerate()
        .map(|(i, note)| NoteEntry {
            index: i + 1,
            text: note.clone(),
        })
        .collect();
    Ok(ListNotesResult { notes })
}

pub fn action_delete_note(
    state: &mut GameState,
    index: usize,
) -> Result<DeleteNoteResult, EngineError> {
    if state.notes.is_empty() {
        return Err(EngineError::InvalidInput("no notes to delete.".to_string()));
    }
    if index < 1 || index > state.notes.len() {
        return Err(EngineError::InvalidInput(format!(
            "index {} out of range; have {} note{}.",
            index,
            state.notes.len(),
            if state.notes.len() == 1 { "" } else { "s" }
        )));
    }

    let deleted = state.notes.remove(index - 1);
    Ok(DeleteNoteResult { index, deleted })
}

pub fn action_list_retainers(state: &GameState) -> Result<ListRetainersResult, EngineError> {
    let retainers = state
        .retainers
        .iter()
        .map(|r| RetainerSummary {
            name: r.name.clone(),
            class: r.class,
            level: r.level,
            hp: r.hp,
            max_hp: r.max_hp,
            loyalty: r.loyalty,
            wage_gp: r.wage_gp,
            alive: r.is_alive(),
        })
        .collect();

    Ok(ListRetainersResult { retainers })
}

pub fn action_dismiss_retainer(
    state: &mut GameState,
    name: &str,
) -> Result<DismissRetainerResult, EngineError> {
    let idx = state
        .retainers
        .iter()
        .position(|r| r.name.eq_ignore_ascii_case(name));

    match idx {
        Some(i) => {
            let removed = state.retainers.remove(i);
            Ok(DismissRetainerResult {
                name: removed.name,
                class: removed.class,
            })
        }
        None => Err(EngineError::InvalidInput(format!(
            "no retainer named '{}'.",
            name
        ))),
    }
}

pub fn action_heal(
    state: &mut GameState,
    char_name: &str,
    amount: i32,
) -> Result<HealResult, EngineError> {
    if amount < 1 {
        return Err(EngineError::InvalidInput(
            "amount must be a positive integer".to_string(),
        ));
    }

    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;

    if !character.is_alive() {
        return Err(EngineError::InvalidInput(format!(
            "{} is dead and cannot be healed (use SetHp to override).",
            character.name
        )));
    }

    let old_hp = character.hp;
    let new_hp = (character.hp + amount).min(character.max_hp).max(character.hp);
    character.hp = new_hp;
    let healed = character.hp - old_hp;

    Ok(HealResult {
        character: character.name.clone(),
        healed,
        old_hp,
        hp: character.hp,
        max_hp: character.max_hp,
    })
}

pub fn action_damage(
    state: &mut GameState,
    char_name: &str,
    amount: i32,
) -> Result<DamageResult, EngineError> {
    if amount < 1 {
        return Err(EngineError::InvalidInput(
            "amount must be a positive integer".to_string(),
        ));
    }

    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;

    let old_hp = character.hp;
    character.hp -= amount;
    let alive = character.is_alive();
    let status = if alive { "wounded" } else { "DEAD" }.to_string();

    Ok(DamageResult {
        character: character.name.clone(),
        damage: amount,
        old_hp,
        hp: character.hp,
        max_hp: character.max_hp,
        alive,
        status,
    })
}

pub fn action_set_hp(
    state: &mut GameState,
    char_name: &str,
    hp: i32,
) -> Result<SetHpResult, EngineError> {
    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;

    let old_hp = character.hp;
    character.hp = hp;
    let alive = character.is_alive();
    let status = if alive { "alive" } else { "DEAD" }.to_string();

    Ok(SetHpResult {
        character: character.name.clone(),
        old_hp,
        hp: character.hp,
        max_hp: character.max_hp,
        alive,
        status,
    })
}

pub fn action_set_rations(
    state: &mut GameState,
    amount: u32,
) -> Result<SetRationsResult, EngineError> {
    let old_rations = state.party.rations;
    state.party.rations = amount;
    Ok(SetRationsResult {
        old_rations,
        rations: amount,
    })
}

pub fn action_add_rations(
    state: &mut GameState,
    amount: u32,
) -> Result<AddRationsResult, EngineError> {
    if amount < 1 {
        return Err(EngineError::InvalidInput(
            "amount must be a positive integer".to_string(),
        ));
    }

    state.party.rations += amount;
    Ok(AddRationsResult {
        added: amount,
        rations: state.party.rations,
    })
}

pub fn action_add_note(state: &mut GameState, text: &str) -> Result<AddNoteResult, EngineError> {
    if text.is_empty() {
        return Err(EngineError::InvalidInput("note text cannot be empty.".to_string()));
    }
    state.notes.push(text.to_string());
    Ok(AddNoteResult {
        text: text.to_string(),
    })
}

pub fn action_award_treasure_xp(
    state: &mut GameState,
    char_name: &str,
    treasure_gp: u64,
    monster_xp: u64,
) -> Result<AwardTreasureXpResult, EngineError> {
    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;

    if !character.is_alive() {
        return Err(EngineError::InvalidInput(format!(
            "{} is dead and cannot receive XP.",
            character.name
        )));
    }

    let result = xp::award_xp(character, treasure_gp, monster_xp);
    Ok(AwardTreasureXpResult {
        character: character.name.clone(),
        treasure_gp,
        monster_xp,
        base_xp: result.base_xp,
        modifier_pct: result.modifier_pct,
        adjusted_xp: result.adjusted_xp,
        total_xp: result.new_total,
        ready_to_train: result.ready_to_train,
    })
}

/// Level up without gold cost (GM fiat). Used by the API `LevelUp` command.
pub fn action_level_up(state: &mut GameState, char_name: &str) -> Result<TrainResult, EngineError> {
    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;
    let cls = character.class;
    let _next_level = check_level_up(cls, character.level, character.xp).ok_or_else(|| {
        let needed = xp_for_level(cls, character.level + 1);
        if needed == u64::MAX {
            EngineError::InvalidInput(format!(
                "{} is at maximum level ({}).",
                character.name, character.level
            ))
        } else {
            EngineError::InvalidInput(format!(
                "{} needs {} XP for level {} (has {}).",
                character.name,
                needed,
                character.level + 1,
                character.xp
            ))
        }
    })?;

    let old_saves = character.saving_throws;
    let old_thac0 = character.thac0;
    let old_hp = character.max_hp;
    let def = class_def(cls);

    let lu = xp::apply_level_up(character);

    let has_thief_skills = thief::has_thief_skills(cls);
    let spell_list_name = match def.spell_list {
        spell::SpellListType::Cleric => "cleric",
        spell::SpellListType::Druid => "druid",
        spell::SpellListType::MagicUser => "magic-user",
        spell::SpellListType::Illusionist => "illusionist",
        spell::SpellListType::DrowArcaneAndDivine => "drow",
        spell::SpellListType::None => "",
    }
    .to_string();

    let gained_first_spells = lu.old_spell_slots.iter().all(|&s| s == 0)
        && lu.new_spell_slots.iter().any(|&s| s > 0);

    let ready_for_next = check_level_up(cls, character.level, character.xp).is_some();
    let next_cost = if ready_for_next {
        Some((character.level + 1) * 100)
    } else {
        None
    };

    Ok(TrainResult {
        character: character.name.clone(),
        new_level: lu.new_level,
        cost_gp: 0,
        gold_remaining: character.gold_gp,
        hp_gained: lu.hp_gained,
        old_hp,
        new_hp: character.max_hp,
        current_hp: character.hp,
        old_thac0,
        new_thac0: lu.new_thac0,
        old_saves,
        new_saves: lu.new_saves,
        old_spell_slots: lu.old_spell_slots,
        new_spell_slots: lu.new_spell_slots,
        has_thief_skills,
        spell_list_name,
        gained_first_spells,
        ready_for_next,
        next_cost,
    })
}

/// Train (level up with gold cost). Used by the CLI `train` command.
pub fn action_train(state: &mut GameState, char_name: &str) -> Result<TrainResult, EngineError> {
    if state.mode == GameMode::Combat {
        return Err(EngineError::WrongState(
            "cannot train during combat.".to_string(),
        ));
    }
    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;
    if !character.is_alive() {
        return Err(EngineError::InvalidInput(format!(
            "{} is dead.",
            character.name
        )));
    }
    let cls = character.class;
    let next_level = match check_level_up(cls, character.level, character.xp) {
        None => {
            let needed = xp_for_level(cls, character.level + 1);
            if needed == u64::MAX {
                return Err(EngineError::InvalidInput(format!(
                    "{} is at maximum level ({}).",
                    character.name, character.level
                )));
            }
            return Err(EngineError::InvalidInput(format!(
                "{} needs {} XP for level {} (has {}).",
                character.name,
                needed,
                character.level + 1,
                character.xp
            )));
        }
        Some(nl) => nl,
    };

    let cost = next_level * 100;
    if character.gold_gp < cost {
        return Err(EngineError::InvalidInput(format!(
            "{} needs {}gp to train for level {} (has {}gp).",
            character.name, cost, next_level, character.gold_gp
        )));
    }

    // Capture old state for report
    let old_saves = character.saving_throws;
    let old_thac0 = character.thac0;
    let old_hp = character.max_hp;
    let def = class_def(cls);

    // Deduct gold and apply level-up
    character.gold_gp -= cost;
    let lu = xp::apply_level_up(character);

    let has_thief_skills = thief::has_thief_skills(cls);
    let spell_list_name = match def.spell_list {
        spell::SpellListType::Cleric => "cleric",
        spell::SpellListType::Druid => "druid",
        spell::SpellListType::MagicUser => "magic-user",
        spell::SpellListType::Illusionist => "illusionist",
        spell::SpellListType::DrowArcaneAndDivine => "drow",
        spell::SpellListType::None => "",
    }
    .to_string();

    let gained_first_spells = lu.old_spell_slots.iter().all(|&s| s == 0)
        && lu.new_spell_slots.iter().any(|&s| s > 0);

    let ready_for_next = check_level_up(cls, character.level, character.xp).is_some();
    let next_cost = if ready_for_next {
        Some((character.level + 1) * 100)
    } else {
        None
    };

    Ok(TrainResult {
        character: character.name.clone(),
        new_level: lu.new_level,
        cost_gp: cost,
        gold_remaining: character.gold_gp,
        hp_gained: lu.hp_gained,
        old_hp,
        new_hp: character.max_hp,
        current_hp: character.hp,
        old_thac0,
        new_thac0: lu.new_thac0,
        old_saves,
        new_saves: lu.new_saves,
        old_spell_slots: lu.old_spell_slots,
        new_spell_slots: lu.new_spell_slots,
        has_thief_skills,
        spell_list_name,
        gained_first_spells,
        ready_for_next,
        next_cost,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    use crate::rules::class::Class;

    fn make_character_with_xp(name: &str, xp: u64) -> Character {
        let mut c = Character::new(name, Class::Fighter);
        c.xp = xp;
        c
    }

    // --- action_award_xp overflow parity (oag-gyymh.1) ---

    #[test]
    fn award_xp_normal_addition() {
        let mut state = GameState::new();
        state.party.add_member(make_character_with_xp("Aldric", 100));
        let result = action_award_xp(&mut state, "Aldric", 50, false).unwrap();
        assert_eq!(result.total_xp, 150);
        assert_eq!(state.party.find_member("Aldric").unwrap().xp, 150);
    }

    #[test]
    fn award_xp_near_max_no_overflow() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_character_with_xp("Aldric", u64::MAX - 10));
        let result = action_award_xp(&mut state, "Aldric", 10, false).unwrap();
        assert_eq!(result.total_xp, u64::MAX);
        assert_eq!(state.party.find_member("Aldric").unwrap().xp, u64::MAX);
    }

    // --- action_add_rations overflow parity (oag-gyymh.1) ---

    #[test]
    fn add_rations_normal_addition() {
        let mut state = GameState::new();
        state.party.rations = 5;
        let result = action_add_rations(&mut state, 3).unwrap();
        assert_eq!(result.rations, 8);
        assert_eq!(state.party.rations, 8);
    }

    #[test]
    fn add_rations_near_max_no_overflow() {
        let mut state = GameState::new();
        state.party.rations = u32::MAX - 10;
        let result = action_add_rations(&mut state, 10).unwrap();
        assert_eq!(result.rations, u32::MAX);
        assert_eq!(state.party.rations, u32::MAX);
    }

    #[test]
    fn add_rations_zero_rejected() {
        let mut state = GameState::new();
        state.party.rations = 5;
        let result = action_add_rations(&mut state, 0);
        assert!(result.is_err());
    }

    // --- action_add_note (oag-mol-jqd) ---

    #[test]
    fn add_note_basic() {
        let mut state = GameState::new();
        let result = action_add_note(&mut state, "Found a secret door").unwrap();
        assert_eq!(result.text, "Found a secret door");
        assert_eq!(state.notes.len(), 1);
        assert_eq!(state.notes[0], "Found a secret door");
    }

    #[test]
    fn add_note_empty_rejected() {
        let mut state = GameState::new();
        let result = action_add_note(&mut state, "");
        assert!(result.is_err());
        assert!(state.notes.is_empty());
    }

    #[test]
    fn add_note_multiple() {
        let mut state = GameState::new();
        action_add_note(&mut state, "Note one").unwrap();
        action_add_note(&mut state, "Note two").unwrap();
        assert_eq!(state.notes.len(), 2);
        assert_eq!(state.notes[0], "Note one");
        assert_eq!(state.notes[1], "Note two");
    }

    // --- action_award_treasure_xp (oag-mol-jqd) ---

    fn make_fighter_with_abilities(name: &str, xp: u64, gold: u32) -> Character {
        let mut c = Character::new(name, Class::Fighter);
        c.abilities = crate::model::AbilityScores {
            strength: 16,
            intelligence: 10,
            wisdom: 10,
            dexterity: 10,
            constitution: 14,
            charisma: 10,
        };
        c.xp = xp;
        c.gold_gp = gold;
        c.hp = 8;
        c.max_hp = 8;
        c
    }

    #[test]
    fn award_treasure_xp_basic() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_fighter_with_abilities("Aldric", 0, 500));
        let result = action_award_treasure_xp(&mut state, "Aldric", 500, 100).unwrap();
        assert_eq!(result.character, "Aldric");
        assert_eq!(result.treasure_gp, 500);
        assert_eq!(result.monster_xp, 100);
        // base_xp = treasure_gp + monster_xp = 600
        assert_eq!(result.base_xp, 600);
        // STR 16 = +10%, so adjusted = 660
        assert_eq!(result.modifier_pct, 10);
        assert_eq!(result.adjusted_xp, 660);
        assert_eq!(result.total_xp, 660);
        assert_eq!(state.party.find_member("Aldric").unwrap().xp, 660);
    }

    #[test]
    fn award_treasure_xp_no_character() {
        let mut state = GameState::new();
        let result = action_award_treasure_xp(&mut state, "Nobody", 100, 50);
        assert!(result.is_err());
    }

    #[test]
    fn award_treasure_xp_zero_amounts() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_fighter_with_abilities("Aldric", 100, 500));
        let result = action_award_treasure_xp(&mut state, "Aldric", 0, 0).unwrap();
        assert_eq!(result.base_xp, 0);
        assert_eq!(result.adjusted_xp, 0);
        assert_eq!(result.total_xp, 100); // unchanged
    }

    #[test]
    fn award_treasure_xp_ready_to_train() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_fighter_with_abilities("Aldric", 1_500, 500));
        // Need 2000 for L2. 1500 + (500 * 1.10) = 2050
        let result = action_award_treasure_xp(&mut state, "Aldric", 500, 0).unwrap();
        assert!(result.ready_to_train);
    }

    // --- action_level_up (oag-mol-jqd) ---

    #[test]
    fn level_up_no_gold_cost() {
        let mut state = GameState::new();
        let mut c = make_fighter_with_abilities("Aldric", 2_100, 50);
        c.hp = 8;
        c.max_hp = 8;
        state.party.add_member(c);
        let result = action_level_up(&mut state, "Aldric").unwrap();
        assert_eq!(result.new_level, 2);
        assert_eq!(result.cost_gp, 0); // no gold cost for level_up
        // Gold should be unchanged
        assert_eq!(result.gold_remaining, 50);
        assert_eq!(state.party.find_member("Aldric").unwrap().gold_gp, 50);
        assert_eq!(state.party.find_member("Aldric").unwrap().level, 2);
    }

    #[test]
    fn level_up_insufficient_xp() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_fighter_with_abilities("Aldric", 100, 500));
        let result = action_level_up(&mut state, "Aldric");
        assert!(result.is_err());
        assert_eq!(state.party.find_member("Aldric").unwrap().level, 1);
    }

    #[test]
    fn level_up_no_character() {
        let mut state = GameState::new();
        let result = action_level_up(&mut state, "Nobody");
        assert!(result.is_err());
    }

    #[test]
    fn level_up_max_level() {
        let mut state = GameState::new();
        let mut c = Character::new("Bilbo", Class::Halfling);
        c.level = 8; // Halfling max
        c.xp = 999_999;
        c.gold_gp = 9999;
        state.party.add_member(c);
        let result = action_level_up(&mut state, "Bilbo");
        assert!(result.is_err());
    }

    #[test]
    fn level_up_hp_increases() {
        let mut state = GameState::new();
        let c = make_fighter_with_abilities("Aldric", 2_100, 50);
        state.party.add_member(c);
        let result = action_level_up(&mut state, "Aldric").unwrap();
        assert!(result.hp_gained >= 1, "should gain at least 1 HP");
        assert!(result.new_hp > result.old_hp, "max HP should increase");
    }

    // --- action_train (oag-mol-jqd) ---

    #[test]
    fn train_deducts_gold() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_fighter_with_abilities("Aldric", 2_100, 500));
        let result = action_train(&mut state, "Aldric").unwrap();
        assert_eq!(result.new_level, 2);
        assert_eq!(result.cost_gp, 200); // level 2 * 100
        assert_eq!(result.gold_remaining, 300); // 500 - 200
        assert_eq!(state.party.find_member("Aldric").unwrap().gold_gp, 300);
    }

    #[test]
    fn train_insufficient_gold() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_fighter_with_abilities("Aldric", 2_100, 50));
        let result = action_train(&mut state, "Aldric");
        assert!(result.is_err());
        // Gold and level unchanged
        assert_eq!(state.party.find_member("Aldric").unwrap().gold_gp, 50);
        assert_eq!(state.party.find_member("Aldric").unwrap().level, 1);
    }

    #[test]
    fn train_insufficient_xp() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_fighter_with_abilities("Aldric", 100, 500));
        let result = action_train(&mut state, "Aldric");
        assert!(result.is_err());
    }

    #[test]
    fn train_dead_character() {
        let mut state = GameState::new();
        let mut c = make_fighter_with_abilities("Aldric", 2_100, 500);
        c.hp = 0;
        state.party.add_member(c);
        let result = action_train(&mut state, "Aldric");
        assert!(result.is_err());
    }

    #[test]
    fn train_in_combat_blocked() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_fighter_with_abilities("Aldric", 2_100, 500));
        state.mode = GameMode::Combat;
        let result = action_train(&mut state, "Aldric");
        assert!(result.is_err());
    }

    #[test]
    fn train_no_character() {
        let mut state = GameState::new();
        let result = action_train(&mut state, "Nobody");
        assert!(result.is_err());
    }

    /// Verify level_up (no cost) vs train (gold cost) semantic distinction.
    #[test]
    fn level_up_vs_train_cost_distinction() {
        // Both should produce the same level outcome but differ on gold
        let base_xp = 2_100u64;
        let base_gold = 500u32;

        // level_up path (API fiat)
        let mut state_lu = GameState::new();
        state_lu
            .party
            .add_member(make_fighter_with_abilities("Aldric", base_xp, base_gold));
        let lu_result = action_level_up(&mut state_lu, "Aldric").unwrap();

        // train path (CLI gold cost)
        let mut state_tr = GameState::new();
        state_tr
            .party
            .add_member(make_fighter_with_abilities("Aldric", base_xp, base_gold));
        let tr_result = action_train(&mut state_tr, "Aldric").unwrap();

        // Both reach level 2
        assert_eq!(lu_result.new_level, 2);
        assert_eq!(tr_result.new_level, 2);

        // level_up costs nothing
        assert_eq!(lu_result.cost_gp, 0);
        assert_eq!(lu_result.gold_remaining, base_gold);

        // train costs 200gp
        assert_eq!(tr_result.cost_gp, 200);
        assert_eq!(tr_result.gold_remaining, base_gold - 200);

        // Both characters are at level 2
        assert_eq!(state_lu.party.find_member("Aldric").unwrap().level, 2);
        assert_eq!(state_tr.party.find_member("Aldric").unwrap().level, 2);

        // Gold diverges
        assert_eq!(state_lu.party.find_member("Aldric").unwrap().gold_gp, 500);
        assert_eq!(state_tr.party.find_member("Aldric").unwrap().gold_gp, 300);
    }

    // --- action_list_notes / action_delete_note (oag-mol-jqd) ---

    #[test]
    fn list_notes_empty() {
        let state = GameState::new();
        let result = action_list_notes(&state).unwrap();
        assert!(result.notes.is_empty());
    }

    #[test]
    fn list_notes_returns_indexed() {
        let mut state = GameState::new();
        state.notes.push("Alpha".to_string());
        state.notes.push("Beta".to_string());
        let result = action_list_notes(&state).unwrap();
        assert_eq!(result.notes.len(), 2);
        assert_eq!(result.notes[0].index, 1);
        assert_eq!(result.notes[0].text, "Alpha");
        assert_eq!(result.notes[1].index, 2);
        assert_eq!(result.notes[1].text, "Beta");
    }

    #[test]
    fn delete_note_basic() {
        let mut state = GameState::new();
        state.notes.push("Keep".to_string());
        state.notes.push("Remove".to_string());
        state.notes.push("Also keep".to_string());
        let result = action_delete_note(&mut state, 2).unwrap();
        assert_eq!(result.index, 2);
        assert_eq!(result.deleted, "Remove");
        assert_eq!(state.notes.len(), 2);
        assert_eq!(state.notes[0], "Keep");
        assert_eq!(state.notes[1], "Also keep");
    }

    #[test]
    fn delete_note_out_of_range() {
        let mut state = GameState::new();
        state.notes.push("Only note".to_string());
        let result = action_delete_note(&mut state, 5);
        assert!(result.is_err());
        assert_eq!(state.notes.len(), 1);
    }

    #[test]
    fn delete_note_zero_rejected() {
        let mut state = GameState::new();
        state.notes.push("Note".to_string());
        let result = action_delete_note(&mut state, 0);
        assert!(result.is_err());
    }

    #[test]
    fn delete_note_empty_list() {
        let mut state = GameState::new();
        let result = action_delete_note(&mut state, 1);
        assert!(result.is_err());
    }

    // --- action_ruling (oag-mol-jqd) ---

    #[test]
    fn ruling_adds_prefixed_note() {
        let mut state = GameState::new();
        let result = action_ruling(&mut state, "The bridge can hold 3 people").unwrap();
        assert_eq!(result.text, "The bridge can hold 3 people");
        assert_eq!(result.note, "[RULING] The bridge can hold 3 people");
        assert_eq!(state.notes.len(), 1);
        assert!(state.notes[0].starts_with("[RULING]"));
    }

    // --- action_thief_skill_check dead character guard (oag-j3yu1) ---

    #[test]
    fn thief_skill_check_dead_character_rejected() {
        let mut state = GameState::new();
        let mut c = Character::new("Shadow", Class::Thief);
        c.hp = -1;
        c.max_hp = 6;
        state.party.add_member(c);
        let result = action_thief_skill_check(&state, "Shadow", "open_locks");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("dead"),
            "error should mention dead: {}",
            err_msg
        );
    }
}

pub fn action_thief_skill_check(
    state: &GameState,
    char_name: &str,
    skill_name: &str,
) -> Result<ThiefSkillCheckResult, EngineError> {
    let character = state
        .party
        .find_member(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;

    if !character.is_alive() {
        return Err(EngineError::InvalidInput(format!(
            "{} is dead and cannot perform actions.",
            character.name
        )));
    }

    if !thief::has_thief_skills(character.class) {
        return Err(EngineError::InvalidInput(format!(
            "{} ({}) does not have thief skills.",
            character.name,
            character.class.name()
        )));
    }

    let skill = match skill_name
        .to_lowercase()
        .replace([' ', '_', '-'], "")
        .as_str()
    {
        "climbwalls" | "climb" => thief::ThiefSkill::ClimbWalls,
        "findtraps" | "traps" => thief::ThiefSkill::FindTraps,
        "hearnoise" | "hear" | "listen" => thief::ThiefSkill::HearNoise,
        "hideshadows" | "hide" | "hideinshadows" => thief::ThiefSkill::HideShadows,
        "movesilently" | "sneak" | "stealth" => thief::ThiefSkill::MoveSilently,
        "openlocks" | "pick" | "lockpick" => thief::ThiefSkill::OpenLocks,
        "pickpockets" | "pickpocket" | "steal" => thief::ThiefSkill::PickPockets,
        "readlanguages" | "read" => thief::ThiefSkill::ReadLanguages,
        _ => {
            return Err(EngineError::InvalidInput(format!(
                "unknown thief skill '{}'.",
                skill_name
            )))
        }
    };

    let target = thief::skill_chance(skill, character.level);
    let roll: u32 = if skill.is_d6() {
        rand::Rng::gen_range(&mut rand::thread_rng(), 1..=6)
    } else {
        rand::Rng::gen_range(&mut rand::thread_rng(), 1..=100)
    };
    let result = thief::check_skill(skill, character.level, roll);
    let die_type = if skill.is_d6() { "d6" } else { "d%" };

    Ok(ThiefSkillCheckResult {
        message: format!(
            "{} attempts {} (level {}): target {}, rolled {} ({}) — {}.",
            character.name,
            skill.name(),
            character.level,
            target,
            roll,
            die_type,
            if result.success { "SUCCESS" } else { "FAILURE" }
        ),
        character: character.name.clone(),
        skill: skill.name().to_string(),
        level: character.level,
        target,
        roll,
        die_type: die_type.to_string(),
        success: result.success,
    })
}
