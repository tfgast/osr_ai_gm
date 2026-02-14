use crate::engine::inventory::results::{
    BuyResult, DropResult, EquipResult, EquipmentEntry, ListEquipmentResult, LootResult,
};
use crate::engine::result::EngineError;
use crate::model::Item;
use crate::persist::GameState;
use crate::rules::{ability, equipment};

fn no_party_member_err(char_name: &str) -> EngineError {
    EngineError::InvalidInput(format!("no party member named '{}'.", char_name))
}

/// Strip non-alphanumeric characters and lowercase for fuzzy matching.
/// "Chain Mail" → "chainmail", "Thieves' tools" → "thievestools"
fn normalize_for_matching(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Look up an item across all equipment tables.
/// Returns (name, cost_gp, weight as f32).
///
/// First tries exact case-insensitive lookup, then falls back to normalized
/// matching (stripping spaces, punctuation) so "Chain Mail" finds "Chainmail"
/// and "Thieves tools" finds "Thieves' tools".
fn find_buyable(name: &str) -> Option<(String, u32, f32)> {
    // Pass 1: exact case-insensitive lookup via HashMap
    if let Some(w) = equipment::find_weapon(name) {
        if w.cost_gp() > 0 {
            return Some((w.name.clone(), w.cost_gp(), w.weight() as f32));
        }
    }
    if let Some(a) = equipment::find_armour(name) {
        if a.cost_gp() > 0 {
            return Some((a.name.clone(), a.cost_gp(), a.weight() as f32));
        }
    }
    if let Some(g) = equipment::find_gear(name) {
        return Some((g.name.clone(), g.cost_gp(), 0.0));
    }
    if let Some(a) = equipment::ammunition()
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))
    {
        return Some((a.name.clone(), a.cost_gp(), 0.0));
    }

    // Pass 2: normalized matching (strip spaces/punctuation, then compare)
    let query_norm = normalize_for_matching(name);
    if query_norm.is_empty() {
        return None;
    }
    for w in equipment::weapons() {
        if w.cost_gp() > 0 && normalize_for_matching(&w.name) == query_norm {
            return Some((w.name.clone(), w.cost_gp(), w.weight() as f32));
        }
    }
    for a in equipment::armour() {
        if a.cost_gp() > 0 && normalize_for_matching(&a.name) == query_norm {
            return Some((a.name.clone(), a.cost_gp(), a.weight() as f32));
        }
    }
    for g in equipment::gear() {
        if normalize_for_matching(&g.name) == query_norm {
            return Some((g.name.clone(), g.cost_gp(), 0.0));
        }
    }
    for a in equipment::ammunition() {
        if normalize_for_matching(&a.name) == query_norm {
            return Some((a.name.clone(), a.cost_gp(), 0.0));
        }
    }

    None
}

/// Check if a query fuzzy-matches an item name for suggestion purposes.
/// Handles substring matching in both directions, plus normalized matching
/// (stripping spaces/punctuation) so "Leather Armor" suggests "Leather" and
/// "Thieves tools" suggests "Thieves' tools".
fn names_match_for_suggestion(item_name: &str, query: &str) -> bool {
    let item_lower = item_name.to_lowercase();
    let query_lower = query.to_lowercase();

    // Bidirectional substring: item contains query or query contains item
    if item_lower.contains(&query_lower) || query_lower.contains(&item_lower) {
        return true;
    }

    // Normalized containment: strip non-alphanumeric, then check both directions
    let item_norm = normalize_for_matching(item_name);
    let query_norm = normalize_for_matching(query);
    item_norm.contains(&query_norm) || query_norm.contains(&item_norm)
}

/// Find equipment names that fuzzy-match the query.
/// Returns up to 3 suggestions.
fn suggest_equipment(query: &str) -> Vec<String> {
    let mut suggestions: Vec<String> = Vec::new();

    for w in equipment::weapons() {
        if w.cost_gp() > 0 && names_match_for_suggestion(&w.name, query) {
            suggestions.push(w.name.clone());
        }
    }

    for a in equipment::armour() {
        if a.cost_gp() > 0 && names_match_for_suggestion(&a.name, query) {
            suggestions.push(a.name.clone());
        }
    }

    for g in equipment::gear() {
        if names_match_for_suggestion(&g.name, query) {
            suggestions.push(g.name.clone());
        }
    }

    for a in equipment::ammunition() {
        if names_match_for_suggestion(&a.name, query) {
            suggestions.push(a.name.clone());
        }
    }

    suggestions.truncate(3);
    suggestions
}

