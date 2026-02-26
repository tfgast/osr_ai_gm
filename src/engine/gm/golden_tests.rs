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

    snapshot.cli.output == snapshot.api.message
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
            output_data_parity: "Same message, with API exposing additional typed fields (`module_name`, `level_range`, `room_count`).",
            class: "B",
            notes: "Both adapters call `exploration::action_load_module`; API adds structured response data.",
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

// ===========================================================================
// Unified dispatch parity tests (oag-mol-jqd)
//
// These tests verify that CLI and API paths produce identical state mutations
// when both call the same shared engine action.
// ===========================================================================

use crate::command::gm_cmds::AwardTreasureXpCommand;
use crate::command::system::{NoteCommand, NoteDeleteCommand, NotesCommand};
use crate::model::{AbilityScores, Character};


fn state_with_fighter() -> GameState {
    let mut state = GameState::new();
    let mut c = Character::new("Aldric", "Fighter");
    c.abilities = AbilityScores {
        strength: 16,
        intelligence: 10,
        wisdom: 10,
        dexterity: 10,
        constitution: 14,
        charisma: 10,
    };
    c.xp = 0;
    c.gold_gp = 500;
    c.hp = 8;
    c.max_hp = 8;
    state.party.add_member(c);
    state
}

fn state_with_notes() -> GameState {
    let mut state = GameState::new();
    state.notes.push("Clue one".to_string());
    state.notes.push("Clue two".to_string());
    state.notes.push("Clue three".to_string());
    state
}

/// Summarize state with party details for management command parity.
fn summarize_management_state(state: &GameState) -> Value {
    let party: Vec<Value> = state
        .party
        .members
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "xp": c.xp,
                "level": c.level,
                "gold_gp": c.gold_gp,
                "hp": c.hp,
                "max_hp": c.max_hp,
            })
        })
        .collect();

    json!({
        "mode": state.mode.to_string(),
        "party": party,
        "notes": state.notes,
        "notes_len": state.notes.len(),
    })
}

fn capture_management_parity(
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
        state_after: summarize_management_state(&cli_state),
    };

    let mut api_state = initial_state;
    let api_result = run_api(api_command, &mut api_state);
    let api = ApiSnapshot {
        success: api_result.success,
        message: api_result.message,
        error: api_result.error,
        data: api_result.data,
        state_after: summarize_management_state(&api_state),
    };

    CommandParitySnapshot { command, cli, api }
}

/// award_treasure_xp: both CLI and API call gm::action_award_treasure_xp.
/// State mutations must be identical.
#[test]
fn award_treasure_xp_parity() {
    let snapshot = capture_management_parity(
        "award_treasure_xp",
        state_with_fighter(),
        |state| AwardTreasureXpCommand.execute(&["Aldric", "500", "100"], state),
        GMCommand::AwardTreasureXp {
            character: "Aldric".to_string(),
            treasure_gp: 500,
            monster_xp: 100,
        },
    );

    assert!(
        snapshot.cli.success,
        "CLI award_treasure_xp should succeed: {}",
        snapshot.cli.output
    );
    assert!(
        snapshot.api.success,
        "API AwardTreasureXp should succeed: {}",
        snapshot.api.message
    );
    assert_eq!(
        snapshot.cli.state_after, snapshot.api.state_after,
        "award_treasure_xp: CLI and API should produce identical state"
    );

    // Both should report the same XP values
    assert!(
        snapshot.cli.output.contains("500gp treasure"),
        "CLI should mention treasure_gp"
    );
    assert!(
        snapshot.cli.output.contains("100xp monsters"),
        "CLI should mention monster_xp"
    );
}

