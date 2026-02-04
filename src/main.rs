pub mod command;
pub mod dice;
pub mod engine;
pub mod gmapi;
pub mod model;
pub mod persist;
pub mod rules;
pub mod session;
pub mod state;

use command::{Command, CommandRegistry, CommandResult};
use command::gm_cmds;
use persist::GameState;
use std::io::{self, BufRead, Write};
use engine::chargen;
use engine::encounter_engine;
use engine::exploration;
use engine::wilderness_engine;
use rules::class::{self, Class};
use rules::ability;
use rules::equipment;
use model::{CombatState, Monster};
use engine::combat;
use state::dungeon::{DungeonState, Room, Door, DoorState};
use state::time::{TimeTracker, LightSourceKind};
use state::wilderness::{WildernessState, HexCell, Terrain};
use gmapi::protocol::{self, GMCommand};

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
// Exploration commands
// =============================================================================

struct EnterDungeonCommand;
impl Command for EnterDungeonCommand {
    fn name(&self) -> &str { "enter_dungeon" }
    fn help(&self) -> &str { "Enter dungeon exploration mode (enter_dungeon <level> <room_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: enter_dungeon <level> [room_name]");
        }
        let level: u32 = match args[0].parse() {
            Ok(n) if n >= 1 => n,
            _ => return CommandResult::error("level must be a positive integer"),
        };
        let room_name = if args.len() > 1 { args[1..].join(" ") } else { "Entrance".to_string() };

        let mut dungeon = DungeonState::new(level);
        dungeon.add_room(Room::new(0, &room_name));
        dungeon.explore_current();

        let time = TimeTracker::new();

        state.dungeon = Some(dungeon);
        state.time = Some(time);
        state.dungeon_level = level;

        CommandResult::ok(format!(
            "Entered dungeon level {}. Starting room: {}.\n\
             Use 'light torch <carrier>' or 'light lantern <carrier>' to light the way.\n\
             Use 'explore' to advance a dungeon turn.",
            level, room_name
        ))
    }
}

struct LightCommand;
impl Command for LightCommand {
    fn name(&self) -> &str { "light" }
    fn help(&self) -> &str { "Light a torch or lantern (light torch|lantern <carrier_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: light torch|lantern <carrier_name>");
        }
        let kind = match args[0].to_lowercase().as_str() {
            "torch" => LightSourceKind::Torch,
            "lantern" => LightSourceKind::Lantern,
            _ => return CommandResult::error("light source must be 'torch' or 'lantern'"),
        };
        let carrier = args[1];
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode. Use 'enter_dungeon' first."),
        };
        time.light(kind, carrier);
        CommandResult::ok(format!(
            "{} lights a {} ({} turns).",
            carrier, kind.name(), kind.max_turns()
        ))
    }
}

struct ExploreCommand;
impl Command for ExploreCommand {
    fn name(&self) -> &str { "explore" }
    fn help(&self) -> &str { "Advance one dungeon turn of exploration" }
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
        let result = exploration::advance_dungeon_turn(time, dungeon, level);
        CommandResult::ok(format!("{}", result))
    }
}

struct SearchCommand;
impl Command for SearchCommand {
    fn name(&self) -> &str { "search" }
    fn help(&self) -> &str { "Search the current room (1-in-6, elves 2-in-6). Takes one turn." }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let is_elf = args.first().map(|a| a.eq_ignore_ascii_case("elf")).unwrap_or(false);
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let result = exploration::search_room(time, dungeon, is_elf);
        CommandResult::ok(result)
    }
}

struct ListenCommand;
impl Command for ListenCommand {
    fn name(&self) -> &str { "listen" }
    fn help(&self) -> &str { "Listen at a door (1-in-6, demihumans 2-in-6)" }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        let is_demihuman = args.first()
            .map(|a| a.eq_ignore_ascii_case("demihuman") || a.eq_ignore_ascii_case("elf"))
            .unwrap_or(false);
        let result = exploration::listen_at_door(is_demihuman);
        CommandResult::ok(result)
    }
}