pub fn action_buy(
    state: &mut GameState,
    char_name: &str,
    item_name: &str,
) -> Result<BuyResult, EngineError> {
    let (canonical_name, cost, weight) = match find_buyable(item_name) {
        Some(info) => info,
        None => {
            let suggestions = suggest_equipment(item_name);
            let message = if suggestions.is_empty() {
                format!("unknown item '{}'. Check equipment tables.", item_name)
            } else {
                format!(
                    "unknown item '{}'. Did you mean: {}?",
                    item_name,
                    suggestions.join(", ")
                )
            };
            return Err(EngineError::InvalidInput(message));
        }
    };

    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;

    if character.gold_gp < cost {
        return Err(EngineError::InvalidInput(format!(
            "{} has {} gp but {} costs {} gp.",
            character.name, character.gold_gp, canonical_name, cost
        )));
    }

    character.gold_gp -= cost;
    character
        .inventory
        .push(Item::new(&canonical_name, weight, cost));

    let message = format!(
        "{} buys {} for {} gp. ({} gp remaining)",
        character.name, canonical_name, cost, character.gold_gp
    );

    Ok(BuyResult {
        message,
        character: character.name.clone(),
        item: canonical_name,
        cost_gp: cost,
        gold_remaining: character.gold_gp,
    })
}

pub fn action_drop(
    state: &mut GameState,
    char_name: &str,
    item_name: &str,
) -> Result<DropResult, EngineError> {
    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;

    let idx = character
        .inventory
        .iter()
        .position(|i| i.name.eq_ignore_ascii_case(item_name));

    match idx {
        Some(i) => {
            let dropped = character.inventory.remove(i);
            Ok(DropResult {
                message: format!("{} drops {}.", character.name, dropped.name),
                character: character.name.clone(),
                item: dropped.name,
            })
        }
        None => Err(EngineError::InvalidInput(format!(
            "{} does not have '{}'.",
            character.name, item_name
        ))),
    }
}

pub fn action_equip(
    state: &mut GameState,
    char_name: &str,
    item_name: &str,
) -> Result<EquipResult, EngineError> {
    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;

    let idx = character
        .inventory
        .iter()
        .position(|i| i.name.eq_ignore_ascii_case(item_name));

    let idx = idx.ok_or_else(|| {
        EngineError::InvalidInput(format!("{} does not have '{}'.", character.name, item_name))
    })?;

    let was_equipped = character.inventory[idx].equipped;
    character.inventory[idx].equipped = !was_equipped;

    let action = if was_equipped { "unequips" } else { "equips" };
    let item_display = character.inventory[idx].name.clone();

    let armour_ac = character
        .inventory
        .iter()
        .filter(|i| i.equipped)
        .filter_map(|i| equipment::find_armour(&i.name))
        .filter(|a| !a.is_shield())
        .map(|a| a.ac_descending())
        .min()
        .unwrap_or(9);

    let has_shield = character.inventory.iter().any(|i| {
        i.equipped
            && equipment::find_armour(&i.name)
                .map(|a| a.is_shield())
                .unwrap_or(false)
    });

    let dex_mod = ability::dex_ac_mod(character.abilities.dexterity);
    character.ac = equipment::calculate_ac(armour_ac, has_shield, dex_mod);

    Ok(EquipResult {
        message: format!(
            "{} {} {}. (AC {})",
            character.name, action, item_display, character.ac
        ),
        character: character.name.clone(),
        item: item_display,
        action: action.to_string(),
        ac: character.ac,
    })
}

