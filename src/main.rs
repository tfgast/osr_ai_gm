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
use command::inventory_cmds::*;
use command::retainer_cmds::*;
use command::wilderness_cmds::*;
use command::gm_cmds::{AdvanceTurnCommand, AwardXpCommand, RulingCommand, HealCommand, DamageCommand, SetHpCommand};
use command::system::*;
use persist::GameState;
use std::io::{self, BufRead, Write};

fn build_registry() -> CommandRegistry {
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
    registry.register(Box::new(CloseCommand));
    registry.register(Box::new(RetreatCommand));
    registry.register(Box::new(WithdrawalCommand));
    registry.register(Box::new(DeclareSpellCommand));
    registry.register(Box::new(CombatStatusCommand));
    registry.register(Box::new(CombatLogCommand));
    registry.register(Box::new(EndCombatCommand));
    registry.register(Box::new(SetHelplessCommand));
    registry.register(Box::new(KillCommand));
    // Inventory
    registry.register(Box::new(BuyCommand));
    registry.register(Box::new(DropCommand));
    registry.register(Box::new(LootCommand));
    registry.register(Box::new(EquipCommand));
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
    registry.register(Box::new(EncounterCommand));
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
    // Retainers
    registry.register(Box::new(HireCommand));
    registry.register(Box::new(RetainersCommand));
    registry.register(Box::new(DismissCommand));
    registry.register(Box::new(RetainerMoraleCommand));
    // GM
    registry.register(Box::new(AdvanceTurnCommand));
    registry.register(Box::new(AwardXpCommand));
    registry.register(Box::new(RulingCommand));
    registry.register(Box::new(HealCommand));
    registry.register(Box::new(DamageCommand));
    registry.register(Box::new(SetHpCommand));
    // Notes
    registry.register(Box::new(NoteCommand));
    registry.register(Box::new(NotesCommand));
    registry.register(Box::new(NoteDeleteCommand));
    // System
    registry.register(Box::new(RollCommand));
    registry.register(Box::new(SaveCommand));
    registry.register(Box::new(LoadCommand));
    registry.register(Box::new(HelpCommand));
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
