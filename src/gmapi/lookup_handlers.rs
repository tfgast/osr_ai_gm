use crate::engine::result::EngineError;
use crate::engine::treasure as treasure_engine;
use crate::gmapi::protocol::GMResponse;
use crate::persist::GameState;
use crate::rules::magic_item::{
    find_magic_item, find_magic_items_partial, search_magic_items, ItemCategory,
};
use serde::Serialize;

fn ok_with_typed_data<T: Serialize>(
    id: &str,
    state: &GameState,
    message: String,
    payload: T,
) -> GMResponse {
    match serde_json::to_value(payload) {
        Ok(data) => GMResponse::ok_with_data(id, message, state.mode.clone(), data),
        Err(err) => GMResponse::err(
            id,
            format!("internal error: failed to serialize response: {err}"),
            state.mode.clone(),
        ),
    }
}

// =============================================================================
// Magic item lookup
// =============================================================================

pub(super) fn lookup_item(id: &str, state: &GameState, name: &str) -> GMResponse {
    // Try exact match first
    if let Some(item) = find_magic_item(name) {
        return GMResponse::ok_with_data(
            id,
            format_magic_item_message(item),
            state.mode.clone(),
            magic_item_json(item),
        );
    }

    // Try partial match
    let matches = find_magic_items_partial(name);
    match matches.len() {
        0 => GMResponse::err(id, format!("no magic item found matching '{}'.", name), state.mode.clone()),
        1 => GMResponse::ok_with_data(
            id,
            format_magic_item_message(matches[0]),
            state.mode.clone(),
            magic_item_json(matches[0]),
        ),
        n if n <= 10 => {
            let names: Vec<&str> = matches.iter().map(|i| i.name.as_str()).collect();
            GMResponse::ok_with_data(
                id,
                format!("multiple items match '{}'. Did you mean: {}?", name, names.join(", ")),
                state.mode.clone(),
                serde_json::json!({
                    "matches": names,
                    "count": n,
                }),
            )
        }
        n => GMResponse::ok_with_data(
            id,
            format!("found {} items matching '{}'. Please be more specific.", n, name),
            state.mode.clone(),
            serde_json::json!({ "count": n }),
        ),
    }
}

fn format_magic_item_message(item: &crate::rules::magic_item::MagicItemDef) -> String {
    let mut msg = format!("{} ({})", item.name, item.category.name());
    if item.cursed {
        msg.push_str(" [CURSED]");
    }
    if let Some(ref desc) = item.description {
        msg.push_str(&format!(": {}", desc));
    }
    msg
}

fn magic_item_json(item: &crate::rules::magic_item::MagicItemDef) -> serde_json::Value {
    let properties: Vec<serde_json::Value> = item.properties.iter().map(|p| {
        if let Some(ref key) = p.key {
            serde_json::json!({ "key": key, "value": &p.value })
        } else {
            serde_json::json!({ "value": &p.value })
        }
    }).collect();

    serde_json::json!({
        "name": item.name,
        "category": item.category.name(),
        "cursed": item.cursed,
        "description": item.description,
        "properties": properties,
    })
}

// =============================================================================
// Magic item search
// =============================================================================

pub(super) fn search_items(id: &str, state: &GameState, query: &str) -> GMResponse {
    let results = search_magic_items(query);

    if results.is_empty() {
        return GMResponse::ok_with_data(
            id,
            format!("no magic items found matching '{}'.", query),
            state.mode.clone(),
            serde_json::json!({ "matches": [], "count": 0 }),
        );
    }

    // Group by category
    let mut by_category: std::collections::HashMap<ItemCategory, Vec<&str>> =
        std::collections::HashMap::new();
    for item in &results {
        by_category.entry(item.category).or_default().push(&item.name);
    }

    let mut categories_json = serde_json::Map::new();
    for (category, names) in &by_category {
        categories_json.insert(
            category.name().to_string(),
            serde_json::json!(names),
        );
    }

    GMResponse::ok_with_data(
        id,
        format!("found {} item(s) matching '{}'.", results.len(), query),
        state.mode.clone(),
        serde_json::json!({
            "count": results.len(),
            "by_category": categories_json,
        }),
    )
}

// =============================================================================
// Treasure type lookup
// =============================================================================

pub(super) fn lookup_treasure_type(id: &str, state: &GameState, letter: &str) -> GMResponse {
    match treasure_engine::action_lookup_treasure_type(letter) {
        Ok(result) => ok_with_typed_data(id, state, result.message, result.data),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}

// =============================================================================
// Treasure rolling
// =============================================================================

pub(super) fn roll_treasure(id: &str, state: &GameState, letter: &str) -> GMResponse {
    match treasure_engine::action_roll_treasure(letter) {
        Ok(result) => ok_with_typed_data(id, state, result.message, result.data),
        Err(EngineError::InvalidInput(msg)) if msg.starts_with("unknown treasure type '") => {
            GMResponse::err(id, format!("{msg} Valid types are A-V."), state.mode.clone())
        }
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}
