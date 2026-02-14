use crate::command::retainer_cmds::{
    DismissCommand, HireCommand, RetainerMoraleCommand, RetainersCommand,
};
use crate::command::{Command, CommandResult};
use crate::engine::retainer::Retainer;
use crate::gmapi::interface::handle_request;
use crate::gmapi::protocol::{GMCommand, GMRequest, GMResponse};
use crate::model::Character;
use crate::persist::GameState;
use crate::rules::class::Class;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum GateStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum ParityClass {
    A,
    B,
    C,
}

#[derive(Debug, Serialize)]
struct CompatibilityGates {
    state_mutation_parity: GateStatus,
    output_parity: GateStatus,
    data_fields_parity: GateStatus,
}

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
    cli_command: &'static str,
    api_command: &'static str,
    class: ParityClass,
    gates: CompatibilityGates,
    notes: &'static str,
    cli: CliSnapshot,
    api: ApiSnapshot,
}

fn run_api(command: GMCommand, state: &mut GameState) -> GMResponse {
    let request = GMRequest {
        id: "retainer-golden".to_string(),
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
                "charisma": c.abilities.charisma,
            })
        })
        .collect();

    let retainers: Vec<Value> = state
        .retainers
        .iter()
        .map(|r| {
            json!({
                "name": r.name,
                "class": r.class,
                "level": r.level,
                "hp": r.hp,
                "max_hp": r.max_hp,
                "loyalty": r.loyalty,
                "wage_gp": r.wage_gp,
                "alive": r.is_alive(),
            })
        })
        .collect();

    json!({
        "mode": state.mode.to_string(),
        "party": party,
        "retainers": retainers,
    })
}

fn output_parity(cli: &CliSnapshot, api: &ApiSnapshot) -> bool {
    if cli.success != api.success {
        return false;
    }

    if cli.success {
        return cli.output.trim() == api.message.trim();
    }

    let cli_error = cli.output.strip_prefix("Error: ").unwrap_or(&cli.output);
    cli_error.trim() == api.message.trim()
}

fn classify(
    cli_success: bool,
    api_success: bool,
    state_parity: bool,
    output_parity: bool,
    data_parity: bool,
) -> ParityClass {
    if !state_parity || cli_success != api_success {
        ParityClass::C
    } else if output_parity && data_parity {
        ParityClass::A
    } else {
        ParityClass::B
    }
}

fn capture_parity(
    cli_command: &'static str,
    api_command: &'static str,
    initial_state: GameState,
    cli_exec: impl FnOnce(&mut GameState) -> CommandResult,
    api_exec: GMCommand,
    notes: &'static str,
    data_fields_check: impl Fn(Option<&Value>) -> bool,
) -> CommandParitySnapshot {
    let mut cli_state = initial_state.clone();
    let cli_result = cli_exec(&mut cli_state);
    let cli = CliSnapshot {
        success: cli_result.success,
        output: cli_result.output,
        state_after: summarize_state(&cli_state),
    };

    let mut api_state = initial_state;
    let api_result = run_api(api_exec, &mut api_state);
    let api = ApiSnapshot {
        success: api_result.success,
        message: api_result.message,
        error: api_result.error,
        data: api_result.data,
        state_after: summarize_state(&api_state),
    };

    let state_parity = cli.state_after == api.state_after;
    let output_gate = output_parity(&cli, &api);
    let data_gate = data_fields_check(api.data.as_ref());

    let class = classify(
        cli.success,
        api.success,
        state_parity,
        output_gate,
        data_gate,
    );

    CommandParitySnapshot {
        cli_command,
        api_command,
        class,
        gates: CompatibilityGates {
            state_mutation_parity: if state_parity {
                GateStatus::Pass
            } else {
                GateStatus::Fail
            },
            output_parity: if output_gate {
                GateStatus::Pass
            } else {
                GateStatus::Fail
            },
            data_fields_parity: if data_gate {
                GateStatus::Pass
            } else {
                GateStatus::Fail
            },
        },
        notes,
        cli,
        api,
    }
}

fn base_state(charisma: i32) -> GameState {
    let mut state = GameState::new();
    let mut employer = Character::new("Aldric", Class::Fighter);
    employer.abilities.charisma = charisma;
    state.party.add_member(employer);
    state
}

fn state_at_retainer_cap() -> GameState {
    let mut state = base_state(14);
    for idx in 1..=5 {
        state
            .retainers
            .push(Retainer::new(&format!("R{idx}"), Class::Fighter, 1, 4, 7, 25));
    }
    state
}

fn state_with_one_retainer() -> GameState {
    let mut state = base_state(12);
    state
        .retainers
        .push(Retainer::new("Hob", Class::Fighter, 1, 6, 7, 25));
    state
}

