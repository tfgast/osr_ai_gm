use super::{Command, CommandResult};
use crate::persist::GameState;
use crate::engine::encounter;
use crate::engine::result::EngineError;
use crate::state::game::GameMode;

pub struct EncounterCommand;
impl Command for EncounterCommand {
    fn name(&self) -> &str { "encounter" }
    fn help(&self) -> &str { "Roll a full encounter: table + number appearing + surprise + distance" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match encounter::action_roll_encounter(state) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => {
                let msg = match &e {
                    EngineError::WrongState(s) if s == "dungeon level not set. Use EnterDungeon first." => {
                        "dungeon level not set. Use 'enter_dungeon <level>' first.".to_string()
                    }
                    EngineError::WrongState(s) if s == "encounter requires exploration or wilderness mode." => {
                        "encounter requires exploration or wilderness mode. Use 'enter_dungeon' or 'enter_wilderness' first.".to_string()
                    }
                    EngineError::Internal(s) if s == "no encounter found for this roll." => {
                        "no encounter found for this roll".to_string()
                    }
                    EngineError::Internal(s) if s == "no encounter found for this terrain." => {
                        "no encounter found for this terrain".to_string()
                    }
                    _ => e.to_string(),
                };
                CommandResult::error(msg)
            }
        }
    }
}

pub struct SurpriseCommand;
impl Command for SurpriseCommand {
    fn name(&self) -> &str { "surprise" }
    fn help(&self) -> &str { "Roll surprise for an encounter (1-2 on d6 = surprised)" }
    fn execute(&self, _args: &[&str], state: &mut GameState) -> CommandResult {
        match encounter::action_roll_surprise(state) {
            Ok(result) => CommandResult::ok(result.cli_message()),
            Err(e) => CommandResult::error(e.to_string()),
        }
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
        match encounter::action_roll_reaction(state, args[0]) {
            Ok(result) => CommandResult::ok(result.cli_message()),
            Err(e) => CommandResult::error(e.to_string()),
        }
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
        match encounter::action_evade(state, monster_count, monster_movement) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
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
            .map(npc_party::npc_member_to_monster)
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
