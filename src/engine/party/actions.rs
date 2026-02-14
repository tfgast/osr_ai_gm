use crate::engine::chargen;
use crate::engine::result::EngineError;
use crate::persist::GameState;
use crate::rules::alignment::Alignment;
use crate::rules::class::{self, Class};
use crate::rules::xp::{check_level_up, xp_for_level};

use super::results::{
    AbilityRequirement, ClassSummary, CreateCharacterResult, EligibleClassesResult,
    ListClassesResult, PartyMemberSummary, QueryPartyResult,
};

fn ability_name(idx: usize) -> &'static str {
    ["STR", "INT", "WIS", "DEX", "CON", "CHA"][idx]
}

pub fn action_create_character(
    state: &mut GameState,
    name: &str,
    class: Class,
    alignment: Alignment,
    provided_abilities: Option<[i32; 6]>,
) -> Result<CreateCharacterResult, EngineError> {
    if let Some(abilities) = provided_abilities {
        for score in abilities {
            if !(3..=18).contains(&score) {
                return Err(EngineError::InvalidInput(format!(
                    "ability scores must be 3-18, got {}.",
                    score
                )));
            }
        }
    }

    let base_abilities = provided_abilities.unwrap_or_else(chargen::roll_abilities);
    let mut abilities = base_abilities;

    let def = class::class_def(class);
    let applied_racial_modifiers = !def.racial_modifiers.is_empty();
    if applied_racial_modifiers {
        class::apply_racial_modifiers(class, &mut abilities);
    }

    if !class::meets_requirements(class, &abilities) {
        let eligible_classes = class::eligible_classes(&abilities);
        return Ok(CreateCharacterResult {
            name: name.to_string(),
            class,
            alignment,
            used_provided_abilities: provided_abilities.is_some(),
            base_abilities,
            abilities,
            applied_racial_modifiers,
            created: false,
            eligible_classes,
            character_sheet: None,
        });
    }

    let character = chargen::create_character(name, class, abilities, alignment);
    let character_sheet = chargen::character_sheet(&character);
    state.party.add_member(character);

    Ok(CreateCharacterResult {
        name: name.to_string(),
        class,
        alignment,
        used_provided_abilities: provided_abilities.is_some(),
        base_abilities,
        abilities,
        applied_racial_modifiers,
        created: true,
        eligible_classes: Vec::new(),
        character_sheet: Some(character_sheet),
    })
}

pub fn action_query_party(state: &GameState) -> Result<QueryPartyResult, EngineError> {
    let members = state
        .party
        .members
        .iter()
        .map(|character| {
            let next_level_xp = if character.is_alive() {
                let next = xp_for_level(character.class, character.level + 1);
                if next == u64::MAX {
                    None
                } else {
                    Some(next)
                }
            } else {
                None
            };

            PartyMemberSummary {
                name: character.name.clone(),
                class: character.class,
                level: character.level,
                hp: character.hp,
                max_hp: character.max_hp,
                ac: character.ac,
                thac0: character.thac0,
                xp: character.xp,
                alive: character.is_alive(),
                alignment: character.alignment,
                movement_rate: character.movement_rate,
                next_level_xp,
                ready_to_train: character.is_alive()
                    && check_level_up(character.class, character.level, character.xp).is_some(),
            }
        })
        .collect();

    Ok(QueryPartyResult {
        members,
        days_without_food: state.party.days_without_food,
    })
}

pub fn action_list_classes() -> Result<ListClassesResult, EngineError> {
    let classes = Class::ALL
        .iter()
        .map(|&class| {
            let def = class::class_def(class);
            let requirements = def
                .requirements
                .iter()
                .map(|&(idx, minimum)| AbilityRequirement {
                    ability: ability_name(idx).to_string(),
                    minimum,
                })
                .collect();
            ClassSummary {
                name: class.name().to_string(),
                hit_die: def.hit_die,
                requirements,
                is_demihuman: def.is_demihuman,
            }
        })
        .collect();

    Ok(ListClassesResult { classes })
}

pub fn action_eligible_classes(abilities: [i32; 6]) -> Result<EligibleClassesResult, EngineError> {
    for score in abilities {
        if !(3..=18).contains(&score) {
            return Err(EngineError::InvalidInput(format!(
                "ability scores must be 3-18, got {}.",
                score
            )));
        }
    }

    let eligible = class::eligible_classes(&abilities);

    Ok(EligibleClassesResult {
        abilities,
        eligible,
    })
}
