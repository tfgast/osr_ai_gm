use crate::command::{Command, CommandResult};
use crate::engine::{combat, encounter_engine};
use crate::model::{CombatState, Monster};
use crate::persist::GameState;
use crate::rules::ability;
use crate::state::game::GameMode;

/// GM command: spawn an encounter with monsters.
pub struct SpawnEncounterCommand;
impl Command for SpawnEncounterCommand {
    fn name(&self) -> &str { "spawn_encounter" }
    fn help(&self) -> &str { "GM: spawn monsters (spawn_encounter <name> <count> <hd> <ac> <hp> <damage> <morale> <distance>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if state.combat.is_some() {
            return CommandResult::error("combat already active. Use 'end_combat' first.");
        }
        if args.len() < 8 {
            return CommandResult::error(
                "usage: spawn_encounter <name> <count> <hd> <ac> <hp> <damage> <morale> <distance>"
            );
        }
        let name = args[0];
        let count: u32 = match args[1].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("count must be a positive integer"),
        };
        let hd = args[2];
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

        let mut monsters = Vec::new();
        for i in 0..count {
            let monster_name = if count > 1 {
                format!("{} {}", name, i + 1)
            } else {
                name.to_string()
            };
            let mut m = Monster::new(&monster_name, hd);
            m.hp = hp;
            m.max_hp = hp;
            m.ac = ac;
            m.damage = damage.to_string();
            m.morale = morale;
            m.attacks = vec!["attack".to_string()];
            monsters.push(m);
        }

        let combat_state = CombatState::new(monsters, distance);
        let status = combat::combat_status(&combat_state, &state.party.members);
        state.combat = Some(combat_state);
        state.pre_combat_mode = Some(state.mode.clone());
        state.mode = GameMode::Combat;

        let mut out = format!("Encounter spawned! {} {}(s) at {}' distance.\n\n",
            count, name, distance);
        out.push_str(&status);
        out.push_str("\nUse 'initiative' to roll for the first round.");
        CommandResult::ok(out)
    }
}

/// GM command: advance one dungeon turn.
pub struct AdvanceTurnCommand;
impl Command for AdvanceTurnCommand {
    fn name(&self) -> &str { "advance_turn" }
    fn help(&self) -> &str { "GM: advance one dungeon exploration turn" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let level = state.dungeon_level;
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let result = crate::engine::exploration::advance_dungeon_turn(time, dungeon, level);
        CommandResult::ok(format!("{}", result))
    }
}

/// GM command: roll NPC reaction.
pub struct RollReactionCommand;
impl Command for RollReactionCommand {
    fn name(&self) -> &str { "roll_reaction" }
    fn help(&self) -> &str { "GM: roll NPC reaction (roll_reaction <character_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: roll_reaction <character_name>");
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

/// GM command: award XP to a character.
pub struct AwardXpCommand;
impl Command for AwardXpCommand {
    fn name(&self) -> &str { "award_xp" }
    fn help(&self) -> &str { "GM: award XP (award_xp <character> <amount>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: award_xp <character_name> <amount>");
        }
        let xp: u64 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("amount must be a non-negative integer"),
        };
        let character = match state.party.find_member_mut(args[0]) {
            Some(c) => c,
            None => return CommandResult::error(format!("no party member named '{}'.", args[0])),
        };
        character.xp += xp;
        CommandResult::ok(format!(
            "{} awarded {} XP (total: {}).",
            character.name, xp, character.xp
        ))
    }
}

/// GM command: record a ruling.
pub struct RulingCommand;
impl Command for RulingCommand {
    fn name(&self) -> &str { "ruling" }
    fn help(&self) -> &str { "GM: record a ruling (ruling <text>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: ruling <text>");
        }
        let text = args.join(" ");
        state.notes.push(format!("[RULING] {}", text));
        CommandResult::ok(format!("Ruling recorded: {}", text))
    }
}

/// List of command names that require GM privileges.
pub const GM_ONLY_COMMANDS: &[&str] = &[
    "spawn_encounter",
    "advance_turn",
    "roll_reaction",
    "award_xp",
    "ruling",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    use crate::rules::class::Class;

    #[test]
    fn spawn_encounter_basic() {
        let mut state = GameState::new();
        state.party.add_member(Character::new("Aldric", Class::Fighter));
        let cmd = SpawnEncounterCommand;
        let result = cmd.execute(
            &["goblin", "3", "1", "6", "3", "1d6", "7", "60"],
            &mut state,
        );
        assert!(!result.quit);
        assert!(result.output.contains("Encounter spawned"));
        assert!(state.combat.is_some());
        assert_eq!(state.mode, GameMode::Combat);
    }

    #[test]
    fn spawn_encounter_too_few_args() {
        let mut state = GameState::new();
        let cmd = SpawnEncounterCommand;
        let result = cmd.execute(&["goblin", "3"], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn award_xp_basic() {
        let mut state = GameState::new();
        state.party.add_member(Character::new("Aldric", Class::Fighter));
        let cmd = AwardXpCommand;
        let result = cmd.execute(&["Aldric", "500"], &mut state);
        assert!(result.output.contains("500 XP"));
        assert_eq!(state.party.find_member("Aldric").unwrap().xp, 500);
    }

    #[test]
    fn award_xp_no_character() {
        let mut state = GameState::new();
        let cmd = AwardXpCommand;
        let result = cmd.execute(&["Nobody", "100"], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn ruling_basic() {
        let mut state = GameState::new();
        let cmd = RulingCommand;
        let result = cmd.execute(
            &["The", "bridge", "can", "hold", "3", "people"],
            &mut state,
        );
        assert!(result.output.contains("Ruling recorded"));
        assert_eq!(state.notes.len(), 1);
        assert!(state.notes[0].contains("bridge"));
    }

    #[test]
    fn ruling_empty() {
        let mut state = GameState::new();
        let cmd = RulingCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn gm_only_commands_list() {
        assert!(GM_ONLY_COMMANDS.contains(&"spawn_encounter"));
        assert!(GM_ONLY_COMMANDS.contains(&"award_xp"));
        assert!(GM_ONLY_COMMANDS.contains(&"ruling"));
    }
}
