use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::engine::combat;
use crate::model::{CombatState, Monster};
use crate::rules::{ability, equipment};

pub struct StartCombatCommand;
impl Command for StartCombatCommand {
    fn name(&self) -> &str { "start_combat" }
    fn help(&self) -> &str { "Start combat (start_combat <name> <count> <hd> <ac> <hp> <damage> <morale> <distance>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if state.combat.is_some() {
            return CommandResult::error("combat already active. Use 'end_combat' first.");
        }
        if args.len() < 8 {
            return CommandResult::error(
                "usage: start_combat <name> <count> <hd> <ac> <hp> <damage> <morale> <distance>\n  \
                 example: start_combat goblin 3 1 6 3 1d6 7 60"
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

        let mut out = format!("Combat started! {} {}(s) at {}' distance.\n\n",
            count, name, distance);
        out.push_str(&status);
        out.push_str("\nUse 'initiative' to roll for the first round.");
        CommandResult::ok(out)
    }
}

pub struct InitiativeCommand;
impl Command for InitiativeCommand {
    fn name(&self) -> &str { "initiative" }
    fn help(&self) -> &str { "Roll group initiative for the round" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let combat = match state.combat.as_mut() {
            Some(c) => c,
            None => return CommandResult::error("no active combat. Use 'start_combat' first."),
        };
        let (p, m) = combat::roll_initiative(combat);
        let winner = if p > m {
            "Party acts first"
        } else if m > p {
            "Monsters act first"
        } else {
            "Simultaneous — actions resolve at the same time"
        };
        CommandResult::ok(format!(
            "Round {} Initiative: Party {} vs Monsters {} — {}",
            combat.round, p, m, winner
        ))
    }
}

pub struct AttackCommand;
impl Command for AttackCommand {
    fn name(&self) -> &str { "attack" }
    fn help(&self) -> &str { "Melee/missile attack (attack <character> <monster_idx> [weapon])" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: attack <character_name> <monster_index> [weapon_name]\n  \
                 Default weapon: sword (1d8 melee). Use weapon name for others."
            );
        }
        let char_name = args[0];
        let monster_idx: usize = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_index must be a number"),
        };
        let weapon_name = if args.len() >= 3 { args[2..].join(" ") } else { "sword".to_string() };

        let weapon = match equipment::find_weapon(&weapon_name) {
            Some(w) => w,
            None => return CommandResult::error(format!(
                "unknown weapon '{}'. Try: sword, mace, dagger, short bow, etc.", weapon_name
            )),
        };

        let character = match state.party.find_member(char_name) {
            Some(c) => c.clone(),
            None => return CommandResult::error(format!(
                "no party member named '{}'. Use 'party' to list members.", char_name
            )),
        };

        let combat = match state.combat.as_mut() {
            Some(c) => c,
            None => return CommandResult::error("no active combat. Use 'start_combat' first."),
        };

        if monster_idx >= combat.monsters.len() {
            return CommandResult::error(format!(
                "monster index {} out of range (0-{})",
                monster_idx, combat.monsters.len() - 1
            ));
        }
        if !combat.monsters[monster_idx].is_alive() {
            return CommandResult::error(format!(
                "{} is already dead.", combat.monsters[monster_idx].name
            ));
        }
        if !character.is_alive() {
            return CommandResult::error(format!("{} is dead and cannot attack.", character.name));
        }

        if weapon.qualities.missile && !weapon.qualities.melee {
            let dex_mod = ability::dex_missile_mod(character.abilities.dexterity);
            match combat::character_missile_attack(
                combat, &character, monster_idx,
                weapon.damage, dex_mod, weapon.range,
            ) {
                Ok(result) => CommandResult::ok(format!("{}", result)),
                Err(e) => CommandResult::error(e),
            }
        } else if weapon.qualities.missile && weapon.qualities.melee {
            if combat.distance <= 5 {
                let str_mod = ability::str_melee_mod(character.abilities.strength);
                let result = combat::character_melee_attack(
                    combat, &character, monster_idx, weapon.damage, str_mod,
                );
                CommandResult::ok(format!("{}", result))
            } else {
                let dex_mod = ability::dex_missile_mod(character.abilities.dexterity);
                match combat::character_missile_attack(
                    combat, &character, monster_idx,
                    weapon.damage, dex_mod, weapon.range,
                ) {
                    Ok(result) => CommandResult::ok(format!("{}", result)),
                    Err(e) => CommandResult::error(e),
                }
            }
        } else {
            if combat.distance > 5 {
                return CommandResult::error(format!(
                    "{} is a melee weapon but monsters are {}' away. Move closer or use a missile weapon.",
                    weapon.name, combat.distance
                ));
            }
            let str_mod = ability::str_melee_mod(character.abilities.strength);
            let result = combat::character_melee_attack(
                combat, &character, monster_idx, weapon.damage, str_mod,
            );
            CommandResult::ok(format!("{}", result))
        }
    }
}

