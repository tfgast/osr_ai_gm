use rand::Rng;

use crate::engine::result::EngineError;
use crate::engine::retainer::{self, HireReaction, LoyaltyResult, Retainer};
use crate::persist::GameState;
use crate::rules::class::{self, Class};

use super::results::{
    DismissRetainerResult, HireRetainerMode, HireRetainerResult, ListRetainersResult,
    LoyaltyCheckResult, RetainerMoraleCheck, RetainerMoraleResult, RetainerSummary,
};

/// Generate a unique retainer name by auto-numbering if duplicates exist.
fn unique_retainer_name(base_name: &str, existing: &[Retainer]) -> String {
    let base_lower = base_name.to_lowercase();
    let mut max_num: u32 = 0;
    let mut base_exists = false;

    for r in existing {
        let name_lower = r.name.to_lowercase();
        if name_lower == base_lower {
            base_exists = true;
            if max_num == 0 {
                max_num = 1;
            }
        } else if let Some(rest) = name_lower.strip_prefix(&base_lower) {
            let rest = rest.trim();
            if let Ok(n) = rest.parse::<u32>() {
                max_num = max_num.max(n);
            }
        }
    }

    if !base_exists && max_num == 0 {
        base_name.to_string()
    } else {
        format!("{} {}", base_name, max_num + 1)
    }
}

fn loyalty_result_name(result: LoyaltyResult) -> &'static str {
    match result {
        LoyaltyResult::Loyal => "Loyal",
        LoyaltyResult::Wavering => "Wavering",
        LoyaltyResult::Disloyal => "Disloyal",
    }
}

pub fn action_hire_retainer(
    state: &mut GameState,
    retainer_name: &str,
    retainer_class: Class,
    employer_name: Option<&str>,
    retainer_level: u32,
    mode: HireRetainerMode,
) -> Result<HireRetainerResult, EngineError> {
    let employer = if let Some(name) = employer_name {
        state
            .party
            .find_member(name)
            .ok_or_else(|| EngineError::InvalidInput(format!("no party member named '{}'.", name)))?
            .clone()
    } else {
        state.party.members.first().cloned().ok_or_else(|| {
            EngineError::InvalidInput(
                "no party members. Use 'chargen' to create a character first.".to_string(),
            )
        })?
    };

    let cha = employer.abilities.charisma;
    let max = retainer::max_retainers(cha);
    let base_loyalty = retainer::base_loyalty(cha);
    let current = state.retainers.len() as u32;

    if mode == HireRetainerMode::RecruitToParty && current >= max {
        return Err(EngineError::InvalidInput(format!(
            "{} already has max retainers ({}) for CHA {}.",
            employer.name, max, cha
        )));
    }

    let reaction = retainer::hiring_reaction(cha);
    let hired = matches!(reaction, HireReaction::Accepts | HireReaction::Eager);
    let bonus_loyalty = matches!(reaction, HireReaction::Eager);
    let loyalty = if bonus_loyalty {
        base_loyalty + 1
    } else {
        base_loyalty
    };

    let mut final_name = retainer_name.to_string();
    let mut hp = None;
    let mut level = retainer_level;
    let mut wage = retainer::standard_wage(retainer_level);

    let message = match mode {
        HireRetainerMode::AssessOnly => format!(
            "{} attempts to hire {} ({} L{}, {}gp/month). CHA {} (max {} retainers, loyalty {}). Reaction: {} — {}.",
            employer.name,
            retainer_name,
            retainer_class.name(),
            retainer_level,
            wage,
            cha,
            max,
            base_loyalty,
            reaction.name(),
            if hired { "HIRED" } else { "NOT HIRED" }
        ),
        HireRetainerMode::RecruitToParty => {
            let mut out = format!(
                "Hiring {} ({}) — {} (CHA {}) offers employment.\n",
                retainer_name,
                retainer_class.name(),
                employer.name,
                cha
            );
            out.push_str(&format!("Reaction roll: {}\n", reaction.name()));

            match reaction {
                HireReaction::Refused | HireReaction::Reluctant => {
                    out.push_str(&format!("{} will not join the party.", retainer_name));
                }
                HireReaction::Uncertain => {
                    out.push_str(&format!(
                        "{} is uncertain. Try again with a better offer (re-run hire).",
                        retainer_name
                    ));
                }
                HireReaction::Accepts | HireReaction::Eager => {
                    let def = class::class_def(retainer_class);
                    let rolled_hp: i32 = {
                        let mut rng = rand::thread_rng();
                        let roll: i32 = rng.gen_range(1..=def.hit_die as i32);
                        roll.max(1)
                    };
                    final_name = unique_retainer_name(retainer_name, &state.retainers);
                    level = 1;
                    wage = retainer::standard_wage(level);
                    hp = Some(rolled_hp);

                    let retainer = Retainer::new(
                        &final_name,
                        retainer_class,
                        level,
                        rolled_hp,
                        loyalty,
                        wage,
                    );
                    state.retainers.push(retainer);

                    if final_name != retainer_name {
                        out.push_str(&format!(
                            "(Named '{}' to distinguish from existing retainer)\n",
                            final_name
                        ));
                    }

                    out.push_str(&format!(
                        "{} joins as a level 1 {}!\n  HP: {}, Loyalty: {}, Wage: {} gp/month",
                        final_name,
                        retainer_class.name(),
                        rolled_hp,
                        loyalty,
                        wage
                    ));
                    if bonus_loyalty {
                        out.push_str(" (+1 loyalty bonus)");
                    }
                }
            }

            out
        }
    };

    Ok(HireRetainerResult {
        message,
        employer: employer.name,
        retainer: final_name,
        class: retainer_class,
        level,
        reaction: reaction.name().to_string(),
        hired,
        loyalty,
        wage_gp: wage,
        max_retainers: max,
        hp,
        bonus_loyalty,
    })
}

