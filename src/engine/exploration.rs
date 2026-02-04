use rand::Rng;

use crate::model::Character;
use crate::rules::ability;
use crate::rules::encounter;
use crate::state::dungeon::{DoorState, DungeonState};
use crate::state::time::TimeTracker;

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

    // Darkness warning
    if !time.has_light() {
        result.msg("The party is in DARKNESS! Movement is impossible without light.");
    }

    // Rest requirement
    if time.needs_rest() {
        result.msg("The party must rest this turn (1 turn per 5 turns of activity).");
    }

    // Mark current room as explored
    dungeon.explore_current();

    result.msg(format!("Turn {}: Exploring {}.",
        time.total_turns,
        dungeon.find_room(dungeon.current_room)
            .map(|r| r.name.as_str())
            .unwrap_or("unknown area")
    ));

    // Wandering monster check every 2 turns (1-in-6)
    if time.total_turns % 2 == 0 {
        let roll: u32 = rng.gen_range(1..=6);
        if roll == 1 {
            let table_roll: u32 = rng.gen_range(1..=20);
            let entry = encounter::dungeon_encounter(dungeon_level, table_roll);
            result.msg(format!(
                "Wandering monster! {} ({} appearing)",
                entry.name, entry.number
            ));
            result.encounter = Some(entry.clone());
        }
    }

    result
}

/// Search the current room for secret doors and hidden features.
/// Base chance: 1-in-6 (elves get 2-in-6).
/// Takes one turn.
pub fn search_room(time: &mut TimeTracker, dungeon: &mut DungeonState, is_elf: bool) -> String {
    search_room_with(&mut rand::thread_rng(), time, dungeon, is_elf)
}

/// Testable version.
pub fn search_room_with<R: Rng>(
    rng: &mut R,
    time: &mut TimeTracker,
    dungeon: &mut DungeonState,
    is_elf: bool,
) -> String {
    let light_msgs = time.advance_turn();
    let mut out = String::new();
    for msg in light_msgs {
        out.push_str(&msg);
        out.push('\n');
    }

    let threshold = if is_elf { 2 } else { 1 };
    let roll: u32 = rng.gen_range(1..=6);
    let success = roll <= threshold;

    if let Some(room) = dungeon.find_room_mut(dungeon.current_room) {
        room.searched = true;
    }

    if success {
        // Check for secret doors in this room
        let current = dungeon.current_room;
        let mut found_something = false;
        for door in &mut dungeon.doors {
            if door.state == DoorState::Secret && !door.discovered
                && (door.room_a == current || door.room_b == current)
            {
                door.discovered = true;
                found_something = true;
                out.push_str(&format!(
                    "Secret door found! Door {} leads to room {}.\n",
                    door.id,
                    if door.room_a == current { door.room_b } else { door.room_a }
                ));
            }
        }
        if !found_something {
            out.push_str("Search successful — but nothing hidden found here.");
        }
    } else {
        out.push_str("Search reveals nothing.");
    }

    out
}

/// Listen at a door. Base chance 1-in-6 to hear noises behind it.
pub fn listen_at_door(is_elf_or_halfling: bool) -> String {
    listen_at_door_with(&mut rand::thread_rng(), is_elf_or_halfling)
}

