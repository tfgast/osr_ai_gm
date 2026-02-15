use super::{Command, CommandResult};
use crate::engine::inventory;
use crate::persist::GameState;

pub struct BuyCommand;
impl Command for BuyCommand {
    fn name(&self) -> &str {
        "buy"
    }
    fn help(&self) -> &str {
        "Buy equipment (buy <character> <item_name>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: buy <character_name> <item_name>");
        }

        let char_name = args[0];
        let item_name = args[1..].join(" ");

        match inventory::action_buy(state, char_name, &item_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct DropCommand;
impl Command for DropCommand {
    fn name(&self) -> &str {
        "drop"
    }
    fn help(&self) -> &str {
        "Drop an item (drop <character> <item_name>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: drop <character_name> <item_name>");
        }

        let char_name = args[0];
        let item_name = args[1..].join(" ");

        match inventory::action_drop(state, char_name, &item_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct LootCommand;
impl Command for LootCommand {
    fn name(&self) -> &str {
        "loot"
    }
    fn help(&self) -> &str {
        "Pick up loot (loot <character> <item_name> [value_gp])"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: loot <character_name> <item_name> [value_gp]");
        }
        let char_name = args[0];

        let (item_parts, explicit_gp) = if args.len() >= 3 {
            if let Ok(v) = args[args.len() - 1].parse::<u64>() {
                (&args[1..args.len() - 1], Some(v))
            } else {
                (&args[1..], None)
            }
        } else {
            (&args[1..], None)
        };

        let item_name = item_parts.join(" ");
        if item_name.is_empty() {
            return CommandResult::error("usage: loot <character_name> <item_name> [value_gp]");
        }

        match inventory::action_loot(state, char_name, &item_name, explicit_gp) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct ShopCommand;
impl Command for ShopCommand {
    fn name(&self) -> &str {
        "shop"
    }
    fn help(&self) -> &str {
        "List available equipment for purchase (shop [category])"
    }
    fn execute(&self, args: &[&str], _state: &mut GameState) -> CommandResult {
        let result = inventory::action_list_equipment();
        let filter = if args.is_empty() {
            None
        } else {
            Some(args.join(" ").to_lowercase())
        };

        let mut output = String::new();

        let show_section = |out: &mut String,
                            title: &str,
                            items: &[inventory::results::EquipmentEntry]| {
            if items.is_empty() {
                return;
            }
            out.push_str(&format!("=== {} ===\n", title));
            for item in items {
                out.push_str(&format!("  {:30} {:>4} gp\n", item.name, item.cost_gp));
            }
            out.push('\n');
        };

        let show_all = filter.is_none();
        let cat = filter.as_deref().unwrap_or("");

        if show_all || cat == "weapons" || cat == "weapon" {
            show_section(&mut output, "Weapons", &result.weapons);
        }
        if show_all || cat == "armour" || cat == "armor" {
            show_section(&mut output, "Armour", &result.armour);
        }
        if show_all || cat == "gear" || cat == "adventuring gear" {
            show_section(&mut output, "Adventuring Gear", &result.gear);
        }
        if show_all || cat == "ammunition" || cat == "ammo" {
            show_section(&mut output, "Ammunition", &result.ammunition);
        }

        if output.is_empty() {
            return CommandResult::error(format!(
                "unknown category '{}'. Try: weapons, armour, gear, ammunition",
                cat
            ));
        }

        let total =
            result.weapons.len() + result.armour.len() + result.gear.len() + result.ammunition.len();
        output.push_str(&format!("{} items available. Use 'buy <character> <item>' to purchase.", total));

        CommandResult::ok(output)
    }
}

pub struct EquipCommand;
impl Command for EquipCommand {
    fn name(&self) -> &str {
        "equip"
    }
    fn help(&self) -> &str {
        "Equip an item (equip <character> <item_name>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: equip <character_name> <item_name>");
        }

        let char_name = args[0];
        let item_name = args[1..].join(" ");

        match inventory::action_equip(state, char_name, &item_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

pub struct UnequipCommand;
impl Command for UnequipCommand {
    fn name(&self) -> &str {
        "unequip"
    }
    fn help(&self) -> &str {
        "Unequip an item (unequip <character> <item_name>)"
    }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: unequip <character_name> <item_name>");
        }

        let char_name = args[0];
        let item_name = args[1..].join(" ");

        match inventory::action_unequip(state, char_name, &item_name) {
            Ok(result) => CommandResult::ok(result.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Character, Item};
    use crate::rules::class::Class;

    fn state_with_fighter() -> GameState {
        let mut state = GameState::new();
        let mut c = Character::new("Aldric", Class::Fighter);
        c.gold_gp = 100;
        c.abilities.dexterity = 10; // no DEX mod
        state.party.add_member(c);
        state
    }

    #[test]
    fn buy_sword() {
        let cmd = BuyCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "Sword"], &mut state);
        assert!(result.output.contains("buys Sword"));
        assert!(result.output.contains("10 gp"));
        let c = state.party.find_member("Aldric").unwrap();
        assert_eq!(c.gold_gp, 90);
        assert_eq!(c.inventory.len(), 1);
        assert_eq!(c.inventory[0].name, "Sword");
    }

    #[test]
    fn buy_insufficient_gold() {
        let cmd = BuyCommand;
        let mut state = state_with_fighter();
        state.party.find_member_mut("Aldric").unwrap().gold_gp = 5;
        let result = cmd.execute(&["Aldric", "Plate", "mail"], &mut state);
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("5 gp"));
    }

    #[test]
    fn buy_unknown_item() {
        let cmd = BuyCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "Phaser"], &mut state);
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("unknown item"));
    }

    #[test]
    fn buy_unknown_character() {
        let cmd = BuyCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Nobody", "Sword"], &mut state);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn drop_item() {
        let cmd = DropCommand;
        let mut state = state_with_fighter();
        state
            .party
            .find_member_mut("Aldric")
            .unwrap()
            .inventory
            .push(Item::new("Sword", 6.0, 10));
        let result = cmd.execute(&["Aldric", "Sword"], &mut state);
        assert!(result.output.contains("drops Sword"));
        assert!(state
            .party
            .find_member("Aldric")
            .unwrap()
            .inventory
            .is_empty());
    }

    #[test]
    fn drop_missing_item() {
        let cmd = DropCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "Sword"], &mut state);
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("does not have"));
    }

    #[test]
    fn loot_item() {
        let cmd = LootCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "Ruby", "gem", "500"], &mut state);
        assert!(result.output.contains("picks up Ruby gem"));
        assert!(result.output.contains("500 gp"));
        let c = state.party.find_member("Aldric").unwrap();
        assert_eq!(c.inventory.len(), 1);
        assert_eq!(c.inventory[0].name, "Ruby gem");
        assert_eq!(c.inventory[0].value_gp, 500);
    }

    #[test]
    fn loot_item_no_value() {
        let cmd = LootCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "Old", "key"], &mut state);
        assert!(result.output.contains("picks up Old key"));
        assert!(!result.output.contains("gp"));
        let c = state.party.find_member("Aldric").unwrap();
        assert_eq!(c.inventory[0].value_gp, 0);
    }

    #[test]
    fn equip_armour_updates_ac() {
        let equip = EquipCommand;
        let buy = BuyCommand;
        let mut state = state_with_fighter();
        buy.execute(&["Aldric", "Leather"], &mut state);
        let result = equip.execute(&["Aldric", "Leather"], &mut state);
        assert!(result.output.contains("equips Leather"));
        assert!(result.output.contains("AC 7"));
        let c = state.party.find_member("Aldric").unwrap();
        assert_eq!(c.ac, 7);
    }

    #[test]
    fn equip_armour_and_shield() {
        let equip = EquipCommand;
        let buy = BuyCommand;
        let mut state = state_with_fighter();
        buy.execute(&["Aldric", "Chainmail"], &mut state);
        buy.execute(&["Aldric", "Shield"], &mut state);
        equip.execute(&["Aldric", "Chainmail"], &mut state);
        let result = equip.execute(&["Aldric", "Shield"], &mut state);
        assert!(result.output.contains("equips Shield"));
        assert!(result.output.contains("AC 4"));
    }

    #[test]
    fn unequip_armour_resets_ac() {
        let equip = EquipCommand;
        let unequip = UnequipCommand;
        let buy = BuyCommand;
        let mut state = state_with_fighter();
        buy.execute(&["Aldric", "Leather"], &mut state);
        equip.execute(&["Aldric", "Leather"], &mut state);
        let result = unequip.execute(&["Aldric", "Leather"], &mut state);
        assert!(result.output.contains("unequips Leather"));
        assert!(result.output.contains("AC 9"));
    }

    #[test]
    fn equip_already_equipped_shows_error() {
        let equip = EquipCommand;
        let buy = BuyCommand;
        let mut state = state_with_fighter();
        buy.execute(&["Aldric", "Leather"], &mut state);
        equip.execute(&["Aldric", "Leather"], &mut state);
        let result = equip.execute(&["Aldric", "Leather"], &mut state);
        assert!(!result.success);
        assert!(result.output.contains("already has Leather equipped"));
        assert!(result.output.contains("unequip"));
    }

    #[test]
    fn unequip_not_equipped_shows_error() {
        let unequip = UnequipCommand;
        let buy = BuyCommand;
        let mut state = state_with_fighter();
        buy.execute(&["Aldric", "Leather"], &mut state);
        let result = unequip.execute(&["Aldric", "Leather"], &mut state);
        assert!(!result.success);
        assert!(result.output.contains("does not have Leather equipped"));
    }

    #[test]
    fn equip_missing_item() {
        let cmd = EquipCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "Sword"], &mut state);
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("does not have"));
    }

    #[test]
    fn buy_gear() {
        let cmd = BuyCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "Rope", "(50')"], &mut state);
        assert!(result.output.contains("buys Rope (50')"));
        assert!(result.output.contains("1 gp"));
        assert_eq!(state.party.find_member("Aldric").unwrap().gold_gp, 99);
    }

    #[test]
    fn buy_ammunition() {
        let cmd = BuyCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "Arrows", "(quiver", "of", "20)"], &mut state);
        assert!(result.output.contains("buys Arrows (quiver of 20)"));
        assert!(result.output.contains("5 gp"));
    }

    #[test]
    fn buy_suggests_chain_mail() {
        let cmd = BuyCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "chain"], &mut state);
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("Did you mean"));
        assert!(result.output.contains("Chainmail"));
    }

    #[test]
    fn buy_suggests_arrows() {
        let cmd = BuyCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "arrows"], &mut state);
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("Did you mean"));
        assert!(result.output.contains("Arrows (quiver of 20)"));
    }

    #[test]
    fn buy_suggests_rope() {
        let cmd = BuyCommand;
        let mut state = state_with_fighter();
        let result = cmd.execute(&["Aldric", "rope"], &mut state);
        assert!(result.output.contains("Error"));
        assert!(result.output.contains("Did you mean"));
        assert!(result.output.contains("Rope (50')"));
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
        dungeon.add_room(Room::new(1, "Empty Room")).unwrap();
        dungeon.current_room = Some(0);
        dungeon.explored.insert(0);
        state.dungeon = Some(dungeon);
        state
    }

    #[test]
    fn loot_from_current_room() {
        let cmd = LootCommand;
        let mut state = state_with_dungeon_treasure();
        let result = cmd.execute(&["Aldric", "Ruby", "gem"], &mut state);
        assert!(result.output.contains("picks up Ruby gem"));
        assert!(result.output.contains("500 gp"));
        let room = state.dungeon.as_ref().unwrap().find_room(0).unwrap();
        assert!(room.placed_treasure[0].taken);
    }

    #[test]
    fn loot_adhoc_item_in_dungeon() {
        let cmd = LootCommand;
        let mut state = state_with_dungeon_treasure();
        let result = cmd.execute(&["Aldric", "gold", "coins", "50"], &mut state);
        assert!(result.output.contains("picks up gold coins"));
        assert!(result.output.contains("50 gp"));
    }

    #[test]
    fn loot_adhoc_item_in_dungeon_no_value() {
        let cmd = LootCommand;
        let mut state = state_with_dungeon_treasure();
        let result = cmd.execute(&["Aldric", "Old", "scroll"], &mut state);
        assert!(result.output.contains("picks up Old scroll"));
        assert!(!result.output.contains("gp"));
    }

    #[test]
    fn loot_already_taken_becomes_adhoc() {
        let cmd = LootCommand;
        let mut state = state_with_dungeon_treasure();
        // First loot takes from room treasure (500 gp)
        let result1 = cmd.execute(&["Aldric", "Ruby", "gem"], &mut state);
        assert!(result1.output.contains("500 gp"));
        // Second loot of same name creates ad-hoc item (0 gp, no explicit value)
        let result2 = cmd.execute(&["Aldric", "Ruby", "gem"], &mut state);
        assert!(result2.output.contains("picks up Ruby gem"));
        assert!(!result2.output.contains("gp"));
        let c = state.party.find_member("Aldric").unwrap();
        assert_eq!(c.inventory.len(), 2);
    }

    #[test]
    fn loot_zero_value_item_from_room() {
        let cmd = LootCommand;
        let mut state = state_with_dungeon_treasure();
        let result = cmd.execute(&["Aldric", "Old", "key"], &mut state);
        assert!(result.output.contains("picks up Old key"));
        assert!(!result.output.contains("gp"));
    }

    #[test]
    fn shop_lists_all_categories() {
        let cmd = ShopCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&[], &mut state);
        assert!(result.success);
        assert!(result.output.contains("Weapons"));
        assert!(result.output.contains("Armour"));
        assert!(result.output.contains("Adventuring Gear"));
        assert!(result.output.contains("Ammunition"));
        assert!(result.output.contains("Sword"));
        assert!(result.output.contains("Chainmail"));
        assert!(result.output.contains("items available"));
    }

    #[test]
    fn shop_filters_by_category() {
        let cmd = ShopCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["weapons"], &mut state);
        assert!(result.success);
        assert!(result.output.contains("Weapons"));
        assert!(!result.output.contains("Armour"));
        assert!(result.output.contains("Sword"));
    }

    #[test]
    fn shop_invalid_category() {
        let cmd = ShopCommand;
        let mut state = GameState::new();
        let result = cmd.execute(&["potions"], &mut state);
        assert!(!result.success);
        assert!(result.output.contains("unknown category"));
    }
}
