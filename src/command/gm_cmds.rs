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

/// GM command: heal a character.
pub struct HealCommand;
impl Command for HealCommand {
    fn name(&self) -> &str { "heal" }
    fn help(&self) -> &str { "GM: heal a character (heal <character> <amount>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: heal <character_name> <amount>");
        }
        let amount: i32 = match args[1].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("amount must be a positive integer"),
        };
        let character = match state.party.find_member_mut(args[0]) {
            Some(c) => c,
            None => return CommandResult::error(format!("no party member named '{}'.", args[0])),
        };
        let old_hp = character.hp;
        character.hp = (character.hp + amount).min(character.max_hp);
        let healed = character.hp - old_hp;
        CommandResult::ok(format!(
            "{} healed {} HP ({} -> {}/{}).",
            character.name, healed, old_hp, character.hp, character.max_hp
        ))
    }
}

/// GM command: damage a character.
pub struct DamageCommand;
impl Command for DamageCommand {
    fn name(&self) -> &str { "damage" }
    fn help(&self) -> &str { "GM: damage a character (damage <character> <amount>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: damage <character_name> <amount>");
        }
        let amount: i32 = match args[1].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("amount must be a positive integer"),
        };
        let character = match state.party.find_member_mut(args[0]) {
            Some(c) => c,
            None => return CommandResult::error(format!("no party member named '{}'.", args[0])),
        };
        let old_hp = character.hp;
        character.hp -= amount;
        let status = if character.is_alive() { "wounded" } else { "DEAD" };
        CommandResult::ok(format!(
            "{} takes {} damage ({} -> {}/{}). Status: {}.",
            character.name, amount, old_hp, character.hp, character.max_hp, status
        ))
    }
}

/// GM command: set a character's HP to an exact value.
pub struct SetHpCommand;
impl Command for SetHpCommand {
    fn name(&self) -> &str { "set_hp" }
    fn help(&self) -> &str { "GM: set HP (set_hp <character> <amount>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: set_hp <character_name> <amount>");
        }
        let amount: i32 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("amount must be an integer"),
        };
        let character = match state.party.find_member_mut(args[0]) {
            Some(c) => c,
            None => return CommandResult::error(format!("no party member named '{}'.", args[0])),
        };
        let old_hp = character.hp;
        character.hp = amount;
        let status = if character.is_alive() { "alive" } else { "DEAD" };
        CommandResult::ok(format!(
            "{} HP set to {} (was {}). Max HP: {}. Status: {}.",
            character.name, character.hp, old_hp, character.max_hp, status
        ))
    }
}

/// GM command: set the party's rations.
pub struct SetRationsCommand;
impl Command for SetRationsCommand {
    fn name(&self) -> &str { "set_rations" }
    fn help(&self) -> &str { "GM: set party rations (set_rations <amount>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: set_rations <amount>");
        }
        let amount: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("amount must be a non-negative integer"),
        };
        let old = state.party.rations;
        state.party.rations = amount;
        CommandResult::ok(format!(
            "Rations set to {} person-days (was {}).",
            amount, old
        ))
    }
}

/// GM command: add rations to the party's supplies.
pub struct AddRationsCommand;
impl Command for AddRationsCommand {
    fn name(&self) -> &str { "add_rations" }
    fn help(&self) -> &str { "GM: add rations (add_rations <amount>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: add_rations <amount>");
        }
        let amount: u32 = match args[0].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("amount must be a positive integer"),
        };
        state.party.rations += amount;
        CommandResult::ok(format!(
            "Added {} rations. Total: {} person-days.",
            amount, state.party.rations
        ))
    }
}