/// list_notes: both CLI and API call gm::action_list_notes.
/// State is read-only, so both must succeed and see the same data.
#[test]
fn list_notes_parity() {
    let snapshot = capture_management_parity(
        "list_notes",
        state_with_notes(),
        |state| NotesCommand.execute(&[], state),
        GMCommand::ListNotes,
    );

    assert!(snapshot.cli.success);
    assert!(snapshot.api.success);
    assert_eq!(
        snapshot.cli.state_after, snapshot.api.state_after,
        "list_notes: state should be unchanged by both paths"
    );

    // Both should list the notes
    assert!(snapshot.cli.output.contains("Clue one"));
    assert!(snapshot.api.message.contains("Clue one"));
}

/// delete_note: both CLI and API call gm::action_delete_note.
/// State mutations must be identical.
#[test]
fn delete_note_parity() {
    let snapshot = capture_management_parity(
        "delete_note",
        state_with_notes(),
        |state| NoteDeleteCommand.execute(&["2"], state),
        GMCommand::DeleteNote { index: 2 },
    );

    assert!(
        snapshot.cli.success,
        "CLI note_delete should succeed: {}",
        snapshot.cli.output
    );
    assert!(
        snapshot.api.success,
        "API DeleteNote should succeed: {}",
        snapshot.api.message
    );
    assert_eq!(
        snapshot.cli.state_after, snapshot.api.state_after,
        "delete_note: CLI and API should produce identical state"
    );

    // Both should have 2 notes remaining
    assert_eq!(
        snapshot.cli.state_after["notes_len"], 2,
        "should have 2 notes after delete"
    );
}

/// Error path: both CLI and API should fail identically for nonexistent character.
#[test]
fn award_treasure_xp_error_parity() {
    let snapshot = capture_management_parity(
        "award_treasure_xp_error",
        GameState::new(),
        |state| AwardTreasureXpCommand.execute(&["Nobody", "100", "50"], state),
        GMCommand::AwardTreasureXp {
            character: "Nobody".to_string(),
            treasure_gp: 100,
            monster_xp: 50,
        },
    );

    assert!(!snapshot.cli.success, "CLI should fail for nonexistent character");
    assert!(!snapshot.api.success, "API should fail for nonexistent character");
    assert_eq!(
        snapshot.cli.state_after, snapshot.api.state_after,
        "error paths should leave state unchanged"
    );
}

/// Error path: delete_note out of range should fail identically.
#[test]
fn delete_note_error_parity() {
    let state = state_with_notes();
    let snapshot = capture_management_parity(
        "delete_note_error",
        state,
        |state| NoteDeleteCommand.execute(&["99"], state),
        GMCommand::DeleteNote { index: 99 },
    );

    assert!(!snapshot.cli.success, "CLI should fail for out-of-range index");
    assert!(!snapshot.api.success, "API should fail for out-of-range index");
    assert_eq!(
        snapshot.cli.state_after, snapshot.api.state_after,
        "error paths should leave state unchanged"
    );
}

/// add_note (CLI `note`) vs Ruling (API): these use DIFFERENT engine actions
/// by design. This test documents the intentional divergence — CLI uses
/// action_add_note (plain text) while API's Ruling uses action_ruling
/// ([RULING] prefix). Both add to state.notes but with different content.
#[test]
fn note_vs_ruling_intentional_divergence() {
    let text = "The bridge can hold 3 people";

    // CLI path: note command -> action_add_note (plain text)
    let mut cli_state = GameState::new();
    let cli_result = NoteCommand.execute(&["The", "bridge", "can", "hold", "3", "people"], &mut cli_state);
    assert!(cli_result.success);

    // API path: Ruling -> action_ruling ([RULING] prefix)
    let mut api_state = GameState::new();
    let api_result = run_api(GMCommand::Ruling { text: text.to_string() }, &mut api_state);
    assert!(api_result.success);

    // Both should add exactly one note
    assert_eq!(cli_state.notes.len(), 1);
    assert_eq!(api_state.notes.len(), 1);

    // Content diverges by design
    assert_eq!(cli_state.notes[0], text, "CLI note: plain text");
    assert_eq!(
        api_state.notes[0],
        format!("[RULING] {}", text),
        "API ruling: prefixed"
    );
}