pub fn action_loyalty_check(
    retainer_name: &str,
    loyalty: u32,
) -> Result<LoyaltyCheckResult, EngineError> {
    let result = retainer::loyalty_check(loyalty);
    let result_name = loyalty_result_name(result).to_string();
    let message = format!(
        "{} loyalty check (loyalty {}): {}.",
        retainer_name, loyalty, result_name
    );
    Ok(LoyaltyCheckResult {
        message,
        retainer: retainer_name.to_string(),
        loyalty,
        result: result_name,
    })
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

pub fn action_retainer_morale(
    state: &GameState,
    name: Option<&str>,
) -> Result<RetainerMoraleResult, EngineError> {
    if state.retainers.is_empty() {
        return Err(EngineError::InvalidInput(
            "no retainers to check morale for.".to_string(),
        ));
    }

    let retainers_to_check: Vec<&Retainer> = if let Some(name) = name {
        match state
            .retainers
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(name))
        {
            Some(r) => vec![r],
            None => {
                return Err(EngineError::InvalidInput(format!(
                    "no retainer named '{}'. Use 'retainers' to list.",
                    name
                )));
            }
        }
    } else {
        state.retainers.iter().filter(|r| r.is_alive()).collect()
    };

    if retainers_to_check.is_empty() {
        return Err(EngineError::InvalidInput(
            "no living retainers to check.".to_string(),
        ));
    }

    let checks: Vec<RetainerMoraleCheck> = retainers_to_check
        .iter()
        .map(|r| {
            let result = retainer::loyalty_check(r.loyalty);
            let description = match result {
                LoyaltyResult::Loyal => "LOYAL — stays and fights",
                LoyaltyResult::Wavering => "WAVERING — uncertain, may need encouragement",
                LoyaltyResult::Disloyal => "DISLOYAL — flees or refuses orders",
            };
            RetainerMoraleCheck {
                name: r.name.clone(),
                loyalty: r.loyalty,
                result: loyalty_result_name(result).to_string(),
                description: description.to_string(),
            }
        })
        .collect();

    let mut message = String::from("Retainer morale checks:\n");
    for check in &checks {
        message.push_str(&format!(
            "  {} (loyalty {}): {}\n",
            check.name, check.loyalty, check.description
        ));
    }

    Ok(RetainerMoraleResult { message, checks })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;

    fn state_with_party() -> GameState {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.abilities.charisma = 14;
        state.party.add_member(c);
        state
    }

    #[test]
    fn unique_name_no_conflict() {
        let existing: Vec<Retainer> = vec![];
        assert_eq!(
            unique_retainer_name("Torchbearer", &existing),
            "Torchbearer"
        );
    }

    #[test]
    fn unique_name_multiple_conflicts() {
        let existing = vec![
            Retainer::new("Torchbearer", Class::Fighter, 0, 4, 7, 25),
            Retainer::new("Torchbearer 2", Class::Fighter, 0, 4, 7, 25),
            Retainer::new("Torchbearer 3", Class::Fighter, 0, 4, 7, 25),
        ];
        assert_eq!(
            unique_retainer_name("Torchbearer", &existing),
            "Torchbearer 4"
        );
    }

    #[test]
    fn dismiss_case_insensitive() {
        let mut state = GameState::new();
        state
            .retainers
            .push(Retainer::new("Hrothgar", Class::Fighter, 1, 6, 7, 25));
        let result = action_dismiss_retainer(&mut state, "hrothgar").unwrap();
        assert_eq!(result.name, "Hrothgar");
        assert!(state.retainers.is_empty());
    }

    #[test]
    fn hire_assess_unknown_employer() {
        let mut state = GameState::new();
        let result = action_hire_retainer(
            &mut state,
            "Hrothgar",
            Class::Fighter,
            Some("Nobody"),
            1,
            HireRetainerMode::AssessOnly,
        );
        assert!(matches!(result, Err(EngineError::InvalidInput(_))));
    }

    #[test]
    fn hire_recruit_checks_max() {
        let mut state = state_with_party();
        for i in 0..5 {
            state
                .retainers
                .push(Retainer::new(&format!("R{}", i), Class::Fighter, 1, 4, 7, 25));
        }

        let result = action_hire_retainer(
            &mut state,
            "OneMore",
            Class::Fighter,
            None,
            1,
            HireRetainerMode::RecruitToParty,
        );
        assert!(matches!(result, Err(EngineError::InvalidInput(_))));
    }
}
