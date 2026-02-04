pub mod command;
pub mod dice;
pub mod engine;
pub mod model;
pub mod persist;
pub mod rules;
pub mod state;

use command::CommandRegistry;
use command::party::*;
use command::combat_cmds::*;
use command::exploration_cmds::*;
use command::encounter_cmds::*;
use command::wilderness_cmds::*;
use command::system::*;
use persist::GameState;
use std::io::{self, BufRead, Write};

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
