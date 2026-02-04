pub mod command;
pub mod dice;
pub mod engine;
pub mod model;
pub mod persist;
pub mod rules;

use command::{Command, CommandRegistry, CommandResult};
use persist::GameState;
use std::io::{self, BufRead, Write};
use engine::chargen;
use rules::class::{self, Class};
use rules::ability;
use rules::equipment;
use model::{CombatState, Monster};
use engine::combat;

// =============================================================================
// Existing commands (updated to take &mut GameState)
// =============================================================================

struct RollCommand;
impl Command for RollCommand {
    fn name(&self) -> &str { "roll" }
    fn help(&self) -> &str { "Roll dice (e.g., roll 2d6+3, roll d%, roll 3-in-6)" }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: roll <dice expression>");
        }
        let notation = args.join("");
        match dice::roll_str(&notation) {
            Ok(result) => CommandResult::ok(format!("{}", result)),
            Err(e) => CommandResult::error(format!("{}", e)),
        }
    }
}

struct HelpCommand {
    commands: Vec<(String, String)>,
}

impl Command for HelpCommand {
    fn name(&self) -> &str { "help" }
    fn help(&self) -> &str { "Show available commands" }
    fn execute(&self, _args: &[&str], _state: &mut GameState) -> CommandResult {
        let mut out = String::from("Available commands:\n");
        for (name, help) in &self.commands {
            out.push_str(&format!("  {:18} {}\n", name, help));
        }
        CommandResult::ok(out)
    }
}

struct QuitCommand;
impl Command for QuitCommand {
    fn name(&self) -> &str { "quit" }
    fn help(&self) -> &str { "Exit the game" }
    fn execute(&self, _args: &[&str], _state: &mut GameState) -> CommandResult {
        CommandResult::quit()
    }
}

struct ChargenCommand;
impl Command for ChargenCommand {
    fn name(&self) -> &str { "chargen" }
    fn help(&self) -> &str { "Create a character and add to party (chargen <name> <class> [alignment])" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error(
                "usage: chargen <name> <class> [alignment]\n  \
                 alignment: Lawful, Neutral, Chaotic (default: Neutral)\n  \
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
        let alignment = if args.len() >= 3 {
            match args[2].to_lowercase().as_str() {
                "lawful" | "l" => "Lawful",
                "neutral" | "n" => "Neutral",
                "chaotic" | "c" => "Chaotic",
                _ => return CommandResult::error(
                    "alignment must be Lawful (L), Neutral (N), or Chaotic (C)"
                ),
            }
        } else {
            "Neutral"
        };

        // Roll abilities
        let mut abilities = chargen::roll_abilities();
        let mut out = String::new();
        out.push_str(&format!("Rolled abilities: STR {} INT {} WIS {} DEX {} CON {} CHA {}\n",
            abilities[0], abilities[1], abilities[2],
            abilities[3], abilities[4], abilities[5]));

        // Apply racial modifiers for demihuman classes
        let def = class::class_def(class);
        if !def.racial_modifiers.is_empty() {
            class::apply_racial_modifiers(class, &mut abilities);
            out.push_str(&format!(
                "After racial modifiers: STR {} INT {} WIS {} DEX {} CON {} CHA {}\n",
                abilities[0], abilities[1], abilities[2],
                abilities[3], abilities[4], abilities[5]));
        }

        // Validate requirements
        if !class::meets_requirements(class, &abilities) {
            let eligible = class::eligible_classes(&abilities);
            let names: Vec<&str> = eligible.iter().map(|c| c.name()).collect();
            out.push_str(&format!(
                "\nAbilities do not meet requirements for {}.\nEligible classes: {}",
                class.name(), names.join(", ")
            ));
            return CommandResult::ok(out);
        }

        // Create character and add to party
        let c = chargen::create_character(name, class, abilities, alignment);
        out.push('\n');
        out.push_str(&chargen::character_sheet(&c));
        out.push_str(&format!("\n{} added to party.\n", c.name));
        state.party.add_member(c);
        CommandResult::ok(out)
    }
}

struct ClassesCommand;
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

struct EligibleCommand;
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

struct PartyCommand;
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
                format!("HP {}/{}, AC {}, THAC0 {}", c.hp, c.max_hp, c.ac, c.thac0)
            } else {
                "DEAD".to_string()
            };
            out.push_str(&format!("  {} ({} L{}) — {}\n",
                c.name, c.class, c.level, status));
        }
        CommandResult::ok(out)
    }
}

struct SaveCommand;
impl Command for SaveCommand {
    fn name(&self) -> &str { "save" }
    fn help(&self) -> &str { "Save game state (e.g., save game.json)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let path = args.first().copied().unwrap_or("save.json");
        match persist::save(state, std::path::Path::new(path)) {
            Ok(()) => CommandResult::ok(format!("Game saved to {}", path)),
            Err(e) => CommandResult::error(format!("save failed: {}", e)),
        }
    }
}

struct LoadCommand;
impl Command for LoadCommand {
    fn name(&self) -> &str { "load" }
    fn help(&self) -> &str { "Load game state (e.g., load game.json)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let path = args.first().copied().unwrap_or("save.json");
        match persist::load(std::path::Path::new(path)) {
            Ok(loaded) => {
                let msg = format!(
                    "Loaded: turn {}, dungeon level {}, {} party members{}",
                    loaded.turn, loaded.dungeon_level, loaded.party.members.len(),
                    if loaded.combat.is_some() { ", combat active" } else { "" }
                );
                *state = loaded;
                CommandResult::ok(msg)
            }
            Err(e) => CommandResult::error(format!("load failed: {}", e)),
        }
    }
}

