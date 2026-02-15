use rand::Rng;

mod actions;
pub mod results;

pub use actions::{
    action_add_door, action_add_room, action_advance_dungeon_turn, action_enter_dungeon,
    action_exploration_status, action_force_door, action_light, action_listen_at_door, action_look,
    action_move_through_door, action_open_door, action_pick_lock, action_rest, action_search_room,
};

use crate::model::Character;
use crate::rules::ability;
use crate::rules::encounter;
use crate::rules::thief;
use crate::state::dungeon::{DoorState, DungeonState, PlacedMonsterInstance, PlacedTreasureInstance, TrapTrigger};
use crate::state::time::TimeTracker;

/// Check for wandering monsters (1-in-6 every 2 turns).
/// Should be called after any action that consumes a dungeon turn.
fn check_wandering_monster<R: Rng>(
    rng: &mut R,
    time: &TimeTracker,
    dungeon_level: u32,
) -> Option<encounter::EncounterEntry> {
    if time.total_turns.is_multiple_of(2) {
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
    /// Placed monsters from module that should spawn combat.
    pub placed_monsters: Option<Vec<PlacedMonsterInstance>>,
    /// Placed treasure from module found during search.
    pub placed_treasure: Option<Vec<PlacedTreasureInstance>>,
}

impl ExplorationResult {
    fn new() -> Self {
        ExplorationResult {
            messages: Vec::new(),
            encounter: None,
            placed_monsters: None,
            placed_treasure: None,
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
        if let Some(monsters) = &self.placed_monsters {
            for m in monsters {
                writeln!(f, "*** PLACED MONSTER: {} x{} ***", m.name, m.count)?;
            }
        }
        if let Some(treasure) = &self.placed_treasure {
            for t in treasure {
                writeln!(f, "*** TREASURE FOUND: {} ({}gp) ***", t.description, t.gp_value)?;
            }
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

    // Cannot search in darkness
    if !time.has_light() {
        result.msg("The party is in DARKNESS! Cannot search without light.");
        result.msg("Light a torch or lantern before searching.");
        return result;
    }

    let light_msgs = time.advance_turn();
    for msg in light_msgs {
        result.msg(msg);
    }

    let current = match dungeon.current_room {
        Some(id) => id,
        None => return result,
    };

    let threshold = if is_elf { 2 } else { 1 };
    let roll: u32 = rng.gen_range(1..=6);
    let success = roll <= threshold;

    if let Some(room) = dungeon.find_room_mut(current) {
        room.searched = true;
    }

    if success {
        // Check for secret doors in this room
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

    // Check for placed treasure in room (module support)
    if let Some(room) = dungeon.find_room(current) {
        if !room.treasure_looted && !room.placed_treasure.is_empty() {
            let loot: Vec<_> = room.placed_treasure.iter()
                .filter(|t| !t.taken)
                .cloned()
                .collect();
            if !loot.is_empty() {
                // Mark treasure as discovered (but not taken — loot command handles pickup)
                if let Some(room_mut) = dungeon.find_room_mut(current) {
                    room_mut.treasure_looted = true;
                }
                // Report treasure found
                for t in &loot {
                    result.msg(format!("Treasure found: {} ({}gp)", t.description, t.gp_value));
                }
                result.placed_treasure = Some(loot);
            }
        }
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
    dungeon: &DungeonState,
    dungeon_level: u32,
    is_elf_or_halfling: bool,
) -> ExplorationResult {
    let mut result = ExplorationResult::new();

    // Validate that there is at least one door in the current room
    if dungeon.doors_from_current().is_empty() {
        result.msg("There are no doors to listen at in this room.");
        return result;
    }

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

/// Result of a pick lock attempt.
#[derive(Debug)]
pub struct PickLockResult {
    pub success: bool,
    pub message: String,
}

/// Attempt to pick a locked door using thief open_locks skill.
/// On success, door state changes from Locked to Closed.
pub fn pick_lock(
    dungeon: &mut DungeonState,
    door_id: u32,
    character: &Character,
) -> PickLockResult {
    pick_lock_with(&mut rand::thread_rng(), dungeon, door_id, character)
}

/// Testable version.
pub fn pick_lock_with<R: Rng>(
    rng: &mut R,
    dungeon: &mut DungeonState,
    door_id: u32,
    character: &Character,
) -> PickLockResult {
    let door = match dungeon.find_door_mut(door_id) {
        Some(d) => d,
        None => return PickLockResult {
            success: false,
            message: format!("Door {} not found.", door_id),
        },
    };

    if door.state != DoorState::Locked {
        return PickLockResult {
            success: false,
            message: format!("Door {} is not locked.", door_id),
        };
    }

    if !thief::has_thief_skills(character.class) {
        return PickLockResult {
            success: false,
            message: format!("{} does not have lockpicking skills.", character.name),
        };
    }

    if !character.is_alive() {
        return PickLockResult {
            success: false,
            message: format!("{} is dead.", character.name),
        };
    }

    let roll: u32 = rng.gen_range(1..=100);
    let check = thief::check_skill(thief::ThiefSkill::OpenLocks, character.level, roll);

    if check.success {
        let door = dungeon.find_door_mut(door_id)
            .expect("door verified to exist above");
        door.state = DoorState::Closed;
        PickLockResult {
            success: true,
            message: format!(
                "{} picks the lock on door {}! (rolled {} vs {}%) The door is now unlocked.",
                character.name, door_id, roll, check.target
            ),
        }
    } else {
        PickLockResult {
            success: false,
            message: format!(
                "{} fails to pick the lock on door {}. (rolled {} vs {}%)",
                character.name, door_id, roll, check.target
            ),
        }
    }
}

/// Result of a trap check.
#[derive(Debug)]
pub struct TrapResult {
    pub triggered: bool,
    pub message: String,
}

/// Check for trap trigger. Traps trigger on 1-2 on d6.
pub fn check_trap(room_name: &str) -> TrapResult {
    check_trap_with(&mut rand::thread_rng(), room_name)
}

/// Testable version.
pub fn check_trap_with<R: Rng>(rng: &mut R, room_name: &str) -> TrapResult {
    let roll: u32 = rng.gen_range(1..=6);
    if roll <= 2 {
        TrapResult {
            triggered: true,
            message: format!("TRAP TRIGGERED in {}! (rolled {})", room_name, roll),
        }
    } else {
        TrapResult {
            triggered: false,
            message: format!("No trap triggered in {}. (rolled {})", room_name, roll),
        }
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

/// Check for a trap when entering a room. Only `Entry` traps roll automatically
/// on room entry (1-2 on d6). `Action` traps require explicit character
/// interaction and are not triggered on entry — instead, the GM is notified
/// that the room contains a trap requiring a character action.
fn check_room_trap<R: Rng>(
    rng: &mut R,
    dungeon: &mut DungeonState,
    dest: u32,
    result: &mut ExplorationResult,
) {
    let room_info = dungeon.find_room(dest).and_then(|r| {
        if r.trap.is_some() && !r.trap_triggered {
            Some((r.name.clone(), r.trap.clone().unwrap(), r.trap_trigger))
        } else {
            None
        }
    });

    let (room_name, trap_desc, trigger) = match room_info {
        Some(info) => info,
        None => return,
    };

    match trigger {
        TrapTrigger::Entry => {
            let trap = check_trap_with(rng, &room_name);
            if trap.triggered {
                if let Some(room) = dungeon.find_room_mut(dest) {
                    room.trap_triggered = true;
                }
            }
            result.msg(trap.message);
        }
        TrapTrigger::Action => {
            result.msg(format!(
                "TRAP PRESENT in {}: {} (requires character action to trigger)",
                room_name, trap_desc
            ));
        }
    }
}

/// Check for placed monsters (module support) in a room. Collects unspawned
/// monsters, marks them spawned, and reports them on the result.
fn collect_placed_monsters(
    dungeon: &mut DungeonState,
    dest: u32,
    result: &mut ExplorationResult,
) {
    let unspawned: Vec<_> = dungeon.find_room(dest)
        .filter(|room| !room.monsters_cleared)
        .map(|room| {
            room.placed_monsters.iter()
                .filter(|m| !m.spawned)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if unspawned.is_empty() {
        return;
    }

    if let Some(room) = dungeon.find_room_mut(dest) {
        for m in &mut room.placed_monsters {
            m.spawned = true;
        }
    }
    for m in &unspawned {
        let undead_tag = if m.undead == Some(true) { " [undead]" } else { "" };
        result.msg(format!("Monsters present: {} x{}{}", m.name, m.count, undead_tag));
    }
    result.placed_monsters = Some(unspawned);
}

/// Testable version.
pub fn move_through_door_with<R: Rng>(
    rng: &mut R,
    time: &mut TimeTracker,
    dungeon: &mut DungeonState,
    dungeon_level: u32,
    door_id: u32,
) -> Result<ExplorationResult, String> {
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

    // Per OSE, doors close automatically after passing through (unless spiked
    // or permanently open per module definition)
    if door.state == DoorState::Open && !door.module_open {
        if let Some(d) = dungeon.find_door_mut(door_id) {
            d.state = DoorState::Closed;
        }
    }

    let mut result = ExplorationResult::new();

    check_room_trap(rng, dungeon, dest, &mut result);

    let room_name = dungeon.find_room(dest)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| format!("room {}", dest));
    result.msg(format!("Moved to {} (room {}).", room_name, dest));

    // Advance time for the move
    let light_msgs = time.advance_turn();
    for msg in light_msgs {
        result.msg(msg);
    }

    if let Some(summary) = time.light_summary() {
        result.msg(summary);
    }

    collect_placed_monsters(dungeon, dest, &mut result);

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
    fn encounter_result_includes_next_steps() {
        use super::results::ExplorationActionResult;
        // Find a seed that triggers an encounter on turn 2
        for seed in 0..200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut time = TimeTracker::new();
            let mut dungeon = test_dungeon();
            time.light(LightSourceKind::Lantern, "Test");

            advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
            let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
            if result.encounter.is_some() {
                let api_result = ExplorationActionResult::from(result);
                // next_steps should be populated
                assert!(!api_result.next_steps.is_empty(), "encounter should include next_steps");
                assert!(api_result.next_steps.iter().any(|s| s.contains("SpawnMonster")),
                    "next_steps should mention SpawnMonster");
                assert!(api_result.next_steps.iter().any(|s| s.contains("RollReaction")),
                    "next_steps should mention RollReaction");
                assert!(api_result.next_steps.iter().any(|s| s.contains("Evade")),
                    "next_steps should mention Evade");
                // Guidance message should be in messages
                assert!(api_result.messages.iter().any(|m| m.contains("ENCOUNTER RESOLUTION REQUIRED")),
                    "messages should include resolution guidance");
                // Message text should include guidance
                assert!(api_result.message.contains("ENCOUNTER RESOLUTION REQUIRED"),
                    "message text should include resolution guidance");
                return;
            }
        }
        panic!("failed to trigger an encounter in 200 seeds");
    }

    #[test]
    fn no_encounter_means_no_next_steps() {
        use super::results::ExplorationActionResult;
        let mut rng = StdRng::seed_from_u64(42);
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Test");

        // Turn 1 — odd turn, never triggers encounter
        let result = advance_dungeon_turn_with(&mut rng, &mut time, &mut dungeon, 1);
        assert!(result.encounter.is_none());
        let api_result = ExplorationActionResult::from(result);
        assert!(api_result.next_steps.is_empty(), "no encounter means no next_steps");
        assert!(!api_result.message.contains("ENCOUNTER RESOLUTION REQUIRED"),
            "no encounter means no guidance");
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
            let output = result.to_string();
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

    fn test_thief() -> Character {
        let mut c = Character::new("Shadow", Class::Thief);
        c.level = 3; // OpenLocks at level 3 = 25%
        c
    }

    #[test]
    fn pick_lock_success() {
        let mut success = false;
        for seed in 0..200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut dungeon = test_dungeon();
            dungeon.add_door(Door::new(3, 0, 3, DoorState::Locked).unwrap()).unwrap();
            let thief = test_thief();

            let result = pick_lock_with(&mut rng, &mut dungeon, 3, &thief);
            if result.success {
                assert!(result.message.contains("picks the lock"));
                assert_eq!(dungeon.find_door_mut(3).unwrap().state, DoorState::Closed);
                success = true;
                break;
            }
        }
        assert!(success, "thief should eventually pick the lock");
    }

    #[test]
    fn pick_lock_failure() {
        let mut failure = false;
        for seed in 0..200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut dungeon = test_dungeon();
            dungeon.add_door(Door::new(3, 0, 3, DoorState::Locked).unwrap()).unwrap();
            let thief = test_thief();

            let result = pick_lock_with(&mut rng, &mut dungeon, 3, &thief);
            if !result.success {
                assert!(result.message.contains("fails to pick"));
                assert_eq!(dungeon.find_door_mut(3).unwrap().state, DoorState::Locked);
                failure = true;
                break;
            }
        }
        assert!(failure, "thief should sometimes fail");
    }

    #[test]
    fn pick_lock_non_thief_rejected() {
        let mut rng = test_rng();
        let mut dungeon = test_dungeon();
        dungeon.add_door(Door::new(3, 0, 3, DoorState::Locked).unwrap()).unwrap();
        let fighter = test_character(); // Fighter, no thief skills

        let result = pick_lock_with(&mut rng, &mut dungeon, 3, &fighter);
        assert!(!result.success);
        assert!(result.message.contains("does not have lockpicking"));
    }

    #[test]
    fn pick_lock_not_locked_rejected() {
        let mut rng = test_rng();
        let mut dungeon = test_dungeon();
        let thief = test_thief();

        // Door 0 is Closed, not Locked
        let result = pick_lock_with(&mut rng, &mut dungeon, 0, &thief);
        assert!(!result.success);
        assert!(result.message.contains("not locked"));
    }

    #[test]
    fn pick_lock_door_not_found() {
        let mut rng = test_rng();
        let mut dungeon = test_dungeon();
        let thief = test_thief();

        let result = pick_lock_with(&mut rng, &mut dungeon, 99, &thief);
        assert!(!result.success);
        assert!(result.message.contains("not found"));
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
            if result.triggered {
                assert!(result.message.contains("TRIGGERED"), "triggered message should contain TRIGGERED");
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
            if result.to_string().contains("Secret door found") {
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
            if result.to_string().contains("Secret door found") {
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

        // Open door 0 (not module_open) and move through it
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
    fn module_open_door_does_not_auto_close() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = test_dungeon();
        time.light(LightSourceKind::Lantern, "Test");

        // Mark door 0 as module_open (permanent passage from module definition)
        let door = dungeon.find_door_mut(0).unwrap();
        door.state = DoorState::Open;
        door.module_open = true;

        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_ok());
        assert_eq!(dungeon.current_room, Some(1));
        // Module-open doors should NOT auto-close
        assert_eq!(
            dungeon.doors.iter().find(|d| d.id == 0).unwrap().state,
            DoorState::Open,
            "module-open door should remain open after passing through"
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
            let output = result.unwrap().to_string();
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
        let output = result.to_string();
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
        let output = result.to_string();
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
        let output = result.to_string();
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
        let output = result.unwrap().to_string();
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
        let output = result.to_string();
        assert!(output.contains("DARKNESS"), "should mention darkness");
        assert!(
            !output.contains("turns remaining"),
            "should not show torch status in darkness, got: {}",
            output
        );
    }

    // =========================================================================
    // Module support tests: placed monsters and treasure
    // =========================================================================

    #[test]
    fn move_into_room_spawns_placed_monsters() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = DungeonState::new(1);
        time.light(LightSourceKind::Lantern, "Test");

        // Set up rooms
        dungeon.add_room(Room::new(0, "Entrance")).unwrap();
        let monster_room = Room::new(1, "Monster Lair")
            .with_placed_monsters(vec![
                PlacedMonsterInstance::new("skeleton", 3),
                PlacedMonsterInstance::new("zombie", 2),
            ]);
        dungeon.add_room(monster_room).unwrap();
        dungeon.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();

        // Move into monster room
        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_ok());
        let result = result.unwrap();

        // Verify output includes monster info (check before unwrapping placed_monsters)
        let output = result.to_string();
        assert!(output.contains("skeleton x3"), "output should mention skeletons");
        assert!(output.contains("zombie x2"), "output should mention zombies");

        // Verify monsters are reported
        assert!(result.placed_monsters.is_some(), "should report placed monsters");
        let monsters = result.placed_monsters.unwrap();
        assert_eq!(monsters.len(), 2, "should have 2 monster groups");
        assert_eq!(monsters[0].name, "skeleton");
        assert_eq!(monsters[0].count, 3);
        assert_eq!(monsters[1].name, "zombie");
        assert_eq!(monsters[1].count, 2);

        // Verify monsters are marked as spawned
        let room = dungeon.find_room(1).unwrap();
        assert!(room.placed_monsters.iter().all(|m| m.spawned), "all monsters should be marked spawned");
    }

    #[test]
    fn placed_monsters_report_undead_tag() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = DungeonState::new(1);
        time.light(LightSourceKind::Lantern, "Test");

        dungeon.add_room(Room::new(0, "Entrance")).unwrap();
        let mut skeleton = PlacedMonsterInstance::new("Frosted Skeleton", 4);
        skeleton.undead = Some(true);
        let spider = PlacedMonsterInstance::new("Ice Spider", 2);
        let room = Room::new(1, "Bone Crypt")
            .with_placed_monsters(vec![skeleton, spider]);
        dungeon.add_room(room).unwrap();
        dungeon.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();

        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0).unwrap();
        let output = result.to_string();
        assert!(output.contains("Frosted Skeleton x4 [undead]"), "undead monsters should be tagged: {output}");
        assert!(output.contains("Ice Spider x2"), "non-undead should not be tagged: {output}");
        assert!(!output.contains("Ice Spider x2 [undead]"), "non-undead must not have tag");

        let monsters = result.placed_monsters.unwrap();
        assert_eq!(monsters[0].undead, Some(true));
        assert_eq!(monsters[1].undead, None);
    }

    #[test]
    fn move_into_cleared_room_does_not_spawn_monsters() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = DungeonState::new(1);
        time.light(LightSourceKind::Lantern, "Test");

        // Set up rooms with monsters already cleared
        dungeon.add_room(Room::new(0, "Entrance")).unwrap();
        let mut monster_room = Room::new(1, "Monster Lair")
            .with_placed_monsters(vec![PlacedMonsterInstance::new("orc", 4)]);
        monster_room.monsters_cleared = true;
        dungeon.add_room(monster_room).unwrap();
        dungeon.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();

        // Move into cleared room
        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_ok());
        let result = result.unwrap();

        // Should NOT report monsters (room is cleared)
        assert!(result.placed_monsters.is_none(), "should not report monsters in cleared room");
    }

    #[test]
    fn re_entering_room_after_spawn_does_not_spawn_again() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = DungeonState::new(1);
        time.light(LightSourceKind::Lantern, "Test");

        // Set up rooms
        dungeon.add_room(Room::new(0, "Entrance")).unwrap();
        let monster_room = Room::new(1, "Monster Lair")
            .with_placed_monsters(vec![PlacedMonsterInstance::new("goblin", 5)]);
        dungeon.add_room(monster_room).unwrap();
        dungeon.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();

        // First entry - spawns monsters
        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_ok());
        assert!(result.unwrap().placed_monsters.is_some(), "first entry should spawn");

        // Go back to entrance (door auto-closes, so reopen it)
        dungeon.find_door_mut(0).unwrap().state = DoorState::Open;
        let _ = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);

        // Re-enter monster room (door auto-closes, so reopen it)
        dungeon.find_door_mut(0).unwrap().state = DoorState::Open;
        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_ok());
        let result = result.unwrap();

        // Should NOT spawn again (already spawned)
        assert!(result.placed_monsters.is_none(), "re-entry should not spawn monsters again");
    }

    #[test]
    fn search_room_finds_placed_treasure() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = DungeonState::new(1);
        time.light(LightSourceKind::Lantern, "Test");

        // Set up room with treasure
        let treasure_room = Room::new(0, "Treasure Vault")
            .with_placed_treasure(vec![
                PlacedTreasureInstance::new("Gold coins", 500),
                PlacedTreasureInstance::new("Potion of Healing", 50),
            ]);
        dungeon.add_room(treasure_room).unwrap();

        // Search the room
        let result = search_room_with(&mut rng, &mut time, &mut dungeon, 1, false);

        // Verify output includes treasure info (check before unwrapping placed_treasure)
        let output = result.to_string();
        assert!(output.contains("Gold coins"), "output should mention gold");
        assert!(output.contains("500gp"), "output should mention gold value");

        // Verify treasure is reported
        assert!(result.placed_treasure.is_some(), "should report placed treasure");
        let treasure = result.placed_treasure.unwrap();
        assert_eq!(treasure.len(), 2, "should have 2 treasure items");
        assert_eq!(treasure[0].description, "Gold coins");
        assert_eq!(treasure[0].gp_value, 500);
        assert_eq!(treasure[1].description, "Potion of Healing");
        assert_eq!(treasure[1].gp_value, 50);

        // Verify treasure is discovered but NOT taken (loot command handles pickup)
        let room = dungeon.find_room(0).unwrap();
        assert!(room.treasure_looted, "room should be marked as discovered");
        assert!(room.placed_treasure.iter().all(|t| !t.taken), "treasure should not be marked taken until looted");
    }

    #[test]
    fn search_looted_room_finds_no_treasure() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = DungeonState::new(1);
        time.light(LightSourceKind::Lantern, "Test");

        // Set up room with treasure already looted
        let mut treasure_room = Room::new(0, "Empty Vault")
            .with_placed_treasure(vec![PlacedTreasureInstance::new("Ancient sword", 1000)]);
        treasure_room.treasure_looted = true;
        dungeon.add_room(treasure_room).unwrap();

        // Search the room
        let result = search_room_with(&mut rng, &mut time, &mut dungeon, 1, false);

        // Should NOT report treasure (already looted)
        assert!(result.placed_treasure.is_none(), "should not report treasure in looted room");
    }

    #[test]
    fn search_room_twice_only_yields_treasure_once() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = DungeonState::new(1);
        time.light(LightSourceKind::Lantern, "Test");

        // Set up room with treasure
        let treasure_room = Room::new(0, "Treasury")
            .with_placed_treasure(vec![PlacedTreasureInstance::new("Gems", 200)]);
        dungeon.add_room(treasure_room).unwrap();

        // First search - yields treasure
        let result = search_room_with(&mut rng, &mut time, &mut dungeon, 1, false);
        assert!(result.placed_treasure.is_some(), "first search should find treasure");

        // Second search - no treasure
        let result = search_room_with(&mut rng, &mut time, &mut dungeon, 1, false);
        assert!(result.placed_treasure.is_none(), "second search should not find treasure");
    }

    #[test]
    fn room_with_no_placed_content_works_normally() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = DungeonState::new(1);
        time.light(LightSourceKind::Lantern, "Test");

        // Set up regular rooms (no placed monsters/treasure)
        dungeon.add_room(Room::new(0, "Empty Hall")).unwrap();
        dungeon.add_room(Room::new(1, "Another Room")).unwrap();
        dungeon.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();

        // Move into room
        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.placed_monsters.is_none(), "empty room should have no monsters");

        // Search room
        let result = search_room_with(&mut rng, &mut time, &mut dungeon, 1, false);
        assert!(result.placed_treasure.is_none(), "empty room should have no treasure");
    }

    // =========================================================================
    // Trap trigger type tests
    // =========================================================================

    #[test]
    fn action_trap_does_not_trigger_on_room_entry() {
        // Action traps should never auto-trigger on room entry, regardless of RNG
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut time = TimeTracker::new();
            let mut dungeon = DungeonState::new(1);
            time.light(LightSourceKind::Lantern, "Test");

            dungeon.add_room(Room::new(0, "Entrance")).unwrap();
            dungeon.add_room(
                Room::new(1, "Freezing Mirror")
                    .with_trap("Save vs paralysis or be frozen")
                    .with_trap_trigger(TrapTrigger::Action)
            ).unwrap();
            dungeon.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();

            let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
            assert!(result.is_ok());
            let output = result.unwrap().to_string();
            assert!(
                !output.contains("TRAP TRIGGERED"),
                "action trap should never auto-trigger on entry (seed {}), got: {}",
                seed, output
            );
            assert!(
                !dungeon.find_room(1).unwrap().trap_triggered,
                "action trap flag should not be set on entry (seed {})",
                seed
            );
        }
    }

    #[test]
    fn action_trap_notifies_gm_on_entry() {
        let mut rng = test_rng();
        let mut time = TimeTracker::new();
        let mut dungeon = DungeonState::new(1);
        time.light(LightSourceKind::Lantern, "Test");

        dungeon.add_room(Room::new(0, "Entrance")).unwrap();
        dungeon.add_room(
            Room::new(1, "Freezing Mirror")
                .with_trap("Save vs paralysis or be frozen")
                .with_trap_trigger(TrapTrigger::Action)
        ).unwrap();
        dungeon.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();

        let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
        assert!(result.is_ok());
        let output = result.unwrap().to_string();
        assert!(
            output.contains("TRAP PRESENT") && output.contains("requires character action"),
            "should notify GM about action trap, got: {}",
            output
        );
    }

    #[test]
    fn entry_trap_still_triggers_on_room_entry() {
        // Verify Entry traps (the default) still behave as before
        let mut triggered = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut time = TimeTracker::new();
            let mut dungeon = DungeonState::new(1);
            time.light(LightSourceKind::Lantern, "Test");

            dungeon.add_room(Room::new(0, "Entrance")).unwrap();
            dungeon.add_room(
                Room::new(1, "Pit Room")
                    .with_trap("Pit trap (1d6 damage)")
                    .with_trap_trigger(TrapTrigger::Entry)
            ).unwrap();
            dungeon.add_door(Door::new(0, 0, 1, DoorState::Open).unwrap()).unwrap();

            let result = move_through_door_with(&mut rng, &mut time, &mut dungeon, 1, 0);
            assert!(result.is_ok());
            if result.unwrap().to_string().contains("TRAP TRIGGERED") {
                triggered = true;
                assert!(dungeon.find_room(1).unwrap().trap_triggered);
                break;
            }
        }
        assert!(triggered, "entry trap should eventually trigger on room entry");
    }
}
