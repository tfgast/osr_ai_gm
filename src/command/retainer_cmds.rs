use super::{Command, CommandResult};
use crate::engine::retainers::{self as retainer_actions, results::HireRetainerMode};
use crate::persist::GameState;
use crate::rules::class::{ClassId, normalize_class_name};

#[cfg(test)]
use crate::engine::retainer::Retainer;

/// Generate a unique retainer name by auto-numbering if duplicates exist.
/// E.g., if "Torchbearer" exists, returns "Torchbearer 2".
/// If "Torchbearer" and "Torchbearer 2" exist, returns "Torchbearer 3".
#[cfg(test)]
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
        let class = match normalize_class_name(args[1]) {
            Some(c) => c,
            None => return CommandResult::error(format!(
                "unknown class '{}'. Use 'classes' to list available classes.", args[1]
            )),
        };
        let employer = args.get(2).copied();
        match retainer_actions::action_hire_retainer(
            state,
            ret_name,
            ClassId::new(class),
            employer,
            1,
            HireRetainerMode::RecruitToParty,
        ) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct RetainersCommand;
impl Command for RetainersCommand {
    fn name(&self) -> &str { "retainers" }
    fn help(&self) -> &str { "List current retainers" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match retainer_actions::action_list_retainers(state) {
            Ok(result) => {
                if result.retainers.is_empty() {
                    return CommandResult::ok("No retainers. Use 'hire' to recruit one.");
                }
                let mut out = format!("Retainers ({}):\n", result.retainers.len());
                for r in &result.retainers {
                    let status = if r.alive {
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
            Err(e) => CommandResult::error(e.to_string()),
        }
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
        match retainer_actions::action_dismiss_retainer(state, name) {
            Ok(result) => {
                CommandResult::ok(format!("{} ({}) dismissed from service.", result.name, result.class))
            }
            Err(e) => {
                if e.to_string().starts_with("no retainer named") {
                    CommandResult::error(format!("{} Use 'retainers' to list.", e))
                } else {
                    CommandResult::error(e.to_string())
                }
            }
        }
    }
}

pub struct RetainerMoraleCommand;
impl Command for RetainerMoraleCommand {
    fn name(&self) -> &str { "retainer_morale" }
    fn help(&self) -> &str { "Check loyalty/morale for retainers (retainer_morale [name])" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        match retainer_actions::action_retainer_morale(state, args.first().copied()) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => {
                if e.to_string().starts_with("no retainer named") {
                    CommandResult::error(format!("{} Use 'retainers' to list.", e))
                } else {
                    CommandResult::error(e.to_string())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    

    fn state_with_party() -> GameState {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", "Fighter");
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
