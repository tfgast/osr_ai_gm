use super::{Command, CommandResult};
use crate::model::Item;
use crate::persist::GameState;
use crate::rules::equipment::{self, AMMUNITION};

/// Look up an item across all equipment tables.
/// Returns (name, cost_gp, weight as f32).
fn find_buyable(name: &str) -> Option<(&'static str, u32, f32)> {
    if let Some(w) = equipment::find_weapon(name) {
        return Some((w.name, w.cost_gp, w.weight_coins as f32));
    }
    if let Some(a) = equipment::find_armour(name) {
        if a.cost_gp > 0 {
            return Some((a.name, a.cost_gp, a.weight_coins as f32));
        }
    }
    if let Some(g) = equipment::find_gear(name) {
        return Some((g.name, g.cost_gp, 0.0));
    }
    if let Some(a) = AMMUNITION.iter().find(|a| a.name.eq_ignore_ascii_case(name)) {
        return Some((a.name, a.cost_gp, 0.0));
    }
    None
}

pub struct BuyCommand;
impl Command for BuyCommand {
    fn name(&self) -> &str { "buy" }
    fn help(&self) -> &str { "Buy equipment (buy <character> <item_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: buy <character_name> <item_name>");
        }
        let char_name = args[0];
        let item_name = args[1..].join(" ");

        let (canonical_name, cost, weight) = match find_buyable(&item_name) {
            Some(info) => info,
            None => return CommandResult::error(format!(
                "unknown item '{}'. Check equipment tables.", item_name
            )),
        };

        let character = match state.party.find_member_mut(char_name) {
            Some(c) => c,
            None => return CommandResult::error(format!(
                "no party member named '{}'.", char_name
            )),
        };

        if character.gold_gp < cost {
            return CommandResult::error(format!(
                "{} has {} gp but {} costs {} gp.",
                character.name, character.gold_gp, canonical_name, cost
            ));
        }

        character.gold_gp -= cost;
        character.inventory.push(Item::new(canonical_name, weight, cost));

        CommandResult::ok(format!(
            "{} buys {} for {} gp. ({} gp remaining)",
            character.name, canonical_name, cost, character.gold_gp
        ))
    }
}

pub struct DropCommand;
impl Command for DropCommand {
    fn name(&self) -> &str { "drop" }
    fn help(&self) -> &str { "Drop an item (drop <character> <item_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: drop <character_name> <item_name>");
        }
        let char_name = args[0];
        let item_name = args[1..].join(" ");

        let character = match state.party.find_member_mut(char_name) {
            Some(c) => c,
            None => return CommandResult::error(format!(
                "no party member named '{}'.", char_name
            )),
        };

        let idx = character.inventory.iter()
            .position(|i| i.name.eq_ignore_ascii_case(&item_name));

        match idx {
            Some(i) => {
                let dropped = character.inventory.remove(i);
                CommandResult::ok(format!(
                    "{} drops {}.", character.name, dropped.name
                ))
            }
            None => CommandResult::error(format!(
                "{} does not have '{}'.", character.name, item_name
            )),
        }
    }
}

pub struct LootCommand;
impl Command for LootCommand {
    fn name(&self) -> &str { "loot" }
    fn help(&self) -> &str { "Pick up loot (loot <character> <item_name> [value_gp])" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: loot <character_name> <item_name> [value_gp]");
        }
        let char_name = args[0];

        // Parse optional trailing value_gp (last arg if numeric)
        let (item_parts, value_gp) = if args.len() >= 3 {
            if let Ok(v) = args[args.len() - 1].parse::<u32>() {
                (&args[1..args.len() - 1], v)
            } else {
                (&args[1..], 0)
            }
        } else {
            (&args[1..], 0)
        };
        let item_name = item_parts.join(" ");

        if item_name.is_empty() {
            return CommandResult::error("usage: loot <character_name> <item_name> [value_gp]");
        }

        let character = match state.party.find_member_mut(char_name) {
            Some(c) => c,
            None => return CommandResult::error(format!(
                "no party member named '{}'.", char_name
            )),
        };

        character.inventory.push(Item::new(&item_name, 0.0, value_gp));

        let mut out = format!("{} picks up {}.", character.name, item_name);
        if value_gp > 0 {
            out.push_str(&format!(" (worth {} gp)", value_gp));
        }
        CommandResult::ok(out)
    }
}

pub struct EquipCommand;
impl Command for EquipCommand {
    fn name(&self) -> &str { "equip" }
    fn help(&self) -> &str { "Equip or unequip an item (equip <character> <item_name>)" }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: equip <character_name> <item_name>");
        }
        let char_name = args[0];
        let item_name = args[1..].join(" ");

        let character = match state.party.find_member_mut(char_name) {
            Some(c) => c,
            None => return CommandResult::error(format!(
                "no party member named '{}'.", char_name
            )),
        };

        let idx = character.inventory.iter()
            .position(|i| i.name.eq_ignore_ascii_case(&item_name));

        let idx = match idx {
            Some(i) => i,
            None => return CommandResult::error(format!(
                "{} does not have '{}'.", character.name, item_name
            )),
        };

        let was_equipped = character.inventory[idx].equipped;
        character.inventory[idx].equipped = !was_equipped;

        let action = if was_equipped { "unequips" } else { "equips" };
        let item_display = character.inventory[idx].name.clone();

        // Recalculate AC from equipped armour
        let armour_ac = character.inventory.iter()
            .filter(|i| i.equipped)
            .filter_map(|i| equipment::find_armour(&i.name))
            .filter(|a| !a.is_shield)
            .map(|a| a.ac)
            .min()
            .unwrap_or(9);

        let has_shield = character.inventory.iter()
            .any(|i| i.equipped && equipment::find_armour(&i.name).map(|a| a.is_shield).unwrap_or(false));

        let dex_mod = crate::rules::ability::dex_ac_mod(character.abilities.dexterity);
        character.ac = equipment::calculate_ac(armour_ac, has_shield, dex_mod);

        CommandResult::ok(format!(
            "{} {} {}. (AC {})",
            character.name, action, item_display, character.ac
        ))
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
        state.party.find_member_mut("Aldric").unwrap()
            .inventory.push(Item::new("Sword", 60.0, 10));
        let result = cmd.execute(&["Aldric", "Sword"], &mut state);
        assert!(result.output.contains("drops Sword"));
        assert!(state.party.find_member("Aldric").unwrap().inventory.is_empty());
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
        // Buy and equip leather armour
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
        buy.execute(&["Aldric", "Chain", "mail"], &mut state);
        buy.execute(&["Aldric", "Shield"], &mut state);
        equip.execute(&["Aldric", "Chain", "mail"], &mut state);
        let result = equip.execute(&["Aldric", "Shield"], &mut state);
        assert!(result.output.contains("equips Shield"));
        // Chain mail (5) + shield (-1) = AC 4
        assert!(result.output.contains("AC 4"));
    }

    #[test]
    fn unequip_armour_resets_ac() {
        let equip = EquipCommand;
        let buy = BuyCommand;
        let mut state = state_with_fighter();
        buy.execute(&["Aldric", "Leather"], &mut state);
        equip.execute(&["Aldric", "Leather"], &mut state);
        let result = equip.execute(&["Aldric", "Leather"], &mut state);
        assert!(result.output.contains("unequips Leather"));
        assert!(result.output.contains("AC 9"));
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
}