/// Testable version.
pub fn listen_at_door_with<R: Rng>(rng: &mut R, is_elf_or_halfling: bool) -> String {
    let threshold = if is_elf_or_halfling { 2 } else { 1 };
    let roll: u32 = rng.gen_range(1..=6);
    if roll <= threshold {
        "You hear sounds beyond the door!".to_string()
    } else {
        "You hear nothing.".to_string()
    }
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

    // Base 2-in-6, STR modifier adjusts threshold
    let str_mod = ability::str_melee_mod(character.abilities.strength);
    let threshold = (2 + str_mod).max(1).min(5) as u32;
    let roll: u32 = rng.gen_range(1..=6);

    if roll <= threshold {
        // Access door again mutably
        let door = dungeon.find_door_mut(door_id).unwrap();
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
/// Consumes a turn. Checks for traps in the destination room.
pub fn move_through_door(
    time: &mut TimeTracker,
    dungeon: &mut DungeonState,
    door_id: u32,
) -> Result<String, String> {
    move_through_door_with(&mut rand::thread_rng(), time, dungeon, door_id)
}

/// Testable version.
pub fn move_through_door_with<R: Rng>(
    rng: &mut R,
    time: &mut TimeTracker,
    dungeon: &mut DungeonState,
    door_id: u32,
) -> Result<String, String> {
    let door = match dungeon.doors.iter().find(|d| d.id == door_id) {
        Some(d) => d.clone(),
        None => return Err(format!("Door {} not found.", door_id)),
    };

    if !door.is_passable() {
        return Err(format!("Door {} is not open. Force it first.", door_id));
    }

    let dest = if door.room_a == dungeon.current_room {
        door.room_b
    } else if door.room_b == dungeon.current_room {
        door.room_a
    } else {
        return Err(format!("Door {} is not connected to the current room.", door_id));
    };

    dungeon.move_to(dest).map_err(|e| e.to_string())?;

    let mut out = String::new();

    // Check for trap in new room
    let has_trap = dungeon.find_room(dest).and_then(|r| r.trap.as_ref()).is_some();
    let room_name = dungeon.find_room(dest)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| format!("room {}", dest));

    if has_trap {
        let trap_msg = check_trap_with(rng, &room_name);
        out.push_str(&trap_msg);
        out.push('\n');
        if trap_msg.contains("TRIGGERED") {
            if let Some(room) = dungeon.find_room_mut(dest) {
                room.trap_triggered = true;
            }
        }
    }

    out.push_str(&format!(
        "Moved to {} (room {}).",
        room_name, dest
    ));

    // Advance time for the move
    let light_msgs = time.advance_turn();
    for msg in light_msgs {
        out.push('\n');
        out.push_str(&msg);
    }

    Ok(out)
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
    use crate::state::dungeon::{Door, Room};
    use crate::state::time::LightSourceKind;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn test_dungeon() -> DungeonState {
        let mut ds = DungeonState::new(1);
        ds.add_room(Room::new(0, "Entrance Hall"));
        ds.add_room(Room::new(1, "Guard Room"));
        ds.add_room(Room::new(2, "Hidden Chamber"));
        ds.add_room(Room::new(3, "Trap Room").with_trap("Pit trap"));
        ds.add_door(Door::new(0, 0, 1, DoorState::Closed));
        ds.add_door(Door::new(1, 1, 2, DoorState::Secret));
        ds.add_door(Door::new(2, 1, 3, DoorState::Open));
        ds
    }

    fn test_character() -> Character {
        let mut c = Character::new("Arden", "Fighter");
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
    fn darkness_warning_without_light() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();

        let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        assert!(result.messages.iter().any(|m| m.contains("DARKNESS")));
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

        let result = search_room_with(&mut rng, &mut time, &mut dungeon, false);
        // Should either find something or not
        assert!(!result.is_empty());
    }

    #[test]
    fn search_finds_secret_door() {
        // Seed that gives roll of 1 (success)
        let mut found = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut time = TimeTracker::new();
            let mut dungeon = test_dungeon();
            time.light(LightSourceKind::Lantern, "Test");
            dungeon.move_to(1).unwrap(); // Guard Room has secret door

            let result = search_room_with(&mut rng, &mut time, &mut dungeon, false);
            if result.contains("Secret door found") {
                found = true;
                break;
            }
        }
        assert!(found, "should eventually find a secret door");
    }

    #[test]
    fn listen_at_door_basic() {
        let mut rng = test_rng();
        let result = listen_at_door_with(&mut rng, false);
        assert!(result.contains("hear") || result.contains("nothing"));
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
        dungeon.add_door(Door::new(3, 0, 3, DoorState::Locked));
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
        dungeon.move_to(1).unwrap(); // Move to guard room

        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 2);
        assert!(result.is_ok());
        // Should have moved to room 3 (Trap Room)
        assert_eq!(dungeon.current_room, 3);
    }

    #[test]
    fn move_through_closed_door_fails() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Test");

        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 0);
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
}
