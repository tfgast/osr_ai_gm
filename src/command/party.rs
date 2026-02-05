use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::engine::chargen;
use crate::rules::alignment::Alignment;
use crate::rules::class::{self, Class};
use crate::rules::xp::{xp_for_level, check_level_up};

pub struct ChargenCommand;
impl Command for ChargenCommand {
    fn name(&self) -> &str { "chargen" }
    fn help(&self) -> &str { "Create a character (chargen <name> <class> [alignment] [--abilities ...]; quote names with spaces)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: chargen <name> <class> [alignment] [--abilities STR INT WIS DEX CON CHA]\n  \
                 alignment: Lawful, Neutral, Chaotic (default: Neutral)\n  \
                 --abilities: provide 6 pre-rolled scores (3-18) in order: STR INT WIS DEX CON CHA\n  \
                 Use 'classes' to list available classes."
            );
        }
        let name = args[0];
        let class = match Class::parse(args[1]) {
            Some(c) => c,
            None => return CommandResult::error(format!(
                "unknown class '{}'. Use 'classes' to list available classes.", args[1]
            )),
        };

        // Find --abilities flag position to separate alignment from ability scores
        let abilities_pos = args.iter().position(|&a| a == "--abilities");
        let positional_end = abilities_pos.unwrap_or(args.len());

        let alignment: Alignment = if positional_end >= 3 {
            match args[2].parse() {
                Ok(a) => a,
                Err(e) => return CommandResult::error(e),
            }
        } else {
            Alignment::default()
        };

        let mut abilities = if let Some(pos) = abilities_pos {
            let scores = &args[pos + 1..];
            if scores.len() != 6 {
                return CommandResult::error(
                    "--abilities requires exactly 6 scores: STR INT WIS DEX CON CHA"
                );
            }
            let mut vals = [0i32; 6];
            for (i, &s) in scores.iter().enumerate() {
                match s.parse::<i32>() {
                    Ok(v) if (3..=18).contains(&v) => vals[i] = v,
                    _ => return CommandResult::error(format!(
                        "ability scores must be 3-18, got '{}'", s
                    )),
                }
            }
            vals
        } else {
            chargen::roll_abilities()
        };
        let mut out = String::new();
        let label = if abilities_pos.is_some() { "Provided abilities" } else { "Rolled abilities" };
        out.push_str(&format!("{}: STR {} INT {} WIS {} DEX {} CON {} CHA {}\n", label,
            abilities[0], abilities[1], abilities[2],
            abilities[3], abilities[4], abilities[5]));

        let def = class::class_def(class);
        if !def.racial_modifiers.is_empty() {
            class::apply_racial_modifiers(class, &mut abilities);
            out.push_str(&format!(
                "After racial modifiers: STR {} INT {} WIS {} DEX {} CON {} CHA {}\n",
                abilities[0], abilities[1], abilities[2],
                abilities[3], abilities[4], abilities[5]));
        }

        if !class::meets_requirements(class, &abilities) {
            let eligible = class::eligible_classes(&abilities);
            let names: Vec<&str> = eligible.iter().map(|c| c.name()).collect();
            out.push_str(&format!(
                "\nAbilities do not meet requirements for {}.\nEligible classes: {}",
                class.name(), names.join(", ")
            ));
            return CommandResult::ok(out);
        }

        let c = chargen::create_character(name, class, abilities, alignment);
        out.push('\n');
        out.push_str(&chargen::character_sheet(&c));
        out.push_str(&format!("\n{} added to party.\n", c.name));
        state.party.add_member(c);
        CommandResult::ok(out)
    }
}

pub struct ClassesCommand;
impl Command for ClassesCommand {
    fn name(&self) -> &str { "classes" }
    fn help(&self) -> &str { "List all character classes" }
    fn execute(&self, _args: &[&str], _state: &mut GameState) -> CommandResult {
        let mut out = String::from("Character Classes (22):\n");
        for &c in &Class::ALL {
            let def = class::class_def(c);
            let reqs: Vec<String> = def.requirements.iter()
                .map(|&(idx, min)| {
                    let name = ["STR", "INT", "WIS", "DEX", "CON", "CHA"][idx];
                    format!("{} {}", name, min)
                })
                .collect();
            let req_str = if reqs.is_empty() { "none".to_string() } else { reqs.join(", ") };
            out.push_str(&format!("  {:14} HD: d{}  Req: {}",
                c.name(), def.hit_die, req_str));
            if def.is_demihuman {
                out.push_str("  [demihuman]");
            }
            out.push('\n');
        }
        CommandResult::ok(out)
    }
}

pub struct EligibleCommand;
impl Command for EligibleCommand {
    fn name(&self) -> &str { "eligible" }
    fn help(&self) -> &str { "Show eligible classes (eligible <STR> <INT> <WIS> <DEX> <CON> <CHA>)" }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.len() < 6 {
            return CommandResult::error(
                "usage: eligible <STR> <INT> <WIS> <DEX> <CON> <CHA>"
            );
        }
        let mut abilities = [0i32; 6];
        for (i, &s) in args[..6].iter().enumerate() {
            match s.parse::<i32>() {
                Ok(v) if (3..=18).contains(&v) => abilities[i] = v,
                _ => return CommandResult::error(format!(
                    "ability scores must be 3-18, got '{}'", s
                )),
            }
        }
        let eligible = class::eligible_classes(&abilities);
        let names: Vec<&str> = eligible.iter().map(|c| c.name()).collect();
        CommandResult::ok(format!(
            "Abilities: STR {} INT {} WIS {} DEX {} CON {} CHA {}\nEligible classes ({}): {}",
            abilities[0], abilities[1], abilities[2],
            abilities[3], abilities[4], abilities[5],
            eligible.len(), names.join(", ")
        ))
    }
}

pub struct PartyCommand;
impl Command for PartyCommand {
    fn name(&self) -> &str { "party" }
    fn help(&self) -> &str { "Show party members" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        if state.party.members.is_empty() {
            return CommandResult::ok("No party members. Use 'chargen' to create characters.");
        }
        let mut out = format!("Party ({} members):\n", state.party.members.len());
        for c in &state.party.members {
            let status = if c.is_alive() {
                let next_level_xp = xp_for_level(c.class, c.level + 1);
                let xp_str = if next_level_xp == u64::MAX {
                    format!("{}", c.xp) // At max level, just show current XP
                } else {
                    format!("{}/{}", c.xp, next_level_xp)
                };
                let mut status_str = format!("HP {}/{}, AC {}, THAC0 {}, XP {}", c.hp, c.max_hp, c.ac, c.thac0, xp_str);
                if check_level_up(c.class, c.level, c.xp).is_some() {
                    status_str.push_str(" [READY TO TRAIN]");
                }
                status_str
            } else {
                "DEAD".to_string()
            };
            out.push_str(&format!("  {} ({} L{}) — {}\n",
                c.name, c.class.name(), c.level, status));
        }
        // Show starvation status if applicable
        if state.party.days_without_food > 0 {
            let penalty = state.party.days_without_food.min(4) as i32;
            out.push_str(&format!(
                "\n[STARVING] {} days without food — -{} penalty to attacks/saves",
                state.party.days_without_food, penalty
            ));
            if state.party.days_without_food >= 3 {
                out.push_str(" — taking HP damage!");
            }
            out.push('\n');
        }
        CommandResult::ok(out)
    }
}
