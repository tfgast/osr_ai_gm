use crate::engine::result::EngineError;
use crate::engine::xp;
use crate::persist::GameState;
use crate::rules::thief;
use crate::rules::xp::{check_level_up, xp_for_level};

use super::results::{
    AddRationsResult, AwardXpResult, DamageResult, DeleteNoteResult, DismissRetainerResult,
    HealResult, ListNotesResult, ListRetainersResult, NoteEntry, RetainerSummary, RulingResult,
    SetHpResult, SetRationsResult, ThiefSkillCheckResult,
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

    let old_hp = character.hp;
    character.hp = (character.hp + amount).min(character.max_hp);
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
