use crate::command::lookup_cmds::TreasureTypeCommand;
use crate::command::module_cmds::LoadModuleCommand;
use crate::command::treasure_cmds::TreasureCommand;
use crate::command::{Command, CommandResult};
use crate::gmapi::interface::handle_request;
use crate::gmapi::protocol::{GMCommand, GMRequest, GMResponse};
use crate::persist::GameState;
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
    let dungeon = state.dungeon.as_ref().map(|d| {
        let mut rooms = d.rooms.iter().collect::<Vec<_>>();
        rooms.sort_by_key(|r| r.id);
        let rooms = rooms
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "name": r.name,
                    "key": r.key,
                    "placed_monsters": r.placed_monsters.len(),
                    "placed_treasure": r.placed_treasure.len(),
                })
            })
            .collect::<Vec<_>>();

        let mut doors = d.doors.iter().collect::<Vec<_>>();
        doors.sort_by_key(|door| door.id);
        let doors = doors
            .iter()
            .map(|door| {
                json!({
                    "id": door.id,
                    "room_a": door.room_a,
                    "room_b": door.room_b,
                    "state": door.state.to_string(),
                    "discovered": door.discovered,
                })
            })
            .collect::<Vec<_>>();

        let mut explored = d.explored.iter().copied().collect::<Vec<_>>();
        explored.sort_unstable();

        json!({
            "level": d.level,
            "room_count": d.rooms.len(),
            "door_count": d.doors.len(),
            "current_room": d.current_room,
            "explored": explored,
            "rooms": rooms,
            "doors": doors,
            "log_len": d.log.len(),
        })
    });

    let time = state.time.as_ref().map(|t| {
        let lights = t
            .lights
            .iter()
            .map(|light| {
                json!({
                    "kind": light.kind.name(),
                    "carrier": light.carrier,
                    "remaining_turns": light.remaining_turns,
                })
            })
            .collect::<Vec<_>>();

        json!({
            "total_turns": t.total_turns,
            "turns_since_rest": t.turns_since_rest,
            "lights": lights,
            "log_len": t.log.len(),
        })
    });

    json!({
        "mode": state.mode.to_string(),
        "dungeon_level": state.dungeon_level,
        "dungeon": dungeon,
        "time": time,
        "combat_active": state.combat.is_some(),
        "party_size": state.party.members.len(),
        "notes_len": state.notes.len(),
    })
}

fn treasure_type_key_fields_equal(snapshot: &CommandParitySnapshot) -> bool {
    let data = match snapshot.api.data.as_ref() {
        Some(data) => data,
        None => return false,
    };
    let letter = match data.get("letter").and_then(Value::as_str) {
        Some(letter) => letter,
        None => return false,
    };
    let category = match data.get("category").and_then(Value::as_str) {
        Some(category) => category,
        None => return false,
    };
    let average_gp = match data.get("average_gp").and_then(Value::as_f64) {
        Some(average) => average.round() as u64,
        None => return false,
    };

    snapshot
        .cli
        .output
        .contains(&format!("Treasure Type {letter}"))
        && snapshot
            .cli
            .output
            .contains(&format!("Category: {category}"))
        && snapshot
            .cli
            .output
            .contains(&format!("Average Value: {average_gp} gp"))
}

fn parse_cli_treasure_roll_header(output: &str) -> Option<(String, String)> {
    let title = output.lines().next()?;
    let rest = title.strip_prefix("TREASURE TYPE ")?;
    let (letter, category_part) = rest.split_once(" (")?;
    let category = category_part.strip_suffix(')')?;
    Some((letter.to_string(), category.to_string()))
}

fn parse_api_treasure_roll_header(data: &Option<Value>) -> Option<(String, String)> {
    let data = data.as_ref()?;
    Some((
        data.get("letter")?.as_str()?.to_string(),
        data.get("category")?.as_str()?.to_string(),
    ))
}

fn load_module_key_fields_equal(snapshot: &CommandParitySnapshot) -> bool {
    let data = match snapshot.api.data.as_ref() {
        Some(data) => data,
        None => return false,
    };
    let module_name = match data.get("module_name").and_then(Value::as_str) {
        Some(name) => name,
        None => return false,
    };
    let room_count = match data.get("room_count").and_then(Value::as_u64) {
        Some(count) => count,
        None => return false,
    };

    snapshot.cli.output.starts_with(&snapshot.api.message)
        && snapshot.cli.output.contains(module_name)
        && snapshot.cli.output.contains(&format!("{room_count} rooms"))
}

