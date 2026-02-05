use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::engine::encounter_engine;
use crate::rules::ability;
use crate::rules::encounter as encounter_tables;
use crate::state::game::GameMode;
use crate::dice;

/// Roll number appearing. Handles both dice notation ("2d4") and plain integers ("1").
fn roll_number_appearing(notation: &str) -> Result<i32, String> {
    if let Ok(n) = notation.parse::<i32>() {
        return Ok(n);
    }
    dice::roll_str(notation)
        .map(|r| r.total)
        .map_err(|e| format!("bad dice expr '{}': {}", notation, e))
}

pub struct EncounterCommand;
impl Command for EncounterCommand {
    fn name(&self) -> &str { "encounter" }
    fn help(&self) -> &str { "Roll a full encounter: table + number appearing + surprise + distance" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        let mut rng = rand::thread_rng();
        let table_roll: u32 = rand::Rng::gen_range(&mut rng, 1..=20);

        match state.mode {
            GameMode::Exploration => {
                let level = state.dungeon_level;
                if level == 0 {
                    return CommandResult::error(
                        "dungeon level not set. Use 'enter_dungeon <level>' first.",
                    );
                }
                let entry = match encounter_tables::dungeon_encounter_d40(level, table_roll) {
                    Some(e) => e,
                    None => return CommandResult::error("no encounter found for this roll"),
                };
                let num_appearing = match roll_number_appearing(&entry.number) {
                    Ok(n) => n,
                    Err(e) => return CommandResult::error(e),
                };
                let seq = encounter_engine::begin_encounter_dungeon();

                let surprise_line = format!(
                    "Surprise: party {}, monsters {} — {}",
                    seq.party_surprise_roll, seq.monster_surprise_roll, seq.surprise
                );
                CommandResult::ok(format!(
                    "ENCOUNTER — Dungeon Level {}\n\
                     Table roll: {} → {}\n\
                     Number appearing: {} → {}\n\
                     {}\n\
                     Distance: {}' feet",
                    level,
                    table_roll, entry.name,
                    entry.number, num_appearing,
                    surprise_line,
                    seq.distance,
                ))
            }
            GameMode::Wilderness => {
                let ws = match state.wilderness.as_ref() {
                    Some(w) => w,
                    None => return CommandResult::error("no wilderness state."),
                };
                let hex = match ws.current_hex() {
                    Some(h) => h,
                    None => return CommandResult::error("no current hex."),
                };
                let terrain = hex.terrain;
                let entry = match encounter_tables::wilderness_encounter_simple(terrain, table_roll) {
                    Some(e) => e,
                    None => return CommandResult::error("no encounter found for this terrain"),
                };
                let num_appearing = match roll_number_appearing(&entry.number) {
                    Ok(n) => n,
                    Err(e) => return CommandResult::error(e),
                };
                let seq = encounter_engine::begin_encounter_wilderness();

                let surprise_line = format!(
                    "Surprise: party {}, monsters {} — {}",
                    seq.party_surprise_roll, seq.monster_surprise_roll, seq.surprise
                );
                CommandResult::ok(format!(
                    "ENCOUNTER — Wilderness ({})\n\
                     Table roll: {} → {}\n\
                     Number appearing: {} → {}\n\
                     {}\n\
                     Distance: {} yards",
                    terrain.name(),
                    table_roll, entry.name,
                    entry.number, num_appearing,
                    surprise_line,
                    seq.distance,
                ))
            }
            _ => CommandResult::error(
                "encounter requires exploration or wilderness mode. Use 'enter_dungeon' or 'enter_wilderness' first.",
            ),
        }
    }
}

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

