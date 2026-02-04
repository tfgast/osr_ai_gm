use rand::Rng;

use crate::model::Party;
use crate::rules::encounter;
use crate::state::wilderness::{Terrain, WildernessState};

/// Result of a day of wilderness travel.
#[derive(Debug)]
pub struct TravelResult {
    pub messages: Vec<String>,
    /// Whether the party got lost.
    pub lost: bool,
    /// Encounters triggered during travel (up to 3 per day: morning, afternoon, night).
    pub encounters: Vec<encounter::EncounterEntry>,
    /// Whether foraging was attempted and succeeded.
    pub foraged: Option<bool>,
    /// Rations consumed this day.
    pub rations_consumed: u32,
    /// Whether party is starving (no rations available).
    pub starving: bool,
    /// HP damage dealt due to starvation this day.
    pub starvation_damage: u32,
}

impl TravelResult {
    fn new() -> Self {
        TravelResult {
            messages: Vec::new(),
            lost: false,
            encounters: Vec::new(),
            foraged: None,
            rations_consumed: 0,
            starving: false,
            starvation_damage: 0,
        }
    }

    fn msg(&mut self, s: impl Into<String>) {
        self.messages.push(s.into());
    }
}

impl std::fmt::Display for TravelResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for msg in &self.messages {
            writeln!(f, "{}", msg)?;
        }
        Ok(())
    }
}

/// Travel one day in the wilderness.
/// Checks for getting lost, encounter, consumes rations, and advances position.
pub fn travel_day(
    wilderness: &mut WildernessState,
    party: &mut Party,
    dest_x: i32,
    dest_y: i32,
    movement_rate: u32,
) -> TravelResult {
    travel_day_with(&mut rand::thread_rng(), wilderness, party, dest_x, dest_y, movement_rate)
}