#[test]
fn treasure_module_command_parity_golden_scaffold_captures_snapshots() {
    let treasure_type_snapshot = capture_parity(
        "treasure_type",
        GameState::new(),
        |state| TreasureTypeCommand.execute(&["A"], state),
        GMCommand::LookupTreasureType {
            letter: "A".to_string(),
        },
    );

    let treasure_roll_snapshot = capture_parity(
        "treasure",
        GameState::new(),
        |state| TreasureCommand.execute(&["P"], state),
        GMCommand::RollTreasure {
            letter: "P".to_string(),
        },
    );

    let load_module_snapshot = capture_parity(
        "load_module",
        GameState::new(),
        |state| {
            LoadModuleCommand.execute(&["data/modules/sample_crypt/module.json"], state)
        },
        GMCommand::LoadModule {
            path: "data/modules/sample_crypt/module.json".to_string(),
        },
    );

    let treasure_type_state_equal =
        treasure_type_snapshot.cli.state_after == treasure_type_snapshot.api.state_after;
    let treasure_type_output_key_fields_equal =
        treasure_type_key_fields_equal(&treasure_type_snapshot);

    let treasure_roll_state_equal =
        treasure_roll_snapshot.cli.state_after == treasure_roll_snapshot.api.state_after;
    let treasure_roll_output_key_fields_equal =
        parse_cli_treasure_roll_header(&treasure_roll_snapshot.cli.output)
            == parse_api_treasure_roll_header(&treasure_roll_snapshot.api.data);

    let load_module_state_equal =
        load_module_snapshot.cli.state_after == load_module_snapshot.api.state_after;
    let load_module_output_key_fields_equal =
        load_module_key_fields_equal(&load_module_snapshot);

    let gates = vec![
        CompatibilityGate {
            cli_command: "treasure_type",
            gm_api_command: "LookupTreasureType",
            state_mutation_parity: "Same state mutation (read-only)",
            output_data_parity: "Same treasure metadata with different adapter formatting (CLI table vs API JSON payload).",
            class: "B",
            notes: "Core treasure table lookup matches; API provides structured entries while CLI renders prose.",
            state_equal: treasure_type_state_equal,
            output_key_fields_equal: treasure_type_output_key_fields_equal,
            snapshot: treasure_type_snapshot,
        },
        CompatibilityGate {
            cli_command: "treasure",
            gm_api_command: "RollTreasure",
            state_mutation_parity: "Same state mutation (read-only)",
            output_data_parity: "Same roll intent with different adapter contract (CLI formatted haul vs API itemized payload).",
            class: "B",
            notes: "Both paths roll the same treasure type; RNG outcomes differ independently, so parity checks anchor on shared header fields.",
            state_equal: treasure_roll_state_equal,
            output_key_fields_equal: treasure_roll_output_key_fields_equal,
            snapshot: treasure_roll_snapshot,
        },
        CompatibilityGate {
            cli_command: "load_module",
            gm_api_command: "LoadModule",
            state_mutation_parity: "Same dungeon/time/mode mutation via shared engine action",
            output_data_parity: "CLI prepends API message then appends onboarding hints; API exposes additional typed fields (`module_name`, `level_range`, `room_count`).",
            class: "B",
            notes: "Both adapters call `exploration::action_load_module`; CLI adds onboarding guidance lines, API adds structured response data.",
            state_equal: load_module_state_equal,
            output_key_fields_equal: load_module_output_key_fields_equal,
            snapshot: load_module_snapshot,
        },
    ];

    assert_eq!(gates.len(), 3);
    assert_eq!(gates.iter().filter(|g| g.class == "B").count(), 3);

    for gate in &gates {
        assert!(
            gate.state_equal,
            "{} should keep parity for state mutation",
            gate.cli_command
        );
        assert!(
            gate.output_key_fields_equal,
            "{} should keep parity for key output/data fields",
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
        assert!(
            gate.snapshot.cli.success,
            "{} CLI path should succeed for this golden input",
            gate.cli_command
        );
        assert!(
            gate.snapshot.api.success,
            "{} API path should succeed for this golden input",
            gate.gm_api_command
        );
    }

    let serialized = serde_json::to_string_pretty(&gates).unwrap();
    assert!(serialized.contains("\"cli_command\": \"treasure_type\""));
    assert!(serialized.contains("\"gm_api_command\": \"RollTreasure\""));
    assert!(serialized.contains("\"gm_api_command\": \"LoadModule\""));

    // Useful with --nocapture for quick audit review.
    println!("{serialized}");
}
