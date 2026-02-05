use rand::Rng;

use crate::model::Character;
use crate::rules::ability;
use crate::rules::encounter;
use crate::state::dungeon::{DoorState, DungeonState};
use crate::state::time::TimeTracker;

/// Check for wandering monsters (1-in-6 every 2 turns).
/// Should be called after any action that consumes a dungeon turn.
fn check_wandering_monster<R: Rng>(
    rng: &mut R,
    time: &TimeTracker,
    dungeon_level: u32,
) -> Option<encounter::EncounterEntry> {
    if time.total_turns % 2 == 0 {
        let roll: u32 = rng.gen_range(1..=6);
        if roll == 1 {
            let table_roll: u32 = rng.gen_range(1..=40);
            return encounter::dungeon_encounter_d40(dungeon_level, table_roll);
        }
    }
    None
}

/// Result of a dungeon exploration action.
#[derive(Debug)]
pub struct ExplorationResult {
    pub messages: Vec<String>,
    /// Whether a wandering monster encounter was triggered.
    pub encounter: Option<encounter::EncounterEntry>,
}

impl ExplorationResult {
    fn new() -> Self {
        ExplorationResult {
            messages: Vec::new(),
            encounter: None,
        }
    }

    fn msg(&mut self, s: impl Into<String>) {
        self.messages.push(s.into());
    }
}

impl std::fmt::Display for ExplorationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for msg in &self.messages {
            writeln!(f, "{}", msg)?;
        }
        if let Some(enc) = &self.encounter {
            writeln!(f, "*** WANDERING MONSTER: {} ({}) ***", enc.name, enc.number)?;
        }
        Ok(())
    }
}

/// Advance one dungeon turn of exploration movement.
/// Handles time tracking, light sources, and wandering monster checks.
/// Wandering monsters are checked every 2 turns (1-in-6).
pub fn advance_dungeon_turn(
    time: &mut TimeTracker,
    dungeon: &mut DungeonState,
    dungeon_level: u32,
) -> ExplorationResult {
    advance_dungeon_turn_with(&mut rand::thread_rng(), time, dungeon, dungeon_level)
}

/// Testable version with explicit RNG.
pub fn advance_dungeon_turn_with<R: Rng>(
    rng: &mut R,
    time: &mut TimeTracker,
    dungeon: &mut DungeonState,
    dungeon_level: u32,
) -> ExplorationResult {
    let mut result = ExplorationResult::new();

    // Advance time (ticks lights, increments turn counter)
    let light_msgs = time.advance_turn();
    for msg in light_msgs {
        result.msg(msg);
    }

    // Darkness — block exploration movement
    if !time.has_light() {
        result.msg("The party is in DARKNESS! Movement is impossible without light.");
        result.msg("Light a torch or lantern before exploring.");
        return result;
    }

    // Rest requirement — per OSE, -1 to attack and damage when rest is overdue
    if time.needs_rest() {
        result.msg("The party must rest this turn (1 turn per 5 turns of activity). Penalty: -1 to attack and damage rolls.");
    }

    // Mark current room as explored
    dungeon.explore_current();

    let current_room_name = dungeon.current_room
        .and_then(|id| dungeon.find_room(id))
        .map(|r| r.name.as_str())
        .unwrap_or("unknown area");
    result.msg(format!("Turn {}: Exploring {}.",
        time.total_turns, current_room_name
    ));

    // Always show light status so the GM can track torch burn
    if let Some(summary) = time.light_summary() {
        result.msg(summary);
    }

    // Wandering monster check every 2 turns (1-in-6)
    if let Some(entry) = check_wandering_monster(rng, time, dungeon_level) {
        result.msg(format!(
            "Wandering monster! {} ({} appearing)",
            entry.name, entry.number
        ));
        result.encounter = Some(entry);
    }

    result
}

/// Search the current room for secret doors and hidden features.
/// Base chance: 1-in-6 (elves get 2-in-6).
/// Takes one turn. Also checks for wandering monsters.
pub fn search_room(time: &mut TimeTracker, dungeon: &mut DungeonState, dungeon_level: u32, is_elf: bool) -> ExplorationResult {
    search_room_with(&mut rand::thread_rng(), time, dungeon, dungeon_level, is_elf)
}

