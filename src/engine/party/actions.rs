use crate::engine::chargen;
use crate::engine::result::EngineError;
use crate::persist::GameState;
use crate::rules::alignment::AlignmentId;
use crate::rules::class::{self, Class, ClassId};
use crate::rules::encumbrance;
use crate::rules::xp::{check_level_up, xp_for_level};

use super::results::{
    AbilityRequirement, ClassSummary, CreateCharacterResult, EligibleClassesResult,
    ListClassesResult, MemberInventorySummary, PartyMemberSummary, QueryPartyResult,
};

fn ability_name(idx: usize) -> &'static str {
    ["STR", "INT", "WIS", "DEX", "CON", "CHA"][idx]
}

pub fn action_create_character(
    state: &mut GameState,
    name: &str,
    class: impl Into<ClassId>,
    alignment: impl Into<AlignmentId>,
    provided_abilities: Option<[i32; 6]>,
) -> Result<CreateCharacterResult, EngineError> {
    let class_id: ClassId = class.into();
    let alignment_id: AlignmentId = alignment.into();
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(EngineError::InvalidInput(
            "character name must not be empty.".to_string(),
        ));
    }
    if state.party.find_member(trimmed).is_some() {
        return Err(EngineError::InvalidInput(format!(
            "a character named '{}' already exists in the party.",
            trimmed
        )));
    }
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

    let def = class::class_def(&class_id);
    let applied_racial_modifiers = !def.racial_modifiers.is_empty();
    if applied_racial_modifiers {
        class::apply_racial_modifiers(&class_id, &mut abilities);
    }

    if !class::meets_requirements(&class_id, &abilities) {
        let eligible_classes = class::eligible_classes(&abilities);
        return Ok(CreateCharacterResult {
            name: name.to_string(),
            class: class_id,
            alignment: alignment_id,
            used_provided_abilities: provided_abilities.is_some(),
            base_abilities,
            abilities,
            applied_racial_modifiers,
            created: false,
            eligible_classes,
            character_sheet: None,
        });
    }

    let character = chargen::create_character(name, class_id.clone(), abilities, alignment_id.clone());
    let character_sheet = chargen::character_sheet(&character);
    state.party.add_member(character);

    Ok(CreateCharacterResult {
        name: name.to_string(),
        class: class_id,
        alignment: alignment_id,
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
                let next = xp_for_level(&character.class, character.level + 1);
                if next == u64::MAX {
                    None
                } else {
                    Some(next)
                }
            } else {
                None
            };

            let item_weights: Vec<u32> = character
                .inventory
                .iter()
                .map(|item| (item.weight * 10.0) as u32)
                .collect();
            let total_weight_cn =
                encumbrance::total_weight(&item_weights, character.gold_gp);
            let encumbrance_level = encumbrance::encumbrance_level(total_weight_cn);
            let equipped_items: Vec<String> = character
                .inventory
                .iter()
                .filter(|item| item.equipped)
                .map(|item| item.name.clone())
                .collect();

            PartyMemberSummary {
                name: character.name.clone(),
                class: character.class.clone(),
                level: character.level,
                hp: character.hp,
                max_hp: character.max_hp,
                ac: character.ac,
                thac0: character.thac0,
                xp: character.xp,
                alive: character.is_alive(),
                alignment: character.alignment.clone(),
                movement_rate: character.movement_rate,
                next_level_xp,
                ready_to_train: character.is_alive()
                    && check_level_up(&character.class, character.level, character.xp).is_some(),
                inventory: MemberInventorySummary {
                    total_weight_cn,
                    encumbrance_level,
                    item_count: character.inventory.len() as u32,
                    equipped_items,
                },
            }
        })
        .collect();

    Ok(QueryPartyResult {
        members,
        days_without_food: state.party.days_without_food,
        rations: state.party.rations,
        party_gold: state.party.gold,
    })
}

pub fn action_list_classes() -> Result<ListClassesResult, EngineError> {
    let classes = Class::ALL
        .iter()
        .map(|&class| {
            let class_id: ClassId = class.into();
            let def = class::class_def(&class_id);
            let requirements = def
                .requirements
                .iter()
                .map(|&(idx, minimum)| AbilityRequirement {
                    ability: ability_name(idx).to_string(),
                    minimum,
                })
                .collect();
            ClassSummary {
                name: class_id.name().to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Character, Item};
    use crate::rules::encumbrance::EncumbranceLevel;

    fn state_with_equipped_fighter() -> GameState {
        let mut state = GameState::new();
        let mut fighter = Character::new("Grond", Class::Fighter);
        fighter.hp = 8;
        fighter.max_hp = 8;
        fighter.ac = 4;
        fighter.gold_gp = 50;

        let mut sword = Item::new("Sword", 6.0, 10);
        sword.equipped = true;
        let mut plate = Item::new("Plate mail", 50.0, 60);
        plate.equipped = true;
        let torch = Item::new("Torch", 0.0, 0);
        fighter.inventory = vec![sword, plate, torch];

        state.party.add_member(fighter);
        state.party.rations = 7;
        state.party.gold = 200;
        state
    }

    #[test]
    fn query_party_includes_inventory_summary() {
        let state = state_with_equipped_fighter();
        let result = action_query_party(&state).unwrap();

        assert_eq!(result.members.len(), 1);
        let member = &result.members[0];
        let inv = &member.inventory;

        // Sword 60cn + Plate 500cn + Torch 0cn + 50gp = 610cn
        assert_eq!(inv.total_weight_cn, 610);
        assert_eq!(inv.encumbrance_level, EncumbranceLevel::Heavy);
        assert_eq!(inv.item_count, 3);
        assert_eq!(inv.equipped_items, vec!["Sword", "Plate mail"]);
    }

    #[test]
    fn query_party_includes_rations_and_gold() {
        let state = state_with_equipped_fighter();
        let result = action_query_party(&state).unwrap();

        assert_eq!(result.rations, 7);
        assert_eq!(result.party_gold, 200);
    }

    #[test]
    fn query_party_empty_inventory() {
        let mut state = GameState::new();
        let fighter = Character::new("Arden", Class::Fighter);
        state.party.add_member(fighter);

        let result = action_query_party(&state).unwrap();
        let inv = &result.members[0].inventory;

        assert_eq!(inv.total_weight_cn, 0);
        assert_eq!(inv.encumbrance_level, EncumbranceLevel::Unencumbered);
        assert_eq!(inv.item_count, 0);
        assert!(inv.equipped_items.is_empty());
    }

    #[test]
    fn query_party_overloaded_character() {
        let mut state = GameState::new();
        let mut fighter = Character::new("Mule", Class::Fighter);
        fighter.gold_gp = 1700; // Over 1600cn max
        state.party.add_member(fighter);

        let result = action_query_party(&state).unwrap();
        let inv = &result.members[0].inventory;

        assert_eq!(inv.total_weight_cn, 1700);
        assert_eq!(inv.encumbrance_level, EncumbranceLevel::Overloaded);
    }
}