pub struct SpawnNpcCommand;
impl Command for SpawnNpcCommand {
    fn name(&self) -> &str { "spawn_npc" }
    fn help(&self) -> &str { "Spawn NPC party (spawn_npc <basic|expert|cleric|fighter|mage> [distance])" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        use crate::model::{CombatState, Monster};
        use crate::engine::combat;
        use crate::rules::npc_party;

        if state.combat.is_some() {
            return CommandResult::error("combat already active. Use 'end_combat' first.");
        }
        if args.is_empty() {
            return CommandResult::error(
                "usage: spawn_npc <party_type> [distance]\n  \
                 party_type: basic, expert, cleric, fighter, mage\n  \
                 distance: encounter distance in feet (default 60)"
            );
        }
        let party_type = args[0];
        let distance: u32 = if args.len() >= 2 {
            match args[1].parse() {
                Ok(n) => n,
                _ => return CommandResult::error("distance must be a non-negative integer"),
            }
        } else {
            60
        };

        let mut rng = rand::thread_rng();
        let party = match party_type {
            "basic" => npc_party::generate_basic_party(&mut rng),
            "expert" => npc_party::generate_expert_party(&mut rng),
            "cleric" => npc_party::generate_high_level_cleric_party(&mut rng),
            "fighter" => npc_party::generate_high_level_fighter_party(&mut rng),
            "mage" => npc_party::generate_high_level_magic_user_party(&mut rng),
            _ => return CommandResult::error(
                "unknown party type. Valid: basic, expert, cleric, fighter, mage"
            ),
        };

        let mut output = format!("NPC {} party ({} members) at {}' distance:\n",
            party.party_type, party.members.len(), distance);
        for m in &party.members {
            let role_str = m.role.as_deref().map(|r| format!(" ({})", r)).unwrap_or_default();
            output.push_str(&format!("  {} Lv{} [{}]{}\n", m.class, m.level, m.alignment, role_str));
        }
        if party.mounted {
            output.push_str("  Party is mounted.\n");
        }
        for note in &party.notes {
            output.push_str(&format!("  {}\n", note));
        }

        let monsters: Vec<Monster> = party.members.iter()
            .map(|m| npc_party::npc_member_to_monster(m))
            .collect();

        let combat_state = CombatState::new(monsters, distance);
        let status = combat::combat_status(&combat_state, &state.party.members);
        state.combat = Some(combat_state);
        state.pre_combat_mode = Some(state.mode.clone());
        state.mode = GameMode::Combat;

        output.push_str(&format!("\nCombat started! {}", serde_json::to_string(&status).unwrap_or_default()));
        CommandResult::ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::dungeon::DungeonState;
    use crate::state::time::TimeTracker;
    use crate::state::wilderness::{WildernessState, HexCell, Terrain};

    fn dungeon_state(level: u32) -> GameState {
        let mut state = GameState::new();
        state.mode = GameMode::Exploration;
        state.dungeon_level = level;
        state.dungeon = Some(DungeonState::new(level));
        state.time = Some(TimeTracker::new());
        state
    }

    fn wilderness_state(terrain: Terrain) -> GameState {
        let mut state = GameState::new();
        state.mode = GameMode::Wilderness;
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, terrain)).unwrap();
        state.wilderness = Some(ws);
        state
    }

    #[test]
    fn encounter_dungeon_outputs_all_sections() {
        let cmd = EncounterCommand;
        let mut state = dungeon_state(1);
        let result = cmd.execute(&[], &mut state);
        assert!(!result.quit);
        assert!(result.output.contains("ENCOUNTER"), "missing header: {}", result.output);
        assert!(result.output.contains("Dungeon Level 1"), "missing level: {}", result.output);
        assert!(result.output.contains("Table roll:"), "missing table roll: {}", result.output);
        assert!(result.output.contains("Number appearing:"), "missing number: {}", result.output);
        assert!(result.output.contains("Surprise:"), "missing surprise: {}", result.output);
        assert!(result.output.contains("Distance:"), "missing distance: {}", result.output);
        assert!(result.output.contains("feet"), "missing feet unit: {}", result.output);
    }

    #[test]
    fn encounter_wilderness_outputs_all_sections() {
        let cmd = EncounterCommand;
        let mut state = wilderness_state(Terrain::Forest);
        let result = cmd.execute(&[], &mut state);
        assert!(!result.quit);
        assert!(result.output.contains("ENCOUNTER"), "missing header: {}", result.output);
        assert!(result.output.contains("Wilderness"), "missing wilderness: {}", result.output);
        assert!(result.output.contains("Forest"), "missing terrain: {}", result.output);
        assert!(result.output.contains("Table roll:"), "missing table roll: {}", result.output);
        assert!(result.output.contains("Number appearing:"), "missing number: {}", result.output);
        assert!(result.output.contains("Surprise:"), "missing surprise: {}", result.output);
        assert!(result.output.contains("yards"), "missing yards unit: {}", result.output);
    }

    #[test]
    fn encounter_idle_mode_error() {
        let cmd = EncounterCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"), "expected error: {}", result.output);
        assert!(result.output.contains("exploration or wilderness"), "{}", result.output);
    }

    #[test]
    fn encounter_dungeon_level_zero_error() {
        let cmd = EncounterCommand;
        let mut state = GameState::new();
        state.mode = GameMode::Exploration;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"), "expected error: {}", result.output);
        assert!(result.output.contains("dungeon level"), "{}", result.output);
    }

    #[test]
    fn encounter_wilderness_no_state_error() {
        let cmd = EncounterCommand;
        let mut state = GameState::new();
        state.mode = GameMode::Wilderness;
        let result = cmd.execute(&[], &mut state);
        assert!(result.output.contains("Error"), "expected error: {}", result.output);
    }

    #[test]
    fn encounter_dungeon_number_appearing_positive() {
        let cmd = EncounterCommand;
        let mut state = dungeon_state(3);
        // Run multiple times to exercise randomness
        for _ in 0..20 {
            let result = cmd.execute(&[], &mut state);
            assert!(result.output.contains("Number appearing:"), "{}", result.output);
            // The number after the arrow should be >= 1
            let num_line = result.output.lines()
                .find(|l| l.contains("Number appearing:"))
                .unwrap();
            let total: &str = num_line.rsplit("→ ").next().unwrap().trim();
            let n: i32 = total.parse().unwrap();
            assert!(n >= 1, "number appearing should be >= 1, got {}", n);
        }
    }

    #[test]
    fn encounter_all_dungeon_levels() {
        let cmd = EncounterCommand;
        for level in 1..=9 {
            let mut state = dungeon_state(level);
            let result = cmd.execute(&[], &mut state);
            assert!(!result.output.starts_with("Error"),
                "level {} failed: {}", level, result.output);
        }
    }

    #[test]
    fn encounter_all_wilderness_terrains() {
        let cmd = EncounterCommand;
        let terrains = [
            Terrain::Clear, Terrain::Forest, Terrain::Hills,
            Terrain::Mountains, Terrain::Desert, Terrain::Swamp,
            Terrain::Jungle, Terrain::Ocean,
        ];
        for terrain in &terrains {
            let mut state = wilderness_state(*terrain);
            let result = cmd.execute(&[], &mut state);
            assert!(!result.output.starts_with("Error"),
                "{:?} failed: {}", terrain, result.output);
        }
    }
}