/// Testable version.
pub fn search_room_with<R: Rng>(
    rng: &mut R,
    time: &mut TimeTracker,
    dungeon: &mut DungeonState,
    dungeon_level: u32,
    is_elf: bool,
) -> ExplorationResult {
    let mut result = ExplorationResult::new();
    let light_msgs = time.advance_turn();
    for msg in light_msgs {
        result.msg(msg);
    }

    let threshold = if is_elf { 2 } else { 1 };
    let roll: u32 = rng.gen_range(1..=6);
    let success = roll <= threshold;

    if let Some(current) = dungeon.current_room {
        if let Some(room) = dungeon.find_room_mut(current) {
            room.searched = true;
        }
    }

    if success {
        // Check for secret doors in this room
        let current = dungeon.current_room.unwrap_or(0);
        let mut found_something = false;
        for door in &mut dungeon.doors {
            if door.state == DoorState::Secret && !door.discovered
                && (door.room_a == current || door.room_b == current)
            {
                door.discovered = true;
                found_something = true;
                result.msg(format!(
                    "Secret door found! Door {} leads to room {}.",
                    door.id,
                    if door.room_a == current { door.room_b } else { door.room_a }
                ));
            }
        }
        if !found_something {
            result.msg("Search successful — but nothing hidden found here.");
        }
    } else {
        result.msg("Search reveals nothing.");
    }

    // Always show light status so the GM can track torch burn
    if let Some(summary) = time.light_summary() {
        result.msg(summary);
    }

    // Wandering monster check (search consumes a turn)
    if let Some(entry) = check_wandering_monster(rng, time, dungeon_level) {
        result.msg(format!(
            "Wandering monster! {} ({} appearing)",
            entry.name, entry.number
        ));
        result.encounter = Some(entry);
    }

    result
}

/// Listen at a door. Base chance 1-in-6 to hear noises behind it.
/// Per OSE, listening takes one dungeon turn.
pub fn listen_at_door(
    time: &mut TimeTracker,
    dungeon: &DungeonState,
    dungeon_level: u32,
    is_elf_or_halfling: bool,
) -> ExplorationResult {
    listen_at_door_with(&mut rand::thread_rng(), time, dungeon, dungeon_level, is_elf_or_halfling)
}

/// Testable version.
pub fn listen_at_door_with<R: Rng>(
    rng: &mut R,
    time: &mut TimeTracker,
    _dungeon: &DungeonState,
    dungeon_level: u32,
    is_elf_or_halfling: bool,
) -> ExplorationResult {
    let mut result = ExplorationResult::new();

    // Listening consumes one turn
    let light_msgs = time.advance_turn();
    for msg in light_msgs {
        result.msg(msg);
    }

    let threshold = if is_elf_or_halfling { 2 } else { 1 };
    let roll: u32 = rng.gen_range(1..=6);
    if roll <= threshold {
        result.msg("You hear sounds beyond the door!");
    } else {
        result.msg("You hear nothing.");
    }

    // Always show light status so the GM can track torch burn
    if let Some(summary) = time.light_summary() {
        result.msg(summary);
    }

    // Wandering monster check (listening consumes a turn)
    if let Some(entry) = check_wandering_monster(rng, time, dungeon_level) {
        result.msg(format!(
            "Wandering monster! {} ({} appearing)",
            entry.name, entry.number
        ));
        result.encounter = Some(entry);
    }

    result
}

/// Force open a door. Base chance 2-in-6, modified by STR.
/// OSE: Characters can force doors on a roll of 1-2 on d6.
pub fn force_door(
    dungeon: &mut DungeonState,
    door_id: u32,
    character: &Character,
) -> String {
    force_door_with(&mut rand::thread_rng(), dungeon, door_id, character)
}

/// Testable version.
pub fn force_door_with<R: Rng>(
    rng: &mut R,
    dungeon: &mut DungeonState,
    door_id: u32,
    character: &Character,
) -> String {
    let door = match dungeon.find_door_mut(door_id) {
        Some(d) => d,
        None => return format!("Door {} not found.", door_id),
    };

    if door.state == DoorState::Open {
        return "Door is already open.".to_string();
    }
    if door.state == DoorState::Locked {
        return "Door is locked — requires a key or lockpicking.".to_string();
    }
    if door.state == DoorState::Secret && !door.discovered {
        return "You see no door here.".to_string();
    }

    // Use STR open doors chance (X-in-6) per OSE rules
    let threshold = ability::str_open_doors(character.abilities.strength);
    let roll: u32 = rng.gen_range(1..=6);

    if roll <= threshold {
        // Access door again mutably (safe: existence confirmed above)
        let door = dungeon.find_door_mut(door_id)
            .expect("door verified to exist above");
        door.state = DoorState::Open;
        format!(
            "{} forces door {} open! (rolled {} vs {})",
            character.name, door_id, roll, threshold
        )
    } else {
        format!(
            "{} fails to force door {}. (rolled {} vs {})",
            character.name, door_id, roll, threshold
        )
    }
}