/// List of command names that require GM privileges.
pub const GM_ONLY_COMMANDS: &[&str] = &[
    "spawn_encounter",
    "advance_turn",
    "roll_reaction",
    "award_xp",
    "ruling",
    "heal",
    "damage",
    "set_hp",
    "set_rations",
    "add_rations",
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
        assert!(GM_ONLY_COMMANDS.contains(&"heal"));
        assert!(GM_ONLY_COMMANDS.contains(&"damage"));
        assert!(GM_ONLY_COMMANDS.contains(&"set_hp"));
    }

    #[test]
    fn heal_basic() {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.hp = 3;
        c.max_hp = 10;
        state.party.add_member(c);
        let cmd = HealCommand;
        let result = cmd.execute(&["Aldric", "5"], &mut state);
        assert!(result.output.contains("healed 5 HP"));
        assert_eq!(state.party.find_member("Aldric").unwrap().hp, 8);
    }

    #[test]
    fn heal_capped_at_max() {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.hp = 8;
        c.max_hp = 10;
        state.party.add_member(c);
        let cmd = HealCommand;
        let result = cmd.execute(&["Aldric", "20"], &mut state);
        assert!(result.output.contains("healed 2 HP"));
        assert_eq!(state.party.find_member("Aldric").unwrap().hp, 10);
    }

    #[test]
    fn heal_no_character() {
        let mut state = GameState::new();
        let cmd = HealCommand;
        let result = cmd.execute(&["Nobody", "5"], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn heal_missing_args() {
        let mut state = GameState::new();
        let cmd = HealCommand;
        let result = cmd.execute(&["Aldric"], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn damage_basic() {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.hp = 10;
        c.max_hp = 10;
        state.party.add_member(c);
        let cmd = DamageCommand;
        let result = cmd.execute(&["Aldric", "3"], &mut state);
        assert!(result.output.contains("takes 3 damage"));
        assert!(result.output.contains("wounded"));
        assert_eq!(state.party.find_member("Aldric").unwrap().hp, 7);
    }

    #[test]
    fn damage_kills() {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.hp = 3;
        c.max_hp = 10;
        state.party.add_member(c);
        let cmd = DamageCommand;
        let result = cmd.execute(&["Aldric", "5"], &mut state);
        assert!(result.output.contains("DEAD"));
        assert_eq!(state.party.find_member("Aldric").unwrap().hp, -2);
    }

    #[test]
    fn damage_no_character() {
        let mut state = GameState::new();
        let cmd = DamageCommand;
        let result = cmd.execute(&["Nobody", "5"], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn damage_missing_args() {
        let mut state = GameState::new();
        let cmd = DamageCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn set_hp_basic() {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.hp = 5;
        c.max_hp = 10;
        state.party.add_member(c);
        let cmd = SetHpCommand;
        let result = cmd.execute(&["Aldric", "8"], &mut state);
        assert!(result.output.contains("HP set to 8"));
        assert!(result.output.contains("was 5"));
        assert!(result.output.contains("alive"));
        assert_eq!(state.party.find_member("Aldric").unwrap().hp, 8);
    }

    #[test]
    fn set_hp_to_zero() {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.hp = 5;
        c.max_hp = 10;
        state.party.add_member(c);
        let cmd = SetHpCommand;
        let result = cmd.execute(&["Aldric", "0"], &mut state);
        assert!(result.output.contains("DEAD"));
        assert_eq!(state.party.find_member("Aldric").unwrap().hp, 0);
    }

    #[test]
    fn set_hp_no_character() {
        let mut state = GameState::new();
        let cmd = SetHpCommand;
        let result = cmd.execute(&["Nobody", "5"], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn set_hp_missing_args() {
        let mut state = GameState::new();
        let cmd = SetHpCommand;
        let result = cmd.execute(&["Aldric"], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn set_rations_basic() {
        let mut state = GameState::new();
        state.party.rations = 5;
        let cmd = SetRationsCommand;
        let result = cmd.execute(&["20"], &mut state);
        assert!(result.output.contains("set to 20"));
        assert!(result.output.contains("was 5"));
        assert_eq!(state.party.rations, 20);
    }

    #[test]
    fn set_rations_zero() {
        let mut state = GameState::new();
        state.party.rations = 10;
        let cmd = SetRationsCommand;
        let result = cmd.execute(&["0"], &mut state);
        assert!(result.output.contains("set to 0"));
        assert_eq!(state.party.rations, 0);
    }

    #[test]
    fn set_rations_missing_args() {
        let mut state = GameState::new();
        let cmd = SetRationsCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn add_rations_basic() {
        let mut state = GameState::new();
        state.party.rations = 5;
        let cmd = AddRationsCommand;
        let result = cmd.execute(&["10"], &mut state);
        assert!(result.output.contains("Added 10"));
        assert!(result.output.contains("Total: 15"));
        assert_eq!(state.party.rations, 15);
    }

    #[test]
    fn add_rations_missing_args() {
        let mut state = GameState::new();
        let cmd = AddRationsCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn add_rations_invalid_amount() {
        let mut state = GameState::new();
        let cmd = AddRationsCommand;
        let result = cmd.execute(&["0"], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn gm_commands_include_rations() {
        assert!(GM_ONLY_COMMANDS.contains(&"set_rations"));
        assert!(GM_ONLY_COMMANDS.contains(&"add_rations"));
    }
}
