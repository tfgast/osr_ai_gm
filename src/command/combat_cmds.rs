use super::{Command, CommandResult};
use crate::engine::combat::{self, SpawnEncounterParams};
use crate::engine::combat::results::RetainerLoyaltyOutcome;
use crate::persist::GameState;
use crate::rules::attack::HitDice;

pub struct StartCombatCommand;
impl Command for StartCombatCommand {
    fn name(&self) -> &str {
        "start_combat"
    }
    fn help(&self) -> &str {
        "Start combat (start_combat <name> <count> <hd> <ac> <hp> <damage> <morale> <distance> [xp])"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 8 {
            return CommandResult::error(
                "usage: start_combat <name> <count> <hd> <ac> <hp> <damage> <morale> <distance> [xp]\n  \
                 example: start_combat goblin 3 1 6 3 1d6 7 60\n  \
                 XP is auto-looked up from monster database if available, or specify manually."
            );
        }
        let name = args[0];
        let count: u32 = match args[1].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("count must be a positive integer"),
        };
        let hd: HitDice = match args[2].parse() {
            Ok(h) => h,
            Err(e) => return CommandResult::error(format!("invalid hit dice '{}': {}", args[2], e)),
        };
        let ac: i32 = match args[3].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("ac must be an integer"),
        };
        let hp: i32 = match args[4].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("hp must be a positive integer"),
        };
        let damage = args[5];
        let morale: u32 = match args[6].parse() {
            Ok(n) if (2..=12).contains(&n) => n,
            _ => return CommandResult::error("morale must be 2-12"),
        };
        let distance: u32 = match args[7].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("distance must be a non-negative integer"),
        };

        let xp_value = if args.len() >= 9 {
            match args[8].parse() {
                Ok(n) => Some(n),
                _ => return CommandResult::error("xp must be a non-negative integer"),
            }
        } else {
            None
        };

        match combat::action_spawn_encounter(
            state,
            &SpawnEncounterParams {
                name, count, hit_dice: &hd, ac, hp, damage, morale, distance, xp_value,
            },
        ) {
            Ok(result) => {
                let mut out = format!(
                    "Combat started! {} {}(s) at {}' distance.\n",
                    result.count, result.encounter_name, result.distance
                );
                if result.xp_per_monster > 0 {
                    out.push_str(&format!("XP per monster: {}\n", result.xp_per_monster));
                } else {
                    out.push_str(
                        "Warning: monster XP is 0. Specify XP as 9th argument or use a known monster name.\n",
                    );
                }
                out.push('\n');
                out.push_str(&result.status);
                out.push_str("\nUse 'initiative' to roll for the first round.");
                CommandResult::ok(out)
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct AddMonsterCommand;
impl Command for AddMonsterCommand {
    fn name(&self) -> &str {
        "add_monster"
    }
    fn help(&self) -> &str {
        "Add monsters to active combat (add_monster <name> <count> <hd> <ac> <hp> <damage> <morale> [xp])"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 7 {
            return CommandResult::error(
                "usage: add_monster <name> <count> <hd> <ac> <hp> <damage> <morale> [xp]\n  \
                 example: add_monster orc 2 1 6 4 1d6 8\n  \
                 Adds monsters to an existing combat encounter.\n  \
                 XP is auto-looked up from monster database if available, or specify manually."
            );
        }
        let name = args[0];
        let count: u32 = match args[1].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("count must be a positive integer"),
        };
        let hd: HitDice = match args[2].parse() {
            Ok(h) => h,
            Err(e) => return CommandResult::error(format!("invalid hit dice '{}': {}", args[2], e)),
        };
        let ac: i32 = match args[3].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("ac must be an integer"),
        };
        let hp: i32 = match args[4].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("hp must be a positive integer"),
        };
        let damage = args[5];
        let morale: u32 = match args[6].parse() {
            Ok(n) if (2..=12).contains(&n) => n,
            _ => return CommandResult::error("morale must be 2-12"),
        };

        let xp_value = if args.len() >= 8 {
            match args[7].parse() {
                Ok(n) => Some(n),
                _ => return CommandResult::error("xp must be a non-negative integer"),
            }
        } else {
            None
        };

        match combat::action_add_monster(
            state,
            &SpawnEncounterParams {
                name, count, hit_dice: &hd, ac, hp, damage, morale, distance: 0, xp_value,
            },
        ) {
            Ok(result) => {
                let mut out = format!(
                    "{} {}(s) added to combat. Total monsters: {}\n",
                    result.count, result.monster_name, result.total_monsters
                );
                if result.xp_per_monster > 0 {
                    out.push_str(&format!("XP per monster: {}\n", result.xp_per_monster));
                }
                out.push('\n');
                out.push_str(&result.status);
                CommandResult::ok(out)
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct InitiativeCommand;
impl Command for InitiativeCommand {
    fn name(&self) -> &str {
        "initiative"
    }
    fn help(&self) -> &str {
        "Roll group initiative for the round"
    }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match combat::action_roll_initiative(state) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct NextPhaseCommand;
impl Command for NextPhaseCommand {
    fn name(&self) -> &str {
        "next_phase"
    }
    fn help(&self) -> &str {
        "Advance to the next combat phase"
    }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match combat::action_next_phase(state) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct AttackCommand;
impl Command for AttackCommand {
    fn name(&self) -> &str {
        "attack"
    }
    fn help(&self) -> &str {
        "Melee/missile attack (attack <character> <monster_idx> [weapon]). Auto-kills helpless targets."
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: attack <character_name> <monster_index> [weapon_name]\n  \
                 Default weapon: sword (1d8 melee). Use weapon name for others.\n  \
                 Helpless targets (sleeping, paralyzed, etc.) are auto-killed.",
            );
        }
        let char_name = args[0];
        let monster_idx: usize = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_index must be a number"),
        };
        let weapon_name = if args.len() >= 3 {
            args[2..].join(" ")
        } else {
            "sword".to_string()
        };

        match combat::action_attack(state, char_name, monster_idx, &weapon_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct MonsterAttackCommand;
impl Command for MonsterAttackCommand {
    fn name(&self) -> &str {
        "monster_attack"
    }
    fn help(&self) -> &str {
        "Monster attacks character (monster_attack <monster_idx> <character>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: monster_attack <monster_index> <character_name>");
        }
        let monster_idx: usize = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_index must be a number"),
        };

        match combat::action_monster_attack(state, monster_idx, args[1]) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct MoraleCommand;
impl Command for MoraleCommand {
    fn name(&self) -> &str {
        "morale"
    }
    fn help(&self) -> &str {
        "Check morale for a monster type (morale <monster_name>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        match combat::action_morale(state, args.first().copied()) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct TurnUndeadCommand;
impl Command for TurnUndeadCommand {
    fn name(&self) -> &str {
        "turn_undead"
    }
    fn help(&self) -> &str {
        "Cleric turns undead (turn_undead <character> <monster_idx>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: turn_undead <character_name> <monster_index>");
        }
        let char_name = args[0];
        let monster_idx: usize = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_index must be a number"),
        };
        match combat::action_turn_undead(state, char_name, monster_idx) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

/// Resolve a character name from args, auto-selecting if only one alive party member.
fn resolve_character_arg(args: &[&str], state: &GameState) -> Result<String, String> {
    if !args.is_empty() {
        return Ok(args[0].to_string());
    }
    let alive: Vec<&str> = state.party.members.iter()
        .filter(|c| c.is_alive())
        .map(|c| c.name.as_str())
        .collect();
    match alive.len() {
        0 => Err("no alive party members.".to_string()),
        1 => Ok(alive[0].to_string()),
        _ => Err(format!(
            "multiple alive party members — specify which character: {}",
            alive.join(", ")
        )),
    }
}

pub struct RetreatCommand;
impl Command for RetreatCommand {
    fn name(&self) -> &str {
        "retreat"
    }
    fn help(&self) -> &str {
        "Retreat from combat — full speed, enemies get free attack at +2"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let char_name = match resolve_character_arg(args, state) {
            Ok(name) => name,
            Err(e) => return CommandResult::error(e),
        };
        match combat::action_retreat(state, &char_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct WithdrawalCommand;
impl Command for WithdrawalCommand {
    fn name(&self) -> &str {
        "withdrawal"
    }
    fn help(&self) -> &str {
        "Fighting withdrawal — half speed, no free attacks"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let char_name = match resolve_character_arg(args, state) {
            Ok(name) => name,
            Err(e) => return CommandResult::error(e),
        };
        match combat::action_fighting_withdrawal(state, &char_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct CloseCommand;
impl Command for CloseCommand {
    fn name(&self) -> &str {
        "close"
    }
    fn help(&self) -> &str {
        "Close distance to monsters (close <character> [feet])"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error(
                "usage: close <character_name> [feet]\n  \
                 close Grond       — close to melee range\n  \
                 close Grond 30    — close 30 feet",
            );
        }
        let char_name = args[0];
        let feet: Option<u32> = if args.len() >= 2 {
            match args[1].parse() {
                Ok(n) if n > 0 => Some(n),
                _ => return CommandResult::error("feet must be a positive integer"),
            }
        } else {
            None
        };

        match combat::action_close(state, char_name, feet) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct DeclareSpellCommand;
impl Command for DeclareSpellCommand {
    fn name(&self) -> &str {
        "declare_spell"
    }
    fn help(&self) -> &str {
        "Declare spell casting (declare_spell <character> <spell_name>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: declare_spell <character_name> <spell_name>");
        }
        let char_name = args[0];
        let spell_name = args[1..].join(" ");
        match combat::action_declare_spell(state, char_name, &spell_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct CastSpellCommand;
impl Command for CastSpellCommand {
    fn name(&self) -> &str {
        "cast_spell"
    }
    fn help(&self) -> &str {
        "Resolve a declared spell during the magic phase (cast_spell <character>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: cast_spell <character_name>");
        }
        let char_name = args[0];
        match combat::action_cast_spell(state, char_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct CombatStatusCommand;
impl Command for CombatStatusCommand {
    fn name(&self) -> &str {
        "combat_status"
    }
    fn help(&self) -> &str {
        "Show current combat status"
    }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match combat::action_combat_status(state) {
            Ok(result) => CommandResult::ok(result.status),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct EndCombatCommand;
impl Command for EndCombatCommand {
    fn name(&self) -> &str {
        "end_combat"
    }
    fn help(&self) -> &str {
        "End the current combat encounter"
    }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match combat::action_end_combat(state) {
            Ok(result) => {
                let mut out = format!(
                    "Combat ended after {} rounds.\n{} of {} monsters defeated.",
                    result.rounds, result.monsters_defeated, result.total_monsters
                );
                if result.total_xp > 0 {
                    out.push_str(&format!(
                        "\nTotal XP from defeated monsters: {}",
                        result.total_xp
                    ));
                    if !result.xp_awards.is_empty() {
                        out.push_str(&format!(
                            "\nXP per survivor: {} (split among {})",
                            result.xp_per_survivor,
                            result.xp_awards.len()
                        ));
                        for award in &result.xp_awards {
                            let train_note = if award.ready_to_train {
                                " ★ READY TO TRAIN"
                            } else {
                                ""
                            };
                            if award.modifier_pct != 0 {
                                out.push_str(&format!(
                                    "\n  {} — {} XP ({:+}% prime req → {} adjusted, total: {}){}",
                                    award.character,
                                    award.base_xp,
                                    award.modifier_pct,
                                    award.adjusted_xp,
                                    award.total_xp,
                                    train_note
                                ));
                            } else {
                                out.push_str(&format!(
                                    "\n  {} — {} XP (total: {}){}",
                                    award.character,
                                    award.adjusted_xp,
                                    award.total_xp,
                                    train_note
                                ));
                            }
                        }
                    }
                    if let Some(xp_each) = result.retainer_xp_each {
                        if !result.retainer_xp_recipients.is_empty() {
                            out.push_str(&format!(
                                "\nRetainer XP (half share): {} each for {}",
                                xp_each,
                                result.retainer_xp_recipients.join(", ")
                            ));
                        }
                    }
                }
                if !result.retainer_loyalty_checks.is_empty() {
                    out.push_str("\n\nRetainer loyalty checks:");
                    for check in &result.retainer_loyalty_checks {
                        let desc = match check.outcome {
                            RetainerLoyaltyOutcome::Loyal => "LOYAL",
                            RetainerLoyaltyOutcome::Wavering => "WAVERING",
                            RetainerLoyaltyOutcome::Disloyal => "DISLOYAL — may leave!",
                        };
                        out.push_str(&format!(
                            "\n  {} (loyalty {}): {}",
                            check.name, check.loyalty, desc
                        ));
                    }
                }
                if result.party_casualties > 0 {
                    out.push_str(&format!("\nParty casualties: {}", result.party_casualties));
                }
                CommandResult::ok(out)
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct CombatLogCommand;
impl Command for CombatLogCommand {
    fn name(&self) -> &str {
        "combat_log"
    }
    fn help(&self) -> &str {
        "Show combat log for current encounter"
    }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match combat::action_query_combat_log(state) {
            Ok(result) => {
                if result.log.is_empty() {
                    CommandResult::ok("No combat events logged yet.")
                } else {
                    CommandResult::ok(result.message)
                }
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct SetHelplessCommand;
impl Command for SetHelplessCommand {
    fn name(&self) -> &str {
        "set_helpless"
    }
    fn help(&self) -> &str {
        "Mark monster as helpless/sleeping (set_helpless <monster_idx> [false])"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error(
                "usage: set_helpless <monster_index> [false]\n  \
                 Mark a monster as helpless (sleeping, paralyzed, held, etc.).\n  \
                 Helpless creatures can be auto-killed with any attack.\n  \
                 Use 'set_helpless <idx> false' to remove the helpless condition.",
            );
        }
        let monster_idx: usize = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_index must be a number"),
        };
        let helpless = args.get(1).is_none_or(|s| *s != "false");

        match combat::action_set_helpless(state, monster_idx, helpless) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct KillCommand;
impl Command for KillCommand {
    fn name(&self) -> &str {
        "kill"
    }
    fn help(&self) -> &str {
        "Auto-kill a helpless monster (kill <character> <monster_idx>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: kill <character_name> <monster_index>\n  \
                 Instantly kill a helpless monster (sleeping, paralyzed, held, etc.).\n  \
                 The monster must be marked as helpless first with 'set_helpless'.",
            );
        }
        let char_name = args[0];
        let monster_idx: usize = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_index must be a number"),
        };
        match combat::action_coup_de_grace(state, char_name, monster_idx) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}