/// Check for trap trigger. Traps trigger on 1-2 on d6.
pub fn check_trap(room_name: &str) -> String {
    check_trap_with(&mut rand::thread_rng(), room_name)
}

/// Testable version.
pub fn check_trap_with<R: Rng>(rng: &mut R, room_name: &str) -> String {
    let roll: u32 = rng.gen_range(1..=6);
    if roll <= 2 {
        format!("TRAP TRIGGERED in {}! (rolled {})", room_name, roll)
    } else {
        format!("No trap triggered in {}. (rolled {})", room_name, roll)
    }
}

/// Move the party to a new room through a door.
/// Consumes a turn. Checks for traps and wandering monsters.
pub fn move_through_door(
    time: &mut TimeTracker,
    dungeon: &mut DungeonState,
    dungeon_level: u32,
    door_id: u32,
) -> Result<ExplorationResult, String> {
    move_through_door_with(&mut rand::thread_rng(), time, dungeon, dungeon_level, door_id)
}

/// Testable version.
pub fn move_through_door_with<R: Rng>(
    rng: &mut R,
    time: &mut TimeTracker,
    dungeon: &mut DungeonState,
    dungeon_level: u32,
    door_id: u32,
) -> Result<ExplorationResult, String> {
    // Block movement in darkness
    if !time.has_light() {
        return Err("Cannot move — the party is in DARKNESS! Light a torch or lantern first.".to_string());
    }

    let door = match dungeon.doors.iter().find(|d| d.id == door_id) {
        Some(d) => d.clone(),
        None => return Err(format!("Door {} not found.", door_id)),
    };

    if !door.is_passable() {
        return Err(format!("Door {} is not open. Force it first.", door_id));
    }

    let current = dungeon.current_room
        .ok_or_else(|| "no current room set".to_string())?;
    let dest = if door.room_a == current {
        door.room_b
    } else if door.room_b == current {
        door.room_a
    } else {
        return Err(format!("Door {} is not connected to the current room.", door_id));
    };

    dungeon.move_to(dest).map_err(|e| e.to_string())?;

    // Per OSE, doors close automatically after passing through (unless spiked)
    if door.state == DoorState::Open {
        if let Some(d) = dungeon.find_door_mut(door_id) {
            d.state = DoorState::Closed;
        }
    }

    let mut result = ExplorationResult::new();

    // Check for trap in new room (only if not already triggered)
    let has_untriggered_trap = dungeon.find_room(dest)
        .map(|r| r.trap.is_some() && !r.trap_triggered)
        .unwrap_or(false);
    let room_name = dungeon.find_room(dest)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| format!("room {}", dest));

    if has_untriggered_trap {
        let trap_msg = check_trap_with(rng, &room_name);
        if trap_msg.contains("TRIGGERED") {
            if let Some(room) = dungeon.find_room_mut(dest) {
                room.trap_triggered = true;
            }
        }
        result.msg(trap_msg);
    }

    result.msg(format!("Moved to {} (room {}).", room_name, dest));

    // Advance time for the move
    let light_msgs = time.advance_turn();
    for msg in light_msgs {
        result.msg(msg);
    }

    // Always show light status so the GM can track torch burn
    if let Some(summary) = time.light_summary() {
        result.msg(summary);
    }

    // Wandering monster check (moving consumes a turn)
    if let Some(entry) = check_wandering_monster(rng, time, dungeon_level) {
        result.msg(format!(
            "Wandering monster! {} ({} appearing)",
            entry.name, entry.number
        ));
        result.encounter = Some(entry);
    }

    Ok(result)
}

