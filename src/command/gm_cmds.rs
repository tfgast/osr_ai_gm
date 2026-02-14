use super::{Command, CommandResult};
use crate::engine::{combat, exploration, gm};
use crate::persist::GameState;

pub struct AdvanceTurnCommand;
impl Command for AdvanceTurnCommand {
    fn name(&self) -> &str {
        "advance_turn"
    }
    fn help(&self) -> &str {
        "GM: advance one dungeon exploration turn"
    }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match exploration::action_advance_dungeon_turn(state) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct AwardXpCommand;
impl Command for AwardXpCommand {
    fn name(&self) -> &str {
        "award_xp"
    }
    fn help(&self) -> &str {
        "GM: award XP (award_xp <character> <amount>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: award_xp <character_name> <amount>");
        }
        let amount: u64 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("amount must be a non-negative integer"),
        };
        match gm::action_award_xp(state, args[0], amount, true) {
            Ok(result) => {
                let xp_str = match result.next_level_xp {
                    Some(next_xp) => format!("{}/{}", result.total_xp, next_xp),
                    None => result.total_xp.to_string(),
                };
                let mut out = format!(
                    "{} awarded {} XP ({} base, {:+}% prime req). Total: {}.",
                    result.character,
                    result.adjusted_xp,
                    result.base_xp,
                    result.modifier_pct,
                    xp_str
                );
                if result.ready_to_train {
                    out.push_str(" Ready to train!");
                }
                CommandResult::ok(out)
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct RulingCommand;
impl Command for RulingCommand {
    fn name(&self) -> &str {
        "ruling"
    }
    fn help(&self) -> &str {
        "GM: record a ruling (ruling <text>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: ruling <text>");
        }
        let text = args.join(" ");
        match gm::action_ruling(state, &text) {
            Ok(result) => CommandResult::ok(format!("Ruling recorded: {}", result.text)),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct HealCommand;
impl Command for HealCommand {
    fn name(&self) -> &str {
        "heal"
    }
    fn help(&self) -> &str {
        "GM: heal a character (heal <character> <amount>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: heal <character_name> <amount>");
        }
        let amount: i32 = match args[1].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("amount must be a positive integer"),
        };
        match gm::action_heal(state, args[0], amount) {
            Ok(result) => CommandResult::ok(format!(
                "{} healed {} HP ({} -> {}/{}).",
                result.character, result.healed, result.old_hp, result.hp, result.max_hp
            )),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct DamageCommand;
impl Command for DamageCommand {
    fn name(&self) -> &str {
        "damage"
    }
    fn help(&self) -> &str {
        "GM: damage a character (damage <character> <amount>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: damage <character_name> <amount>");
        }
        let amount: i32 = match args[1].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("amount must be a positive integer"),
        };
        match gm::action_damage(state, args[0], amount) {
            Ok(result) => CommandResult::ok(format!(
                "{} takes {} damage ({} -> {}/{}). Status: {}.",
                result.character,
                result.damage,
                result.old_hp,
                result.hp,
                result.max_hp,
                result.status
            )),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct SetHpCommand;
impl Command for SetHpCommand {
    fn name(&self) -> &str {
        "set_hp"
    }
    fn help(&self) -> &str {
        "GM: set HP (set_hp <character> <amount>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: set_hp <character_name> <amount>");
        }
        let amount: i32 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("amount must be an integer"),
        };
        match gm::action_set_hp(state, args[0], amount) {
            Ok(result) => CommandResult::ok(format!(
                "{} HP set to {} (was {}). Max HP: {}. Status: {}.",
                result.character, result.hp, result.old_hp, result.max_hp, result.status
            )),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct SetRationsCommand;
impl Command for SetRationsCommand {
    fn name(&self) -> &str {
        "set_rations"
    }
    fn help(&self) -> &str {
        "GM: set party rations (set_rations <amount>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: set_rations <amount>");
        }
        let amount: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("amount must be a non-negative integer"),
        };
        match gm::action_set_rations(state, amount) {
            Ok(result) => CommandResult::ok(format!(
                "Rations set to {} person-days (was {}).",
                result.rations, result.old_rations
            )),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct AddRationsCommand;
impl Command for AddRationsCommand {
    fn name(&self) -> &str {
        "add_rations"
    }
    fn help(&self) -> &str {
        "GM: add rations (add_rations <amount>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: add_rations <amount>");
        }
        let amount: u32 = match args[0].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("amount must be a positive integer"),
        };
        match gm::action_add_rations(state, amount) {
            Ok(result) => CommandResult::ok(format!(
                "Added {} rations. Total: {} person-days.",
                result.added, result.rations
            )),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct TrainCommand;
impl Command for TrainCommand {
    fn name(&self) -> &str {
        "train"
    }
    fn help(&self) -> &str {
        "GM: train to level up (train <character>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: train <character_name>");
        }
        match gm::action_train(state, args[0]) {
            Ok(result) => {
                let mut out = format!(
                    "{} trains to level {}! (cost: {}gp, {}gp remaining)\n",
                    result.character, result.new_level, result.cost_gp, result.gold_remaining
                );
                out.push_str(&format!(
                    "  HP: {} -> {} (+{})\n",
                    result.old_hp, result.new_hp, result.hp_gained
                ));
                if result.new_thac0 != result.old_thac0 {
                    out.push_str(&format!(
                        "  THAC0: {} -> {}\n",
                        result.old_thac0, result.new_thac0
                    ));
                }
                if let Some(old) = result.old_saves {
                    let new = &result.new_saves;
                    if old != *new {
                        out.push_str(&format!(
                            "  Saves: D{}->{} W{}->{} P{}->{} B{}->{} S{}->{}\n",
                            old.death, new.death, old.wands, new.wands,
                            old.paralysis, new.paralysis, old.breath, new.breath,
                            old.spells, new.spells,
                        ));
                    }
                }
                if result.old_spell_slots != result.new_spell_slots {
                    let fmt = |slots: &[u32; 6]| -> String {
                        slots.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("/")
                    };
                    out.push_str(&format!(
                        "  Spell slots: {} -> {}\n",
                        fmt(&result.old_spell_slots),
                        fmt(&result.new_spell_slots)
                    ));
                }
                if result.has_thief_skills {
                    out.push_str(&format!(
                        "  Thief skills improved (now level {} rates)\n",
                        result.new_level
                    ));
                }
                if result.ready_for_next {
                    out.push_str(&format!(
                        "  Ready to train again! (next: level {}, cost: {}gp)",
                        result.new_level + 1,
                        result.next_cost.unwrap_or(0)
                    ));
                }
                if result.gained_first_spells && !result.spell_list_name.is_empty() {
                    out.push_str(&format!(
                        "  {} can now cast {} spells!\n",
                        result.character, result.spell_list_name
                    ));
                }
                CommandResult::ok(out.trim_end().to_string())
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct BackstabCommand;
impl Command for BackstabCommand {
    fn name(&self) -> &str {
        "backstab"
    }
    fn help(&self) -> &str {
        "Thief backstab (backstab <character> <monster_idx> [weapon])"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: backstab <character_name> <monster_index> [weapon_name]\n  \
                 Default weapon: sword. Only Thieves and Assassins can backstab.",
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

        match combat::action_backstab(state, char_name, monster_idx, &weapon_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct ThiefCheckCommand;
impl Command for ThiefCheckCommand {
    fn name(&self) -> &str {
        "thief_check"
    }
    fn help(&self) -> &str {
        "Thief skill check (thief_check <character> <skill>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: thief_check <character_name> <skill_name>\n  \
                 Skills: climb_walls, find_traps, hear_noise, hide_shadows,\n  \
                 move_silently, open_locks, pick_pockets, read_languages",
            );
        }
        let char_name = args[0];
        let skill_name = args[1..].join(" ");
        match gm::action_thief_skill_check(state, char_name, &skill_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct AwardTreasureXpCommand;
impl Command for AwardTreasureXpCommand {
    fn name(&self) -> &str {
        "award_treasure_xp"
    }
    fn help(&self) -> &str {
        "GM: award treasure XP (award_treasure_xp <character> <treasure_gp> <monster_xp>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 3 {
            return CommandResult::error(
                "usage: award_treasure_xp <character_name> <treasure_gp> <monster_xp>\n  \
                 1gp = 1xp base, with prime requisite modifier applied.",
            );
        }
        let treasure_gp: u64 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("treasure_gp must be a non-negative integer"),
        };
        let monster_xp: u64 = match args[2].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_xp must be a non-negative integer"),
        };
        match gm::action_award_treasure_xp(state, args[0], treasure_gp, monster_xp) {
            Ok(result) => {
                let mut msg = format!(
                    "{}: base {}xp ({}gp treasure + {}xp monsters), {:+}% prime req modifier = {} adjusted XP. Total: {}.",
                    result.character, result.base_xp, result.treasure_gp, result.monster_xp,
                    result.modifier_pct, result.adjusted_xp, result.total_xp,
                );
                if result.ready_to_train {
                    msg.push_str(" Ready to train!");
                }
                CommandResult::ok(msg)
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub const GM_ONLY_COMMANDS: &[&str] = &[
    "start_combat",
    "advance_turn",
    "award_xp",
    "award_treasure_xp",
    "backstab",
    "thief_check",
    "train",
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
    use crate::model::{Character, CombatState};
    use crate::rules::class::Class;
    use crate::state::game::GameMode;

    #[test]
    fn award_xp_basic() {
        let mut state = GameState::new();
        state
            .party
            .add_member(Character::new("Aldric", Class::Fighter));
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
        let result = cmd.execute(&["The", "bridge", "can", "hold", "3", "people"], &mut state);
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
        assert!(GM_ONLY_COMMANDS.contains(&"start_combat"));
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
    fn heal_above_max_hp_does_not_reduce() {
        let mut state = GameState::new();
        let mut c = Character::new("Zara", Class::Fighter);
        c.hp = 8;
        c.max_hp = 5;
        state.party.add_member(c);
        let cmd = HealCommand;
        let result = cmd.execute(&["Zara", "5"], &mut state);
        assert!(result.output.contains("healed 0 HP"));
        assert_eq!(state.party.find_member("Zara").unwrap().hp, 8);
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

    #[test]
    fn gm_commands_include_train() {
        assert!(GM_ONLY_COMMANDS.contains(&"train"));
    }

    fn make_leveled_fighter(name: &str, xp: u64, gold: u32) -> Character {
        let mut c = Character::new(name, Class::Fighter);
        c.abilities = crate::model::AbilityScores {
            strength: 16,
            intelligence: 10,
            wisdom: 10,
            dexterity: 10,
            constitution: 14,
            charisma: 10,
        };
        c.xp = xp;
        c.gold_gp = gold;
        c.hp = 8;
        c.max_hp = 8;
        c
    }

    #[test]
    fn train_success() {
        let mut state = GameState::new();
        // Fighter needs 2000 XP for L2. Give enough XP and gold.
        state
            .party
            .add_member(make_leveled_fighter("Aldric", 2_100, 500));
        let cmd = TrainCommand;
        let result = cmd.execute(&["Aldric"], &mut state);
        assert!(
            !result.output.starts_with("Error"),
            "train failed: {}",
            result.output
        );
        assert!(result.output.contains("trains to level 2"));
        assert!(result.output.contains("200gp")); // cost = 2 * 100
        let c = state.party.find_member("Aldric").unwrap();
        assert_eq!(c.level, 2);
        assert!(c.max_hp > 8);
        assert_eq!(c.gold_gp, 300); // 500 - 200
    }

    #[test]
    fn train_insufficient_xp() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_leveled_fighter("Aldric", 100, 500));
        let cmd = TrainCommand;
        let result = cmd.execute(&["Aldric"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("needs"));
        assert_eq!(state.party.find_member("Aldric").unwrap().level, 1);
    }

    #[test]
    fn train_insufficient_gold() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_leveled_fighter("Aldric", 2_100, 50));
        let cmd = TrainCommand;
        let result = cmd.execute(&["Aldric"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("needs 200gp"));
        assert!(result.output.contains("has 50gp"));
        assert_eq!(state.party.find_member("Aldric").unwrap().level, 1);
    }

    #[test]
    fn train_max_level() {
        let mut state = GameState::new();
        // Halfling max level is 8
        let mut c = Character::new("Bilbo", Class::Halfling);
        c.level = 8;
        c.xp = 999_999;
        c.gold_gp = 9999;
        state.party.add_member(c);
        let cmd = TrainCommand;
        let result = cmd.execute(&["Bilbo"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("maximum level"));
    }

    #[test]
    fn train_dead_character() {
        let mut state = GameState::new();
        let mut c = make_leveled_fighter("Aldric", 2_100, 500);
        c.hp = 0;
        state.party.add_member(c);
        let cmd = TrainCommand;
        let result = cmd.execute(&["Aldric"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("dead"));
    }

    #[test]
    fn train_in_combat_blocked() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_leveled_fighter("Aldric", 2_100, 500));
        state.mode = GameMode::Combat;
        let cmd = TrainCommand;
        let result = cmd.execute(&["Aldric"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("combat"));
    }

    #[test]
    fn train_no_args() {
        let mut state = GameState::new();
        let cmd = TrainCommand;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.starts_with("Error"));
    }

    #[test]
    fn award_xp_shows_ready_to_train() {
        let mut state = GameState::new();
        // Fighter near level-up threshold
        let c = make_leveled_fighter("Aldric", 1_900, 500);
        state.party.add_member(c);
        let cmd = AwardXpCommand;
        let result = cmd.execute(&["Aldric", "100"], &mut state);
        // STR 16 = +10%, so 100 base -> 110 adjusted, total 2010 >= 2000
        assert!(result.output.contains("Ready to train!"));
    }

    #[test]
    fn award_xp_applies_prime_req_modifier() {
        let mut state = GameState::new();
        let c = make_leveled_fighter("Aldric", 0, 500);
        state.party.add_member(c);
        let cmd = AwardXpCommand;
        let result = cmd.execute(&["Aldric", "1000"], &mut state);
        // STR 16 = +10%
        assert!(result.output.contains("+10%"));
        assert!(result.output.contains("1100 XP")); // 1000 * 1.10
    }

    // === BackstabCommand tests ===

    fn make_combat_state_with_thief() -> GameState {
        let mut state = GameState::new();
        let mut c = Character::new("Shadow", Class::Thief);
        c.hp = 6;
        c.max_hp = 6;
        c.thac0 = 19;
        c.abilities.strength = 12;
        state.party.add_member(c);
        // Set up active combat with a monster
        let mut m = crate::model::Monster::new("Goblin", "1".parse().unwrap());
        m.hp = 5;
        m.max_hp = 5;
        m.ac = 6;
        state.combat = Some(CombatState::new(vec![m], 10));
        state.mode = GameMode::Combat;
        state
    }

    #[test]
    fn backstab_basic() {
        let mut state = make_combat_state_with_thief();
        let cmd = BackstabCommand;
        let result = cmd.execute(&["Shadow", "0", "sword"], &mut state);
        assert!(
            !result.output.starts_with("Error"),
            "backstab should succeed: {}",
            result.output
        );
        assert!(result.output.contains("backstab"));
    }

    #[test]
    fn backstab_default_weapon() {
        let mut state = make_combat_state_with_thief();
        let cmd = BackstabCommand;
        let result = cmd.execute(&["Shadow", "0"], &mut state);
        assert!(
            !result.output.starts_with("Error"),
            "backstab with default weapon: {}",
            result.output
        );
    }

    #[test]
    fn backstab_non_thief_rejected() {
        let mut state = make_combat_state_with_thief();
        // Add a fighter and try to backstab with them
        let mut f = Character::new("Aldric", Class::Fighter);
        f.hp = 10;
        f.max_hp = 10;
        state.party.add_member(f);
        let cmd = BackstabCommand;
        let result = cmd.execute(&["Aldric", "0"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("cannot backstab"));
    }

    #[test]
    fn backstab_no_combat() {
        let mut state = GameState::new();
        let c = Character::new("Shadow", Class::Thief);
        state.party.add_member(c);
        let cmd = BackstabCommand;
        let result = cmd.execute(&["Shadow", "0"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("no active combat"));
    }

    #[test]
    fn backstab_invalid_monster_index() {
        let mut state = make_combat_state_with_thief();
        let cmd = BackstabCommand;
        let result = cmd.execute(&["Shadow", "99"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("out of range"));
    }

    #[test]
    fn backstab_no_character() {
        let mut state = make_combat_state_with_thief();
        let cmd = BackstabCommand;
        let result = cmd.execute(&["Nobody", "0"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("no party member"));
    }

    #[test]
    fn backstab_missing_args() {
        let mut state = GameState::new();
        let cmd = BackstabCommand;
        let result = cmd.execute(&["Shadow"], &mut state);
        assert!(result.output.starts_with("Error"));
    }

    #[test]
    fn backstab_unknown_weapon() {
        let mut state = make_combat_state_with_thief();
        let cmd = BackstabCommand;
        let result = cmd.execute(&["Shadow", "0", "blastergun"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("unknown weapon"));
    }

    #[test]
    fn backstab_multi_word_weapon() {
        let mut state = make_combat_state_with_thief();
        let cmd = BackstabCommand;
        let result = cmd.execute(&["Shadow", "0", "Short", "sword"], &mut state);
        assert!(
            !result.output.starts_with("Error"),
            "backstab with multi-word weapon should succeed: {}",
            result.output
        );
        assert!(result.output.contains("backstab"));
    }

    // === AwardTreasureXpCommand tests ===

    #[test]
    fn award_treasure_xp_basic() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_leveled_fighter("Aldric", 0, 500));
        let cmd = AwardTreasureXpCommand;
        let result = cmd.execute(&["Aldric", "500", "100"], &mut state);
        assert!(
            !result.output.starts_with("Error"),
            "award_treasure_xp failed: {}",
            result.output
        );
        assert!(result.output.contains("500gp treasure"));
        assert!(result.output.contains("100xp monsters"));
        assert!(result.output.contains("prime req modifier"));
    }

    #[test]
    fn award_treasure_xp_applies_modifier() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_leveled_fighter("Aldric", 0, 500));
        let cmd = AwardTreasureXpCommand;
        let result = cmd.execute(&["Aldric", "1000", "0"], &mut state);
        // STR 16 = +10% prime req
        assert!(result.output.contains("+10%"));
    }

    #[test]
    fn award_treasure_xp_no_character() {
        let mut state = GameState::new();
        let cmd = AwardTreasureXpCommand;
        let result = cmd.execute(&["Nobody", "100", "50"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("no party member"));
    }

    #[test]
    fn award_treasure_xp_missing_args() {
        let mut state = GameState::new();
        let cmd = AwardTreasureXpCommand;
        let result = cmd.execute(&["Aldric", "100"], &mut state);
        assert!(result.output.starts_with("Error"));
    }

    #[test]
    fn award_treasure_xp_invalid_amount() {
        let mut state = GameState::new();
        state
            .party
            .add_member(make_leveled_fighter("Aldric", 0, 500));
        let cmd = AwardTreasureXpCommand;
        let result = cmd.execute(&["Aldric", "abc", "100"], &mut state);
        assert!(result.output.starts_with("Error"));
    }

    #[test]
    fn gm_commands_include_new() {
        assert!(GM_ONLY_COMMANDS.contains(&"backstab"));
        assert!(GM_ONLY_COMMANDS.contains(&"award_treasure_xp"));
        assert!(GM_ONLY_COMMANDS.contains(&"thief_check"));
    }

    // === ThiefCheckCommand tests ===

    #[test]
    fn thief_check_basic() {
        let mut state = GameState::new();
        let mut c = Character::new("Shadow", Class::Thief);
        c.hp = 6;
        c.max_hp = 6;
        state.party.add_member(c);
        let cmd = ThiefCheckCommand;
        let result = cmd.execute(&["Shadow", "climb_walls"], &mut state);
        assert!(
            !result.output.starts_with("Error"),
            "thief_check should succeed: {}",
            result.output
        );
        assert!(result.output.contains("Climb Walls"));
    }

    #[test]
    fn thief_check_multi_word_skill() {
        let mut state = GameState::new();
        let mut c = Character::new("Shadow", Class::Thief);
        c.hp = 6;
        c.max_hp = 6;
        state.party.add_member(c);
        let cmd = ThiefCheckCommand;
        let result = cmd.execute(&["Shadow", "move", "silently"], &mut state);
        assert!(
            !result.output.starts_with("Error"),
            "multi-word skill should work: {}",
            result.output
        );
        assert!(result.output.contains("Move Silently"));
    }

    #[test]
    fn thief_check_non_thief_rejected() {
        let mut state = GameState::new();
        let c = Character::new("Aldric", Class::Fighter);
        state.party.add_member(c);
        let cmd = ThiefCheckCommand;
        let result = cmd.execute(&["Aldric", "climb_walls"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("does not have thief skills"));
    }

    #[test]
    fn thief_check_no_character() {
        let mut state = GameState::new();
        let cmd = ThiefCheckCommand;
        let result = cmd.execute(&["Nobody", "climb_walls"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("no party member"));
    }

    #[test]
    fn thief_check_unknown_skill() {
        let mut state = GameState::new();
        let c = Character::new("Shadow", Class::Thief);
        state.party.add_member(c);
        let cmd = ThiefCheckCommand;
        let result = cmd.execute(&["Shadow", "fly"], &mut state);
        assert!(result.output.starts_with("Error"));
        assert!(result.output.contains("unknown thief skill"));
    }

    #[test]
    fn thief_check_missing_args() {
        let mut state = GameState::new();
        let cmd = ThiefCheckCommand;
        let result = cmd.execute(&["Shadow"], &mut state);
        assert!(result.output.starts_with("Error"));
    }
}
