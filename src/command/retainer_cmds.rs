use super::{Command, CommandResult};
use crate::engine::retainer::{
    self, HireReaction, LoyaltyResult, Retainer,
};
use crate::persist::GameState;
use crate::rules::class::{self, Class};

/// Generate a unique retainer name by auto-numbering if duplicates exist.
/// E.g., if "Torchbearer" exists, returns "Torchbearer 2".
/// If "Torchbearer" and "Torchbearer 2" exist, returns "Torchbearer 3".
fn unique_retainer_name(base_name: &str, existing: &[Retainer]) -> String {
    // Check if base name (or numbered variants) already exist
    let base_lower = base_name.to_lowercase();

    // Find all existing names that match the base pattern
    let mut max_num: u32 = 0;
    let mut base_exists = false;

    for r in existing {
        let name_lower = r.name.to_lowercase();
        if name_lower == base_lower {
            base_exists = true;
            if max_num == 0 {
                max_num = 1; // The base name counts as "1"
            }
        } else if let Some(rest) = name_lower.strip_prefix(&base_lower) {
            // Check for pattern "base N" where N is a number
            let rest = rest.trim();
            if let Ok(n) = rest.parse::<u32>() {
                max_num = max_num.max(n);
            }
        }
    }

    if !base_exists && max_num == 0 {
        // No conflicts, use the base name as-is
        base_name.to_string()
    } else {
        // Generate the next numbered name
        format!("{} {}", base_name, max_num + 1)
    }
}

pub struct HireCommand;
impl Command for HireCommand {
    fn name(&self) -> &str { "hire" }
    fn help(&self) -> &str { "Hire a retainer (hire <name> <class> [employer])" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: hire <name> <class> [employer_name]\n  \
                 Rolls a hiring reaction using employer's CHA.\n  \
                 If no employer given, uses first party member."
            );
        }
        let ret_name = args[0];
        let class = match Class::parse(args[1]) {
            Some(c) => c,
            None => return CommandResult::error(format!(
                "unknown class '{}'. Use 'classes' to list available classes.", args[1]
            )),
        };

        // Find the employing character (for CHA-based rolls)
        let employer = if args.len() >= 3 {
            match state.party.find_member(args[2]) {
                Some(c) => c.clone(),
                None => return CommandResult::error(format!(
                    "no party member named '{}'.", args[2]
                )),
            }
        } else {
            match state.party.members.first() {
                Some(c) => c.clone(),
                None => return CommandResult::error(
                    "no party members. Use 'chargen' to create a character first."
                ),
            }
        };

        let cha = employer.abilities.charisma;

        // Check max retainers
        let max = retainer::max_retainers(cha);
        let current = state.retainers.len() as u32;
        if current >= max {
            return CommandResult::error(format!(
                "{} already has max retainers ({}) for CHA {}.",
                employer.name, max, cha
            ));
        }

        // Roll hiring reaction
        let reaction = retainer::hiring_reaction(cha);
        let mut out = format!(
            "Hiring {} ({}) — {} (CHA {}) offers employment.\n",
            ret_name, class.name(), employer.name, cha
        );
        out.push_str(&format!("Reaction roll: {}\n", reaction.name()));

        match reaction {
            HireReaction::Refused | HireReaction::Reluctant => {
                out.push_str(&format!("{} will not join the party.", ret_name));
                return CommandResult::ok(out);
            }
            HireReaction::Uncertain => {
                out.push_str(&format!(
                    "{} is uncertain. Try again with a better offer (re-run hire).",
                    ret_name
                ));
                return CommandResult::ok(out);
            }
            HireReaction::Accepts | HireReaction::Eager => {}
        }

        // Create the retainer with a unique name
        let def = class::class_def(class);
        let base_loyalty = retainer::base_loyalty(cha);
        let loyalty = if reaction == HireReaction::Eager {
            base_loyalty + 1
        } else {
            base_loyalty
        };

        let hp = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let roll: i32 = rng.gen_range(1..=def.hit_die as i32);
            roll.max(1)
        };

        // Auto-number duplicate names to ensure uniqueness
        let unique_name = unique_retainer_name(ret_name, &state.retainers);

        let wage = retainer::standard_wage(1);
        let r = Retainer::new(&unique_name, class.name(), 1, hp, loyalty, wage);

        // Note if the name was auto-numbered
        if unique_name != ret_name {
            out.push_str(&format!(
                "(Named '{}' to distinguish from existing retainer)\n",
                unique_name
            ));
        }

        out.push_str(&format!(
            "{} joins as a level 1 {}!\n  HP: {}, Loyalty: {}, Wage: {} gp/month",
            r.name, r.class, r.hp, r.loyalty, r.wage_gp
        ));
        if reaction == HireReaction::Eager {
            out.push_str(" (+1 loyalty bonus)");
        }

        state.retainers.push(r);
        CommandResult::ok(out)
    }
}