struct ForceDoorCommand;
impl Command for ForceDoorCommand {
    fn name(&self) -> &str { "force_door" }
    fn help(&self) -> &str { "Force open a door (force_door <door_id> <character_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: force_door <door_id> <character_name>");
        }
        let door_id: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("door_id must be a number"),
        };
        let char_name = args[1];
        let character = match state.party.find_member(char_name) {
            Some(c) => c.clone(),
            None => return CommandResult::error(format!("no party member named '{}'.", char_name)),
        };
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let result = exploration::force_door(dungeon, door_id, &character);
        CommandResult::ok(result)
    }
}

struct AddRoomCommand;
impl Command for AddRoomCommand {
    fn name(&self) -> &str { "add_room" }
    fn help(&self) -> &str { "Add a room to dungeon (add_room <id> <name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: add_room <id> <name>");
        }
        let id: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("room id must be a number"),
        };
        let name = args[1..].join(" ");
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        dungeon.add_room(Room::new(id, &name));
        CommandResult::ok(format!("Added room {}: {}", id, name))
    }
}

struct AddDoorCommand;
impl Command for AddDoorCommand {
    fn name(&self) -> &str { "add_door" }
    fn help(&self) -> &str { "Add a door (add_door <id> <room_a> <room_b> [open|closed|stuck|locked|secret])" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 3 {
            return CommandResult::error(
                "usage: add_door <id> <room_a> <room_b> [open|closed|stuck|locked|secret]"
            );
        }
        let id: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("door id must be a number"),
        };
        let room_a: u32 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("room_a must be a number"),
        };
        let room_b: u32 = match args[2].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("room_b must be a number"),
        };
        let door_state = if args.len() > 3 {
            match args[3].to_lowercase().as_str() {
                "open" => DoorState::Open,
                "closed" => DoorState::Closed,
                "stuck" => DoorState::Stuck,
                "locked" => DoorState::Locked,
                "secret" => DoorState::Secret,
                _ => return CommandResult::error("state must be open, closed, stuck, locked, or secret"),
            }
        } else {
            DoorState::Closed
        };
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        dungeon.add_door(Door::new(id, room_a, room_b, door_state));
        CommandResult::ok(format!("Added door {} between rooms {} and {} ({:?})", id, room_a, room_b, door_state))
    }
}

struct MoveRoomCommand;
impl Command for MoveRoomCommand {
    fn name(&self) -> &str { "move" }
    fn help(&self) -> &str { "Move through a door (move <door_id>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("usage: move <door_id>");
        }
        let door_id: u32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("door_id must be a number"),
        };
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        let dungeon = match state.dungeon.as_mut() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        match exploration::move_through_door(time, dungeon, door_id) {
            Ok(msg) => CommandResult::ok(msg),
            Err(e) => CommandResult::error(e),
        }
    }
}

struct RestCommand;
impl Command for RestCommand {
    fn name(&self) -> &str { "rest" }
    fn help(&self) -> &str { "Rest for one turn (required after 5 turns of activity)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let time = match state.time.as_mut() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        time.rest();
        CommandResult::ok("Party rests for one turn. Activity counter reset.")
    }
}

struct ExplorationStatusCommand;
impl Command for ExplorationStatusCommand {
    fn name(&self) -> &str { "exploration_status" }
    fn help(&self) -> &str { "Show current exploration state (time, light, dungeon map)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let time = match state.time.as_ref() {
            Some(t) => t,
            None => return CommandResult::error("not in exploration mode."),
        };
        let dungeon = match state.dungeon.as_ref() {
            Some(d) => d,
            None => return CommandResult::error("no dungeon state."),
        };
        let status = exploration::exploration_status(time, dungeon);
        CommandResult::ok(status)
    }
}

// Encounter commands
struct SurpriseCommand;
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

struct ReactionCommand;
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

struct EvadeCommand;
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

// Wilderness commands
struct EnterWildernessCommand;
impl Command for EnterWildernessCommand {
    fn name(&self) -> &str { "enter_wilderness" }
    fn help(&self) -> &str { "Enter wilderness travel mode (enter_wilderness <terrain>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        let terrain = if args.is_empty() {
            Terrain::Clear
        } else {
            match args[0].to_lowercase().as_str() {
                "clear" => Terrain::Clear,
                "forest" => Terrain::Forest,
                "hills" => Terrain::Hills,
                "mountains" => Terrain::Mountains,
                "desert" => Terrain::Desert,
                "swamp" => Terrain::Swamp,
                "jungle" => Terrain::Jungle,
                "ocean" => Terrain::Ocean,
                "river" => Terrain::River,
                "barren" => Terrain::Barren,
                "city" => Terrain::City,
                _ => return CommandResult::error(
                    "terrain must be: clear, forest, hills, mountains, desert, swamp, jungle, ocean, river, barren, city"
                ),
            }
        };
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, terrain));
        state.wilderness = Some(ws);
        CommandResult::ok(format!(
            "Entered wilderness. Starting hex: (0, 0) — {}.\n\
             Use 'add_hex' to build the map, 'travel' to move.",
            terrain.name()
        ))
    }
}