// =============================================================================
// Combat commands
// =============================================================================

struct StartCombatCommand;
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

struct InitiativeCommand;
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

struct AttackCommand;
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

        // Look up weapon
        let weapon = match equipment::find_weapon(&weapon_name) {
            Some(w) => w,
            None => return CommandResult::error(format!(
                "unknown weapon '{}'. Try: sword, mace, dagger, short bow, etc.", weapon_name
            )),
        };

        // Look up character
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

        // Determine melee vs missile
        if weapon.qualities.missile && !weapon.qualities.melee {
            // Pure missile weapon (bow, crossbow)
            let dex_mod = ability::dex_missile_mod(character.abilities.dexterity);
            match combat::character_missile_attack(
                combat, &character, monster_idx,
                weapon.damage, dex_mod, weapon.range,
            ) {
                Ok(result) => CommandResult::ok(format!("{}", result)),
                Err(e) => CommandResult::error(e),
            }
        } else if weapon.qualities.missile && weapon.qualities.melee {
            // Dual-use weapon (dagger, spear, hand axe) — use melee if close
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
            // Melee weapon
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

struct MonsterAttackCommand;
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

        // Validate monster
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

        // Look up character mutably
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

struct MoraleCommand;
impl Command for MoraleCommand {
    fn name(&self) -> &str { "morale" }
    fn help(&self) -> &str { "Check morale for monsters (2d6 vs morale score)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let combat = match state.combat.as_mut() {
            Some(c) => c,
            None => return CommandResult::error("no active combat."),
        };
        if combat.living_monster_count() == 0 {
            return CommandResult::error("no living monsters to check morale for.");
        }
        let result = combat::check_morale(combat);
        CommandResult::ok(format!("{}", result))
    }
}

struct TurnUndeadCommand;
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

struct RetreatCommand;
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

struct WithdrawalCommand;
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

struct DeclareSpellCommand;
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

struct CombatStatusCommand;
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

struct EndCombatCommand;
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
        // Report party casualties
        let dead_party = state.party.members.iter().filter(|c| !c.is_alive()).count();
        if dead_party > 0 {
            out.push_str(&format!("\nParty casualties: {}", dead_party));
        }
        CommandResult::ok(out)
    }
}

struct CombatLogCommand;
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

// =============================================================================
// Registry & Main
// =============================================================================

fn build_registry() -> CommandRegistry {
    let commands_info: Vec<(String, String)> = vec![
        // Character & Party
        ("chargen".into(), "Create a character and add to party".into()),
        ("classes".into(), "List all character classes".into()),
        ("eligible".into(), "Show eligible classes for ability scores".into()),
        ("party".into(), "Show party members".into()),
        // Combat
        ("start_combat".into(), "Start combat encounter".into()),
        ("initiative".into(), "Roll group initiative".into()),
        ("attack".into(), "Melee/missile attack".into()),
        ("monster_attack".into(), "Monster attacks character".into()),
        ("morale".into(), "Check monster morale".into()),
        ("turn_undead".into(), "Cleric turns undead".into()),
        ("retreat".into(), "Retreat from combat".into()),
        ("withdrawal".into(), "Fighting withdrawal".into()),
        ("declare_spell".into(), "Declare spell casting".into()),
        ("combat_status".into(), "Show combat status".into()),
        ("combat_log".into(), "Show combat log".into()),
        ("end_combat".into(), "End combat encounter".into()),
        // System
        ("roll".into(), "Roll dice (e.g., roll 2d6+3)".into()),
        ("save".into(), "Save game state".into()),
        ("load".into(), "Load game state".into()),
        ("help".into(), "Show available commands".into()),
        ("quit".into(), "Exit the game".into()),
    ];

    let mut registry = CommandRegistry::new();
    // Character & Party
    registry.register(Box::new(ChargenCommand));
    registry.register(Box::new(ClassesCommand));
    registry.register(Box::new(EligibleCommand));
    registry.register(Box::new(PartyCommand));
    // Combat
    registry.register(Box::new(StartCombatCommand));
    registry.register(Box::new(InitiativeCommand));
    registry.register(Box::new(AttackCommand));
    registry.register(Box::new(MonsterAttackCommand));
    registry.register(Box::new(MoraleCommand));
    registry.register(Box::new(TurnUndeadCommand));
    registry.register(Box::new(RetreatCommand));
    registry.register(Box::new(WithdrawalCommand));
    registry.register(Box::new(DeclareSpellCommand));
    registry.register(Box::new(CombatStatusCommand));
    registry.register(Box::new(CombatLogCommand));
    registry.register(Box::new(EndCombatCommand));
    // System
    registry.register(Box::new(RollCommand));
    registry.register(Box::new(SaveCommand));
    registry.register(Box::new(LoadCommand));
    registry.register(Box::new(HelpCommand { commands: commands_info }));
    registry.register(Box::new(QuitCommand));
    registry
}

fn main() {
    println!("OSR AI Game Master v{}", env!("CARGO_PKG_VERSION"));
    println!("Type 'help' for available commands, 'quit' to exit.\n");

    let registry = build_registry();
    let mut state = GameState::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            print!("> ");
            let _ = stdout.flush();
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd_name = parts[0];
        let args = &parts[1..];

        let result = registry.dispatch(cmd_name, args, &mut state);
        println!("{}", result.output);

        if result.quit {
            break;
        }

        print!("> ");
        let _ = stdout.flush();
    }
}
