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

    // Early same-hex check: traveling to the current position is a no-op.
    if dest_x == wilderness.current_x && dest_y == wilderness.current_y {
        result.msg(format!(
            "Already at ({}, {}). No travel needed.",
            dest_x, dest_y
        ));
        return result;
    }

    // Early range check: reject travel to out-of-range destinations before
    // consuming any resources (rations, day counter, etc.)
    let hexes = WildernessState::hexes_per_day(movement_rate, terrain);
    let dx = (dest_x - wilderness.current_x).unsigned_abs();
    let dy = (dest_y - wilderness.current_y).unsigned_abs();
    let distance = dx.max(dy);
    if distance > hexes {
        result.msg(format!(
            "Destination ({}, {}) is {} hexes away — exceeds travel range of {} hexes/day \
             (movement rate {}', {} terrain). No travel attempted.",
            dest_x, dest_y, distance, hexes, movement_rate, terrain.name()
        ));
        return result;
    }

    result.msg(format!(
        "Day {}: Travelling through {} terrain.",
        wilderness.travel_day, terrain.name()
    ));

    // Consume rations and handle starvation
    let overhead = apply_daily_overhead(rng, party);
    result.rations_consumed = overhead.rations_consumed;
    result.starving = overhead.starving;
    result.starvation_damage = overhead.starvation_damage;
    for msg in &overhead.messages {
        result.msg(msg.clone());
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
            if let Err(e) = wilderness.move_to(rx, ry) {
                result.msg(format!("The party tries to wander but cannot: {}", e));
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
        // Travel speed (range already validated at top of function)
        result.msg(format!(
            "Travel speed: {} hexes/day (movement rate {}').",
            hexes, movement_rate
        ));

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
            if let Some(entry) = encounter::wilderness_encounter_simple(current_terrain, table_roll) {
                result.msg(format!(
                    "{} encounter! {} ({} appearing).",
                    period, entry.name, entry.number
                ));
                result.encounters.push(entry);
            }
        }
    }

    wilderness.travel_day += 1;

    result
}

/// Result of daily overhead (ration consumption + starvation).
#[derive(Debug, Default)]
pub struct DayOverhead {
    pub messages: Vec<String>,
    pub rations_consumed: u32,
    pub starving: bool,
    pub starvation_damage: u32,
}

/// Apply daily overhead: consume rations, handle starvation.
/// Called by all full-day actions (travel, forage, hunt, orient).
pub fn apply_daily_overhead<R: Rng>(rng: &mut R, party: &mut Party) -> DayOverhead {
    let mut overhead = DayOverhead::default();

    let party_size = party.members.iter().filter(|c| c.is_alive()).count() as u32;
    if party_size == 0 {
        return overhead;
    }

    if party.rations >= party_size {
        party.rations -= party_size;
        overhead.rations_consumed = party_size;
        overhead.messages.push(format!(
            "Consumed {} rations ({} remaining).",
            party_size, party.rations
        ));
    } else if party.rations > 0 {
        overhead.rations_consumed = party.rations;
        party.rations = 0;
        overhead.starving = true;
        overhead.messages.push(format!(
            "Only {} rations available for {} party members. The party is STARVING!",
            overhead.rations_consumed, party_size
        ));
    } else {
        overhead.rations_consumed = 0;
        overhead.starving = true;
        overhead.messages.push("No rations! The party is STARVING!".to_string());
    }

    if overhead.starving {
        party.days_without_food += 1;
        let days = party.days_without_food;

        let penalty = days.min(4) as i32;
        overhead.messages.push(format!(
            "Starvation day {}: -{} penalty to attack rolls and saving throws.",
            days, penalty
        ));

        if days >= 3 {
            let hp_damage: u32 = rng.gen_range(1..=4);
            overhead.starvation_damage = hp_damage;
            overhead.messages.push(format!(
                "Severe starvation! Each party member takes {} HP damage.",
                hp_damage
            ));
            for member in party.members.iter_mut().filter(|c| c.is_alive()) {
                member.hp = (member.hp - hp_damage as i32).max(0);
            }
            let dead: Vec<String> = party.members.iter()
                .filter(|c| c.hp <= 0)
                .map(|c| c.name.clone())
                .collect();
            if !dead.is_empty() {
                overhead.messages.push(format!("Died from starvation: {}.", dead.join(", ")));
            }
        }
    } else {
        if party.days_without_food > 0 {
            overhead.messages.push("The party is well-fed. Starvation effects end.".to_string());
        }
        party.days_without_food = 0;
    }

    overhead
}

