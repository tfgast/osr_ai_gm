use crate::command::lookup_cmds::{
    ItemCommand, SearchItemsCommand, SpellCommand, TreasureTypeCommand,
};
use crate::command::treasure_cmds::TreasureCommand;
use crate::command::Command;
use crate::gmapi::interface::handle_request;
use crate::gmapi::protocol::{GMCommand, GMRequest, GMResponse};
use crate::persist::GameState;
use crate::rules::magic_item::{
    find_magic_item, find_magic_items_partial, search_magic_items, ItemCategory, MagicItemDef,
};
use crate::rules::spell_data::{self, SpellList};
use crate::rules::treasure::{find_treasure_type, TreasureItemType};
use serde_json::Value;
use std::collections::HashMap;

fn run_api(command: GMCommand, state: &mut GameState) -> GMResponse {
    let request = GMRequest {
        id: "lookup-golden".to_string(),
        command,
    };
    handle_request(&request, state)
}

fn state_json(state: &GameState) -> Value {
    serde_json::to_value(state).expect("game state should serialize")
}

fn has_exact_keys(data: &Value, expected_keys: &[&str]) -> bool {
    let Some(obj) = data.as_object() else {
        return false;
    };
    if obj.len() != expected_keys.len() {
        return false;
    }
    expected_keys.iter().all(|key| obj.contains_key(*key))
}

fn pre_format_magic_item(item: &MagicItemDef) -> String {
    let mut out = format!("=== {} ===\nCategory: {}", item.name, item.category.name());

    if item.cursed {
        out.push_str(" [CURSED]");
    }
    out.push('\n');

    if let Some(ref desc) = item.description {
        out.push_str(&format!("\n{}\n", desc));
    }

    if !item.properties.is_empty() {
        out.push_str("\nProperties:\n");
        for prop in &item.properties {
            if let Some(ref key) = prop.key {
                out.push_str(&format!("  {}: {}\n", key, prop.value));
            } else {
                out.push_str(&format!("  - {}\n", prop.value));
            }
        }
    }

    out
}

fn pre_item_cli_output(args: &[&str]) -> String {
    if args.is_empty() {
        return "Error: usage: item <name>".to_string();
    }

    let query = args.join(" ");
    if let Some(item) = find_magic_item(&query) {
        return pre_format_magic_item(item);
    }

    let matches = find_magic_items_partial(&query);
    match matches.len() {
        0 => format!("Error: no magic item found matching '{}'.", query),
        1 => pre_format_magic_item(matches[0]),
        n if n <= 10 => {
            let mut out = format!("Multiple items match '{}'. Did you mean:\n", query);
            for item in &matches {
                out.push_str(&format!("  - {}\n", item.name));
            }
            out
        }
        n => format!(
            "Found {} items matching '{}'. Please be more specific.",
            n, query
        ),
    }
}

fn pre_search_items_cli_output(args: &[&str]) -> String {
    if args.is_empty() {
        return "Error: usage: search_items <query>".to_string();
    }

    let query = args.join(" ");
    let results = search_magic_items(&query);

    match results.len() {
        0 => format!("No magic items found matching '{}'.", query),
        n => {
            let mut out = format!("Found {} item(s) matching '{}':\n\n", n, query);

            let mut by_category: HashMap<ItemCategory, Vec<_>> = HashMap::new();
            for item in results {
                by_category.entry(item.category).or_default().push(item);
            }

            let mut categories: Vec<_> = by_category.keys().copied().collect();
            categories.sort_by_key(|c| c.name());

            for category in categories {
                let items = &by_category[&category];
                out.push_str(&format!("{}:\n", category.name()));
                for item in items {
                    let cursed = if item.cursed { " [CURSED]" } else { "" };
                    out.push_str(&format!("  - {}{}\n", item.name, cursed));
                }
                out.push('\n');
            }

            out
        }
    }
}