struct AddHexCommand;
impl Command for AddHexCommand {
    fn name(&self) -> &str { "add_hex" }
    fn help(&self) -> &str { "Add a hex to the wilderness map (add_hex <x> <y> <terrain>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 3 {
            return CommandResult::error("usage: add_hex <x> <y> <terrain>");
        }
        let x: i32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("x must be an integer"),
        };
        let y: i32 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("y must be an integer"),
        };
        let terrain = match args[2].to_lowercase().as_str() {
            "clear" => Terrain::Clear,
            "forest" => Terrain::Forest,
            "hills" => Terrain::Hills,
            "mountains" => Terrain::Mountains,
            "desert" => Terrain::Desert,
            "swamp" => Terrain::Swamp,
            "jungle" => Terrain::Jungle,
            "ocean" => Terrain::Ocean,
            "river" => Terrain::River,
            "barren" => Terrain::Barren,
            "city" => Terrain::City,
            _ => return CommandResult::error("invalid terrain type"),
        };
        let ws = match state.wilderness.as_mut() {
            Some(w) => w,
            None => return CommandResult::error("not in wilderness mode."),
        };
        ws.add_hex(HexCell::new(x, y, terrain));
        CommandResult::ok(format!("Added hex ({}, {}) — {}.", x, y, terrain.name()))
    }
}

struct TravelCommand;
impl Command for TravelCommand {
    fn name(&self) -> &str { "travel" }
    fn help(&self) -> &str { "Travel to a wilderness hex (travel <x> <y>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: travel <x> <y>");
        }
        let x: i32 = match args[0].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("x must be an integer"),
        };
        let y: i32 = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("y must be an integer"),
        };
        let party_movement = state.party.members.iter()
            .filter(|c| c.is_alive())
            .map(|c| c.movement_rate)
            .min()
            .unwrap_or(120);
        let ws = match state.wilderness.as_mut() {
            Some(w) => w,
            None => return CommandResult::error("not in wilderness mode."),
        };
        let result = wilderness_engine::travel_day(ws, x, y, party_movement);
        CommandResult::ok(format!("{}", result))
    }
}

struct ForageCommand;
impl Command for ForageCommand {
    fn name(&self) -> &str { "forage" }
    fn help(&self) -> &str { "Forage for food in the current hex (takes a full day)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let ws = match state.wilderness.as_ref() {
            Some(w) => w,
            None => return CommandResult::error("not in wilderness mode."),
        };
        let result = wilderness_engine::forage(ws);
        CommandResult::ok(result)
    }
}

struct HuntCommand;
impl Command for HuntCommand {
    fn name(&self) -> &str { "hunt" }
    fn help(&self) -> &str { "Hunt for game in the current hex (takes a full day)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let ws = match state.wilderness.as_ref() {
            Some(w) => w,
            None => return CommandResult::error("not in wilderness mode."),
        };
        let result = wilderness_engine::hunt(ws);
        CommandResult::ok(result)
    }
}