/// Result of a foraging attempt.
#[derive(Debug)]
pub struct ForageResult {
    pub message: String,
    /// Person-days of food found (0 if unsuccessful).
    pub quantity: u32,
    pub success: bool,
    /// Daily overhead info (rations consumed, starvation).
    pub overhead: DayOverhead,
}

/// Attempt to forage for food in the current hex.
/// Takes a full day (no travel). Chance varies by terrain.
/// Consumes rations and advances day counter.
pub fn forage(wilderness: &mut WildernessState, party: &mut Party) -> ForageResult {
    forage_with(&mut rand::thread_rng(), wilderness, party)
}

/// Testable version.
pub fn forage_with<R: Rng>(rng: &mut R, wilderness: &mut WildernessState, party: &mut Party) -> ForageResult {
    let terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);

    if !terrain.can_forage() {
        return ForageResult {
            message: format!("Cannot forage in {} terrain.", terrain.name()),
            quantity: 0,
            success: false,
            overhead: DayOverhead::default(),
        };
    }

    // Forage first (food found can offset starvation)
    let chance = terrain.forage_chance();
    let roll: u32 = rng.gen_range(1..=6);

    let (quantity, success, forage_msg) = if roll <= chance {
        let quantity: u32 = rng.gen_range(1..=6);
        party.rations += quantity;
        let msg = format!(
            "Day {}: Foraging successful! Found {} person-days of food. (rolled {} vs {}-in-6, quantity: 1d6={})\nRations: {} (+{})",
            wilderness.travel_day, quantity, roll, chance, quantity, party.rations, quantity
        );
        (quantity, true, msg)
    } else {
        let msg = format!(
            "Day {}: Foraging unsuccessful. No food found. (rolled {} vs {}-in-6)",
            wilderness.travel_day, roll, chance
        );
        (0, false, msg)
    };

    // Consume daily rations + handle starvation
    let overhead = apply_daily_overhead(rng, party);

    wilderness.travel_day += 1;

    ForageResult {
        message: forage_msg,
        quantity,
        success,
        overhead,
    }
}

/// Result of a hunting attempt.
#[derive(Debug)]
pub struct HuntResult {
    pub message: String,
    /// Person-days of food obtained (0 if unsuccessful).
    pub quantity: u32,
    pub success: bool,
    /// Daily overhead info (rations consumed, starvation).
    pub overhead: DayOverhead,
}

/// Attempt to hunt. Similar to foraging but can be done in more terrain.
/// Takes a full day. 1-in-6 base chance.
/// Consumes rations and advances day counter.
pub fn hunt(wilderness: &mut WildernessState, party: &mut Party) -> HuntResult {
    hunt_with(&mut rand::thread_rng(), wilderness, party)
}

/// Testable version.
pub fn hunt_with<R: Rng>(rng: &mut R, wilderness: &mut WildernessState, party: &mut Party) -> HuntResult {
    let terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);

    if terrain == Terrain::Ocean {
        return HuntResult {
            message: "Cannot hunt on the open ocean.".to_string(),
            quantity: 0,
            success: false,
            overhead: DayOverhead::default(),
        };
    }

    // Hunt first (food found can offset starvation)
    let roll: u32 = rng.gen_range(1..=6);
    let (quantity, success, hunt_msg) = if roll == 1 {
        let quantity: u32 = rng.gen_range(1..=6);
        party.rations += quantity;
        let msg = format!(
            "Day {}: Hunt successful! Killed game sufficient for {} person-days of food. (rolled {}, quantity: 1d6={})\nRations: {} (+{})",
            wilderness.travel_day, quantity, roll, quantity, party.rations, quantity
        );
        (quantity, true, msg)
    } else {
        let msg = format!(
            "Day {}: Hunt unsuccessful. No game found. (rolled {})",
            wilderness.travel_day, roll
        );
        (0, false, msg)
    };

    // Consume daily rations + handle starvation
    let overhead = apply_daily_overhead(rng, party);

    wilderness.travel_day += 1;

    HuntResult {
        message: hunt_msg,
        quantity,
        success,
        overhead,
    }
}

/// Result of an orient attempt.
#[derive(Debug)]
pub struct OrientResult {
    pub message: String,
    /// Whether the attempt succeeded.
    pub success: bool,
    /// The terrain where orientation was attempted.
    pub terrain: Terrain,
    /// Daily overhead info (rations consumed, starvation).
    pub overhead: DayOverhead,
}