pub struct MonsterAttackCommand;
impl Command for MonsterAttackCommand {
    fn name(&self) -> &str { "monster_attack" }
    fn help(&self) -> &str { "Monster attacks character (monster_attack <monster_idx> <character>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: monster_attack <monster_index> <character_name>"
            );
        }
        let monster_idx: usize = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_index must be a number"),
        };
        let char_name = args[1];

        {
            let combat = match state.combat.as_ref() {
                Some(c) => c,
                None => return CommandResult::error("no active combat."),
            };
            if monster_idx >= combat.monsters.len() {
                return CommandResult::error(format!(
                    "monster index {} out of range (0-{})",
                    monster_idx, combat.monsters.len() - 1
                ));
            }
            if !combat.monsters[monster_idx].is_alive() {
                return CommandResult::error(format!(
                    "{} is dead.", combat.monsters[monster_idx].name
                ));
            }
        }

        let character = match state.party.find_member_mut(char_name) {
            Some(c) => c,
            None => return CommandResult::error(format!(
                "no party member named '{}'.", char_name
            )),
        };
        if !character.is_alive() {
            return CommandResult::error(format!("{} is already dead.", character.name));
        }

        let combat = state.combat.as_mut().unwrap();
        let result = combat::monster_attack(combat, monster_idx, character);
        CommandResult::ok(format!("{}", result))
    }
}

pub struct MoraleCommand;
impl Command for MoraleCommand {
    fn name(&self) -> &str { "morale" }
    fn help(&self) -> &str { "Check morale for a monster type (morale <monster_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let combat = match state.combat.as_mut() {
            Some(c) => c,
            None => return CommandResult::error("no active combat."),
        };
        if combat.living_monster_count() == 0 {
            return CommandResult::error("no living monsters to check morale for.");
        }
        let morale_score = if let Some(name) = args.first() {
            let name_lower = name.to_lowercase();
            match combat.monsters.iter()
                .find(|m| m.is_alive() && m.name.to_lowercase() == name_lower)
            {
                Some(m) => m.morale,
                None => return CommandResult::error(format!("no living monster named '{}'.", name)),
            }
        } else {
            combat.living_monsters().first()
                .map(|(_, m)| m.morale)
                .unwrap_or(7)
        };
        let result = combat::check_morale(combat, morale_score);
        CommandResult::ok(format!("{}", result))
    }
}

pub struct TurnUndeadCommand;
impl Command for TurnUndeadCommand {
    fn name(&self) -> &str { "turn_undead" }
    fn help(&self) -> &str { "Cleric turns undead (turn_undead <character> <monster_idx>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: turn_undead <character_name> <monster_index>"
            );
        }
        let char_name = args[0];
        let monster_idx: usize = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_index must be a number"),
        };

        let character = match state.party.find_member(char_name) {
            Some(c) => c.clone(),
            None => return CommandResult::error(format!(
                "no party member named '{}'.", char_name
            )),
        };

        let combat = match state.combat.as_mut() {
            Some(c) => c,
            None => return CommandResult::error("no active combat."),
        };
        if monster_idx >= combat.monsters.len() {
            return CommandResult::error(format!(
                "monster index {} out of range.", monster_idx
            ));
        }
        if !combat.monsters[monster_idx].is_alive() {
            return CommandResult::error("target is already dead.");
        }

        let result = combat::resolve_turn_undead(
            combat, &character, character.level, monster_idx,
        );
        CommandResult::ok(format!("{}", result))
    }
}