fn has_required_keys(data: Option<&Value>, keys: &[&str]) -> bool {
    let Some(obj) = data.and_then(Value::as_object) else {
        return false;
    };
    keys.iter().all(|k| obj.contains_key(*k))
}

fn hire_data_fields(data: Option<&Value>) -> bool {
    if !has_required_keys(
        data,
        &[
            "employer",
            "retainer",
            "class",
            "level",
            "reaction",
            "hired",
            "loyalty",
            "wage_gp",
            "max_retainers",
        ],
    ) {
        return false;
    }

    let Some(obj) = data.and_then(Value::as_object) else {
        return false;
    };

    obj.get("employer") == Some(&json!("Aldric"))
        && obj.get("retainer") == Some(&json!("Sven"))
        && obj.get("class") == Some(&json!("Fighter"))
        && obj.get("level") == Some(&json!(1))
        && obj.get("max_retainers") == Some(&json!(5))
}

fn list_data_fields(data: Option<&Value>) -> bool {
    let Some(obj) = data.and_then(Value::as_object) else {
        return false;
    };
    let Some(retainers) = obj.get("retainers").and_then(Value::as_array) else {
        return false;
    };
    if retainers.len() != 1 {
        return false;
    }

    retainers[0].get("name") == Some(&json!("Hob"))
        && retainers[0].get("class") == Some(&json!("Fighter"))
}

fn dismiss_data_fields(data: Option<&Value>) -> bool {
    if !has_required_keys(data, &["name", "class"]) {
        return false;
    }
    let Some(obj) = data.and_then(Value::as_object) else {
        return false;
    };
    obj.get("name") == Some(&json!("Hob")) && obj.get("class") == Some(&json!("Fighter"))
}

fn loyalty_data_fields(data: Option<&Value>) -> bool {
    if !has_required_keys(data, &["retainer", "loyalty", "result"]) {
        return false;
    }
    let Some(obj) = data.and_then(Value::as_object) else {
        return false;
    };
    obj.get("retainer") == Some(&json!("Hob")) && obj.get("loyalty") == Some(&json!(7))
}

fn parity_for<'a>(
    snapshots: &'a [CommandParitySnapshot],
    cli_command: &str,
) -> &'a CommandParitySnapshot {
    snapshots
        .iter()
        .find(|s| s.cli_command == cli_command)
        .unwrap_or_else(|| panic!("missing snapshot for command '{}'", cli_command))
}

#[test]
fn retainer_command_parity_golden_snapshots_capture_compatibility_gates() {
    let snapshots = vec![
        capture_parity(
            "hire",
            "HireRetainer",
            state_at_retainer_cap(),
            |state| HireCommand.execute(&["Sven", "Fighter", "Aldric"], state),
            GMCommand::HireRetainer {
                employer: "Aldric".to_string(),
                retainer_name: "Sven".to_string(),
                retainer_class: Class::Fighter,
                retainer_level: 1,
            },
            "CLI enforces max-retainer cap before rolling; API inline path does not.",
            hire_data_fields,
        ),
        capture_parity(
            "retainers",
            "ListRetainers",
            state_with_one_retainer(),
            |state| RetainersCommand.execute(&[], state),
            GMCommand::ListRetainers,
            "Read-only list behavior is aligned for populated retainer rosters.",
            list_data_fields,
        ),
        capture_parity(
            "dismiss",
            "DismissRetainer",
            state_with_one_retainer(),
            |state| DismissCommand.execute(&["Hob"], state),
            GMCommand::DismissRetainer {
                name: "Hob".to_string(),
            },
            "Both paths remove the same retainer and report equivalent outcomes.",
            dismiss_data_fields,
        ),
        capture_parity(
            "retainer_morale",
            "LoyaltyCheck",
            base_state(12),
            |state| RetainerMoraleCommand.execute(&["Hob"], state),
            GMCommand::LoyaltyCheck {
                retainer_name: "Hob".to_string(),
                loyalty: 7,
            },
            "CLI validates against in-state retainers; API inline command rolls from provided loyalty only.",
            loyalty_data_fields,
        ),
    ];

    assert_eq!(snapshots.len(), 4);

    assert_eq!(parity_for(&snapshots, "hire").class, ParityClass::C);
    assert_eq!(parity_for(&snapshots, "retainers").class, ParityClass::A);
    assert_eq!(parity_for(&snapshots, "dismiss").class, ParityClass::A);
    assert_eq!(
        parity_for(&snapshots, "retainer_morale").class,
        ParityClass::C
    );

    let serialized = serde_json::to_string_pretty(&snapshots).unwrap();
    assert!(serialized.contains("\"cli_command\": \"hire\""));
    assert!(serialized.contains("\"cli_command\": \"retainer_morale\""));

    // Useful when running with --nocapture to inspect parity drift quickly.
    println!("{serialized}");
}
