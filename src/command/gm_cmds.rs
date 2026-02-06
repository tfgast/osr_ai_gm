use super::{Command, CommandResult};
use crate::dice;
use crate::engine::{combat, gm, xp};
use crate::persist::GameState;
use crate::rules::class::class_def;
use crate::rules::xp::{check_level_up, xp_for_level};
use crate::rules::{ability, equipment, spell, thief};
use crate::state::game::GameMode;

pub struct SpawnEncounterCommand;
impl Command for SpawnEncounterCommand {
    fn name(&self) -> &str {
        "spawn_encounter"
    }
    fn help(&self) -> &str {
        "GM: spawn monsters (spawn_encounter <name> <count> <hd> <ac> <hp> <damage> <morale> <distance>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 8 {
            return CommandResult::error(
                "usage: spawn_encounter <name> <count> <hd> <ac> <hp> <damage> <morale> <distance>",
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

        match combat::action_spawn_encounter(
            state, name, count, hd, ac, hp, damage, morale, distance, None,
        ) {
            Ok(result) => {
                let mut out = format!(
                    "Encounter spawned! {} {}(s) at {}' distance.\n\n",
                    result.count, result.encounter_name, result.distance
                );
                out.push_str(&result.status);
                out.push_str("\nUse 'initiative' to roll for the first round.");
                CommandResult::ok(out)
            }
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct AdvanceTurnCommand;
impl Command for AdvanceTurnCommand {
    fn name(&self) -> &str {
        "advance_turn"
    }
    fn help(&self) -> &str {
        "GM: advance one dungeon exploration turn"
    }
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
        CommandResult::ok(result.to_string())
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
        if state.mode == GameMode::Combat {
            return CommandResult::error("cannot train during combat.");
        }
        let character = match state.party.find_member_mut(args[0]) {
            Some(c) => c,
            None => return CommandResult::error(format!("no party member named '{}'.", args[0])),
        };
        if !character.is_alive() {
            return CommandResult::error(format!("{} is dead.", character.name));
        }
        let cls = character.class;
        match check_level_up(cls, character.level, character.xp) {
            None => {
                let needed = xp_for_level(cls, character.level + 1);
                if needed == u64::MAX {
                    return CommandResult::error(format!(
                        "{} is at maximum level ({}).",
                        character.name, character.level
                    ));
                }
                CommandResult::error(format!(
                    "{} needs {} XP for level {} (has {}).",
                    character.name,
                    needed,
                    character.level + 1,
                    character.xp
                ))
            }
            Some(next_level) => {
                let cost = next_level * 100;
                if character.gold_gp < cost {
                    return CommandResult::error(format!(
                        "{} needs {}gp to train for level {} (has {}gp).",
                        character.name, cost, next_level, character.gold_gp
                    ));
                }

                // Capture old state for report
                let old_saves = character.saving_throws;
                let old_thac0 = character.thac0;
                let old_hp = character.max_hp;
                let def = class_def(cls);

                // Deduct gold and apply level-up
                character.gold_gp -= cost;
                let result = xp::apply_level_up(character);

                // Build report
                let mut out = format!(
                    "{} trains to level {}! (cost: {}gp, {}gp remaining)\n",
                    character.name, result.new_level, cost, character.gold_gp
                );
                out.push_str(&format!(
                    "  HP: {} -> {} (+{})\n",
                    old_hp, character.max_hp, result.hp_gained
                ));
                if result.new_thac0 != old_thac0 {
                    out.push_str(&format!("  THAC0: {} -> {}\n", old_thac0, result.new_thac0));
                }

                // Report saving throw changes
                if let Some(old) = old_saves {
                    let new = &result.new_saves;
                    if old != *new {
                        out.push_str(&format!(
                            "  Saves: D{}->{} W{}->{} P{}->{} B{}->{} S{}->{}\n",
                            old.death,
                            new.death,
                            old.wands,
                            new.wands,
                            old.paralysis,
                            new.paralysis,
                            old.breath,
                            new.breath,
                            old.spells,
                            new.spells,
                        ));
                    }
                }

                // Report spell slot changes (casters only)
                if result.old_spell_slots != result.new_spell_slots {
                    let fmt = |slots: &[u32; 6]| -> String {
                        slots
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join("/")
                    };
                    out.push_str(&format!(
                        "  Spell slots: {} -> {}\n",
                        fmt(&result.old_spell_slots),
                        fmt(&result.new_spell_slots)
                    ));
                }

                // Report thief skill improvement
                if thief::has_thief_skills(cls) {
                    out.push_str(&format!(
                        "  Thief skills improved (now level {} rates)\n",
                        result.new_level
                    ));
                }

                // Check if ready for another level
                if check_level_up(cls, character.level, character.xp).is_some() {
                    let next_cost = (character.level + 1) * 100;
                    out.push_str(&format!(
                        "  Ready to train again! (next: level {}, cost: {}gp)",
                        character.level + 1,
                        next_cost
                    ));
                }

                // Show new spell slots for casters gaining their first spells
                if result.old_spell_slots.iter().all(|&s| s == 0)
                    && result.new_spell_slots.iter().any(|&s| s > 0)
                {
                    let list_name = match def.spell_list {
                        spell::SpellListType::Cleric => "cleric",
                        spell::SpellListType::Druid => "druid",
                        spell::SpellListType::MagicUser => "magic-user",
                        spell::SpellListType::Illusionist => "illusionist",
                        spell::SpellListType::DrowArcaneAndDivine => "drow",
                        spell::SpellListType::None => "",
                    };
                    if !list_name.is_empty() {
                        out.push_str(&format!(
                            "  {} can now cast {} spells!\n",
                            character.name, list_name
                        ));
                    }
                }

                CommandResult::ok(out.trim_end().to_string())
            }
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
        let weapon_name = if args.len() >= 3 { args[2] } else { "sword" };

        let weapon = match equipment::find_weapon(weapon_name) {
            Some(w) => w,
            None => return CommandResult::error(format!("unknown weapon '{}'.", weapon_name)),
        };
        let character = match state.party.find_member(char_name) {
            Some(c) => c.clone(),
            None => return CommandResult::error(format!("no party member named '{}'.", char_name)),
        };
        if !thief::can_backstab(character.class) {
            return CommandResult::error(format!(
                "{} ({}) cannot backstab.",
                character.name,
                character.class.name()
            ));
        }
        let combat_state = match state.combat.as_mut() {
            Some(c) => c,
            None => return CommandResult::error("no active combat."),
        };
        if monster_idx >= combat_state.monsters.len() {
            return CommandResult::error(format!("monster index {} out of range.", monster_idx));
        }
        if !combat_state.monsters[monster_idx].is_alive() {
            return CommandResult::error(format!(
                "{} is already dead.",
                combat_state.monsters[monster_idx].name
            ));
        }

        let multiplier = thief::backstab_multiplier(character.level);
        let str_mod = ability::str_melee_mod(character.abilities.strength);
        let attack_bonus = thief::BACKSTAB_ATTACK_BONUS;

        let target_ac = combat_state.monsters[monster_idx].ac;
        let target_number =
            (character.thac0 as i32 - target_ac - attack_bonus - str_mod).clamp(2, 20);
        let attack_roll: i32 = rand::Rng::gen_range(&mut rand::thread_rng(), 1..=20);

        let hit = attack_roll == 20 || (attack_roll != 1 && attack_roll >= target_number);

        if hit {
            let base_damage = match dice::roll_str(weapon.damage_dice()) {
                Ok(r) => r.total.max(1),
                Err(_) => 1,
            };
            let total_damage = (base_damage + str_mod).max(1) * multiplier as i32;
            combat_state.monsters[monster_idx].hp -= total_damage;
            let monster_name = combat_state.monsters[monster_idx].name.clone();
            let alive = combat_state.monsters[monster_idx].is_alive();
            combat_state.log.push(format!(
                "{} backstabs {} for {} damage (x{}){}",
                character.name,
                monster_name,
                total_damage,
                multiplier,
                if !alive { " — KILLED!" } else { "" }
            ));
            CommandResult::ok(format!(
                "{} backstabs {} (+{} to hit, x{} damage)! Rolled {} vs target {}: HIT for {} damage{}.",
                character.name, monster_name, attack_bonus, multiplier,
                attack_roll, target_number, total_damage,
                if !alive { " — KILLED!" } else { "" }
            ))
        } else {
            combat_state.log.push(format!(
                "{} backstab attempt on {} missed",
                character.name, combat_state.monsters[monster_idx].name
            ));
            CommandResult::ok(format!(
                "{} backstab attempt: rolled {} vs target {} — MISS.",
                character.name, attack_roll, target_number
            ))
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
        let character = match state.party.find_member_mut(args[0]) {
            Some(c) => c,
            None => return CommandResult::error(format!("no party member named '{}'.", args[0])),
        };
        let result = xp::award_xp(character, treasure_gp, monster_xp);
        let mut msg = format!(
            "{}: base {}xp ({}gp treasure + {}xp monsters), {:+}% prime req modifier = {} adjusted XP. Total: {}.",
            character.name, result.base_xp, treasure_gp, monster_xp,
            result.modifier_pct, result.adjusted_xp, result.new_total,
        );
        if result.ready_to_train {
            msg.push_str(" Ready to train!");
        }
        CommandResult::ok(msg)
    }
}

pub const GM_ONLY_COMMANDS: &[&str] = &[
    "spawn_encounter",
    "advance_turn",
    "award_xp",
    "award_treasure_xp",
    "backstab",
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

    #[test]
    fn spawn_encounter_basic() {
        let mut state = GameState::new();
        state
            .party
            .add_member(Character::new("Aldric", Class::Fighter));
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
    fn spawn_encounter_count_zero() {
        let mut state = GameState::new();
        let cmd = SpawnEncounterCommand;
        let result = cmd.execute(
            &["goblin", "0", "1", "6", "3", "1d6", "7", "60"],
            &mut state,
        );
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("count must be a positive integer"));
        assert!(state.combat.is_none());
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
        let mut m = crate::model::Monster::new("Goblin", "1");
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
    }
}
