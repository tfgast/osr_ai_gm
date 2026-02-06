use crate::command::system::{LoadCommand, QuitCommand, RollCommand, SaveCommand};
use crate::command::{Command, CommandRegistry, CommandResult};
use crate::gmapi::interface::handle_request;
use crate::gmapi::protocol::{GMCommand, GMRequest, GMResponse};
use crate::persist::GameState;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct CliSnapshot {
    success: bool,
    quit: bool,
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
struct SystemCommandParitySnapshot {
    cli_command: &'static str,
    api_command: Option<&'static str>,
    class: &'static str,
    state_mutation_parity: &'static str,
    output_data_parity: &'static str,
    notes: &'static str,
    cli: CliSnapshot,
    api: Option<ApiSnapshot>,
}

fn run_api(command: GMCommand, state: &mut GameState) -> GMResponse {
    let request = GMRequest {
        id: "golden".to_string(),
        command,
    };
    handle_request(&request, state)
}

fn summarize_state(state: &GameState) -> Value {
    json!({
        "mode": state.mode.to_string(),
        "pre_combat_mode": state.pre_combat_mode.as_ref().map(|m| m.to_string()),
        "turn": state.turn(),
        "dungeon_level": state.dungeon_level,
        "party_size": state.party.members.len(),
        "rations": state.party.rations,
        "notes": state.notes,
        "combat_active": state.combat.is_some(),
    })
}

fn capture_parity(
    cli_command: &'static str,
    api_command_name: &'static str,
    initial_state: GameState,
    cli_exec: impl FnOnce(&mut GameState) -> CommandResult,
    api_command: GMCommand,
    class: &'static str,
    state_mutation_parity: &'static str,
    output_data_parity: &'static str,
    notes: &'static str,
) -> SystemCommandParitySnapshot {
    let mut cli_state = initial_state.clone();
    let cli_result = cli_exec(&mut cli_state);
    let cli = CliSnapshot {
        success: cli_result.success,
        quit: cli_result.quit,
        output: cli_result.output,
        state_after: summarize_state(&cli_state),
    };

    let mut api_state = initial_state;
    let api_result = run_api(api_command, &mut api_state);
    let api = ApiSnapshot {
        success: api_result.success,
        message: api_result.message,
        error: api_result.error,
        data: api_result.data,
        state_after: summarize_state(&api_state),
    };

    SystemCommandParitySnapshot {
        cli_command,
        api_command: Some(api_command_name),
        class,
        state_mutation_parity,
        output_data_parity,
        notes,
        cli,
        api: Some(api),
    }
}

fn capture_cli_only(
    cli_command: &'static str,
    initial_state: GameState,
    cli_exec: impl FnOnce(&mut GameState) -> CommandResult,
    class: &'static str,
    state_mutation_parity: &'static str,
    output_data_parity: &'static str,
    notes: &'static str,
) -> SystemCommandParitySnapshot {
    let mut cli_state = initial_state;
    let cli_result = cli_exec(&mut cli_state);
    let cli = CliSnapshot {
        success: cli_result.success,
        quit: cli_result.quit,
        output: cli_result.output,
        state_after: summarize_state(&cli_state),
    };

    SystemCommandParitySnapshot {
        cli_command,
        api_command: None,
        class,
        state_mutation_parity,
        output_data_parity,
        notes,
        cli,
        api: None,
    }
}

#[test]
fn system_command_parity_golden_scaffold_captures_snapshots() {
    let snapshots = vec![
        capture_parity(
            "roll invalid",
            "Roll",
            GameState::new(),
            |state| RollCommand.execute(&["not_a_roll"], state),
            GMCommand::Roll {
                notation: "not_a_roll".to_string(),
            },
            "B",
            "same",
            "different",
            "same parse failure and no state changes; CLI uses CommandResult error envelope and API returns structured response fields.",
        ),
        capture_parity(
            "save invalid path",
            "Save",
            GameState::new(),
            |state| SaveCommand.execute(&["bad/path"], state),
            GMCommand::Save {
                path: "bad/path".to_string(),
            },
            "B",
            "same",
            "different",
            "same validation failure; message text differs in punctuation/casing and API has response envelope fields.",
        ),
        capture_parity(
            "load invalid path",
            "Load",
            GameState::new(),
            |state| LoadCommand.execute(&["bad/path"], state),
            GMCommand::Load {
                path: "bad/path".to_string(),
            },
            "B",
            "same",
            "different",
            "same validation failure and unchanged state; adapter-level message formatting differs.",
        ),
        capture_parity(
            "quit",
            "Quit",
            GameState::new(),
            |state| QuitCommand.execute(&[], state),
            GMCommand::Quit,
            "B",
            "same",
            "different",
            "both end the session intentfully with no state mutation, but CLI signals quit via CommandResult.quit while API returns text in GMResponse.",
        ),
        capture_cli_only(
            "help",
            GameState::new(),
            |state| {
                let registry = CommandRegistry::new();
                registry.dispatch("help", &[], state)
            },
            "C",
            "n/a",
            "different",
            "CLI help is handled by CommandRegistry dispatch; GM API has no Help command variant, so no direct parity path exists in interface.rs.",
        ),
    ];

    assert_eq!(snapshots.len(), 5);
    assert_eq!(snapshots.iter().filter(|s| s.class == "B").count(), 4);
    assert_eq!(snapshots.iter().filter(|s| s.class == "C").count(), 1);

    for snapshot in &snapshots {
        assert!(!snapshot.cli.output.is_empty());
        if let Some(api) = &snapshot.api {
            assert!(!api.message.is_empty());
        }
    }

    let serialized = serde_json::to_string_pretty(&snapshots).unwrap();
    assert!(serialized.contains("\"roll invalid\""));
    assert!(serialized.contains("\"help\""));

    // Useful when running with --nocapture to inspect parity drift quickly.
    println!("{serialized}");
}
