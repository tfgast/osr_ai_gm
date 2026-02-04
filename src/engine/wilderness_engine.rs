use rand::Rng;

use crate::rules::encounter;
use crate::state::wilderness::{Terrain, WildernessState};

/// Result of a day of wilderness travel.
#[derive(Debug)]
pub struct TravelResult {
    pub messages: Vec<String>,
    /// Whether the party got lost.
    pub lost: bool,
    /// Encounter triggered during travel.
    pub encounter: Option<encounter::EncounterEntry>,
    /// Whether foraging was attempted and succeeded.
    pub foraged: Option<bool>,
}

impl TravelResult {
    fn new() -> Self {
        TravelResult {
            messages: Vec::new(),
            lost: false,
            encounter: None,
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
        // When lost, the party moves in a random direction instead
        result.msg("The party wanders aimlessly and makes no progress toward their destination.");
    } else {
        wilderness.lost = false;
        // Travel speed
        let hexes = WildernessState::hexes_per_day(movement_rate, terrain);
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

    // Encounter check: 1-in-6 per day in most terrain, 2-in-6 in some
    let encounter_chance: u32 = match terrain {
        Terrain::City => 1,
        Terrain::Clear | Terrain::Hills | Terrain::Barren => 1,
        Terrain::Forest | Terrain::Desert | Terrain::Mountains => 2,
        Terrain::Swamp | Terrain::Jungle => 2,
        Terrain::Ocean | Terrain::River => 2,
    };
    let encounter_roll: u32 = rng.gen_range(1..=6);
    if encounter_roll <= encounter_chance {
        let table_roll: u32 = rng.gen_range(1..=20);
        let entry = encounter::wilderness_encounter(terrain, table_roll);
        result.msg(format!(
            "Encounter! {} ({} appearing).",
            entry.name, entry.number
        ));
        result.encounter = Some(entry.clone());
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
        format!(
            "Foraging successful! Found enough food for 1d6 person-days. (rolled {} vs {}-in-6)",
            roll, chance
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
        format!(
            "Hunt successful! Killed game sufficient for 1d6 person-days of food. (rolled {})",
            roll
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
        ws.add_hex(HexCell::new(0, 0, Terrain::Clear));
        ws.add_hex(HexCell::new(1, 0, Terrain::Forest));
        ws.add_hex(HexCell::new(2, 0, Terrain::Mountains));
        ws.add_hex(HexCell::new(0, 1, Terrain::Swamp));
        ws.add_hex(HexCell::new(1, 1, Terrain::Desert));
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
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            // Travel through swamp (3-in-6 lost chance)
            ws.move_to(0, 1).unwrap();
            let result = travel_day_with(&mut rng, &mut ws, 1, 1, 120);
            if result.lost {
                lost_count += 1;
            }
        }
        assert!(lost_count > 0, "should get lost sometimes in swamp");
    }

    #[test]
    fn travel_encounter_check() {
        let mut encounters = 0;
        for seed in 0..200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut ws = test_wilderness();
            let result = travel_day_with(&mut rng, &mut ws, 1, 0, 120);
            if result.encounter.is_some() {
                encounters += 1;
            }
        }
        assert!(encounters > 0, "should get encounters sometimes");
        assert!(encounters < 100, "should not get encounters too often");
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
        ws.add_hex(HexCell::new(0, 0, Terrain::Ocean));
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
        ws.add_hex(HexCell::new(0, 0, Terrain::Ocean));
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
            assert!(result.messages.iter().any(|m| m.contains("Cannot travel")));
        }
    }
}
