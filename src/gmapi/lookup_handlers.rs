use crate::dice;
use crate::gmapi::protocol::GMResponse;
use crate::persist::GameState;
use crate::rules::magic_item::{
    find_magic_item, find_magic_items_partial, search_magic_items, ItemCategory,
};
use crate::rules::treasure::{self, TreasureItemType};
use rand::Rng;

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
    let letter = letter.to_uppercase();
    match treasure::find_treasure_type(&letter) {
        Some(tt) => {
            let entries: Vec<serde_json::Value> = tt.entries.iter().map(|e| {
                serde_json::json!({
                    "chance": e.chance,
                    "quantity": &e.quantity,
                    "type": e.item_type.name(),
                    "restriction": &e.restriction,
                    "note": &e.note,
                })
            }).collect();

            let mut msg = format!("treasure type {} ({}), avg {} gp.", tt.letter, tt.category.name(), tt.average_gp);

            let has_coins = tt.entries.iter().any(|e| e.item_type.is_coin());
            let has_gems = tt.entries.iter().any(|e| e.item_type == TreasureItemType::Gems);
            let has_jewellery = tt.entries.iter().any(|e| e.item_type == TreasureItemType::Jewellery);
            let has_magic = tt.entries.iter().any(|e| e.item_type.is_magic());

            let mut contents = Vec::new();
            if has_coins { contents.push("coins"); }
            if has_gems { contents.push("gems"); }
            if has_jewellery { contents.push("jewellery"); }
            if has_magic { contents.push("magic items"); }
            if !contents.is_empty() {
                msg.push_str(&format!(" May contain: {}.", contents.join(", ")));
            }

            GMResponse::ok_with_data(
                id,
                msg,
                state.mode.clone(),
                serde_json::json!({
                    "letter": tt.letter,
                    "category": tt.category.name(),
                    "average_gp": tt.average_gp,
                    "entries": entries,
                }),
            )
        }
        None => GMResponse::err(
            id,
            format!("unknown treasure type '{}'. Valid types are A-V.", letter),
            state.mode.clone(),
        ),
    }
}

// =============================================================================
// Treasure rolling
// =============================================================================

pub(super) fn roll_treasure(id: &str, state: &GameState, letter: &str) -> GMResponse {
    let letter = letter.to_uppercase();
    let treasure_type = match treasure::find_treasure_type(&letter) {
        Some(t) => t,
        None => return GMResponse::err(
            id,
            format!("unknown treasure type '{}'. Valid types are A-V.", letter),
            state.mode.clone(),
        ),
    };

    let mut rng = rand::thread_rng();
    let mut results = Vec::new();
    let mut total_gp: f64 = 0.0;

    for entry in &treasure_type.entries {
        let roll: u32 = rng.gen_range(1..=100);
        if roll > entry.chance {
            continue;
        }

        let quantity = match parse_quantity_with_multiplier(&entry.quantity) {
            Ok(q) => q.max(1),
            Err(_) => continue,
        };

        let mut item = serde_json::Map::new();
        item.insert("type".to_string(), serde_json::json!(entry.item_type.name()));
        item.insert("quantity".to_string(), serde_json::json!(quantity));

        match entry.item_type {
            TreasureItemType::Cp => {
                let gp_value = quantity as f64 * 0.01;
                total_gp += gp_value;
                item.insert("gp_value".to_string(), serde_json::json!(gp_value));
            }
            TreasureItemType::Sp => {
                let gp_value = quantity as f64 * 0.1;
                total_gp += gp_value;
                item.insert("gp_value".to_string(), serde_json::json!(gp_value));
            }
            TreasureItemType::Ep => {
                let gp_value = quantity as f64 * 0.5;
                total_gp += gp_value;
                item.insert("gp_value".to_string(), serde_json::json!(gp_value));
            }
            TreasureItemType::Gp => {
                total_gp += quantity as f64;
                item.insert("gp_value".to_string(), serde_json::json!(quantity));
            }
            TreasureItemType::Pp => {
                let gp_value = quantity as f64 * 5.0;
                total_gp += gp_value;
                item.insert("gp_value".to_string(), serde_json::json!(gp_value));
            }
            TreasureItemType::Gems => {
                let gem_result = treasure::roll_gems(quantity as u32);
                total_gp += gem_result.total_gp as f64;
                item.insert("values".to_string(), serde_json::json!(gem_result.values));
                item.insert("total_gp".to_string(), serde_json::json!(gem_result.total_gp));
            }
            TreasureItemType::Jewellery => {
                let jewellery_result = treasure::roll_jewellery(quantity as u32);
                total_gp += jewellery_result.total_gp as f64;
                item.insert("values".to_string(), serde_json::json!(jewellery_result.values));
                item.insert("total_gp".to_string(), serde_json::json!(jewellery_result.total_gp));
            }
            _ => {
                // Magic items: note the type, GM rolls separately
                if let Some(ref restriction) = entry.restriction {
                    item.insert("restriction".to_string(), serde_json::json!(restriction));
                }
            }
        }

        if let Some(ref note) = entry.note {
            item.insert("note".to_string(), serde_json::json!(note));
        }

        results.push(serde_json::Value::Object(item));
    }

    let msg = if results.is_empty() {
        format!("rolled on treasure type {}: nothing found.", letter)
    } else {
        format!("rolled on treasure type {}: {} item(s), {:.0} gp total value.", letter, results.len(), total_gp)
    };

    GMResponse::ok_with_data(
        id,
        msg,
        state.mode.clone(),
        serde_json::json!({
            "letter": letter,
            "category": treasure_type.category.name(),
            "items": results,
            "total_gp": total_gp,
        }),
    )
}

/// Parse a quantity string that may include a multiplier.
/// Supports formats like "1d6", "1d6 × 1000", "3", "1d4 × 10".
fn parse_quantity_with_multiplier(quantity: &str) -> Result<i32, String> {
    let normalized = quantity
        .replace(['×', 'x'], "*")
        .replace(' ', "");

    if let Some(pos) = normalized.find('*') {
        let dice_part = &normalized[..pos];
        let multiplier_part = &normalized[pos + 1..];

        let base = if dice_part.contains('d') || dice_part.contains('D') {
            dice::roll_str(dice_part)
                .map(|r| r.total)
                .map_err(|e| format!("bad dice expr '{}': {}", dice_part, e))?
        } else {
            dice_part
                .parse::<i32>()
                .map_err(|_| format!("invalid number '{}'", dice_part))?
        };

        let multiplier: i32 = multiplier_part
            .parse()
            .map_err(|_| format!("invalid multiplier '{}'", multiplier_part))?;

        Ok(base * multiplier)
    } else if normalized.contains('d') || normalized.contains('D') {
        dice::roll_str(&normalized)
            .map(|r| r.total)
            .map_err(|e| format!("bad dice expr '{}': {}", quantity, e))
    } else {
        normalized
            .parse::<i32>()
            .map_err(|_| format!("invalid quantity '{}'", quantity))
    }
}