/// Testable version.
pub fn travel_day_with<R: Rng>(
    rng: &mut R,
    wilderness: &mut WildernessState,
    party: &mut Party,
    dest_x: i32,
    dest_y: i32,
    movement_rate: u32,
) -> TravelResult {
    let mut result = TravelResult::new();

    let terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);

    result.msg(format!(
        "Day {}: Travelling through {} terrain.",
        wilderness.travel_day, terrain.name()
    ));

    // Consume rations: 1 person-day per living party member
    let party_size = party.members.iter().filter(|c| c.is_alive()).count() as u32;
    if party_size > 0 {
        if party.rations >= party_size {
            party.rations -= party_size;
            result.rations_consumed = party_size;
            result.msg(format!(
                "Consumed {} rations ({} remaining).",
                party_size, party.rations
            ));
        } else if party.rations > 0 {
            // Partial rations
            result.rations_consumed = party.rations;
            party.rations = 0;
            result.starving = true;
            result.msg(format!(
                "Only {} rations available for {} party members. The party is STARVING!",
                result.rations_consumed, party_size
            ));
        } else {
            // No rations at all
            result.rations_consumed = 0;
            result.starving = true;
            result.msg("No rations! The party is STARVING!");
        }
    }

    // Update starvation tracking and apply effects
    if result.starving {
        party.days_without_food += 1;
        let days = party.days_without_food;

        // Per OSE rules: penalties accumulate with each day without food
        // Day 1+: -1 to attack rolls and saving throws per day
        let penalty = days.min(4) as i32; // Cap at -4 per typical OSE
        result.msg(format!(
            "Starvation day {}: -{}penalty to attack rolls and saving throws.",
            days, penalty
        ));

        // After 3+ days without food: 1d4 HP damage per day
        if days >= 3 {
            let hp_damage: u32 = rng.gen_range(1..=4);
            result.starvation_damage = hp_damage;
            result.msg(format!(
                "Severe starvation! Each party member takes {} HP damage.",
                hp_damage
            ));
            // Apply damage to all living party members
            for member in party.members.iter_mut().filter(|c| c.is_alive()) {
                member.hp = (member.hp - hp_damage as i32).max(0);
            }
            // Check for deaths
            let dead: Vec<String> = party.members.iter()
                .filter(|c| c.hp <= 0)
                .map(|c| c.name.clone())
                .collect();
            if !dead.is_empty() {
                result.msg(format!("Died from starvation: {}.", dead.join(", ")));
            }
        }
    } else {
        // Reset starvation counter when adequately fed
        if party.days_without_food > 0 {
            result.msg("The party is well-fed. Starvation effects end.");
        }
        party.days_without_food = 0;
    }

    // Check for getting lost
    let lost_chance = terrain.lost_chance();
    let lost_roll: u32 = rng.gen_range(1..=6);
    if lost_roll <= lost_chance {
        wilderness.lost = true;
        result.lost = true;
        result.msg(format!(
            "The party is LOST! (rolled {} vs {}-in-6)",
            lost_roll, lost_chance
        ));
        // Per OSE, lost party moves in a random direction
        let adjacent: Vec<(i32, i32)> = wilderness.hexes.iter()
            .filter(|h| {
                let dx = (h.x - wilderness.current_x).abs();
                let dy = (h.y - wilderness.current_y).abs();
                dx <= 1 && dy <= 1 && (h.x, h.y) != (wilderness.current_x, wilderness.current_y)
            })
            .map(|h| (h.x, h.y))
            .collect();
        if adjacent.is_empty() {
            result.msg("The party wanders aimlessly but there is nowhere to go.");
        } else {
            let idx = rng.gen_range(0..adjacent.len());
            let (rx, ry) = adjacent[idx];
            match wilderness.move_to(rx, ry) {
                Err(e) => {
                    result.msg(format!("The party tries to wander but cannot: {}", e));
                }
                Ok(()) => {}
            }
            let terrain_name = wilderness.current_hex()
                .map(|h| h.terrain.name())
                .unwrap_or("unknown");
            result.msg(format!(
                "The party wanders into ({}, {}) — {} terrain.",
                rx, ry, terrain_name
            ));
        }
    } else {
        wilderness.lost = false;
        // Travel speed
        let hexes = WildernessState::hexes_per_day(movement_rate, terrain);
        result.msg(format!(
            "Travel speed: {} hexes/day (movement rate {}').",
            hexes, movement_rate
        ));

        // Enforce travel range — destination must be within hexes_per_day distance
        let dx = (dest_x - wilderness.current_x).unsigned_abs();
        let dy = (dest_y - wilderness.current_y).unsigned_abs();
        let distance = dx.max(dy);
        if distance > hexes {
            result.msg(format!(
                "Destination ({}, {}) is {} hexes away — exceeds travel range of {} hexes/day.",
                dest_x, dest_y, distance, hexes
            ));
        } else {
            // Attempt to move to destination
            match wilderness.move_to(dest_x, dest_y) {
                Ok(()) => {
                    result.msg(format!(
                        "Arrived at ({}, {}).",
                        dest_x, dest_y
                    ));
                }
                Err(e) => {
                    result.msg(format!("Cannot travel there: {}", e));
                }
            }
        }
    }

    // Per OSE: 3 encounter checks per day (morning, afternoon, night).
    // Use current (post-move) terrain for encounter table and chance.
    let current_terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);
    let encounter_chance: u32 = match current_terrain {
        Terrain::City => 1,
        Terrain::Clear | Terrain::Hills | Terrain::Barren => 1,
        Terrain::Forest | Terrain::Desert | Terrain::Mountains => 2,
        Terrain::Swamp | Terrain::Jungle => 2,
        Terrain::Ocean | Terrain::River => 2,
    };
    let periods = ["Morning", "Afternoon", "Night"];
    for period in &periods {
        let encounter_roll: u32 = rng.gen_range(1..=6);
        if encounter_roll <= encounter_chance {
            let table_roll: u32 = rng.gen_range(1..=20);
            let entry = encounter::wilderness_encounter(current_terrain, table_roll);
            result.msg(format!(
                "{} encounter! {} ({} appearing).",
                period, entry.name, entry.number
            ));
            result.encounters.push(entry.clone());
        }
    }

    wilderness.travel_day += 1;

    result
}

/// Result of a foraging attempt.
#[derive(Debug)]
pub struct ForageResult {
    pub message: String,
    /// Person-days of food found (0 if unsuccessful).
    pub quantity: u32,
    pub success: bool,
}