fn pre_treasure_type_cli_output(args: &[&str]) -> String {
    if args.is_empty() {
        return "Error: usage: treasure_type <letter> (A-V)".to_string();
    }

    let letter = args[0].to_uppercase();
    match find_treasure_type(&letter) {
        Some(tt) => {
            let mut out = format!(
                "=== Treasure Type {} ===\n\
                 Category: {}\n\
                 Average Value: {} gp\n\n\
                 Contents:\n",
                tt.letter,
                tt.category.name(),
                tt.average_gp
            );

            for entry in &tt.entries {
                let restriction = entry
                    .restriction
                    .as_ref()
                    .map(|r| format!(" ({})", r))
                    .unwrap_or_default();
                let note = entry
                    .note
                    .as_ref()
                    .map(|n| format!(" [{}]", n))
                    .unwrap_or_default();

                out.push_str(&format!(
                    "  {:3}%: {} {}{}{}\n",
                    entry.chance,
                    entry.quantity,
                    entry.item_type.name(),
                    restriction,
                    note
                ));
            }

            out.push_str("\nPossible contents:\n");
            let has_coins = tt.entries.iter().any(|e| e.item_type.is_coin());
            let has_gems = tt
                .entries
                .iter()
                .any(|e| e.item_type == TreasureItemType::Gems);
            let has_jewellery = tt
                .entries
                .iter()
                .any(|e| e.item_type == TreasureItemType::Jewellery);
            let has_magic = tt.entries.iter().any(|e| e.item_type.is_magic());

            if has_coins {
                out.push_str("  - Coins (copper, silver, electrum, gold, platinum)\n");
            }
            if has_gems {
                out.push_str("  - Gems (10-1000 gp each, rolled on d20 table)\n");
            }
            if has_jewellery {
                out.push_str("  - Jewellery (3d6 x 100 gp each)\n");
            }
            if has_magic {
                out.push_str("  - Magic items\n");
            }

            out
        }
        None => format!(
            "Error: unknown treasure type '{}'. Valid types are A-V.",
            letter
        ),
    }
}

fn pre_parse_spell_list(s: &str) -> Option<SpellList> {
    match s.to_lowercase().as_str() {
        "cleric" => Some(SpellList::Cleric),
        "magicuser" | "magic-user" | "magic_user" | "mu" | "mage" => Some(SpellList::MagicUser),
        "druid" => Some(SpellList::Druid),
        "illusionist" => Some(SpellList::Illusionist),
        _ => None,
    }
}

fn pre_spell_cli_output(args: &[&str]) -> String {
    if args.is_empty() {
        return "Error: usage: spell <name> [list]\n  Lists: cleric, magicuser (or mu/mage), druid, illusionist".to_string();
    }

    let (name_args, list) = if args.len() >= 2 {
        match pre_parse_spell_list(args[args.len() - 1]) {
            Some(l) => (&args[..args.len() - 1], Some(l)),
            None => (args, None),
        }
    } else {
        (args, None)
    };

    let query = name_args.join(" ");
    match spell_data::find_spell(&query, list) {
        Some(spell) => {
            let mut out = format!(
                "=== {} ===\nList: {} (Level {})\nRange: {}\nDuration: {}\n",
                spell.name,
                spell.list.name(),
                spell.level,
                spell.range,
                spell.duration,
            );
            if spell.reversible {
                if let Some(ref rev_name) = spell.reversed_name {
                    out.push_str(&format!("Reversible: {}\n", rev_name));
                } else {
                    out.push_str("Reversible: yes\n");
                }
            }
            out.push_str(&format!("\n{}", spell.description));
            out
        }
        None => {
            let list_hint = list
                .map(|l| format!(" in {} list", l.name()))
                .unwrap_or_default();
            format!("Error: spell '{}' not found{}.", query, list_hint)
        }
    }
}