pub struct RetainersCommand;
impl Command for RetainersCommand {
    fn name(&self) -> &str { "retainers" }
    fn help(&self) -> &str { "List current retainers" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        if state.retainers.is_empty() {
            return CommandResult::ok("No retainers. Use 'hire' to recruit one.");
        }
        let mut out = format!("Retainers ({}):\n", state.retainers.len());
        for r in &state.retainers {
            let status = if r.is_alive() {
                format!("HP {}/{}, Loyalty {}, Wage {} gp/mo",
                    r.hp, r.max_hp, r.loyalty, r.wage_gp)
            } else {
                "DEAD".to_string()
            };
            out.push_str(&format!("  {} ({} L{}) — {}\n",
                r.name, r.class, r.level, status));
        }
        CommandResult::ok(out)
    }
}

pub struct DismissCommand;
impl Command for DismissCommand {
    fn name(&self) -> &str { "dismiss" }
    fn help(&self) -> &str { "Dismiss a retainer (dismiss <name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: dismiss <retainer_name>");
        }
        let name = args[0];
        let idx = state.retainers.iter()
            .position(|r| r.name.eq_ignore_ascii_case(name));
        match idx {
            Some(i) => {
                let removed = state.retainers.remove(i);
                CommandResult::ok(format!("{} ({}) dismissed from service.", removed.name, removed.class))
            }
            None => CommandResult::error(format!(
                "no retainer named '{}'. Use 'retainers' to list.", name
            )),
        }
    }
}

pub struct RetainerMoraleCommand;
impl Command for RetainerMoraleCommand {
    fn name(&self) -> &str { "retainer_morale" }
    fn help(&self) -> &str { "Check loyalty/morale for retainers (retainer_morale [name])" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if state.retainers.is_empty() {
            return CommandResult::error("no retainers to check morale for.");
        }

        let retainers_to_check: Vec<&Retainer> = if let Some(name) = args.first() {
            match state.retainers.iter().find(|r| r.name.eq_ignore_ascii_case(name)) {
                Some(r) => vec![r],
                None => return CommandResult::error(format!(
                    "no retainer named '{}'. Use 'retainers' to list.", name
                )),
            }
        } else {
            state.retainers.iter().filter(|r| r.is_alive()).collect()
        };

        if retainers_to_check.is_empty() {
            return CommandResult::error("no living retainers to check.");
        }

        let mut out = String::from("Retainer morale checks:\n");
        let results: Vec<(String, u32, LoyaltyResult)> = retainers_to_check.iter()
            .map(|r| {
                let result = retainer::loyalty_check(r.loyalty);
                (r.name.clone(), r.loyalty, result)
            })
            .collect();

        for (name, loyalty, result) in &results {
            let desc = match result {
                LoyaltyResult::Loyal => "LOYAL — stays and fights",
                LoyaltyResult::Wavering => "WAVERING — uncertain, may need encouragement",
                LoyaltyResult::Disloyal => "DISLOYAL — flees or refuses orders",
            };
            out.push_str(&format!("  {} (loyalty {}): {}\n", name, loyalty, desc));
        }

        CommandResult::ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    use crate::rules::class::Class;

    fn state_with_party() -> GameState {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.abilities.charisma = 14; // max retainers = 5, loyalty = 8
        state.party.add_member(c);
        state
    }

    #[test]
    fn hire_requires_args() {
        let mut state = state_with_party();
        let cmd = HireCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("usage"));
    }

    #[test]
    fn hire_unknown_class() {
        let mut state = state_with_party();
        let cmd = HireCommand;
        let result = cmd.execute(&["Bob", "Accountant"], &mut state);
        assert!(result.output.contains("unknown class"));
    }

    #[test]
    fn hire_no_party() {
        let mut state = GameState::new();
        let cmd = HireCommand;
        let result = cmd.execute(&["Bob", "Fighter"], &mut state);
        assert!(result.output.contains("no party members"));
    }

    #[test]
    fn hire_max_retainers_enforced() {
        let mut state = state_with_party();
        // CHA 14 => max 5 retainers
        for i in 0..5 {
            state.retainers.push(Retainer::new(
                &format!("R{}", i), "Fighter", 1, 4, 7, 25,
            ));
        }
        let cmd = HireCommand;
        let result = cmd.execute(&["OneMore", "Fighter"], &mut state);
        assert!(result.output.contains("max retainers"));
    }