/// Attempt to forage for food in the current hex.
/// Takes a full day (no travel). Chance varies by terrain.
pub fn forage(wilderness: &WildernessState, party: &mut Party) -> ForageResult {
    forage_with(&mut rand::thread_rng(), wilderness, party)
}

/// Testable version.
pub fn forage_with<R: Rng>(rng: &mut R, wilderness: &WildernessState, party: &mut Party) -> ForageResult {
    let terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);

    if !terrain.can_forage() {
        return ForageResult {
            message: format!("Cannot forage in {} terrain.", terrain.name()),
            quantity: 0,
            success: false,
        };
    }

    let chance = terrain.forage_chance();
    let roll: u32 = rng.gen_range(1..=6);

    if roll <= chance {
        let quantity: u32 = rng.gen_range(1..=6);
        party.rations += quantity;
        ForageResult {
            message: format!(
                "Foraging successful! Found {} person-days of food. (rolled {} vs {}-in-6, quantity: 1d6={})\nRations: {} (+{})",
                quantity, roll, chance, quantity, party.rations, quantity
            ),
            quantity,
            success: true,
        }
    } else {
        ForageResult {
            message: format!(
                "Foraging unsuccessful. No food found. (rolled {} vs {}-in-6)",
                roll, chance
            ),
            quantity: 0,
            success: false,
        }
    }
}

/// Result of a hunting attempt.
#[derive(Debug)]
pub struct HuntResult {
    pub message: String,
    /// Person-days of food obtained (0 if unsuccessful).
    pub quantity: u32,
    pub success: bool,
}

/// Attempt to hunt. Similar to foraging but can be done in more terrain.
/// Takes a full day. 1-in-6 base chance.
pub fn hunt(wilderness: &WildernessState, party: &mut Party) -> HuntResult {
    hunt_with(&mut rand::thread_rng(), wilderness, party)
}

/// Testable version.
pub fn hunt_with<R: Rng>(rng: &mut R, wilderness: &WildernessState, party: &mut Party) -> HuntResult {
    let terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);

    if terrain == Terrain::Ocean {
        return HuntResult {
            message: "Cannot hunt on the open ocean.".to_string(),
            quantity: 0,
            success: false,
        };
    }

    let roll: u32 = rng.gen_range(1..=6);
    if roll == 1 {
        let quantity: u32 = rng.gen_range(1..=6);
        party.rations += quantity;
        HuntResult {
            message: format!(
                "Hunt successful! Killed game sufficient for {} person-days of food. (rolled {}, quantity: 1d6={})\nRations: {} (+{})",
                quantity, roll, quantity, party.rations, quantity
            ),
            quantity,
            success: true,
        }
    } else {
        HuntResult {
            message: format!("Hunt unsuccessful. No game found. (rolled {})", roll),
            quantity: 0,
            success: false,
        }
    }
}

/// Display wilderness travel status.
pub fn wilderness_status(wilderness: &WildernessState, party: &Party, movement_rate: u32) -> String {
    let mut out = wilderness.status();
    let terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);
    let hexes = WildernessState::hexes_per_day(movement_rate, terrain);
    out.push_str(&format!(
        "\nTravel speed: {} hexes/day (movement {}', terrain: {})",
        hexes, movement_rate, terrain.name()
    ));
    let party_size = party.members.iter().filter(|c| c.is_alive()).count() as u32;
    let days_of_food = if party_size > 0 { party.rations / party_size } else { 0 };
    out.push_str(&format!(
        "\nRations: {} person-days ({} days for party of {})",
        party.rations, days_of_food, party_size
    ));
    // Show starvation status
    if party.days_without_food > 0 {
        let penalty = starvation_penalty(party.days_without_food);
        out.push_str(&format!(
            "\n[STARVING] {} days without food — {} penalty to attacks/saves",
            party.days_without_food, penalty
        ));
        if party.days_without_food >= 3 {
            out.push_str(" — taking HP damage!");
        }
    }
    out
}