#[test]
fn lookup_happy_path_cli_api_and_state_parity() {
    let pre_state = state_json(&GameState::new());

    let item_cli = ItemCommand.execute(&["Bag", "of", "Holding"], &mut GameState::new());
    assert_eq!(
        item_cli.output,
        pre_item_cli_output(&["Bag", "of", "Holding"]),
        "item CLI output drifted from pre-migration behavior"
    );
    let mut item_api_state = GameState::new();
    let item_api = run_api(
        GMCommand::LookupItem {
            name: "Bag of Holding".to_string(),
        },
        &mut item_api_state,
    );
    assert!(item_api.success);
    let item_data = item_api
        .data
        .as_ref()
        .expect("lookup_item should include data");
    assert!(has_exact_keys(
        item_data,
        &["name", "category", "cursed", "description", "properties"]
    ));
    assert_eq!(state_json(&item_api_state), pre_state);

    let search_cli = SearchItemsCommand.execute(&["healing"], &mut GameState::new());
    assert_eq!(
        search_cli.output,
        pre_search_items_cli_output(&["healing"]),
        "search_items CLI output drifted from pre-migration behavior"
    );
    let mut search_api_state = GameState::new();
    let search_api = run_api(
        GMCommand::SearchItems {
            query: "healing".to_string(),
        },
        &mut search_api_state,
    );
    assert!(search_api.success);
    let search_data = search_api
        .data
        .as_ref()
        .expect("search_items should include data");
    assert!(has_exact_keys(search_data, &["count", "by_category"]));
    assert!(search_data["count"].as_u64().unwrap_or(0) > 0);
    assert!(search_data["by_category"].is_object());
    assert_eq!(state_json(&search_api_state), pre_state);

    let treasure_type_cli = TreasureTypeCommand.execute(&["A"], &mut GameState::new());
    assert_eq!(
        treasure_type_cli.output,
        pre_treasure_type_cli_output(&["A"]),
        "treasure_type CLI output drifted from pre-migration behavior"
    );
    let mut treasure_type_api_state = GameState::new();
    let treasure_type_api = run_api(
        GMCommand::LookupTreasureType {
            letter: "A".to_string(),
        },
        &mut treasure_type_api_state,
    );
    assert!(treasure_type_api.success);
    let treasure_type_data = treasure_type_api
        .data
        .as_ref()
        .expect("lookup_treasure_type should include data");
    assert!(has_exact_keys(
        treasure_type_data,
        &["letter", "category", "average_gp", "entries"]
    ));
    assert!(treasure_type_data["entries"].is_array());
    let first_entry = &treasure_type_data["entries"][0];
    assert!(has_exact_keys(
        first_entry,
        &["chance", "quantity", "type", "restriction", "note"]
    ));
    assert_eq!(state_json(&treasure_type_api_state), pre_state);

    let spell_cli = SpellCommand.execute(&["Magic", "Missile"], &mut GameState::new());
    assert_eq!(
        spell_cli.output,
        pre_spell_cli_output(&["Magic", "Missile"]),
        "spell CLI output drifted from pre-migration behavior"
    );
    let mut spell_api_state = GameState::new();
    let spell_api = run_api(
        GMCommand::LookupSpell {
            name: "Magic Missile".to_string(),
            list: "".to_string(),
        },
        &mut spell_api_state,
    );
    assert!(spell_api.success);
    let spell_data = spell_api
        .data
        .as_ref()
        .expect("lookup_spell should include data");
    assert!(has_exact_keys(
        spell_data,
        &["name", "list", "level", "range", "duration", "description"]
    ));
    assert_eq!(state_json(&spell_api_state), pre_state);

    let treasure_cli = TreasureCommand.execute(&["P"], &mut GameState::new());
    assert!(treasure_cli.output.contains("TREASURE TYPE P"));
    assert!(
        treasure_cli.output.contains("TOTAL VALUE")
            || treasure_cli.output.contains("Nothing found")
    );
    let mut treasure_api_state = GameState::new();
    let treasure_api = run_api(
        GMCommand::RollTreasure {
            letter: "P".to_string(),
        },
        &mut treasure_api_state,
    );
    assert!(treasure_api.success);
    let treasure_data = treasure_api
        .data
        .as_ref()
        .expect("roll_treasure should include data");
    assert!(has_exact_keys(
        treasure_data,
        &["letter", "category", "items", "total_gp"]
    ));
    assert!(treasure_data["items"].is_array());
    assert!(!treasure_data["items"]
        .as_array()
        .unwrap_or(&vec![])
        .is_empty());
    let first_item = &treasure_data["items"][0];
    let first_item_obj = first_item
        .as_object()
        .expect("rolled item should be an object");
    assert!(first_item_obj.contains_key("type"));
    assert!(first_item_obj.contains_key("quantity"));
    assert_eq!(state_json(&treasure_api_state), pre_state);
}

#[test]
fn lookup_empty_query_parity_restored() {
    // item "" should return a successful suggestion path (browse all items)
    let item_pre = pre_item_cli_output(&[""]);
    assert!(
        !item_pre.starts_with("Error:"),
        "pre-migration item <empty> was a successful suggestion path"
    );
    let item_now = ItemCommand.execute(&[""], &mut GameState::new());
    assert!(
        !item_now.output.starts_with("Error:"),
        "item <empty> should succeed (browse all items)"
    );
    let mut item_api_state = GameState::new();
    let item_api = run_api(
        GMCommand::LookupItem {
            name: "".to_string(),
        },
        &mut item_api_state,
    );
    assert!(
        item_api.success,
        "API item <empty> should succeed (browse all items)"
    );

    // search_items "" should return full catalog
    let search_pre = pre_search_items_cli_output(&[""]);
    assert!(
        !search_pre.starts_with("Error:"),
        "pre-migration search_items <empty> returned full catalog matches"
    );
    let search_now = SearchItemsCommand.execute(&[""], &mut GameState::new());
    assert!(
        !search_now.output.starts_with("Error:"),
        "search_items <empty> should succeed (full catalog)"
    );
    let mut search_api_state = GameState::new();
    let search_api = run_api(
        GMCommand::SearchItems {
            query: "".to_string(),
        },
        &mut search_api_state,
    );
    assert!(
        search_api.success,
        "API search_items <empty> should succeed (full catalog)"
    );
}
