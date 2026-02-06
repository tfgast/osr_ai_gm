use crate::command::inventory_cmds::{BuyCommand, DropCommand, EquipCommand, LootCommand};
use crate::command::{Command, CommandResult};
use crate::gmapi::interface::handle_request;
use crate::gmapi::protocol::{GMCommand, GMRequest, GMResponse};
use crate::model::{Character, Item};
use crate::persist::GameState;
use crate::rules::class::Class;
use crate::state::dungeon::{DungeonState, PlacedTreasureInstance, Room};
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
    cli_output_pre_migration_equal: bool,
    api_payload_shape_matches_typed_result: bool,
    snapshot: CommandParitySnapshot,
}

fn run_api(command: GMCommand, state: &mut GameState) -> GMResponse {
    let request = GMRequest {
        id: "inventory-golden".to_string(),
        command,
    };
    handle_request(&request, state)
}

fn summarize_state(state: &GameState) -> Value {
    let party = state
        .party
        .members
        .iter()
        .map(|member| {
            let inventory = member
                .inventory
                .iter()
                .map(|item| {
                    json!({
                        "name": item.name,
                        "value_gp": item.value_gp,
                        "equipped": item.equipped,
                    })
                })
                .collect::<Vec<_>>();

            json!({
                "name": member.name,
                "ac": member.ac,
                "gold_gp": member.gold_gp,
                "inventory": inventory,
            })
        })
        .collect::<Vec<_>>();

    let dungeon = state.dungeon.as_ref().map(|dungeon| {
        let mut rooms = dungeon.rooms.iter().collect::<Vec<_>>();
        rooms.sort_by_key(|room| room.id);
        let rooms = rooms
            .iter()
            .map(|room| {
                let placed_treasure = room
                    .placed_treasure
                    .iter()
                    .map(|treasure| {
                        json!({
                            "description": treasure.description,
                            "gp_value": treasure.gp_value,
                            "taken": treasure.taken,
                        })
                    })
                    .collect::<Vec<_>>();

                json!({
                    "id": room.id,
                    "name": room.name,
                    "placed_treasure": placed_treasure,
                })
            })
            .collect::<Vec<_>>();

        json!({
            "current_room": dungeon.current_room,
            "rooms": rooms,
        })
    });

    json!({
        "mode": state.mode.to_string(),
        "party": party,
        "dungeon": dungeon,
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
        data: api_result.data,
        state_after: summarize_state(&api_state),
    };

    CommandParitySnapshot { command, cli, api }
}

fn base_state() -> GameState {
    let mut state = GameState::new();
    let mut c = Character::new("Aldric", Class::Fighter);
    c.gold_gp = 100;
    c.abilities.dexterity = 10;
    state.party.add_member(c);
    state
}

fn state_for_drop() -> GameState {
    let mut state = base_state();
    state
        .party
        .find_member_mut("Aldric")
        .expect("Aldric should exist")
        .inventory
        .push(Item::new("Sword", 60.0, 10));
    state
}

fn state_for_equip() -> GameState {
    let mut state = base_state();
    state
        .party
        .find_member_mut("Aldric")
        .expect("Aldric should exist")
        .inventory
        .push(Item::new("Leather", 150.0, 20));
    state
}

fn state_for_loot() -> GameState {
    let mut state = base_state();
    let mut dungeon = DungeonState::new(1);
    let room = Room::new(0, "Vault").with_placed_treasure(vec![
        PlacedTreasureInstance::new("Ruby gem", 500),
        PlacedTreasureInstance::new("Old key", 0),
    ]);
    dungeon.add_room(room).unwrap();
    dungeon.current_room = Some(0);
    dungeon.explored.insert(0);
    state.dungeon = Some(dungeon);
    state
}

fn has_exact_keys(data: &Value, expected_keys: &[&str]) -> bool {
    let Some(object) = data.as_object() else {
        return false;
    };

    if object.len() != expected_keys.len() {
        return false;
    }

    expected_keys.iter().all(|key| object.contains_key(*key))
}

fn buy_payload_matches(data: Option<&Value>, expected_message: &str) -> bool {
    let Some(data) = data else {
        return false;
    };
    has_exact_keys(
        data,
        &["message", "character", "item", "cost_gp", "gold_remaining"],
    ) && data["message"] == expected_message
        && data["character"] == "Aldric"
        && data["item"] == "Sword"
        && data["cost_gp"] == 10
        && data["gold_remaining"] == 90
}

fn drop_payload_matches(data: Option<&Value>, expected_message: &str) -> bool {
    let Some(data) = data else {
        return false;
    };
    has_exact_keys(data, &["message", "character", "item"])
        && data["message"] == expected_message
        && data["character"] == "Aldric"
        && data["item"] == "Sword"
}

fn equip_payload_matches(data: Option<&Value>, expected_message: &str) -> bool {
    let Some(data) = data else {
        return false;
    };
    has_exact_keys(data, &["message", "character", "item", "action", "ac"])
        && data["message"] == expected_message
        && data["character"] == "Aldric"
        && data["item"] == "Leather"
        && data["action"] == "equips"
        && data["ac"] == 7
}

fn loot_payload_matches(data: Option<&Value>, expected_message: &str) -> bool {
    let Some(data) = data else {
        return false;
    };
    has_exact_keys(data, &["message", "character", "item", "value_gp"])
        && data["message"] == expected_message
        && data["character"] == "Aldric"
        && data["item"] == "Ruby gem"
        && data["value_gp"] == 500
}

fn build_gate(
    cli_command: &'static str,
    gm_api_command: &'static str,
    notes: &'static str,
    expected_output: &'static str,
    snapshot: CommandParitySnapshot,
    payload_matches_typed_result: bool,
) -> CompatibilityGate {
    let state_equal = snapshot.cli.state_after == snapshot.api.state_after;
    let cli_output_equal = snapshot.cli.output == expected_output;
    let api_message_equal = snapshot.api.message == expected_output;

    CompatibilityGate {
        cli_command,
        gm_api_command,
        state_mutation_parity: "Same state mutation via shared engine inventory action",
        output_data_parity:
            "CLI output text is unchanged; API adds typed structured payload fields.",
        class: "B",
        notes,
        state_equal,
        cli_output_pre_migration_equal: cli_output_equal && api_message_equal,
        api_payload_shape_matches_typed_result: payload_matches_typed_result,
        snapshot,
    }
}

#[test]
fn inventory_command_parity_golden_scaffold_captures_snapshots() {
    let buy_expected = "Aldric buys Sword for 10 gp. (90 gp remaining)";
    let buy_snapshot = capture_parity(
        "buy",
        base_state(),
        |state| BuyCommand.execute(&["Aldric", "Sword"], state),
        GMCommand::Buy {
            character: "Aldric".to_string(),
            item_name: "Sword".to_string(),
        },
    );

    let drop_expected = "Aldric drops Sword.";
    let drop_snapshot = capture_parity(
        "drop",
        state_for_drop(),
        |state| DropCommand.execute(&["Aldric", "Sword"], state),
        GMCommand::Drop {
            character: "Aldric".to_string(),
            item_name: "Sword".to_string(),
        },
    );

    let equip_expected = "Aldric equips Leather. (AC 7)";
    let equip_snapshot = capture_parity(
        "equip",
        state_for_equip(),
        |state| EquipCommand.execute(&["Aldric", "Leather"], state),
        GMCommand::Equip {
            character: "Aldric".to_string(),
            item_name: "Leather".to_string(),
        },
    );

    let loot_expected = "Aldric picks up Ruby gem. (worth 500 gp)";
    let loot_snapshot = capture_parity(
        "loot",
        state_for_loot(),
        |state| LootCommand.execute(&["Aldric", "Ruby", "gem"], state),
        GMCommand::Loot {
            character: "Aldric".to_string(),
            item_name: "Ruby gem".to_string(),
            value_gp: None,
        },
    );

    let gates = vec![
        build_gate(
            "buy",
            "Buy",
            "Shared buy validation/orchestration and state mutation; API includes typed `BuyResult` data.",
            buy_expected,
            buy_snapshot.clone(),
            buy_payload_matches(buy_snapshot.api.data.as_ref(), buy_expected),
        ),
        build_gate(
            "drop",
            "Drop",
            "Shared drop validation/orchestration and state mutation; API includes typed `DropResult` data.",
            drop_expected,
            drop_snapshot.clone(),
            drop_payload_matches(drop_snapshot.api.data.as_ref(), drop_expected),
        ),
        build_gate(
            "equip",
            "Equip",
            "Shared equip toggle + AC recalculation; API includes typed `EquipResult` data.",
            equip_expected,
            equip_snapshot.clone(),
            equip_payload_matches(equip_snapshot.api.data.as_ref(), equip_expected),
        ),
        build_gate(
            "loot",
            "Loot",
            "Shared loot orchestration including room treasure `taken` mutation; API includes typed `LootResult` data.",
            loot_expected,
            loot_snapshot.clone(),
            loot_payload_matches(loot_snapshot.api.data.as_ref(), loot_expected),
        ),
    ];

    assert_eq!(gates.len(), 4);
    assert_eq!(gates.iter().filter(|gate| gate.class == "B").count(), 4);

    for gate in &gates {
        assert!(
            gate.snapshot.cli.success,
            "{} CLI path should succeed for golden parity input",
            gate.cli_command
        );
        assert!(
            gate.snapshot.api.success,
            "{} API path should succeed for golden parity input",
            gate.gm_api_command
        );
        assert!(
            gate.state_equal,
            "{} should keep CLI/API state mutation parity",
            gate.cli_command
        );
        assert!(
            gate.cli_output_pre_migration_equal,
            "{} should preserve pre-migration CLI output text",
            gate.cli_command
        );
        assert!(
            gate.api_payload_shape_matches_typed_result,
            "{} should emit typed API payload shape",
            gate.gm_api_command
        );
    }

    let serialized = serde_json::to_string_pretty(&gates).unwrap();
    assert!(serialized.contains("\"cli_command\": \"buy\""));
    assert!(serialized.contains("\"gm_api_command\": \"Loot\""));

    // Helpful with --nocapture when reviewing parity snapshots.
    println!("{serialized}");
}