/// Attempt to orient and find bearings when lost.
/// Takes a full day. Success chance varies by terrain.
/// Consumes rations and advances day counter.
pub fn orient(wilderness: &mut WildernessState, party: &mut Party) -> OrientResult {
    orient_with(&mut rand::thread_rng(), wilderness, party)
}

/// Testable version.
pub fn orient_with<R: Rng>(rng: &mut R, wilderness: &mut WildernessState, party: &mut Party) -> OrientResult {
    let terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);

    if !wilderness.lost {
        return OrientResult {
            message: "The party is not lost.".to_string(),
            success: false,
            terrain,
            overhead: DayOverhead::default(),
        };
    }

    // Success chance: 6 minus terrain's lost_chance gives X-in-6.
    // Clear/City/Hills/Mountains have 1-in-6 lost chance -> 5-in-6 orient success
    // Forest/Swamp/Jungle/etc have 2-in-6 lost chance -> 4-in-6 orient success
    let orient_chance = 6 - terrain.lost_chance();
    let roll: u32 = rng.gen_range(1..=6);

    // Consume daily rations + handle starvation
    let overhead = apply_daily_overhead(rng, party);

    wilderness.travel_day += 1;

    if roll <= orient_chance {
        wilderness.lost = false;
        wilderness.log(format!(
            "Day {}: Oriented successfully in {} terrain.",
            wilderness.travel_day - 1, terrain.name()
        ));
        OrientResult {
            message: format!(
                "Day {}: Spent the day finding bearings in {} terrain.\n\
                 Orientation successful! (rolled {} vs {}-in-6)\n\
                 The party is no longer lost.",
                wilderness.travel_day - 1, terrain.name(), roll, orient_chance
            ),
            success: true,
            terrain,
            overhead,
        }
    } else {
        wilderness.log(format!(
            "Day {}: Failed to orient in {} terrain.",
            wilderness.travel_day - 1, terrain.name()
        ));
        OrientResult {
            message: format!(
                "Day {}: Spent the day trying to find bearings in {} terrain.\n\
                 Orientation failed. (rolled {} vs {}-in-6)\n\
                 The party remains lost.",
                wilderness.travel_day - 1, terrain.name(), roll, orient_chance
            ),
            success: false,
            terrain,
            overhead,
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
            let result = forage_with(&mut rng, &mut ws, &mut party);
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
        let result = forage_with(&mut test_rng(), &mut ws, &mut party);
        assert!(result.message.contains("Cannot forage"));
    }

    #[test]
    fn hunt_basic() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        let result = hunt_with(&mut rng, &mut ws, &mut party);
        assert!(result.message.contains("Hunt") || result.message.contains("hunt"));
    }

    #[test]
    fn hunt_ocean_fails() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Ocean)).unwrap();
        let mut party = test_party();
        let result = hunt_with(&mut test_rng(), &mut ws, &mut party);
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
        // Out-of-range destinations are rejected early, before lost check
        assert!(result.messages.iter().any(|m|
            m.contains("exceeds travel range") || m.contains("Cannot travel")
        ));
        assert!(!result.lost, "out-of-range travel should not trigger lost check");
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

        // Trying to travel 5 hexes away should be rejected with no side effects
        let mut rng = test_rng();
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 5, 0, 120);
        assert!(
            result.messages.iter().any(|m| m.contains("exceeds travel range")),
            "should not allow travel beyond daily movement range"
        );
        assert!(!result.lost, "out-of-range travel should not trigger lost check");
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
            let result = forage_with(&mut rng, &mut ws, &mut party);
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
            let mut ws = test_wilderness(); // Clear terrain, hunting allowed
            let mut party = test_party();
            let result = hunt_with(&mut rng, &mut ws, &mut party);
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
        let result = forage_with(&mut test_rng(), &mut ws, &mut party);
        assert!(result.message.contains("Cannot forage"), "should not be able to forage in barren terrain");
    }

    #[test]
    fn forage_city_terrain_fails() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::City)).unwrap();
        let mut party = test_party();
        let result = forage_with(&mut test_rng(), &mut ws, &mut party);
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
            party.rations = 10;
            ws.move_to(1, 0).unwrap(); // Forest (2-in-6 forage chance)

            let result = forage_with(&mut rng, &mut ws, &mut party);
            if result.success {
                assert!(result.quantity > 0);
                // Foraged food added, then daily ration consumed (1 per party member)
                assert_eq!(party.rations, 10 + result.quantity - result.overhead.rations_consumed);
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
            let mut ws = test_wilderness();
            let mut party = test_party();
            party.rations = 10;

            let result = hunt_with(&mut rng, &mut ws, &mut party);
            if result.success {
                assert!(result.quantity > 0);
                // Hunted food added, then daily ration consumed (1 per party member)
                assert_eq!(party.rations, 10 + result.quantity - result.overhead.rations_consumed);
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

    // =========================================================================
    // Orient (navigation out of being lost) tests
    // =========================================================================

    #[test]
    fn orient_when_not_lost_fails() {
        let mut ws = test_wilderness();
        let mut party = test_party();
        ws.lost = false;
        let result = orient_with(&mut test_rng(), &mut ws, &mut party);
        assert!(!result.success);
        assert!(result.message.contains("not lost"));
    }

    #[test]
    fn orient_when_lost_can_succeed() {
        let mut success_count = 0;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            ws.lost = true;
            let result = orient_with(&mut rng, &mut ws, &mut party);
            if result.success {
                success_count += 1;
                assert!(!ws.lost, "lost flag should be cleared on success");
                assert!(result.message.contains("no longer lost"));
            }
        }
        // Clear terrain: 5-in-6 success rate (~83%)
        assert!(success_count > 50, "should succeed often in clear terrain");
    }

    #[test]
    fn orient_advances_travel_day() {
        let mut ws = test_wilderness();
        let mut party = test_party();
        ws.lost = true;
        let start_day = ws.travel_day;
        orient_with(&mut test_rng(), &mut ws, &mut party);
        assert_eq!(ws.travel_day, start_day + 1, "orient should advance the day");
    }

    #[test]
    fn orient_harder_in_difficult_terrain() {
        // Compare success rates in clear vs swamp terrain
        let mut clear_success = 0;
        let mut swamp_success = 0;
        let trials = 500;

        for seed in 0..trials {
            // Clear terrain
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            ws.lost = true;
            let result = orient_with(&mut rng, &mut ws, &mut party);
            if result.success {
                clear_success += 1;
            }

            // Swamp terrain
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            ws.move_to(0, 1).unwrap(); // Swamp
            ws.lost = true;
            let result = orient_with(&mut rng, &mut ws, &mut party);
            if result.success {
                swamp_success += 1;
            }
        }

        assert!(
            clear_success > swamp_success,
            "should be easier to orient in clear ({}) than swamp ({})",
            clear_success, swamp_success
        );
    }

    #[test]
    fn orient_failure_keeps_lost_status() {
        // Find a seed that causes failure
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let mut party = test_party();
            ws.lost = true;
            let result = orient_with(&mut rng, &mut ws, &mut party);
            if !result.success {
                assert!(ws.lost, "lost flag should remain on failure");
                assert!(result.message.contains("remains lost"));
                return;
            }
        }
        panic!("should find at least one failure case");
    }

    #[test]
    fn orient_logs_attempt() {
        let mut ws = test_wilderness();
        let mut party = test_party();
        ws.lost = true;
        orient_with(&mut test_rng(), &mut ws, &mut party);
        assert!(!ws.log.is_empty(), "orient should add to the log");
    }

    // =========================================================================
    // Bug fix: out-of-range travel must not consume rations or advance day
    // =========================================================================

    #[test]
    fn out_of_range_travel_no_rations_consumed() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 10;
        let initial_rations = party.rations;

        // (5,5) is 5 hexes away from (0,0), well beyond 4 hexes/day in clear
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 5, 5, 120);

        assert!(
            result.messages.iter().any(|m| m.contains("exceeds travel range")),
            "should report destination out of range"
        );
        assert_eq!(party.rations, initial_rations, "rations must not be consumed for out-of-range travel");
        assert_eq!(result.rations_consumed, 0, "no rations should be consumed");
        assert!(!result.starving, "should not trigger starvation");
    }

    #[test]
    fn out_of_range_travel_no_day_advance() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        let initial_day = ws.travel_day;

        // 5 hexes away, max 4/day in clear terrain
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 5, 5, 120);

        assert!(
            result.messages.iter().any(|m| m.contains("exceeds travel range")),
            "should report destination out of range"
        );
        assert_eq!(ws.travel_day, initial_day, "day counter must not advance for out-of-range travel");
    }

    #[test]
    fn out_of_range_travel_no_position_change() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();

        let result = travel_day_with(&mut rng, &mut ws, &mut party, 5, 5, 120);

        assert!(
            result.messages.iter().any(|m| m.contains("exceeds travel range")),
            "should report destination out of range"
        );
        assert_eq!((ws.current_x, ws.current_y), (0, 0), "position must not change");
    }

    #[test]
    fn out_of_range_travel_no_encounters() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();

        let result = travel_day_with(&mut rng, &mut ws, &mut party, 5, 5, 120);

        assert!(result.encounters.is_empty(), "no encounters should occur for out-of-range travel");
    }

    #[test]
    fn out_of_range_in_mountains_no_side_effects() {
        // Mountains: 1 hex/day at 120'. Destination 2 hexes away should be rejected.
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 10;
        // Move step-by-step to mountains at (2,0): (0,0) -> (1,0) -> (2,0)
        ws.move_to(1, 0).unwrap();
        ws.move_to(2, 0).unwrap();
        let initial_rations = party.rations;
        let initial_day = ws.travel_day;

        // From (2,0) mountains, try to reach (0,0) which is 2 hexes away
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 0, 0, 120);

        assert!(
            result.messages.iter().any(|m| m.contains("exceeds travel range")),
            "2-hex travel in mountains should exceed 1 hex/day range"
        );
        assert_eq!(party.rations, initial_rations, "rations must not be consumed");
        assert_eq!(ws.travel_day, initial_day, "day must not advance");
        assert_eq!((ws.current_x, ws.current_y), (2, 0), "position must not change");
    }

    // =========================================================================
    // Bug fix: travel to current hex is a no-op (oag-vsjm7)
    // =========================================================================

    #[test]
    fn travel_to_current_hex_is_noop() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 10;
        let initial_rations = party.rations;
        let initial_day = ws.travel_day;

        // Travel to (0,0) — the starting position
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 0, 0, 120);

        assert!(
            result.messages.iter().any(|m| m.contains("Already at")),
            "should report already at position: {:?}", result.messages
        );
        assert_eq!(party.rations, initial_rations, "rations must not be consumed");
        assert_eq!(ws.travel_day, initial_day, "day must not advance");
        assert_eq!((ws.current_x, ws.current_y), (0, 0), "position must not change");
        assert!(result.encounters.is_empty(), "no encounters should occur");
        assert!(!result.starving, "should not trigger starvation");
    }

    #[test]
    fn travel_to_current_hex_after_move_is_noop() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 10;

        // First move to (1,0) normally
        let _ = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);
        let rations_after_move = party.rations;
        let day_after_move = ws.travel_day;

        // Now travel to (1,0) again — the current position
        let result = travel_day_with(&mut rng, &mut ws, &mut party, 1, 0, 120);

        assert!(
            result.messages.iter().any(|m| m.contains("Already at")),
            "should report already at position: {:?}", result.messages
        );
        assert_eq!(party.rations, rations_after_move, "rations must not be consumed on same-hex travel");
        assert_eq!(ws.travel_day, day_after_move, "day must not advance on same-hex travel");
    }

    // =========================================================================
    // Bug fix: forage/hunt/orient must consume rations and advance day (oag-iasbo)
    // =========================================================================

    #[test]
    fn forage_consumes_rations_and_advances_day() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        ws.move_to(1, 0).unwrap(); // Forest (can forage)
        let mut party = test_party();
        party.rations = 10;
        let initial_day = ws.travel_day;

        let result = forage_with(&mut rng, &mut ws, &mut party);
        assert_eq!(ws.travel_day, initial_day + 1, "forage should advance the day");
        assert!(result.overhead.rations_consumed > 0, "forage should consume rations");
    }

    #[test]
    fn hunt_consumes_rations_and_advances_day() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 10;
        let initial_day = ws.travel_day;

        let result = hunt_with(&mut rng, &mut ws, &mut party);
        assert_eq!(ws.travel_day, initial_day + 1, "hunt should advance the day");
        assert!(result.overhead.rations_consumed > 0, "hunt should consume rations");
    }

    #[test]
    fn orient_consumes_rations() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let mut party = test_party();
        party.rations = 10;
        ws.lost = true;

        let result = orient_with(&mut rng, &mut ws, &mut party);
        assert!(result.overhead.rations_consumed > 0, "orient should consume rations");
    }

    #[test]
    fn forage_starvation_during_forage() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        ws.move_to(1, 0).unwrap(); // Forest
        let mut party = test_party();
        party.rations = 0;
        party.days_without_food = 2; // Next day will be day 3 (HP damage)
        party.members[0].hp = 10;

        let result = forage_with(&mut rng, &mut ws, &mut party);
        assert!(result.overhead.starving, "should be starving with no rations");
        assert!(result.overhead.starvation_damage > 0, "day 3+ starvation should deal HP damage");
    }
}
