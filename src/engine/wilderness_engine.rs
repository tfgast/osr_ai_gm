use rand::Rng;

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
}

impl TravelResult {
    fn new() -> Self {
        TravelResult {
            messages: Vec::new(),
            lost: false,
            encounters: Vec::new(),
            foraged: None,
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
/// Checks for getting lost, encounter, and advances position.
pub fn travel_day(
    wilderness: &mut WildernessState,
    dest_x: i32,
    dest_y: i32,
    movement_rate: u32,
) -> TravelResult {
    travel_day_with(&mut rand::thread_rng(), wilderness, dest_x, dest_y, movement_rate)
}

/// Testable version.
pub fn travel_day_with<R: Rng>(
    rng: &mut R,
    wilderness: &mut WildernessState,
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

/// Attempt to forage for food in the current hex.
/// Takes a full day (no travel). Chance varies by terrain.
pub fn forage(wilderness: &WildernessState) -> String {
    forage_with(&mut rand::thread_rng(), wilderness)
}

/// Testable version.
pub fn forage_with<R: Rng>(rng: &mut R, wilderness: &WildernessState) -> String {
    let terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);

    if !terrain.can_forage() {
        return format!("Cannot forage in {} terrain.", terrain.name());
    }

    let chance = terrain.forage_chance();
    let roll: u32 = rng.gen_range(1..=6);

    if roll <= chance {
        let quantity: u32 = rng.gen_range(1..=6);
        format!(
            "Foraging successful! Found enough food for {} person-days. (rolled {} vs {}-in-6, quantity: 1d6={})",
            quantity, roll, chance, quantity
        )
    } else {
        format!(
            "Foraging unsuccessful. No food found. (rolled {} vs {}-in-6)",
            roll, chance
        )
    }
}

/// Attempt to hunt. Similar to foraging but can be done in more terrain.
/// Takes a full day. 1-in-6 base chance.
pub fn hunt(wilderness: &WildernessState) -> String {
    hunt_with(&mut rand::thread_rng(), wilderness)
}

/// Testable version.
pub fn hunt_with<R: Rng>(rng: &mut R, wilderness: &WildernessState) -> String {
    let terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);

    if terrain == Terrain::Ocean {
        return "Cannot hunt on the open ocean.".to_string();
    }

    let roll: u32 = rng.gen_range(1..=6);
    if roll == 1 {
        let quantity: u32 = rng.gen_range(1..=6);
        format!(
            "Hunt successful! Killed game sufficient for {} person-days of food. (rolled {}, quantity: 1d6={})",
            quantity, roll, quantity
        )
    } else {
        format!("Hunt unsuccessful. No game found. (rolled {})", roll)
    }
}

/// Display wilderness travel status.
pub fn wilderness_status(wilderness: &WildernessState, movement_rate: u32) -> String {
    let mut out = wilderness.status();
    let terrain = wilderness.current_hex()
        .map(|h| h.terrain)
        .unwrap_or(Terrain::Clear);
    let hexes = WildernessState::hexes_per_day(movement_rate, terrain);
    out.push_str(&format!(
        "\nTravel speed: {} hexes/day (movement {}', terrain: {})",
        hexes, movement_rate, terrain.name()
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn travel_day_basic() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let result = travel_day_with(&mut rng, &mut ws, 1, 0, 120);
        assert!(!result.messages.is_empty());
        assert!(result.messages.iter().any(|m| m.contains("Day 1")));
    }

    #[test]
    fn travel_increments_day() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        travel_day_with(&mut rng, &mut ws, 1, 0, 120);
        assert_eq!(ws.travel_day, 2);
    }

    #[test]
    fn travel_can_get_lost() {
        let mut lost_count = 0;
        let mut moved_randomly = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            // Travel through swamp (2-in-6 lost chance)
            ws.move_to(0, 1).unwrap();
            let start_x = ws.current_x;
            let start_y = ws.current_y;
            let result = travel_day_with(&mut rng, &mut ws, 1, 1, 120);
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
            let result = travel_day_with(&mut rng, &mut ws, 1, 0, 120);
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
            // Forest has 2-in-6 encounter chance per check
            ws.move_to(1, 0).unwrap();
            let result = travel_day_with(&mut rng, &mut ws, 0, 0, 120);
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
            ws.move_to(1, 0).unwrap(); // Forest
            let result = forage_with(&mut rng, &ws);
            if result.contains("successful!") {
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
        let result = forage_with(&mut test_rng(), &ws);
        assert!(result.contains("Cannot forage"));
    }

    #[test]
    fn hunt_basic() {
        let mut rng = test_rng();
        let ws = test_wilderness();
        let result = hunt_with(&mut rng, &ws);
        assert!(result.contains("Hunt") || result.contains("hunt"));
    }

    #[test]
    fn hunt_ocean_fails() {
        let mut ws = WildernessState::new();
        ws.add_hex(HexCell::new(0, 0, Terrain::Ocean)).unwrap();
        let result = hunt_with(&mut test_rng(), &ws);
        assert!(result.contains("Cannot hunt"));
    }

    #[test]
    fn wilderness_status_display() {
        let ws = test_wilderness();
        let status = wilderness_status(&ws, 120);
        assert!(status.contains("Clear"));
        assert!(status.contains("hexes/day"));
    }

    #[test]
    fn travel_to_invalid_hex_reports_error() {
        let mut rng = test_rng();
        let mut ws = test_wilderness();
        let result = travel_day_with(&mut rng, &mut ws, 99, 99, 120);
        // Should get a message about not being able to travel there
        // (unless lost, in which case travel doesn't attempt the move)
        if !result.lost {
            assert!(result.messages.iter().any(|m|
                m.contains("exceeds travel range") || m.contains("Cannot travel")
            ));
        }
    }
}
