use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::engine::party;
use crate::rules::alignment::{Alignment, AlignmentId};
use crate::rules::class::normalize_class_name;
use crate::rules::encumbrance::{EncumbranceLevel, MAX_CAPACITY_CN};

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
        let class = match normalize_class_name(args[1]) {
            Some(c) => c,
            None => return CommandResult::error(format!(
                "unknown class '{}'. Use 'classes' to list available classes.", args[1]
            )),
        };

        // Find --abilities flag position to separate alignment from ability scores
        let abilities_pos = args.iter().position(|&a| a == "--abilities");
        let positional_end = abilities_pos.unwrap_or(args.len());

        let alignment: AlignmentId = if positional_end >= 3 {
            match args[2].parse::<Alignment>() {
                Ok(a) => AlignmentId::from_enum(a),
                Err(e) => return CommandResult::error(e),
            }
        } else {
            AlignmentId::default()
        };

        let provided_abilities = if let Some(pos) = abilities_pos {
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
            Some(vals)
        } else {
            None
        };
        match party::action_create_character(state, name, class, alignment, provided_abilities) {
            Ok(result) => {
                let mut out = String::new();
                let label = if result.used_provided_abilities {
                    "Provided abilities"
                } else {
                    "Rolled abilities"
                };
                out.push_str(&format!(
                    "{}: STR {} INT {} WIS {} DEX {} CON {} CHA {}\n",
                    label,
                    result.base_abilities[0],
                    result.base_abilities[1],
                    result.base_abilities[2],
                    result.base_abilities[3],
                    result.base_abilities[4],
                    result.base_abilities[5]
                ));

                if result.applied_racial_modifiers {
                    out.push_str(&format!(
                        "After racial modifiers: STR {} INT {} WIS {} DEX {} CON {} CHA {}\n",
                        result.abilities[0],
                        result.abilities[1],
                        result.abilities[2],
                        result.abilities[3],
                        result.abilities[4],
                        result.abilities[5]
                    ));
                }

                if !result.created {
                    out.push_str(&format!(
                        "\nAbilities do not meet requirements for {}.\nEligible classes: {}",
                        result.class,
                        result.eligible_classes.iter().map(|c| c.name()).collect::<Vec<_>>().join(", ")
                    ));
                    return CommandResult::ok(out);
                }

                out.push('\n');
                if let Some(sheet) = result.character_sheet {
                    out.push_str(&sheet);
                }
                out.push_str(&format!("\n{} added to party.\n", result.name));
                CommandResult::ok(out)
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct ClassesCommand;
impl Command for ClassesCommand {
    fn name(&self) -> &str { "classes" }
    fn help(&self) -> &str { "List all character classes" }
    fn execute(&self, _args: &[&str], _state: &mut GameState) -> CommandResult {
        match party::action_list_classes() {
            Ok(result) => {
                let mut out = format!("Character Classes ({}):\n", result.classes.len());
                for class in result.classes {
                    let reqs: Vec<String> = class
                        .requirements
                        .iter()
                        .map(|requirement| {
                            format!("{} {}", requirement.ability, requirement.minimum)
                        })
                        .collect();
                    let req_str = if reqs.is_empty() {
                        "none".to_string()
                    } else {
                        reqs.join(", ")
                    };
                    out.push_str(&format!(
                        "  {:14} HD: d{}  Req: {}",
                        class.name, class.hit_die, req_str
                    ));
                    if class.is_demihuman {
                        out.push_str("  [demihuman]");
                    }
                    out.push('\n');
                }
                CommandResult::ok(out)
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
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
        match party::action_eligible_classes(abilities) {
            Ok(result) => CommandResult::ok(format!(
                "Abilities: STR {} INT {} WIS {} DEX {} CON {} CHA {}\nEligible classes ({}): {}",
                result.abilities[0],
                result.abilities[1],
                result.abilities[2],
                result.abilities[3],
                result.abilities[4],
                result.abilities[5],
                result.eligible.len(),
                result.eligible.iter().map(|c| c.name()).collect::<Vec<_>>().join(", ")
            )),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Character, Item};

    fn state_with_inventory() -> GameState {
        let mut state = GameState::new();
        let mut fighter = Character::new("Grond", "Fighter");
        fighter.hp = 8;
        fighter.max_hp = 8;
        fighter.ac = 4;
        fighter.thac0 = 19;
        fighter.gold_gp = 50;
        fighter.movement_rate = 120;

        let mut sword = Item::new("Sword", 6.0, 10);
        sword.equipped = true;
        let mut plate = Item::new("Plate mail", 50.0, 60);
        plate.equipped = true;
        fighter.inventory = vec![sword, plate];

        state.party.add_member(fighter);
        state.party.rations = 14;
        state.party.gold = 300;
        state
    }

    #[test]
    fn party_command_shows_inventory_section() {
        let mut state = state_with_inventory();
        let result = PartyCommand.execute(&[], &mut state);
        assert!(result.success);
        assert!(result.output.contains("Inventory:"));
        assert!(result.output.contains("Grond:"));
        assert!(result.output.contains("cn/1600 cn"));
        assert!(result.output.contains("Sword, Plate mail"));
    }

    #[test]
    fn party_command_shows_resources() {
        let mut state = state_with_inventory();
        let result = PartyCommand.execute(&[], &mut state);
        assert!(result.success);
        assert!(result.output.contains("Resources: 300 gp (treasury), 14 rations"));
    }

    #[test]
    fn party_command_shows_overloaded_tag() {
        let mut state = GameState::new();
        let mut fighter = Character::new("Mule", "Fighter");
        fighter.hp = 8;
        fighter.max_hp = 8;
        fighter.gold_gp = 1700;
        fighter.movement_rate = 120;
        state.party.add_member(fighter);

        let result = PartyCommand.execute(&[], &mut state);
        assert!(result.success);
        assert!(result.output.contains("[OVERLOADED]"));
    }

    #[test]
    fn party_command_hides_inventory_when_empty() {
        let mut state = GameState::new();
        let mut fighter = Character::new("Arden", "Fighter");
        fighter.hp = 8;
        fighter.max_hp = 8;
        fighter.movement_rate = 120;
        state.party.add_member(fighter);

        let result = PartyCommand.execute(&[], &mut state);
        assert!(result.success);
        assert!(!result.output.contains("Inventory:"));
        assert!(!result.output.contains("Resources:"));
    }

    #[test]
    fn party_command_shows_encumbrance_tag() {
        let mut state = GameState::new();
        let mut fighter = Character::new("Heavy", "Fighter");
        fighter.hp = 8;
        fighter.max_hp = 8;
        fighter.movement_rate = 120;
        // 700cn worth of items = Heavy encumbrance
        fighter.inventory = vec![Item::new("Big Sack", 70.0, 0)];
        state.party.add_member(fighter);

        let result = PartyCommand.execute(&[], &mut state);
        assert!(result.success);
        assert!(result.output.contains("[HEAVILY ENCUMBERED]"));
    }
}

pub struct PartyCommand;
impl Command for PartyCommand {
    fn name(&self) -> &str { "party" }
    fn help(&self) -> &str { "Show party members and inventory summary" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match party::action_query_party(state) {
            Ok(result) => {
                if result.members.is_empty() {
                    return CommandResult::ok("No party members. Use 'chargen' to create characters.");
                }

                let mut out = format!("Party ({} members):\n", result.members.len());
                for member in &result.members {
                    let status = if member.alive {
                        let xp_str = match member.next_level_xp {
                            Some(next_level_xp) => format!("{}/{}", member.xp, next_level_xp),
                            None => member.xp.to_string(),
                        };
                        let mut status_str = format!(
                            "HP {}/{}, AC {}, THAC0 {}, XP {}",
                            member.hp, member.max_hp, member.ac, member.thac0, xp_str
                        );
                        if member.ready_to_train {
                            status_str.push_str(" [READY TO TRAIN]");
                        }
                        status_str
                    } else {
                        "DEAD".to_string()
                    };

                    out.push_str(&format!(
                        "  {} ({} L{}) — {}\n",
                        member.name, member.class, member.level, status
                    ));
                }

                // Active effects per member (parity with GM API QueryParty)
                let has_effects = result.members.iter().any(|m| {
                    state.party.find_member(&m.name)
                        .map(|c| !c.effects.is_empty())
                        .unwrap_or(false)
                });
                if has_effects {
                    out.push_str("\nActive Effects:\n");
                    for m in &result.members {
                        if let Some(c) = state.party.find_member(&m.name) {
                            if !c.effects.is_empty() {
                                out.push_str(&format!("  {} —\n", m.name));
                                for e in &c.effects {
                                    out.push_str(&format!("    {}\n", e.detail_lines().replace('\n', "\n    ")));
                                }
                            }
                        }
                    }
                }

                if result.days_without_food > 0 {
                    let penalty = result.days_without_food.min(4) as i32;
                    out.push_str(&format!(
                        "\n[STARVING] {} days without food — -{} penalty to attacks/saves",
                        result.days_without_food, penalty
                    ));
                    if result.days_without_food >= 3 {
                        out.push_str(" — taking HP damage!");
                    }
                    out.push('\n');
                }

                // Inventory summary section
                let alive_members: Vec<_> = result.members.iter().filter(|m| m.alive).collect();
                let has_inventory = alive_members
                    .iter()
                    .any(|m| m.inventory.item_count > 0 || m.inventory.total_weight_cn > 0);

                if has_inventory {
                    out.push_str("\nInventory:\n");
                    for member in &alive_members {
                        let inv = &member.inventory;
                        if inv.item_count == 0 && inv.total_weight_cn == 0 {
                            continue;
                        }
                        let enc_tag = match inv.encumbrance_level {
                            EncumbranceLevel::Unencumbered => String::new(),
                            EncumbranceLevel::Overloaded => " [OVERLOADED]".to_string(),
                            level => format!(" [{}]", level.name().to_uppercase()),
                        };
                        let equipped_str = if inv.equipped_items.is_empty() {
                            String::new()
                        } else {
                            format!(" — {}", inv.equipped_items.join(", "))
                        };
                        out.push_str(&format!(
                            "  {}: {} cn/{} cn ({} items){}{}\n",
                            member.name,
                            inv.total_weight_cn,
                            MAX_CAPACITY_CN,
                            inv.item_count,
                            enc_tag,
                            equipped_str,
                        ));
                    }
                }

                // Party resources
                if result.rations > 0 || result.party_gold > 0 {
                    out.push_str(&format!(
                        "\nResources: {} gp (treasury), {} rations\n",
                        result.party_gold, result.rations
                    ));
                }

                CommandResult::ok(out)
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}
