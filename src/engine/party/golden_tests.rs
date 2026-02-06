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

#[derive(Debug, Clone, Serialize)]
struct CliSnapshot {
    success: bool,
    output: String,
    state_after: Value,
}

#[derive(Debug, Clone, Serialize)]
struct ApiSnapshot {
    success: bool,
    message: String,
    error: Option<String>,
    data: Option<Value>,
    state_after: Value,
}

#[derive(Debug, Clone, Serialize)]
struct CommandParitySnapshot {
    command: &'static str,
    cli: CliSnapshot,
    api: ApiSnapshot,
}

#[derive(Debug, Clone, Serialize)]
struct CompatibilityGate {
    cli_command: &'static str,
    gm_api_command: &'static str,
    state_mutation_parity: &'static str,
    output_data_parity: &'static str,
    class: &'static str,
    notes: &'static str,
    state_equal: bool,
    output_key_fields_equal: bool,
    snapshot: CommandParitySnapshot,
}

fn capture_parity(
    command: &'static str,
    initial_state: GameState,
    cli_exec: impl FnOnce(&mut GameState) -> CommandResult,
    api_command: GMCommand,
) -> CommandParitySnapshot {
    let mut cli_state = initial_state.clone();
    let cli_result = cli_exec(&mut cli_state);
    let cli = CliSnapshot {
        success: cli_result.success,
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

    CommandParitySnapshot { command, cli, api }
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
        "party": party,
        "party_rations": state.party.rations,
        "days_without_food": state.party.days_without_food,
        "notes_len": state.notes.len(),
        "combat_active": state.combat.is_some(),
    })
}

fn empty_state() -> GameState {
    GameState::new()
}

fn state_with_party() -> GameState {
    let mut state = GameState::new();

    let mut fighter = Character::new("Bran", Class::Fighter);
    fighter.level = 2;
    fighter.hp = 9;
    fighter.max_hp = 11;
    fighter.ac = 4;
    fighter.thac0 = 19;
    fighter.xp = 2_200;

    let mut thief = Character::new("Nyx", Class::Thief);
    thief.level = 1;
    thief.hp = 0;
    thief.max_hp = 5;
    thief.ac = 6;
    thief.thac0 = 19;
    thief.xp = 100;

    state.party.add_member(fighter);
    state.party.add_member(thief);
    state.party.rations = 0;
    state.party.days_without_food = 2;
    state
}

fn parse_cli_class_names(output: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in output.lines() {
        if !line.starts_with("  ") {
            continue;
        }
        let Some(rest) = line.strip_prefix("  ") else {
            continue;
        };
        let Some((name, _)) = rest.split_once("HD:") else {
            continue;
        };
        let name = name.trim();
        if !name.is_empty() {
            names.push(name.to_string());
        }
    }
    names
}