/// Display the current exploration status.
pub fn exploration_status(time: &TimeTracker, dungeon: &DungeonState) -> String {
    let mut out = String::new();
    out.push_str(&time.status());
    out.push('\n');
    out.push_str(&dungeon.status());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AbilityScores;
    use crate::rules::class::Class;
    use crate::state::dungeon::{Door, Room};
    use crate::state::time::LightSourceKind;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn test_dungeon() -> DungeonState {
        let mut ds = DungeonState::new(1);
        ds.add_room(Room::new(0, "Entrance Hall")).unwrap();
        ds.add_room(Room::new(1, "Guard Room")).unwrap();
        ds.add_room(Room::new(2, "Hidden Chamber")).unwrap();
        ds.add_room(Room::new(3, "Trap Room").with_trap("Pit trap")).unwrap();
        ds.add_door(Door::new(0, 0, 1, DoorState::Closed).unwrap()).unwrap();
        ds.add_door(Door::new(1, 1, 2, DoorState::Secret).unwrap()).unwrap();
        ds.add_door(Door::new(2, 1, 3, DoorState::Open).unwrap()).unwrap();
        ds
    }

    fn test_character() -> Character {
        let mut c = Character::new("Arden", Class::Fighter);
        c.abilities = AbilityScores {
            strength: 14,
            intelligence: 10,
            wisdom: 10,
            dexterity: 12,
            constitution: 13,
            charisma: 10,
        };
        c
    }

    #[test]
    fn advance_turn_tracks_time() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Torch, "Arden");

        let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        assert_eq!(time.total_turns, 1);
        assert!(result.messages.iter().any(|m| m.contains("Turn 1")));
    }

    #[test]
    fn advance_turn_marks_explored() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Torch, "Arden");

        advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        assert!(dungeon.explored.contains(&0));
    }

    #[test]
    fn darkness_blocks_exploration() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();

        let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        assert!(result.messages.iter().any(|m| m.contains("DARKNESS")));
        // Should not have explored any rooms (blocked by darkness)
        assert!(!dungeon.explored.contains(&0), "should not explore in darkness");
    }

    #[test]
    fn darkness_blocks_door_movement() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        // Open door 0 so we can try to move
        dungeon.find_door_mut(0).unwrap().state = DoorState::Open;

        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("DARKNESS"));
    }

    #[test]
    fn wandering_monster_check_every_2_turns() {
        // Run many turns to verify wandering monsters can occur
        let mut encounters = 0;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut time = TimeTracker::new();
            let mut dungeon = test_dungeon();
            time.light(LightSourceKind::Lantern, "Test");

            // Turn 1 — no check
            advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
            // Turn 2 — check happens
            let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
            if result.encounter.is_some() {
                encounters += 1;
            }
        }
        // With 1-in-6 chance, expect roughly 16-17 out of 100
        assert!(encounters > 0, "should get at least some encounters");
        assert!(encounters < 50, "should not get encounters most of the time");
    }

    #[test]
    fn search_room_basic() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Torch, "Arden");

        let result = search_room_with(&mut rng, &mut time, &mut dungeon, 1, false);
        assert!(!result.messages.is_empty());
    }

    #[test]
    fn search_finds_secret_door() {
        let mut found = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut time = TimeTracker::new();
            let mut dungeon = test_dungeon();
            time.light(LightSourceKind::Lantern, "Test");
            // Open door 0 so we can move to room 1
            dungeon.find_door_mut(0).unwrap().state = DoorState::Open;
            dungeon.move_to(1).unwrap(); // Guard Room has secret door

            let result = search_room_with(&mut rng, &mut time, &mut dungeon, 1, false);
            let output = format!("{}", result);
            if output.contains("Secret door found") {
                found = true;
                break;
            }
        }
        assert!(found, "should eventually find a secret door");
    }

    #[test]
    fn listen_at_door_basic() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let dungeon = test_dungeon();
        time.light(LightSourceKind::Torch, "Arden");
        let result = listen_at_door_with(&mut rng, &mut time, &dungeon, 1, false);
        assert!(result.messages.iter().any(|m| m.contains("hear") || m.contains("nothing")));
    }

    #[test]
    fn listen_at_door_consumes_turn() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let dungeon = test_dungeon();
        time.light(LightSourceKind::Torch, "Arden");
        let turns_before = time.total_turns;
        listen_at_door_with(&mut rng, &mut time, &dungeon, 1, false);
        assert_eq!(time.total_turns, turns_before + 1, "listening should consume one turn");
    }

    #[test]
    fn force_door_success() {
        let mut found_success = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut dungeon = test_dungeon();
            let character = test_character();

            let result = force_door_with(&mut rng, &mut dungeon, 0, &character);
            if result.contains("forces door 0 open") {
                found_success = true;
                break;
            }
        }
        assert!(found_success, "should eventually force a door open");
    }

    #[test]
    fn force_locked_door_refused() {
        let mut rng = test_rng();
        let mut dungeon = test_dungeon();
        // Add a locked door
        dungeon.add_door(Door::new(3, 0, 3, DoorState::Locked).unwrap()).unwrap();
        let character = test_character();

        let result = force_door_with(&mut rng, &mut dungeon, 3, &character);
        assert!(result.contains("locked"));
    }

    #[test]
    fn force_open_door_no_op() {
        let mut rng = test_rng();
        let mut dungeon = test_dungeon();
        let character = test_character();

        let result = force_door_with(&mut rng, &mut dungeon, 2, &character);
        assert!(result.contains("already open"));
    }

    #[test]
    fn check_trap_can_trigger() {
        let mut triggered = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let result = check_trap_with(&mut rng, "Test Room");
            if result.contains("TRIGGERED") {
                triggered = true;
                break;
            }
        }
        assert!(triggered, "trap should eventually trigger");
    }

    #[test]
    fn move_through_open_door() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Test");
        // Open door 0 so we can move to room 1
        dungeon.find_door_mut(0).unwrap().state = DoorState::Open;
        dungeon.move_to(1).unwrap(); // Move to guard room

        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 2);
        assert!(result.is_ok());
        // Should have moved to room 3 (Trap Room)
        assert_eq!(dungeon.current_room, Some(3));
    }

    #[test]
    fn move_through_closed_door_fails() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Test");

        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not open"));
    }

    #[test]
    fn exploration_status_display() {
        let time = TimeTracker::new();
        let dungeon = test_dungeon();
        let status = exploration_status(&time, &dungeon);
        assert!(status.contains("Turn:"));
        assert!(status.contains("Level: 1"));
    }

    // =========================================================================
    // Additional QA tests for exploration flows
    // =========================================================================

    #[test]
    fn torch_expires_during_exploration() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Torch, "Arden");

        // Torch lasts 6 turns; after 6 advance_turn calls, light should be gone
        for _ in 0..6 {
            advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        }
        assert!(!time.has_light(), "torch should be expired after 6 turns");
        // Next turn should be blocked by darkness
        let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        assert!(result.messages.iter().any(|m| m.contains("DARKNESS")));
    }

    #[test]
    fn lantern_lasts_longer_than_torch() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Brin");

        // After 6 turns (torch would be dead), lantern still active
        for _ in 0..6 {
            advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        }
        assert!(time.has_light(), "lantern should still be active after 6 turns");

        // After 24 turns total, lantern expires
        for _ in 0..18 {
            advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        }
        assert!(!time.has_light(), "lantern should expire after 24 turns");
    }

    #[test]
    fn stuck_door_treated_as_closed_for_forcing() {
        // Stuck doors behave like closed doors for force_door
        let mut found_success = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut dungeon = test_dungeon();
            // Add a stuck door
            dungeon.add_door(Door::new(3, 0, 3, DoorState::Stuck).unwrap()).unwrap();
            let character = test_character();
            let result = force_door_with(&mut rng, &mut dungeon, 3, &character);
            if result.contains("forces door 3 open") {
                found_success = true;
                break;
            }
        }
        assert!(found_success, "should eventually force a stuck door open");
    }

    #[test]
    fn elf_has_better_secret_door_detection() {
        // Elf gets 2-in-6, non-elf gets 1-in-6
        let mut elf_finds = 0;
        let mut human_finds = 0;
        let trials = 500;
        for seed in 0..trials {
            // Elf search
            let mut rng = StdRng::seed_from_u64(seed);
            let mut time = TimeTracker::new();
            let mut dungeon = test_dungeon();
            time.light(LightSourceKind::Lantern, "Test");
            dungeon.find_door_mut(0).unwrap().state = DoorState::Open;
            dungeon.move_to(1).unwrap();
            let result = search_room_with(&mut rng, &mut time, &mut dungeon, 1, true);
            if format!("{}", result).contains("Secret door found") {
                elf_finds += 1;
            }

            // Human search with same seed
            let mut rng = StdRng::seed_from_u64(seed);
            let mut time = TimeTracker::new();
            let mut dungeon = test_dungeon();
            time.light(LightSourceKind::Lantern, "Test");
            dungeon.find_door_mut(0).unwrap().state = DoorState::Open;
            dungeon.move_to(1).unwrap();
            let result = search_room_with(&mut rng, &mut time, &mut dungeon, 1, false);
            if format!("{}", result).contains("Secret door found") {
                human_finds += 1;
            }
        }
        assert!(
            elf_finds > human_finds,
            "elf should find secret doors more often ({} vs {} in {} trials)",
            elf_finds, human_finds, trials
        );
    }

    #[test]
    fn move_to_nonexistent_room_fails() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Test");

        // Try to move through a non-existent door
        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 99);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn move_through_door_not_connected_to_current_room() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Test");
        // Door 2 connects rooms 1-3, but party is in room 0
        dungeon.find_door_mut(2).unwrap().state = DoorState::Open;
        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not connected"));
    }

    #[test]
    fn door_auto_closes_after_passing() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Test");

        // Open door 0 and move through it
        dungeon.find_door_mut(0).unwrap().state = DoorState::Open;
        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_ok());
        assert_eq!(dungeon.current_room, Some(1));
        // Per OSE, door should auto-close after passing through
        assert_eq!(
            dungeon.doors.iter().find(|d| d.id == 0).unwrap().state,
            DoorState::Closed,
            "door should auto-close after passing through per OSE rules"
        );
    }

    #[test]
    fn rest_requirement_message_after_5_turns() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Test");

        // Advance 5 turns to trigger rest requirement
        for _ in 0..5 {
            advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        }
        assert!(time.needs_rest(), "should need rest after 5 turns");
        assert_eq!(time.rest_penalty(), -1, "should have -1 penalty");

        // 6th turn should mention rest requirement
        let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        assert!(
            result.messages.iter().any(|m| m.contains("must rest")),
            "should warn about rest requirement"
        );
    }

    #[test]
    fn wandering_monster_never_on_odd_turns() {
        // Wandering monsters only checked on even turns (every 2 turns)
        for seed in 0..200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut time = TimeTracker::new();
            let mut dungeon = test_dungeon();
            time.light(LightSourceKind::Lantern, "Test");

            // Turn 1 (odd) — should never trigger encounter from advance_dungeon_turn
            let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
            assert!(
                result.encounter.is_none(),
                "turn 1 (odd) should never have wandering monster (seed {})",
                seed
            );
        }
    }

    #[test]
    fn search_consumes_a_turn() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Test");

        let turns_before = time.total_turns;
        search_room_with(&mut rng, &mut time, &mut dungeon, 1, false);
        assert_eq!(time.total_turns, turns_before + 1, "search should consume one turn");
    }

    #[test]
    fn move_through_door_triggers_trap() {
        let mut triggered = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut time = TimeTracker::new();
            let mut dungeon = test_dungeon();
            time.light(LightSourceKind::Lantern, "Test");

            // Move to room 1 first, then through door 2 to trap room 3
            dungeon.find_door_mut(0).unwrap().state = DoorState::Open;
            dungeon.move_to(1).unwrap();

            let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 2);
            assert!(result.is_ok());
            let output = format!("{}", result.unwrap());
            if output.contains("TRAP TRIGGERED") {
                triggered = true;
                // Verify trap_triggered flag is set
                assert!(dungeon.find_room(3).unwrap().trap_triggered);
                break;
            }
        }
        assert!(triggered, "moving into trap room should eventually trigger a trap");
    }

    #[test]
    fn explore_output_includes_torch_status() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Torch, "Arden");

        let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        let output = format!("{}", result);
        assert!(
            output.contains("Torch: 5 turns remaining"),
            "explore output should show torch status, got: {}",
            output
        );
    }

    #[test]
    fn search_output_includes_torch_status() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Torch, "Arden");

        let result = search_room_with(&mut rng, &mut time, &mut dungeon, 1, false);
        let output = format!("{}", result);
        assert!(
            output.contains("Torch: 5 turns remaining"),
            "search output should show torch status, got: {}",
            output
        );
    }

    #[test]
    fn listen_output_includes_torch_status() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let dungeon = test_dungeon();
        time.light(LightSourceKind::Torch, "Arden");

        let result = listen_at_door_with(&mut rng, &mut time, &dungeon, 1, false);
        let output = format!("{}", result);
        assert!(
            output.contains("Torch: 5 turns remaining"),
            "listen output should show torch status, got: {}",
            output
        );
    }

    #[test]
    fn move_output_includes_torch_status() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Brin");
        dungeon.find_door_mut(0).unwrap().state = DoorState::Open;

        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_ok());
        let output = format!("{}", result.unwrap());
        assert!(
            output.contains("Lantern: 23 turns remaining"),
            "move output should show lantern status, got: {}",
            output
        );
    }

    #[test]
    fn explore_no_torch_status_in_darkness() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        // No light source — should show DARKNESS, not torch status

        let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        let output = format!("{}", result);
        assert!(output.contains("DARKNESS"), "should mention darkness");
        assert!(
            !output.contains("turns remaining"),
            "should not show torch status in darkness, got: {}",
            output
        );
    }
}