pub struct RetreatCommand;
impl Command for RetreatCommand {
    fn name(&self) -> &str { "retreat" }
    fn help(&self) -> &str { "Retreat from combat — full speed, enemies get free attack at +2" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: retreat <character_name>");
        }
        let character = match state.party.find_member(args[0]) {
            Some(c) => c.clone(),
            None => return CommandResult::error(format!(
                "no party member named '{}'.", args[0]
            )),
        };
        let combat = match state.combat.as_mut() {
            Some(c) => c,
            None => return CommandResult::error("no active combat."),
        };
        let msg = combat::retreat(combat, &character);
        CommandResult::ok(msg)
    }
}

pub struct WithdrawalCommand;
impl Command for WithdrawalCommand {
    fn name(&self) -> &str { "withdrawal" }
    fn help(&self) -> &str { "Fighting withdrawal — half speed, no free attacks" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: withdrawal <character_name>");
        }
        let character = match state.party.find_member(args[0]) {
            Some(c) => c.clone(),
            None => return CommandResult::error(format!(
                "no party member named '{}'.", args[0]
            )),
        };
        let combat = match state.combat.as_mut() {
            Some(c) => c,
            None => return CommandResult::error("no active combat."),
        };
        let msg = combat::fighting_withdrawal(combat, &character);
        CommandResult::ok(msg)
    }
}

pub struct DeclareSpellCommand;
impl Command for DeclareSpellCommand {
    fn name(&self) -> &str { "declare_spell" }
    fn help(&self) -> &str { "Declare spell casting (declare_spell <character> <spell_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: declare_spell <character_name> <spell_name>"
            );
        }
        let char_name = args[0];
        let spell_name = args[1..].join(" ");

        if state.party.find_member(char_name).is_none() {
            return CommandResult::error(format!(
                "no party member named '{}'.", char_name
            ));
        }

        let combat = match state.combat.as_mut() {
            Some(c) => c,
            None => return CommandResult::error("no active combat."),
        };
        combat::declare_spell(combat, char_name, &spell_name);
        CommandResult::ok(format!(
            "{} declares: casting {}. Spell will be disrupted if {} takes damage before the magic phase.",
            char_name, spell_name, char_name
        ))
    }
}

pub struct CombatStatusCommand;
impl Command for CombatStatusCommand {
    fn name(&self) -> &str { "combat_status" }
    fn help(&self) -> &str { "Show current combat status" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let combat = match state.combat.as_ref() {
            Some(c) => c,
            None => return CommandResult::error("no active combat."),
        };
        let status = combat::combat_status(combat, &state.party.members);
        CommandResult::ok(status)
    }
}

pub struct EndCombatCommand;
impl Command for EndCombatCommand {
    fn name(&self) -> &str { "end_combat" }
    fn help(&self) -> &str { "End the current combat encounter" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        if state.combat.is_none() {
            return CommandResult::error("no active combat.");
        }
        let combat = state.combat.take().unwrap();
        let dead_monsters = combat.monsters.iter().filter(|m| !m.is_alive()).count();
        let total_xp: u64 = combat.monsters.iter()
            .filter(|m| !m.is_alive())
            .map(|m| m.xp_value)
            .sum();
        let mut out = format!(
            "Combat ended after {} rounds.\n{} of {} monsters defeated.",
            combat.round, dead_monsters, combat.monsters.len()
        );
        if total_xp > 0 {
            out.push_str(&format!("\nTotal XP from defeated monsters: {}", total_xp));
        }
        let dead_party = state.party.members.iter().filter(|c| !c.is_alive()).count();
        if dead_party > 0 {
            out.push_str(&format!("\nParty casualties: {}", dead_party));
        }
        CommandResult::ok(out)
    }
}

pub struct CombatLogCommand;
impl Command for CombatLogCommand {
    fn name(&self) -> &str { "combat_log" }
    fn help(&self) -> &str { "Show combat log for current encounter" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let combat = match state.combat.as_ref() {
            Some(c) => c,
            None => return CommandResult::error("no active combat."),
        };
        if combat.log.is_empty() {
            return CommandResult::ok("No combat events logged yet.");
        }
        let mut out = String::from("Combat Log:\n");
        for (i, entry) in combat.log.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, entry));
        }
        CommandResult::ok(out)
    }
}