fn parse_api_class_names(data: &Option<Value>) -> Vec<String> {
    data.as_ref()
        .and_then(|d| d.get("classes"))
        .and_then(|v| v.as_array())
        .map(|classes| {
            classes
                .iter()
                .filter_map(|c| c.get("name").and_then(Value::as_str))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_cli_eligible_names(output: &str) -> Vec<String> {
    for line in output.lines() {
        if !line.starts_with("Eligible classes (") {
            continue;
        }
        let Some((_, names_str)) = line.split_once(": ") else {
            continue;
        };
        return names_str
            .split(", ")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
    }
    Vec::new()
}

fn parse_api_eligible_names(data: &Option<Value>) -> Vec<String> {
    data.as_ref()
        .and_then(|d| d.get("eligible"))
        .and_then(|v| v.as_array())
        .map(|eligible| {
            eligible
                .iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_cli_party_members(output: &str) -> Vec<(String, String, u32)> {
    let mut members = Vec::new();
    for line in output.lines() {
        if !line.starts_with("  ") {
            continue;
        }
        let trimmed = line.trim();
        let Some((name, rest)) = trimmed.split_once(" (") else {
            continue;
        };
        let Some((class_and_level, _)) = rest.split_once(')') else {
            continue;
        };
        let Some((class, level_str)) = class_and_level.rsplit_once(" L") else {
            continue;
        };
        let Ok(level) = level_str.parse::<u32>() else {
            continue;
        };
        members.push((name.to_string(), class.to_string(), level));
    }
    members
}

fn parse_api_party_members(data: &Option<Value>) -> Vec<(String, String, u32)> {
    data.as_ref()
        .and_then(|d| d.get("members"))
        .and_then(|v| v.as_array())
        .map(|members| {
            members
                .iter()
                .filter_map(|m| {
                    let name = m.get("name")?.as_str()?;
                    let class = m.get("class")?.as_str()?;
                    let level = m.get("level")?.as_u64()? as u32;
                    Some((name.to_string(), class.to_string(), level))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn party_chargen_command_parity_golden_scaffold_captures_snapshots() {
    let chargen_snapshot = capture_parity(
        "chargen",
        empty_state(),
        |state| {
            ChargenCommand.execute(
                &[
                    "SirGavin",
                    "Paladin",
                    "Lawful",
                    "--abilities",
                    "9",
                    "9",
                    "9",
                    "9",
                    "9",
                    "8",
                ],
                state,
            )
        },
        GMCommand::CreateCharacter {
            name: "SirGavin".to_string(),
            class: Class::Paladin,
            alignment: Alignment::Lawful,
            abilities: Some([9, 9, 9, 9, 9, 8]),
        },
    );

    let classes_snapshot = capture_parity(
        "classes",
        state_with_party(),
        |state| ClassesCommand.execute(&[], state),
        GMCommand::ListClasses,
    );

    let eligible_snapshot = capture_parity(
        "eligible",
        state_with_party(),
        |state| EligibleCommand.execute(&["16", "13", "10", "12", "14", "12"], state),
        GMCommand::EligibleClasses {
            abilities: [16, 13, 10, 12, 14, 12],
        },
    );

    let party_snapshot = capture_parity(
        "party",
        state_with_party(),
        |state| PartyCommand.execute(&[], state),
        GMCommand::QueryParty,
    );

    let chargen_state_equal = chargen_snapshot.cli.state_after == chargen_snapshot.api.state_after;
    let chargen_output_key_fields_equal = false;

    let classes_state_equal = classes_snapshot.cli.state_after == classes_snapshot.api.state_after;
    let classes_output_key_fields_equal = parse_cli_class_names(&classes_snapshot.cli.output)
        == parse_api_class_names(&classes_snapshot.api.data);

    let eligible_state_equal =
        eligible_snapshot.cli.state_after == eligible_snapshot.api.state_after;
    let eligible_output_key_fields_equal = parse_cli_eligible_names(&eligible_snapshot.cli.output)
        == parse_api_eligible_names(&eligible_snapshot.api.data);

    let party_state_equal = party_snapshot.cli.state_after == party_snapshot.api.state_after;
    let party_output_key_fields_equal = parse_cli_party_members(&party_snapshot.cli.output)
        == parse_api_party_members(&party_snapshot.api.data);

    let gates = vec![
        CompatibilityGate {
            cli_command: "chargen",
            gm_api_command: "CreateCharacter",
            state_mutation_parity: "Same state mutation (none)",
            output_data_parity: "Divergent success contract: CLI returns explanatory success text, API returns structured error.",
            class: "C",
            notes: "When provided abilities fail class requirements, CLI returns OK text with eligible classes while API returns error.",
            state_equal: chargen_state_equal,
            output_key_fields_equal: chargen_output_key_fields_equal,
            snapshot: chargen_snapshot,
        },
        CompatibilityGate {
            cli_command: "classes",
            gm_api_command: "ListClasses",
            state_mutation_parity: "Same state mutation (read-only)",
            output_data_parity: "Same class set with different adapter formatting (CLI table vs API JSON payload).",
            class: "B",
            notes: "Both paths expose the same class list and requirements without mutating game state.",
            state_equal: classes_state_equal,
            output_key_fields_equal: classes_output_key_fields_equal,
            snapshot: classes_snapshot,
        },
        CompatibilityGate {
            cli_command: "eligible",
            gm_api_command: "EligibleClasses",
            state_mutation_parity: "Same state mutation (read-only)",
            output_data_parity: "Same eligible class list with different adapter formatting (CLI summary vs API abilities+eligible JSON).",
            class: "B",
            notes: "Eligibility evaluation logic is shared; API exposes additional structured fields.",
            state_equal: eligible_state_equal,
            output_key_fields_equal: eligible_output_key_fields_equal,
            snapshot: eligible_snapshot,
        },
        CompatibilityGate {
            cli_command: "party",
            gm_api_command: "QueryParty",
            state_mutation_parity: "Same state mutation (read-only)",
            output_data_parity: "Same roster core fields with different adapter formatting (CLI status text vs API member objects).",
            class: "B",
            notes: "Both paths report the same party roster while formatting XP/starvation context differently.",
            state_equal: party_state_equal,
            output_key_fields_equal: party_output_key_fields_equal,
            snapshot: party_snapshot,
        },
    ];

    assert_eq!(gates.len(), 4);
    assert_eq!(gates.iter().filter(|g| g.class == "C").count(), 1);
    assert_eq!(gates.iter().filter(|g| g.class == "B").count(), 3);

    for gate in &gates {
        assert!(
            gate.state_equal,
            "{} should keep parity for state mutation",
            gate.cli_command
        );
        assert!(
            !gate.snapshot.cli.output.is_empty(),
            "{} should capture non-empty CLI output",
            gate.cli_command
        );
        assert!(
            !gate.snapshot.api.message.is_empty(),
            "{} should capture non-empty API message",
            gate.cli_command
        );
    }

    assert!(gates[0].snapshot.cli.success);
    assert!(!gates[0].snapshot.api.success);
    assert!(!gates[0].output_key_fields_equal);
    assert!(gates[0]
        .snapshot
        .cli
        .output
        .contains("do not meet requirements for Paladin"));
    assert!(gates[0]
        .snapshot
        .api
        .message
        .contains("do not meet requirements for Paladin"));
    assert!(gates[1].output_key_fields_equal);
    assert!(gates[2].output_key_fields_equal);
    assert!(gates[3].output_key_fields_equal);

    let serialized = serde_json::to_string_pretty(&gates).unwrap();
    assert!(serialized.contains("\"cli_command\": \"chargen\""));
    assert!(serialized.contains("\"gm_api_command\": \"QueryParty\""));

    // Useful with --nocapture for quick audit review.
    println!("{serialized}");
}
