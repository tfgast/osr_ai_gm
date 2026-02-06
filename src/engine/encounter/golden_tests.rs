use crate::command::encounter_cmds::{
    EncounterCommand, EvadeCommand, ReactionCommand, SurpriseCommand,
};
use crate::command::{Command, CommandResult};
use crate::gmapi::interface::handle_request;
use crate::gmapi::protocol::{GMCommand, GMRequest, GMResponse};
use crate::model::Character;
use crate::persist::GameState;
use crate::rules::class::Class;
use crate::state::dungeon::DungeonState;
use crate::state::game::GameMode;
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
    data_keys: Vec<String>,
    state_after: Value,
}

#[derive(Debug, Serialize)]
struct CommandParitySnapshot {
    command: &'static str,
    cli: CliSnapshot,
    api: ApiSnapshot,
    state_equal: bool,
}

#[derive(Debug, Serialize)]
struct CompatibilityGateRow {
    cli_command: &'static str,
    gm_api_command: &'static str,
    state_mutation_parity: &'static str,
    output_data_parity: &'static str,
    class: &'static str,
    notes: &'static str,
    snapshot: CommandParitySnapshot,
}

fn sorted_data_keys(data: &Option<Value>) -> Vec<String> {
    let mut keys = match data {
        Some(Value::Object(map)) => map.keys().cloned().collect(),
        _ => Vec::new(),
    };
    keys.sort();
    keys
}

fn run_api(command: GMCommand, state: &mut GameState) -> GMResponse {
    let request = GMRequest {
        id: "encounter-golden".to_string(),
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
                "hp": c.hp,
                "max_hp": c.max_hp,
                "alive": c.is_alive(),
                "move": c.movement_rate,
            })
        })
        .collect();

    json!({
        "mode": state.mode.to_string(),
        "dungeon_level": state.dungeon_level,
        "has_dungeon": state.dungeon.is_some(),
        "has_wilderness": state.wilderness.is_some(),
        "has_combat": state.combat.is_some(),
        "party": party,
    })
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
        data_keys: sorted_data_keys(&api_result.data),
        data: api_result.data,
        state_after: summarize_state(&api_state),
    };

    let state_equal = cli.state_after == api.state_after;
    CommandParitySnapshot {
        command,
        cli,
        api,
        state_equal,
    }
}

fn base_state() -> GameState {
    let mut state = GameState::new();

    let mut aldric = Character::new("Aldric", Class::Cleric);
    aldric.hp = 9;
    aldric.max_hp = 9;
    aldric.abilities.charisma = 14;

    let mut grond = Character::new("Grond", Class::Fighter);
    grond.hp = 12;
    grond.max_hp = 12;

    state.party.add_member(aldric);
    state.party.add_member(grond);
    state
}

fn state_for_dungeon_encounter() -> GameState {
    let mut state = base_state();
    state.mode = GameMode::Exploration;
    state.dungeon_level = 1;
    state.dungeon = Some(DungeonState::new(1));
    state
}

#[test]
fn encounter_command_parity_compatibility_gates() {
    let encounter = capture_parity(
        "encounter",
        state_for_dungeon_encounter(),
        |state| EncounterCommand.execute(&[], state),
        GMCommand::RollEncounter,
    );
    assert!(encounter.cli.success);
    assert!(encounter.api.success);
    assert!(encounter.state_equal);
    assert!(encounter.cli.output.contains("ENCOUNTER"));
    assert!(encounter.api.data_keys.contains(&"distance".to_string()));

    let surprise = capture_parity(
        "surprise",
        base_state(),
        |state| SurpriseCommand.execute(&[], state),
        GMCommand::RollSurprise,
    );
    assert!(surprise.cli.success);
    assert!(surprise.api.success);
    assert!(surprise.state_equal);
    assert_eq!(
        surprise.api.data_keys,
        vec!["monster_roll", "party_roll", "result"]
    );

    let reaction = capture_parity(
        "reaction",
        base_state(),
        |state| ReactionCommand.execute(&["Aldric"], state),
        GMCommand::RollReaction {
            character: "Aldric".to_string(),
        },
    );
    assert!(reaction.cli.success);
    assert!(reaction.api.success);
    assert!(reaction.state_equal);
    assert_eq!(
        reaction.api.data_keys,
        vec![
            "cha_modifier",
            "character",
            "charisma",
            "modified_roll",
            "raw_roll",
            "reaction",
        ]
    );

    let evade = capture_parity(
        "evade",
        base_state(),
        |state| EvadeCommand.execute(&["0", "120"], state),
        GMCommand::Evade {
            monster_count: 0,
            monster_movement: 120,
        },
    );
    assert!(!evade.cli.success);
    assert!(evade.api.success);
    assert!(evade.state_equal);
    assert_eq!(
        evade.api.data_keys,
        vec![
            "escaped",
            "monster_count",
            "monster_movement",
            "party_movement",
            "party_size",
        ]
    );

    let gates = vec![
        CompatibilityGateRow {
            cli_command: "encounter",
            gm_api_command: "RollEncounter",
            state_mutation_parity: "Same (read-only; no state mutation on either path)",
            output_data_parity:
                "CLI and API both report full encounter sequence; API adds structured data payload.",
            class: "B",
            notes: "Equivalent encounter flow with adapter-level response envelope differences.",
            snapshot: encounter,
        },
        CompatibilityGateRow {
            cli_command: "surprise",
            gm_api_command: "RollSurprise",
            state_mutation_parity: "Same (read-only)",
            output_data_parity:
                "Same surprise semantics; formatting differs and API includes structured fields.",
            class: "B",
            notes: "Shared encounter engine roll, adapter-level message/data differences only.",
            snapshot: surprise,
        },
        CompatibilityGateRow {
            cli_command: "reaction",
            gm_api_command: "RollReaction",
            state_mutation_parity: "Same (read-only)",
            output_data_parity:
                "Same CHA-modified reaction roll semantics; API adds explicit data fields.",
            class: "B",
            notes: "Shared `encounter_engine::reaction_roll` behavior with presentation differences.",
            snapshot: reaction,
        },
        CompatibilityGateRow {
            cli_command: "evade",
            gm_api_command: "Evade",
            state_mutation_parity: "Same (read-only)",
            output_data_parity:
                "Validation diverges: CLI rejects monster_count=0, API accepts and returns success payload.",
            class: "C",
            notes: "Input validation mismatch causes behavioral divergence on identical invalid input.",
            snapshot: evade,
        },
    ];

    assert_eq!(gates.len(), 4);
    assert_eq!(gates.iter().filter(|g| g.class == "B").count(), 3);
    assert_eq!(gates.iter().filter(|g| g.class == "C").count(), 1);
    assert!(gates.iter().all(|g| g.snapshot.state_equal));

    let serialized = serde_json::to_string_pretty(&gates).unwrap();
    assert!(serialized.contains("\"cli_command\": \"encounter\""));
    assert!(serialized.contains("\"cli_command\": \"evade\""));

    // Useful with --nocapture as a parity-audit artifact.
    println!("{serialized}");
}
