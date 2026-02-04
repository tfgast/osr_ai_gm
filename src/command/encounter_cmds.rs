use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::engine::encounter_engine;
use crate::rules::ability;

pub struct SurpriseCommand;
impl Command for SurpriseCommand {
    fn name(&self) -> &str { "surprise" }
    fn help(&self) -> &str { "Roll surprise for an encounter (1-2 on d6 = surprised)" }
    fn execute(&self, _args: &[&str], _state: &mut GameState) -> CommandResult {
        let (result, p, m) = encounter_engine::check_surprise();
        CommandResult::ok(format!(
            "Party roll: {}  Monster roll: {}\n{}",
            p, m, result
        ))
    }
}

pub struct ReactionCommand;
impl Command for ReactionCommand {
    fn name(&self) -> &str { "reaction" }
    fn help(&self) -> &str { "Roll NPC reaction (reaction <character_name>). Uses CHA modifier." }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: reaction <character_name>");
        }
        let character = match state.party.find_member(args[0]) {
            Some(c) => c,
            None => return CommandResult::error(format!("no party member named '{}'.", args[0])),
        };
        let cha = character.abilities.charisma;
        let (reaction, raw, modified) = encounter_engine::reaction_roll(cha);
        let cha_mod = ability::cha_reaction_mod(cha);
        CommandResult::ok(format!(
            "{} speaks (CHA {}, modifier {:+}).\n\
             Reaction roll: {} (2d6) {:+} = {}\n{}",
            character.name, cha, cha_mod, raw, cha_mod, modified, reaction
        ))
    }
}

pub struct EvadeCommand;
impl Command for EvadeCommand {
    fn name(&self) -> &str { "evade" }
    fn help(&self) -> &str { "Attempt to evade an encounter (evade <monster_count> <monster_movement>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: evade <monster_count> <monster_movement>");
        }
        let monster_count: u32 = match args[0].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("monster_count must be a positive integer"),
        };
        let monster_movement: u32 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_movement must be a non-negative integer"),
        };
        let party_size = state.party.members.iter().filter(|c| c.is_alive()).count() as u32;
        if party_size == 0 {
            return CommandResult::error("no living party members.");
        }
        let party_movement = state.party.members.iter()
            .filter(|c| c.is_alive())
            .map(|c| c.movement_rate)
            .min()
            .unwrap_or(120);
        let result = encounter_engine::attempt_evasion(
            party_size, party_movement, monster_count, monster_movement,
        );
        CommandResult::ok(format!(
            "Party ({} members, {}' movement) vs {} monsters ({}' movement)\n{}",
            party_size, party_movement, monster_count, monster_movement, result
        ))
    }
}