pub fn action_loot(
    state: &mut GameState,
    char_name: &str,
    item_name: &str,
    explicit_gp: Option<u32>,
) -> Result<LootResult, EngineError> {
    let room_gp = if let Some(dungeon) = &mut state.dungeon {
        if let Some(current) = dungeon.current_room {
            if let Some(room) = dungeon.find_room_mut(current) {
                let idx = room
                    .placed_treasure
                    .iter()
                    .position(|t| !t.taken && t.description.eq_ignore_ascii_case(item_name));

                match idx {
                    Some(i) => {
                        let gp = room.placed_treasure[i].gp_value;
                        room.placed_treasure[i].taken = true;
                        Some(gp)
                    }
                    None => None, // not in room treasure → ad-hoc item
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let value_gp = explicit_gp.unwrap_or(room_gp.unwrap_or(0) as u32);

    let character = state
        .party
        .find_member_mut(char_name)
        .ok_or_else(|| no_party_member_err(char_name))?;

    character
        .inventory
        .push(Item::new(item_name, 0.0, value_gp));

    let mut message = format!("{} picks up {}.", character.name, item_name);
    if value_gp > 0 {
        message.push_str(&format!(" (worth {} gp)", value_gp));
    }

    Ok(LootResult {
        message,
        character: character.name.clone(),
        item: item_name.to_string(),
        value_gp,
    })
}

/// List all buyable equipment from the equipment tables.
pub fn action_list_equipment() -> ListEquipmentResult {
    let weapons: Vec<EquipmentEntry> = equipment::weapons()
        .iter()
        .filter(|w| w.cost_gp() > 0)
        .map(|w| EquipmentEntry {
            name: w.name.clone(),
            cost_gp: w.cost_gp(),
            category: w.category.clone(),
        })
        .collect();

    let armour: Vec<EquipmentEntry> = equipment::armour()
        .iter()
        .filter(|a| a.cost_gp() > 0)
        .map(|a| EquipmentEntry {
            name: a.name.clone(),
            cost_gp: a.cost_gp(),
            category: a.category.clone(),
        })
        .collect();

    let gear: Vec<EquipmentEntry> = equipment::gear()
        .iter()
        .map(|g| EquipmentEntry {
            name: g.name.clone(),
            cost_gp: g.cost_gp(),
            category: g.category.clone(),
        })
        .collect();

    let ammunition: Vec<EquipmentEntry> = equipment::ammunition()
        .iter()
        .map(|a| EquipmentEntry {
            name: a.name.clone(),
            cost_gp: a.cost_gp(),
            category: a.category.clone(),
        })
        .collect();

    ListEquipmentResult {
        weapons,
        armour,
        gear,
        ammunition,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    use crate::rules::class::Class;

    fn state_with_fighter() -> GameState {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.gold_gp = 100;
        c.abilities.dexterity = 10;
        state.party.add_member(c);
        state
    }

    #[test]
    fn buy_sword() {
        let mut state = state_with_fighter();
        let result = action_buy(&mut state, "Aldric", "Sword").expect("buy should succeed");
        assert!(result.message.contains("buys Sword"));
        assert_eq!(result.cost_gp, 10);
        assert_eq!(result.gold_remaining, 90);
        let c = state.party.find_member("Aldric").unwrap();
        assert_eq!(c.inventory.len(), 1);
        assert_eq!(c.inventory[0].name, "Sword");
    }

    #[test]
    fn buy_torch_costs_gp() {
        let mut state = state_with_fighter();
        let result =
            action_buy(&mut state, "Aldric", "Torches (6)").expect("buy torches should succeed");
        assert_eq!(result.cost_gp, 1);
        assert_eq!(result.gold_remaining, 99);
    }

    #[test]
    fn buy_holy_water_costs_gp() {
        let mut state = state_with_fighter();
        let result = action_buy(&mut state, "Aldric", "Holy water (vial)")
            .expect("buy holy water should succeed");
        assert_eq!(result.cost_gp, 25);
        assert_eq!(result.gold_remaining, 75);
    }

    #[test]
    fn buy_improvised_weapon_not_free() {
        // "Torch" as improvised weapon has no cost — should not be buyable at 0 gp.
        // It should fall through to gear lookup or fail, not return cost=0.
        let result = find_buyable("Torch");
        assert!(
            result.is_none() || result.unwrap().1 > 0,
            "Improvised weapons without a price should not be buyable at 0 gp"
        );
    }

    #[test]
    fn buy_suggests_chainmail() {
        let mut state = state_with_fighter();
        let err = action_buy(&mut state, "Aldric", "chain")
            .expect_err("partial query should suggest matches");
        assert!(err.to_string().contains("Did you mean"));
        assert!(err.to_string().contains("Chainmail"));
    }

    // ---- Fuzzy matching tests (bug oag-ntpgk) ----

    #[test]
    fn buy_chain_mail_with_space_resolves_to_chainmail() {
        let mut state = state_with_fighter();
        let result =
            action_buy(&mut state, "Aldric", "Chain Mail").expect("should resolve Chain Mail");
        assert!(result.message.contains("buys Chainmail"));
        assert_eq!(result.cost_gp, 40);
    }

    #[test]
    fn buy_thieves_tools_without_apostrophe() {
        let mut state = state_with_fighter();
        let result = action_buy(&mut state, "Aldric", "Thieves tools")
            .expect("should resolve without apostrophe");
        assert!(result.message.contains("Thieves' tools"));
        assert_eq!(result.cost_gp, 25);
    }

    #[test]
    fn buy_leather_armor_suggests_leather() {
        let mut state = state_with_fighter();
        let err = action_buy(&mut state, "Aldric", "Leather Armor")
            .expect_err("Leather Armor should not match exactly");
        let msg = err.to_string();
        assert!(msg.contains("Did you mean"), "should suggest: {}", msg);
        assert!(msg.contains("Leather"), "should suggest Leather: {}", msg);
    }

    #[test]
    fn buy_plate_mail_with_extra_space_resolves() {
        let mut state = state_with_fighter();
        let result = action_buy(&mut state, "Aldric", "Plate Mail")
            .expect("should resolve Plate Mail with different spacing");
        assert!(result.message.contains("Plate mail"));
    }

    #[test]
    fn buy_war_hammer_without_space_resolves() {
        let mut state = state_with_fighter();
        let result = action_buy(&mut state, "Aldric", "Warhammer")
            .expect("should resolve Warhammer to War hammer");
        assert!(result.message.contains("War hammer"));
    }

    #[test]
    fn drop_item() {
        let mut state = state_with_fighter();
        state
            .party
            .find_member_mut("Aldric")
            .unwrap()
            .inventory
            .push(Item::new("Sword", 60.0, 10));
        let result = action_drop(&mut state, "Aldric", "Sword").expect("drop should succeed");
        assert!(result.message.contains("drops Sword"));
        assert!(state
            .party
            .find_member("Aldric")
            .unwrap()
            .inventory
            .is_empty());
    }

    #[test]
    fn equip_updates_ac() {
        let mut state = state_with_fighter();
        action_buy(&mut state, "Aldric", "Leather").expect("buy leather should succeed");
        let result = action_equip(&mut state, "Aldric", "Leather").expect("equip should succeed");
        assert!(result.message.contains("equips Leather"));
        assert_eq!(result.ac, 7);
        assert_eq!(state.party.find_member("Aldric").unwrap().ac, 7);
    }

    fn state_with_dungeon_treasure() -> GameState {
        use crate::state::dungeon::{DungeonState, PlacedTreasureInstance, Room};

        let mut state = state_with_fighter();
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

    #[test]
    fn loot_from_current_room_marks_treasure_taken() {
        let mut state = state_with_dungeon_treasure();
        let result = action_loot(&mut state, "Aldric", "Ruby gem", None)
            .expect("loot from room should succeed");
        assert!(result.message.contains("500 gp"));
        assert_eq!(result.value_gp, 500);

        let room = state.dungeon.as_ref().unwrap().find_room(0).unwrap();
        assert!(room.placed_treasure[0].taken);
    }

    #[test]
    fn loot_adhoc_item_in_dungeon() {
        let mut state = state_with_dungeon_treasure();
        let result = action_loot(&mut state, "Aldric", "Diamond", Some(200))
            .expect("ad-hoc loot in dungeon should succeed");
        assert!(result.message.contains("picks up Diamond"));
        assert!(result.message.contains("200 gp"));
        assert_eq!(result.value_gp, 200);
    }

    #[test]
    fn loot_adhoc_item_in_dungeon_no_value() {
        let mut state = state_with_dungeon_treasure();
        let result = action_loot(&mut state, "Aldric", "Old scroll", None)
            .expect("ad-hoc loot without value should succeed");
        assert!(result.message.contains("picks up Old scroll"));
        assert_eq!(result.value_gp, 0);
    }

    #[test]
    fn list_equipment_has_items() {
        let result = action_list_equipment();
        assert!(!result.weapons.is_empty(), "should have weapons");
        assert!(!result.armour.is_empty(), "should have armour");
        assert!(!result.gear.is_empty(), "should have gear");
        assert!(!result.ammunition.is_empty(), "should have ammunition");
    }

    #[test]
    fn list_equipment_contains_sword() {
        let result = action_list_equipment();
        assert!(
            result.weapons.iter().any(|w| w.name == "Sword"),
            "weapons should contain Sword"
        );
    }

    #[test]
    fn list_equipment_contains_chainmail() {
        let result = action_list_equipment();
        assert!(
            result.armour.iter().any(|a| a.name == "Chainmail"),
            "armour should contain Chainmail"
        );
    }

    #[test]
    fn list_equipment_all_have_cost() {
        let result = action_list_equipment();
        for w in &result.weapons {
            assert!(w.cost_gp > 0, "weapon {} should have cost > 0", w.name);
        }
        for a in &result.armour {
            assert!(a.cost_gp > 0, "armour {} should have cost > 0", a.name);
        }
    }
}