    #[test]
    fn retainers_empty() {
        let mut state = GameState::new();
        let cmd = RetainersCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("No retainers"));
    }

    #[test]
    fn retainers_lists_members() {
        let mut state = GameState::new();
        state.retainers.push(Retainer::new("Hrothgar", "Fighter", 1, 6, 7, 25));
        let cmd = RetainersCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Hrothgar"));
        assert!(result.output.contains("Fighter"));
    }

    #[test]
    fn dismiss_removes_retainer() {
        let mut state = GameState::new();
        state.retainers.push(Retainer::new("Hrothgar", "Fighter", 1, 6, 7, 25));
        let cmd = DismissCommand;
        let result = cmd.execute(&["Hrothgar"], &mut state);
        assert!(result.output.contains("dismissed"));
        assert!(state.retainers.is_empty());
    }

    #[test]
    fn dismiss_not_found() {
        let mut state = GameState::new();
        let cmd = DismissCommand;
        let result = cmd.execute(&["Nobody"], &mut state);
        assert!(result.output.contains("no retainer"));
    }

    #[test]
    fn dismiss_case_insensitive() {
        let mut state = GameState::new();
        state.retainers.push(Retainer::new("Hrothgar", "Fighter", 1, 6, 7, 25));
        let cmd = DismissCommand;
        let result = cmd.execute(&["hrothgar"], &mut state);
        assert!(result.output.contains("dismissed"));
        assert!(state.retainers.is_empty());
    }

    #[test]
    fn unique_name_no_conflict() {
        let existing: Vec<Retainer> = vec![];
        assert_eq!(unique_retainer_name("Torchbearer", &existing), "Torchbearer");
    }

    #[test]
    fn unique_name_one_conflict() {
        let existing = vec![
            Retainer::new("Torchbearer", "Fighter", 0, 4, 7, 25),
        ];
        assert_eq!(unique_retainer_name("Torchbearer", &existing), "Torchbearer 2");
    }

    #[test]
    fn unique_name_multiple_conflicts() {
        let existing = vec![
            Retainer::new("Torchbearer", "Fighter", 0, 4, 7, 25),
            Retainer::new("Torchbearer 2", "Fighter", 0, 4, 7, 25),
            Retainer::new("Torchbearer 3", "Fighter", 0, 4, 7, 25),
        ];
        assert_eq!(unique_retainer_name("Torchbearer", &existing), "Torchbearer 4");
    }

    #[test]
    fn unique_name_gap_in_numbering() {
        // If there's a gap (e.g., 1, 3 but no 2), use max+1
        let existing = vec![
            Retainer::new("Torchbearer", "Fighter", 0, 4, 7, 25),
            Retainer::new("Torchbearer 3", "Fighter", 0, 4, 7, 25),
        ];
        assert_eq!(unique_retainer_name("Torchbearer", &existing), "Torchbearer 4");
    }

    #[test]
    fn unique_name_case_insensitive() {
        let existing = vec![
            Retainer::new("TORCHBEARER", "Fighter", 0, 4, 7, 25),
        ];
        assert_eq!(unique_retainer_name("Torchbearer", &existing), "Torchbearer 2");
    }

    #[test]
    fn unique_name_different_names_no_conflict() {
        let existing = vec![
            Retainer::new("Guard", "Fighter", 0, 4, 7, 25),
            Retainer::new("Porter", "Fighter", 0, 4, 7, 25),
        ];
        assert_eq!(unique_retainer_name("Torchbearer", &existing), "Torchbearer");
    }

    #[test]
    fn dismiss_numbered_retainer() {
        let mut state = GameState::new();
        state.retainers.push(Retainer::new("Torchbearer", "Fighter", 0, 4, 7, 25));
        state.retainers.push(Retainer::new("Torchbearer 2", "Fighter", 0, 4, 7, 25));
        state.retainers.push(Retainer::new("Torchbearer 3", "Fighter", 0, 4, 7, 25));

        let cmd = DismissCommand;

        // Dismiss the second one specifically
        let result = cmd.execute(&["Torchbearer 2"], &mut state);
        assert!(result.output.contains("dismissed"));
        assert_eq!(state.retainers.len(), 2);
        assert_eq!(state.retainers[0].name, "Torchbearer");
        assert_eq!(state.retainers[1].name, "Torchbearer 3");
    }

    #[test]
    fn morale_check_numbered_retainer() {
        let mut state = GameState::new();
        state.retainers.push(Retainer::new("Torchbearer", "Fighter", 0, 4, 7, 25));
        state.retainers.push(Retainer::new("Torchbearer 2", "Fighter", 0, 4, 7, 25));

        let cmd = RetainerMoraleCommand;

        // Check morale for a specific numbered retainer
        let result = cmd.execute(&["Torchbearer 2"], &mut state);
        assert!(result.output.contains("Torchbearer 2"));
        // Should only check the one we asked for
        assert!(!result.output.contains("Torchbearer (loyalty"));
    }
}