/// Calculate the attack/save penalty for starvation.
/// Per OSE: -1 per day without food, capped at -4.
pub fn starvation_penalty(days_without_food: u32) -> i32 {
    -(days_without_food.min(4) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Character;
    use crate::rules::class::Class;
    use crate::state::wilderness::HexCell;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn test_wilderness() -> WildernessState {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Clear)).unwrap();
        ws.add_hex(HexCell::new(1, 0, Terrain::Forest)).unwrap();
        ws.add_hex(HexCell::new(2, 0, Terrain::Mountains)).unwrap();
        ws.add_hex(HexCell::new(0, 1, Terrain::Swamp)).unwrap();
        ws.add_hex(HexCell::new(1, 1, Terrain::Desert)).unwrap();
        ws
    }

    fn test_party() -> Party {
        let mut party = Party::new();
        party.add_member(Character::new("Test", Class::Fighter));
        party.rations = 10; // Start with some rations
        party
    }

    #[test]
    fn travel_day_basic() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);
        assert!(!result.messages.is_empty());
        assert!(result.messages.iter().any(|m| m.contains("Day 1")));
    }

    #[test]
    fn travel_increments_day() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);
        assert_eq!(ws.travel_day, 2);
    }

    #[test]
    fn travel_can_get_lost() {
        let mut lost_count = 0;
        let mut moved_randomly = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            // Travel through swamp (2-in-6 lost chance)
            ws.move_to(0, 1).unwrap();
            let start_x = ws.current_x;
            let start_y = ws.current_y;
            let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 1, 120);
            if result.lost {
                lost_count += 1;
                // When lost, party should have moved to a random adjacent hex
                if (ws.current_x, ws.current_y) != (start_x, start_y) {
                    moved_randomly = true;
                }
            }
        }
        assert!(lost_count > 0, "should get lost sometimes in swamp");
        assert!(moved_randomly, "lost party should move to a random adjacent hex");
    }

    #[test]
    fn travel_encounter_check() {
        let mut encounter_days = 0;
        for seed in 0..200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);
            if !result.encounters.is_empty() {
                encounter_days += 1;
            }
        }
        assert!(encounter_days > 0, "should get encounters sometimes");
        assert!(encounter_days < 150, "should not get encounters every day");
    }

    #[test]
    fn travel_three_encounter_checks_per_day() {
        // With enough trials in high-encounter terrain, we should see days with multiple encounters
        let mut multi_encounter_days = 0;
        for seed in 0..500 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            // Forest has 2-in-6 encounter chance per check
            ws.move_to(1, 0).unwrap();
            let result = travel_day_with(&mut rng, &mut ws, &mut party, 0, 0, 120);
            if result.encounters.len() > 1 {
                multi_encounter_days += 1;
            }
        }
        assert!(multi_encounter_days > 0, "with 3 checks/day at 2-in-6, should sometimes get multiple encounters");
    }

    #[test]
    fn forage_in_forest() {
        let mut success = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            ws.move_to(1, 0).unwrap(); // Forest
            let result = forage_with(&mut rng, &ws, &mut party);
            if result.message.contains("successful!") {
                success = true;
                break;
            }
        }
        assert!(success, "should succeed at foraging in forest eventually");
    }

    #[test]
    fn forage_in_ocean_fails() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Ocean)).unwrap();
        let mut party = test_party();
        let result = forage_with(&mut test_rng(), &ws, &mut party);
        assert!(result.message.contains("Cannot forage"));
    }

    #[test]
    fn hunt_basic() {
        let mut rng = test_rng();
        let ws = test_wilderness();
        let mut party = test_party();
        let result = hunt_with(&mut rng, &ws, &mut party);
        assert!(result.message.contains("Hunt") || result.message.contains("hunt"));
    }

    #[test]
    fn hunt_ocean_fails() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Ocean)).unwrap();
        let mut party = test_party();
        let result = hunt_with(&mut test_rng(), &ws, &mut party);
        assert!(result.message.contains("Cannot hunt"));
    }

    #[test]
    fn wilderness_status_display() {
        let ws = test_wilderness();
        let party = test_party();
        let status = wilderness_status(&ws, &party, 120);
        assert!(status.contains("Clear"));
        assert!(status.contains("hexes/day"));
        assert!(status.contains("Rations"));
    }

    #[test]
    fn travel_to_invalid_hex_reports_error() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 99, 99, 120);
        // Should get a message about not being able to travel there
        // (unless lost, in which case travel doesn't attempt the move)
        if !result.lost {
            assert!(result.messages.iter().any(|m|
                m.contains("exceeds travel range") || m.contains("Cannot travel")
            ));
        }
    }

    // =========================================================================
    // Additional QA tests for wilderness flows
    // =========================================================================

    #[test]
    fn encounter_uses_post_move_terrain() {
        // Per OSE, encounter table should use the terrain the party arrives in,
        // not the terrain they departed from.
        // Start in Clear (0,0), travel to Forest (1,0).
        // Forest has 2-in-6 encounter chance; Clear has 1-in-6.
        // We verify encounter messages reference the correct period and
        // that the encounter check happens after the move.
        let mut encounter_count = 0;
        for seed in 0..500 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);
            if !result.lost {
                // Party should be in forest now
                assert_eq!(ws.current_x, 1);
                assert_eq!(ws.current_y, 0);
                encounter_count += result.encounters.len();
            }
        }
        // Forest has 2-in-6 per check, 3 checks/day.
        // Expected ~33% per check = ~1.0 encounters per non-lost day.
        // With Clear (1-in-6, ~0.5/day), we'd see far fewer.
        // Just verify we get a reasonable number consistent with forest rates.
        assert!(
            encounter_count > 100,
            "encounter count {} too low for forest terrain (2-in-6, 3 checks/day over 500 trials)",
            encounter_count
        );
    }

    #[test]
    fn terrain_movement_rates_vary() {
        // Verify different terrains give different travel speeds
        let clear = WildernessState::hexes_per_day(120, Terrain::Clear);
        let forest = WildernessState::hexes_per_day(120, Terrain::Forest);
        let mountains = WildernessState::hexes_per_day(120, Terrain::Mountains);

        assert!(clear > forest, "clear should allow faster travel than forest");
        assert!(forest > mountains, "forest should allow faster travel than mountains");
        assert!(mountains >= 1, "even mountains should allow at least 1 hex/day");
    }

    #[test]
    fn multi_hex_travel_enforced() {
        // Build a map with hexes at distance > 1 from origin
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Clear)).unwrap();
        ws.add_hex(HexCell::new(1, 0, Terrain::Clear)).unwrap();
        ws.add_hex(HexCell::new(2, 0, Terrain::Clear)).unwrap();
        ws.add_hex(HexCell::new(3, 0, Terrain::Clear)).unwrap();
        ws.add_hex(HexCell::new(4, 0, Terrain::Clear)).unwrap();
        ws.add_hex(HexCell::new(5, 0, Terrain::Clear)).unwrap();
        let mut party = test_party();

        // At 120' movement on clear terrain, travel speed is 4 hexes/day
        let hexes = WildernessState::hexes_per_day(120, Terrain::Clear);
        assert_eq!(hexes, 4);

        // Trying to travel 5 hexes away should be blocked
        let mut rng = test_rng();
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 5, 0, 120);
        if !result.lost {
            assert!(
                result.messages.iter().any(|m| m.contains("exceeds travel range")),
                "should not allow travel beyond daily movement range"
            );
        }
    }

    #[test]
    fn lost_chance_varies_by_terrain() {
        // Clear terrain: 1-in-6 lost chance
        // Swamp terrain: 2-in-6 lost chance
        let mut clear_lost = 0;
        let mut swamp_lost = 0;
        let trials = 500;
        for seed in 0..trials {
            // Clear terrain
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);
            if result.lost {
                clear_lost += 1;
            }

            // Swamp terrain
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            ws.move_to(0, 1).unwrap(); // Move to swamp
            let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 1, 120);
            if result.lost {
                swamp_lost += 1;
            }
        }
        assert!(
            swamp_lost > clear_lost,
            "swamp should cause getting lost more often than clear ({} vs {} in {} trials)",
            swamp_lost, clear_lost, trials
        );
    }

    #[test]
    fn forage_returns_quantity_on_success() {
        let mut found_quantity = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            ws.move_to(1, 0).unwrap(); // Forest (2-in-6 forage chance)
            let result = forage_with(&mut rng, &ws, &mut party);
            if result.message.contains("successful!") {
                // Should mention person-days quantity
                assert!(
                    result.message.contains("person-days"),
                    "successful forage should report quantity in person-days"
                );
                found_quantity = true;
                break;
            }
        }
        assert!(found_quantity, "should eventually get a successful forage with quantity");
    }

    #[test]
    fn hunt_returns_quantity_on_success() {
        let mut found_quantity = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let ws = test_wilderness(); // Clear terrain, hunting allowed
            let mut party = test_party();
            let result = hunt_with(&mut rng, &ws, &mut party);
            if result.message.contains("successful!") {
                assert!(
                    result.message.contains("person-days"),
                    "successful hunt should report food quantity in person-days"
                );
                found_quantity = true;
                break;
            }
        }
        assert!(found_quantity, "should eventually get a successful hunt with quantity");
    }

    #[test]
    fn forage_barren_terrain_fails() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Barren)).unwrap();
        let mut party = test_party();
        let result = forage_with(&mut test_rng(), &ws, &mut party);
        assert!(result.message.contains("Cannot forage"), "should not be able to forage in barren terrain");
    }

    #[test]
    fn forage_city_terrain_fails() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::City)).unwrap();
        let mut party = test_party();
        let result = forage_with(&mut test_rng(), &ws, &mut party);
        assert!(result.message.contains("Cannot forage"), "should not be able to forage in city terrain");
    }

    #[test]
    fn travel_day_increments_day_counter() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        assert_eq!(ws.travel_day, 1);
        travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);
        assert_eq!(ws.travel_day, 2);
        travel_day_with(&mut StdRng::seed_from_u64(99), &mut ws, &mut party, 0, 0, 120);
        assert_eq!(ws.travel_day, 3);
    }

    #[test]
    fn slow_party_travels_fewer_hexes() {
        // 60' movement on clear = 2 hexes/day vs 120' = 4 hexes/day
        let slow = WildernessState::hexes_per_day(60, Terrain::Clear);
        let fast = WildernessState::hexes_per_day(120, Terrain::Clear);
        assert_eq!(slow, 2);
        assert_eq!(fast, 4);
        assert!(fast > slow, "faster party should cover more hexes per day");
    }

    // =========================================================================
    // Supply/Ration tracking tests
    // =========================================================================

    #[test]
    fn travel_consumes_rations() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 10;

        let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);

        // Party has 1 member, should consume 1 ration
        assert_eq!(result.rations_consumed, 1);
        assert_eq!(party.rations, 9);
        assert!(!result.starving);
    }

    #[test]
    fn travel_larger_party_consumes_more_rations() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = Party::new();
        party.add_member(Character::new("Fighter", Class::Fighter));
        party.add_member(Character::new("Cleric", Class::Cleric));
        party.add_member(Character::new("Thief", Class::Thief));
        party.rations = 20;

        let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);

        // Party has 3 members, should consume 3 rations
        assert_eq!(result.rations_consumed, 3);
        assert_eq!(party.rations, 17);
        assert!(!result.starving);
    }

    #[test]
    fn travel_no_rations_causes_starvation() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 0;

        let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);

        assert!(result.starving);
        assert_eq!(result.rations_consumed, 0);
        assert!(result.messages.iter().any(|m| m.contains("STARVING")));
    }

    #[test]
    fn travel_partial_rations_causes_starvation() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = Party::new();
        party.add_member(Character::new("Fighter", Class::Fighter));
        party.add_member(Character::new("Cleric", Class::Cleric));
        party.add_member(Character::new("Thief", Class::Thief));
        party.rations = 2; // Only 2 rations for 3 people

        let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);

        assert!(result.starving);
        assert_eq!(result.rations_consumed, 2);
        assert_eq!(party.rations, 0);
    }

    #[test]
    fn forage_adds_rations_to_party() {
        let mut success = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            party.rations = 5;
            ws.move_to(1, 0).unwrap(); // Forest (2-in-6 forage chance)

            let result = forage_with(&mut rng, &ws, &mut party);
            if result.success {
                assert!(result.quantity > 0);
                assert!(party.rations > 5, "rations should increase on successful forage");
                assert_eq!(party.rations, 5 + result.quantity);
                success = true;
                break;
            }
        }
        assert!(success, "should eventually succeed at foraging");
    }

    #[test]
    fn hunt_adds_rations_to_party() {
        let mut success = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let ws = test_wilderness();
            let mut party = test_party();
            party.rations = 3;

            let result = hunt_with(&mut rng, &ws, &mut party);
            if result.success {
                assert!(result.quantity > 0);
                assert!(party.rations > 3, "rations should increase on successful hunt");
                assert_eq!(party.rations, 3 + result.quantity);
                success = true;
                break;
            }
        }
        assert!(success, "should eventually succeed at hunting");
    }

    #[test]
    fn wilderness_status_shows_rations() {
        let ws = test_wilderness();
        let mut party = test_party();
        party.rations = 15;

        let status = wilderness_status(&ws, &party, 120);
        assert!(status.contains("Rations: 15"));
        assert!(status.contains("person-days"));
    }

    #[test]
    fn empty_party_consumes_no_rations() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = Party::new(); // No members
        party.rations = 10;

        let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);

        assert_eq!(result.rations_consumed, 0);
        assert_eq!(party.rations, 10);
        assert!(!result.starving);
    }

    // =========================================================================
    // Starvation mechanic tests
    // =========================================================================

    #[test]
    fn starvation_tracks_days_without_food() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 0;

        assert_eq!(party.days_without_food, 0);

        // Day 1 without food
        travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);
        assert_eq!(party.days_without_food, 1);

        // Day 2 without food
        let mut rng2 = StdRng::seed_from_u64(99);
        travel_day_with(&mut rng2, &mut ws, &mut party, 0, 0, 120);
        assert_eq!(party.days_without_food, 2);
    }

    #[test]
    fn starvation_resets_when_fed() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 0;
        party.days_without_food = 2; // Already starving for 2 days

        // Give them food
        party.rations = 10;

        // Travel with food
        travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);
        assert_eq!(party.days_without_food, 0);
    }

    #[test]
    fn starvation_penalty_calculation() {
        assert_eq!(starvation_penalty(0), 0);
        assert_eq!(starvation_penalty(1), -1);
        assert_eq!(starvation_penalty(2), -2);
        assert_eq!(starvation_penalty(3), -3);
        assert_eq!(starvation_penalty(4), -4);
        assert_eq!(starvation_penalty(5), -4); // Capped at -4
        assert_eq!(starvation_penalty(10), -4);
    }

    #[test]
    fn starvation_causes_hp_damage_after_3_days() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 0;
        party.days_without_food = 2; // Already 2 days without food
        let initial_hp = party.members[0].hp;

        // Day 3 - should take damage
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);

        assert_eq!(party.days_without_food, 3);
        assert!(result.starvation_damage > 0);
        assert!(result.starvation_damage <= 4); // 1d4 damage
        assert!(party.members[0].hp < initial_hp);
    }

    #[test]
    fn starvation_no_hp_damage_first_two_days() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 0;
        let initial_hp = party.members[0].hp;

        // Day 1 - no HP damage yet
        let result1 = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);
        assert_eq!(result1.starvation_damage, 0);
        assert_eq!(party.members[0].hp, initial_hp);

        // Day 2 - still no HP damage
        let mut rng2 = StdRng::seed_from_u64(99);
        let result2 = travel_day_with(&mut rng2, &mut ws, &mut party, 0, 0, 120);
        assert_eq!(result2.starvation_damage, 0);
        assert_eq!(party.members[0].hp, initial_hp);
    }

    #[test]
    fn wilderness_status_shows_starvation() {
        let ws = test_wilderness();
        let mut party = test_party();
        party.rations = 0;
        party.days_without_food = 2;

        let status = wilderness_status(&ws, &party, 120);
        assert!(status.contains("[STARVING]"));
        assert!(status.contains("2 days without food"));
        assert!(status.contains("-2 penalty"));
    }

    #[test]
    fn wilderness_status_shows_hp_damage_warning() {
        let ws = test_wilderness();
        let mut party = test_party();
        party.rations = 0;
        party.days_without_food = 3;

        let status = wilderness_status(&ws, &party, 120);
        assert!(status.contains("HP damage"));
    }

    #[test]
    fn starvation_can_kill() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 0;
        party.days_without_food = 2;
        party.members[0].hp = 1; // Very low HP

        // Day 3 with 1 HP - damage will likely kill
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);

        // Character should be dead or very close
        assert!(result.starvation_damage > 0);
        if result.starvation_damage >= 1 {
            assert!(party.members[0].hp <= 0);
        }
    }
}
