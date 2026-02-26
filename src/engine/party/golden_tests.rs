use crate::command::party::{ChargenCommand, ClassesCommand, EligibleCommand, PartyCommand};
use crate::command::{Command, CommandResult};
use crate::gmapi::interface::handle_request;
use crate::gmapi::protocol::{GMCommand, GMRequest, GMResponse};
use crate::model::Character;
use crate::persist::GameState;
use crate::rules::alignment::Alignment;
use crate::rules::class::Class;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct CliSnapshot {
    success: bool,
    output: String,
    state_after: Value,
}

#[derive(Debug, Serialize)]
struct ApiSnapshot {
    success: bool,
    message: String,
    error: Option<String>,
    data: Option<Value>,
    state_after: Value,
}

#[derive(Debug, Serialize)]
struct CommandParitySnapshot {
    command: &'static str,
    api_command: &'static str,
    parity_class: &'static str,
    divergence: &'static str,
    cli: CliSnapshot,
    api: ApiSnapshot,
}

fn capture_parity(
    command: &'static str,
    api_command: &'static str,
    parity_class: &'static str,
    divergence: &'static str,
    initial_state: GameState,
    cli_exec: impl FnOnce(&mut GameState) -> CommandResult,
    api_command_value: GMCommand,
) -> CommandParitySnapshot {
    let mut cli_state = initial_state.clone();
    let cli_result = cli_exec(&mut cli_state);
    let cli = CliSnapshot {
        success: cli_result.success,
        output: cli_result.output,
        state_after: summarize_state(&cli_state),
    };

    let mut api_state = initial_state;
    let api_result = run_api(api_command_value, &mut api_state);
    let api = ApiSnapshot {
        success: api_result.success,
        message: api_result.message,
        error: api_result.error,
        data: api_result.data,
        state_after: summarize_state(&api_state),
    };

    CommandParitySnapshot {
        command,
        api_command,
        parity_class,
        divergence,
        cli,
        api,
    }
}

fn run_api(command: GMCommand, state: &mut GameState) -> GMResponse {
    let request = GMRequest {
        id: "golden".to_string(),
        command,
    };
    handle_request(&request, state)
}

fn summarize_state(state: &GameState) -> Value {
    let party: Vec<Value> = state
        .party
        .members
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "class": c.class.name(),
                "level": c.level,
                "hp": c.hp,
                "max_hp": c.max_hp,
                "ac": c.ac,
                "thac0": c.thac0,
                "xp": c.xp,
                "alive": c.is_alive(),
                "alignment": c.alignment.name(),
                "movement_rate": c.movement_rate,
            })
        })
        .collect();

    json!({
        "mode": state.mode.to_string(),
        "party_size": state.party.members.len(),
        "days_without_food": state.party.days_without_food,
        "party": party,
    })
}

fn state_with_party_and_starvation() -> GameState {
    let mut state = GameState::new();
    let mut fighter = Character::new("Grond", Class::Fighter);
    fighter.level = 1;
    fighter.xp = 2_000;
    fighter.hp = 8;
    fighter.max_hp = 8;
    fighter.ac = 4;
    state.party.add_member(fighter);
    state.party.days_without_food = 3;
    state
}

#[test]
fn party_command_parity_golden_scaffold_captures_snapshots() {
    let snapshots = vec![
        capture_parity(
            "chargen",
            "CreateCharacter",
            "C",
            "CLI returns success output for ineligible abilities, API returns an error response.",
            GameState::new(),
            |state| {
                ChargenCommand.execute(
                    &[
                        "Borin",
                        "Knight",
                        "Neutral",
                        "--abilities",
                        "8",
                        "8",
                        "8",
                        "8",
                        "8",
                        "8",
                    ],
                    state,
                )
            },
            GMCommand::CreateCharacter {
                name: "Borin".to_string(),
                class: Class::Knight.into(),
                alignment: Alignment::Neutral,
                abilities: Some([8, 8, 8, 8, 8, 8]),
            },
        ),
        capture_parity(
            "classes",
            "ListClasses",
            "B",
            "Shared class definitions, but CLI text formatting differs from API message + structured class payload.",
            GameState::new(),
            |state| ClassesCommand.execute(&[], state),
            GMCommand::ListClasses,
        ),
        capture_parity(
            "eligible",
            "EligibleClasses",
            "B",
            "Shared eligibility rules, but output contracts differ (CLI text list vs API count + structured abilities/eligible fields).",
            GameState::new(),
            |state| EligibleCommand.execute(&["13", "12", "11", "10", "9", "8"], state),
            GMCommand::EligibleClasses {
                abilities: [13, 12, 11, 10, 9, 8],
            },
        ),
        capture_parity(
            "party",
            "QueryParty",
            "B",
            "Both read current party state, but CLI emits readiness/starvation annotations while API emits a normalized member payload.",
            state_with_party_and_starvation(),
            |state| PartyCommand.execute(&[], state),
            GMCommand::QueryParty,
        ),
    ];

    assert_eq!(snapshots.len(), 4);
    for snapshot in &snapshots {
        assert!(!snapshot.command.is_empty());
        assert!(!snapshot.api_command.is_empty());
        assert!(matches!(snapshot.parity_class, "A" | "B" | "C"));
        assert!(!snapshot.cli.output.is_empty());
        assert!(!snapshot.api.message.is_empty());
    }

    let chargen = snapshots
        .iter()
        .find(|snapshot| snapshot.command == "chargen")
        .expect("missing chargen snapshot");
    assert!(chargen.cli.success);
    assert!(!chargen.api.success);
    assert_eq!(chargen.cli.state_after, chargen.api.state_after);

    let serialized = serde_json::to_string_pretty(&snapshots).unwrap();
    assert!(serialized.contains("\"chargen\""));
    assert!(serialized.contains("\"QueryParty\""));

    // Useful when running with --nocapture to inspect parity drift quickly.
    println!("{serialized}");
}