struct WildernessStatusCommand;
impl Command for WildernessStatusCommand {
    fn name(&self) -> &str { "wilderness_status" }
    fn help(&self) -> &str { "Show current wilderness travel status" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let ws = match state.wilderness.as_ref() {
            Some(w) => w,
            None => return CommandResult::error("not in wilderness mode."),
        };
        let party_movement = state.party.members.iter()
            .filter(|c| c.is_alive())
            .map(|c| c.movement_rate)
            .min()
            .unwrap_or(120);
        let status = wilderness_engine::wilderness_status(ws, party_movement);
        CommandResult::ok(status)
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
        // Dungeon Exploration
        ("enter_dungeon".into(), "Enter dungeon exploration mode".into()),
        ("light".into(), "Light a torch or lantern".into()),
        ("explore".into(), "Advance one dungeon turn".into()),
        ("search".into(), "Search current room for secrets".into()),
        ("listen".into(), "Listen at a door".into()),
        ("force_door".into(), "Force open a door".into()),
        ("add_room".into(), "Add a room to dungeon".into()),
        ("add_door".into(), "Add a door between rooms".into()),
        ("move".into(), "Move through a door".into()),
        ("rest".into(), "Rest for one turn".into()),
        ("exploration_status".into(), "Show exploration state".into()),
        // Encounter
        ("surprise".into(), "Roll surprise check".into()),
        ("reaction".into(), "Roll NPC reaction".into()),
        ("evade".into(), "Attempt to evade encounter".into()),
        // Wilderness
        ("enter_wilderness".into(), "Enter wilderness travel mode".into()),
        ("add_hex".into(), "Add hex to wilderness map".into()),
        ("travel".into(), "Travel to a hex".into()),
        ("forage".into(), "Forage for food".into()),
        ("hunt".into(), "Hunt for game".into()),
        ("wilderness_status".into(), "Show wilderness status".into()),
        // GM-only
        ("spawn_encounter".into(), "GM: spawn monsters".into()),
        ("advance_turn".into(), "GM: advance one dungeon turn".into()),
        ("roll_reaction".into(), "GM: roll NPC reaction".into()),
        ("award_xp".into(), "GM: award XP to character".into()),
        ("ruling".into(), "GM: record a ruling".into()),
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
    // Dungeon Exploration
    registry.register(Box::new(EnterDungeonCommand));
    registry.register(Box::new(LightCommand));
    registry.register(Box::new(ExploreCommand));
    registry.register(Box::new(SearchCommand));
    registry.register(Box::new(ListenCommand));
    registry.register(Box::new(ForceDoorCommand));
    registry.register(Box::new(AddRoomCommand));
    registry.register(Box::new(AddDoorCommand));
    registry.register(Box::new(MoveRoomCommand));
    registry.register(Box::new(RestCommand));
    registry.register(Box::new(ExplorationStatusCommand));
    // Encounter
    registry.register(Box::new(SurpriseCommand));
    registry.register(Box::new(ReactionCommand));
    registry.register(Box::new(EvadeCommand));
    // Wilderness
    registry.register(Box::new(EnterWildernessCommand));
    registry.register(Box::new(AddHexCommand));
    registry.register(Box::new(TravelCommand));
    registry.register(Box::new(ForageCommand));
    registry.register(Box::new(HuntCommand));
    registry.register(Box::new(WildernessStatusCommand));
    // GM-only
    registry.register(Box::new(gm_cmds::SpawnEncounterCommand));
    registry.register(Box::new(gm_cmds::AdvanceTurnCommand));
    registry.register(Box::new(gm_cmds::RollReactionCommand));
    registry.register(Box::new(gm_cmds::AwardXpCommand));
    registry.register(Box::new(gm_cmds::RulingCommand));
    // System
    registry.register(Box::new(RollCommand));
    registry.register(Box::new(SaveCommand));
    registry.register(Box::new(LoadCommand));
    registry.register(Box::new(HelpCommand { commands: commands_info }));
    registry.register(Box::new(QuitCommand));
    registry
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args.iter().any(|a| a == "--json");

    if json_mode {
        run_json_mode();
    } else {
        run_cli_mode();
    }
}

/// Interactive CLI mode for human players (original behavior).
fn run_cli_mode() {
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

/// JSON pipe mode for AI GM — reads GMRequest JSON lines, writes GMResponse JSON lines.
fn run_json_mode() {
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
            continue;
        }

        let request = match protocol::parse_request(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let resp = gmapi::protocol::GMResponse::err(
                    "?", e, state.mode.clone(),
                );
                println!("{}", protocol::serialize_response(&resp));
                let _ = stdout.flush();
                continue;
            }
        };

        let is_quit = matches!(request.command, GMCommand::Quit);
        let response = gmapi::interface::handle_request(&request, &mut state);
        println!("{}", protocol::serialize_response(&response));
        let _ = stdout.flush();

        if is_quit {
            break;
        }
    }
}
