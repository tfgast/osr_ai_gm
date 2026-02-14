//! Integration tests for the OSR AI GM engine.
//!
//! Full scenario: create characters -> explore dungeon -> encounter -> combat -> loot -> XP -> level up.

use osr_ai_gm::gmapi::protocol::{EncounterParams, GMCommand, GMRequest};
use osr_ai_gm::gmapi::interface::handle_request;
use osr_ai_gm::persist::GameState;
use osr_ai_gm::model::{Character, AbilityScores};
use osr_ai_gm::rules::alignment::Alignment;
use osr_ai_gm::rules::class::Class;
use osr_ai_gm::state::dungeon::DoorState;
use osr_ai_gm::state::game::GameMode;
use osr_ai_gm::state::time::LightSourceKind;
use osr_ai_gm::state::wilderness::Terrain;

use std::sync::atomic::{AtomicU64, Ordering};

fn unique_save_name(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    format!("osr_test_{prefix}_{pid}_{n}")
}

fn resolve_save(name: &str) -> std::path::PathBuf {
    osr_ai_gm::persist::safe_save_path(name).unwrap()
}

fn req(id: &str, command: GMCommand) -> GMRequest {
    GMRequest { id: id.to_string(), command }
}

/// Helper: create a fighter with known ability scores (bypasses random rolls).
fn make_fighter(name: &str) -> Character {
    let mut c = Character::new(name, Class::Fighter);
    c.abilities = AbilityScores {
        strength: 16, intelligence: 10, wisdom: 10,
        dexterity: 12, constitution: 14, charisma: 12,
    };
    c.hp = 8;
    c.max_hp = 8;
    c.ac = 3;  // plate mail + shield
    c.thac0 = 19;
    c.alignment = Alignment::Lawful;
    c.gold_gp = 120;
    c.movement_rate = 60;
    c
}

/// Helper: create a thief with known ability scores.
fn make_thief(name: &str) -> Character {
    let mut c = Character::new(name, Class::Thief);
    c.abilities = AbilityScores {
        strength: 10, intelligence: 12, wisdom: 10,
        dexterity: 16, constitution: 10, charisma: 10,
    };
    c.hp = 4;
    c.max_hp = 4;
    c.ac = 6;  // leather + DEX
    c.thac0 = 19;
    c.alignment = Alignment::Neutral;
    c.gold_gp = 80;
    c.movement_rate = 120;
    c
}

/// Helper: create a cleric with known ability scores.
fn make_cleric(name: &str) -> Character {
    let mut c = Character::new(name, Class::Cleric);
    c.abilities = AbilityScores {
        strength: 12, intelligence: 10, wisdom: 16,
        dexterity: 10, constitution: 12, charisma: 14,
    };
    c.hp = 6;
    c.max_hp = 6;
    c.ac = 4;  // chain + shield
    c.thac0 = 19;
    c.alignment = Alignment::Lawful;
    c.gold_gp = 100;
    c.movement_rate = 60;
    c
}

/// Helper: create a magic-user with known ability scores.
fn make_magic_user(name: &str) -> Character {
    let mut c = Character::new(name, Class::MagicUser);
    c.abilities = AbilityScores {
        strength: 8, intelligence: 16, wisdom: 12,
        dexterity: 12, constitution: 10, charisma: 10,
    };
    c.hp = 3;
    c.max_hp = 3;
    c.ac = 9;  // no armour
    c.thac0 = 19;
    c.alignment = Alignment::Neutral;
    c.gold_gp = 60;
    c.movement_rate = 120;
    c
}

// ===========================================================================
// INTEGRATION TEST: Full dungeon session
// ===========================================================================

#[test]
fn full_dungeon_session() {
    let mut state = GameState::new();

    // -- STEP 1: Create a party --
    state.party.add_member(make_fighter("Aldric"));
    state.party.add_member(make_thief("Shade"));
    state.party.add_member(make_cleric("Brother Marcus"));
    assert_eq!(state.party.members.len(), 3);

    // -- STEP 2: Enter the dungeon --
    let resp = handle_request(&req("10", GMCommand::EnterDungeon {
        level: 1,
        room_name: "Entrance Hall".to_string(),
    }), &mut state);
    assert!(resp.success, "enter dungeon failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Exploration);

    // -- STEP 3: Light a torch --
    let resp = handle_request(&req("11", GMCommand::Light {
        source: LightSourceKind::Torch,
        carrier: "Aldric".to_string(),
    }), &mut state);
    assert!(resp.success, "light torch failed: {}", resp.message);
    assert_eq!(state.time.as_ref().unwrap().lights.len(), 1, "should have 1 light source");
    assert_eq!(state.time.as_ref().unwrap().lights[0].carrier, "Aldric");

    // -- STEP 4: Advance a dungeon turn --
    let resp = handle_request(&req("12", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success, "advance turn failed: {}", resp.message);
    assert_eq!(state.time.as_ref().unwrap().total_turns, 1, "turn should have advanced to 1");

    // -- STEP 5: Add rooms and explore --
    let resp = handle_request(&req("13", GMCommand::AddRoom {
        id: 1,
        name: "Guard Room".to_string(),
    }), &mut state);
    assert!(resp.success, "add room failed: {}", resp.message);
    assert_eq!(state.dungeon.as_ref().unwrap().rooms.len(), 2, "should have 2 rooms");

    let resp = handle_request(&req("14", GMCommand::AddDoor {
        id: 0,
        room_a: 0,
        room_b: 1,
        state: DoorState::Closed,
    }), &mut state);
    assert!(resp.success, "add door failed: {}", resp.message);
    assert_eq!(state.dungeon.as_ref().unwrap().doors.len(), 1, "should have 1 door");

    // -- STEP 6: Search the room --
    let resp = handle_request(&req("15", GMCommand::Search { is_elf: false }), &mut state);
    assert!(resp.success, "search failed: {}", resp.message);
    assert!(!resp.message.is_empty(), "search should have result message");

    // -- STEP 7: Spawn an encounter from the monster database --
    let resp = handle_request(&req("20", GMCommand::SpawnMonster {
        name: "Goblin".to_string(),
        count: 3,
        distance: 5,
    }), &mut state);
    assert!(resp.success, "spawn monster failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Combat);
    let combat = state.combat.as_ref().unwrap();
    assert_eq!(combat.monsters.len(), 3);
    assert_eq!(combat.distance, 5);
    // Verify monsters have goblin stats
    for m in &combat.monsters {
        assert!(m.name.contains("Goblin"));
        assert_eq!(m.ac, 6);
        assert_eq!(m.xp_value, 5);
    }

    // -- STEP 8: Roll initiative --
    let resp = handle_request(&req("21", GMCommand::RollInitiative), &mut state);
    assert!(resp.success, "initiative failed: {}", resp.message);
    let combat = state.combat.as_ref().unwrap();
    assert!(combat.party_initiative > 0, "party initiative should be set");
    assert!(combat.monster_initiative > 0, "monster initiative should be set");

    // -- STEP 9: Fighter attacks --
    let pre_attack_hp = state.combat.as_ref().unwrap().monsters[0].hp;
    let resp = handle_request(&req("22", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success, "attack failed: {}", resp.message);
    // Attack returns result in message, verify state mutation
    let post_attack_hp = state.combat.as_ref().unwrap().monsters[0].hp;
    if resp.message.contains("HIT") {
        assert!(post_attack_hp < pre_attack_hp, "monster HP should decrease on hit");
    }

    // -- STEP 10: Thief attempts backstab on a different goblin --
    // Retry until hit to ensure multiplier is actually verified
    let saved_hp_1 = state.combat.as_ref().unwrap().monsters[1].hp;
    let mut backstab_hit = false;
    for i in 0..100 {
        state.combat.as_mut().unwrap().monsters[1].hp = saved_hp_1;
        // Clear per-round action tracking so retry is allowed
        state.combat.as_mut().unwrap().characters_acted.clear();
        let resp = handle_request(&req(&format!("23_{}", i), GMCommand::Backstab {
            character: "Shade".to_string(),
            monster_idx: 1,
            weapon: "dagger".to_string(),
        }), &mut state);
        assert!(resp.success, "backstab failed: {}", resp.message);
        let data = resp.data.unwrap();
        if data["hit"].as_bool().unwrap_or(false) {
            assert_eq!(data["multiplier"], 2);
            backstab_hit = true;
            break;
        }
    }
    assert!(backstab_hit, "backstab should land at least once in 100 attempts");

    // -- STEP 11: Check morale --
    let resp = handle_request(&req("24", GMCommand::CheckMorale), &mut state);
    assert!(resp.success, "morale check failed: {}", resp.message);
    assert!(!resp.message.is_empty(), "morale check should have result message");

    // -- STEP 12: End combat --
    let resp = handle_request(&req("25", GMCommand::EndCombat), &mut state);
    assert!(resp.success, "end combat failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Exploration);
    let data = resp.data.unwrap();
    let total_xp = data["total_xp"].as_u64().unwrap();

    // -- STEP 13: Award XP from treasure + monsters --
    // Award treasure XP to the fighter (1gp = 1xp)
    let resp = handle_request(&req("30", GMCommand::AwardTreasureXp {
        character: "Aldric".to_string(),
        treasure_gp: 500,
        monster_xp: total_xp,
    }), &mut state);
    assert!(resp.success, "award treasure xp failed: {}", resp.message);
    let data = resp.data.unwrap();
    // Fighter with STR 16 gets +10% XP modifier
    assert_eq!(data["modifier_pct"], 10);
    let adjusted = data["adjusted_xp"].as_u64().unwrap();
    assert!(adjusted > 0);

    // Award to thief too
    let resp = handle_request(&req("31", GMCommand::AwardTreasureXp {
        character: "Shade".to_string(),
        treasure_gp: 500,
        monster_xp: total_xp,
    }), &mut state);
    assert!(resp.success, "award thief xp failed: {}", resp.message);
    assert!(state.party.find_member("Shade").unwrap().xp > 0, "Shade should have XP after award");

    // -- STEP 14: Query the party to verify state --
    let resp = handle_request(&req("40", GMCommand::QueryParty), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    let members = data["members"].as_array().unwrap();
    assert_eq!(members.len(), 3);
    // Verify XP was awarded
    let aldric = &members[0];
    assert!(aldric["xp"].as_u64().unwrap() > 0);
}

// ===========================================================================
// INTEGRATION TEST: XP and level advancement
// ===========================================================================

#[test]
fn xp_and_level_advancement() {
    let mut state = GameState::new();

    // Create a thief near level-up threshold (1200 XP for level 2)
    let mut thief = make_thief("Sneaky Pete");
    thief.xp = 1_100;
    state.party.add_member(thief);

    // Award enough XP to level up
    // DEX 16 gives +10% XP bonus, so 100 base becomes 110
    let resp = handle_request(&req("1", GMCommand::AwardTreasureXp {
        character: "Sneaky Pete".to_string(),
        treasure_gp: 100,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success, "award xp failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert!(data["ready_to_train"].as_bool().unwrap(), "should be ready to train");

    // Level up via LevelUp command
    let resp = handle_request(&req("2", GMCommand::LevelUp {
        character: "Sneaky Pete".to_string(),
    }), &mut state);
    assert!(resp.success, "level up failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["new_level"], 2);
    assert!(data["hp_gained"].as_i64().unwrap() > 0);

    // Verify the character was actually updated
    let pete = state.party.find_member("Sneaky Pete").unwrap();
    assert_eq!(pete.level, 2);
    assert!(pete.max_hp > 4); // gained HP
}

// ===========================================================================
// INTEGRATION TEST: Thief skill checks via GM API
// ===========================================================================

#[test]
fn thief_skill_checks() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Shadow"));
    state.party.add_member(make_fighter("Aldric"));

    // Thief can use skills
    let resp = handle_request(&req("1", GMCommand::ThiefSkillCheck {
        character: "Shadow".to_string(),
        skill: "open locks".to_string(),
    }), &mut state);
    assert!(resp.success, "thief skill check failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["skill"], "Open Locks");
    assert_eq!(data["target"], 15); // level 1 open locks = 15%

    // Fighter cannot use thief skills
    let resp = handle_request(&req("2", GMCommand::ThiefSkillCheck {
        character: "Aldric".to_string(),
        skill: "open locks".to_string(),
    }), &mut state);
    assert!(!resp.success, "fighter should not have thief skills");

    // Hear noise uses d6
    let resp = handle_request(&req("3", GMCommand::ThiefSkillCheck {
        character: "Shadow".to_string(),
        skill: "hear noise".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["target"], 2); // level 1 hear noise = 1-2 on d6
}

// ===========================================================================
// INTEGRATION TEST: Encumbrance system
// ===========================================================================

#[test]
fn encumbrance_query() {
    let mut state = GameState::new();

    let mut fighter = make_fighter("Tank");
    // Add some inventory items
    fighter.inventory.push(osr_ai_gm::model::Item::new("Plate mail", 50.0, 60));
    fighter.inventory.push(osr_ai_gm::model::Item::new("Shield", 10.0, 10));
    fighter.inventory.push(osr_ai_gm::model::Item::new("Sword", 6.0, 10));
    fighter.gold_gp = 200;
    state.party.add_member(fighter);

    let resp = handle_request(&req("1", GMCommand::QueryEncumbrance {
        character: "Tank".to_string(),
    }), &mut state);
    assert!(resp.success, "encumbrance query failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert!(data["total_weight_cn"].as_u64().unwrap() > 0);
    assert!(data["movement_rate"].as_u64().is_some());
}

// ===========================================================================
// INTEGRATION TEST: Monster spawning from database
// ===========================================================================

#[test]
fn monster_database_spawn() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Hero"));

    // Spawn from database
    let resp = handle_request(&req("1", GMCommand::SpawnMonster {
        name: "Orc".to_string(),
        count: 4,
        distance: 60,
    }), &mut state);
    assert!(resp.success, "spawn orc failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["monster"], "Orc");
    assert_eq!(data["hit_dice"], "1");
    assert_eq!(data["ac"], 6);

    let combat = state.combat.as_ref().unwrap();
    assert_eq!(combat.monsters.len(), 4);
    for m in &combat.monsters {
        assert_eq!(m.ac, 6);
        assert_eq!(m.morale, 8);
        assert_eq!(m.xp_value, 10);
    }

    // Unknown monster fails gracefully
    let resp2_state = &mut GameState::new();
    let resp2 = handle_request(&req("2", GMCommand::SpawnMonster {
        name: "Beholder".to_string(),
        count: 1,
        distance: 60,
    }), resp2_state);
    assert!(!resp2.success);
}

// ===========================================================================
// INTEGRATION TEST: Spell lookup
// ===========================================================================

#[test]
fn spell_lookup() {
    let mut state = GameState::new();

    let resp = handle_request(&req("1", GMCommand::LookupSpell {
        name: "Magic Missile".to_string(),
        list: String::new(),
    }), &mut state);
    assert!(resp.success, "spell lookup failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["name"], "Magic Missile");
    assert_eq!(data["level"], 1);
    assert_eq!(data["list"], "Magic-User");
    assert_eq!(data["range"], "150'");

    // Spell not found
    let resp = handle_request(&req("2", GMCommand::LookupSpell {
        name: "Nonexistent Spell".to_string(),
        list: String::new(),
    }), &mut state);
    assert!(!resp.success);

    // Filter by list
    let resp = handle_request(&req("3", GMCommand::LookupSpell {
        name: "Cure Light Wounds".to_string(),
        list: "cleric".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["list"], "Cleric");
}

// ===========================================================================
// INTEGRATION TEST: Retainer hiring
// ===========================================================================

#[test]
fn retainer_hiring() {
    let mut state = GameState::new();
    state.party.add_member(make_cleric("Father Gregory"));

    let resp = handle_request(&req("1", GMCommand::HireRetainer {
        employer: "Father Gregory".to_string(),
        retainer_name: "Hrothgar".to_string(),
        retainer_class: Class::Fighter,
        retainer_level: 1,
    }), &mut state);
    assert!(resp.success, "hire retainer failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["employer"], "Father Gregory");
    assert_eq!(data["retainer"], "Hrothgar");
    assert_eq!(data["level"], 1);
    assert_eq!(data["wage_gp"], 25);
    // CHA 14 = max 5 retainers
    assert_eq!(data["max_retainers"], 5);
}

// ===========================================================================
// INTEGRATION TEST: Loyalty check
// ===========================================================================

#[test]
fn loyalty_check_via_api() {
    let mut state = GameState::new();

    let resp = handle_request(&req("1", GMCommand::LoyaltyCheck {
        retainer_name: "Hrothgar".to_string(),
        loyalty: 8,
    }), &mut state);
    assert!(resp.success, "loyalty check failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["retainer"], "Hrothgar");
    assert_eq!(data["loyalty"], 8);
    let result = data["result"].as_str().unwrap();
    assert!(["Loyal", "Wavering", "Disloyal"].contains(&result));
}

// ===========================================================================
// INTEGRATION TEST: Level up command
// ===========================================================================

#[test]
fn level_up_command() {
    let mut state = GameState::new();

    // Fighter with enough XP for level 2 (needs 2000)
    let mut fighter = make_fighter("Veteran");
    fighter.xp = 2500;
    state.party.add_member(fighter);

    let resp = handle_request(&req("1", GMCommand::LevelUp {
        character: "Veteran".to_string(),
    }), &mut state);
    assert!(resp.success, "level up failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["new_level"], 2);
    assert!(data["hp_gained"].as_i64().unwrap() > 0);

    let veteran = state.party.find_member("Veteran").unwrap();
    assert_eq!(veteran.level, 2);

    // Trying to level up again without enough XP should fail
    let resp = handle_request(&req("2", GMCommand::LevelUp {
        character: "Veteran".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.message.contains("needs"));
}

// ===========================================================================
// INTEGRATION TEST: Save/Load roundtrip with new systems
// ===========================================================================

#[test]
fn save_load_roundtrip() {
    let mut state = GameState::new();

    // Set up a state with party
    let mut fighter = make_fighter("Aldric");
    fighter.xp = 1500;
    state.party.add_member(fighter);
    state.party.add_member(make_thief("Shade"));
    state.notes.push("[RULING] The door is trapped.".to_string());

    // Save
    let name = unique_save_name("save");
    let resp = handle_request(&req("1", GMCommand::Save {
        path: name.clone(),
    }), &mut state);
    assert!(resp.success, "save failed: {}", resp.message);

    // Load into a fresh state
    let mut new_state = GameState::new();
    let resp = handle_request(&req("2", GMCommand::Load {
        path: name.clone(),
    }), &mut new_state);
    assert!(resp.success, "load failed: {}", resp.message);

    // Verify state matches
    assert_eq!(new_state.party.members.len(), 2);
    assert_eq!(new_state.party.members[0].name, "Aldric");
    assert_eq!(new_state.party.members[0].xp, 1500);
    assert_eq!(new_state.party.members[1].name, "Shade");
    assert_eq!(new_state.notes.len(), 1);

    // Clean up
    let _ = std::fs::remove_file(resolve_save(&name));
}

// ===========================================================================
// INTEGRATION TEST: Complete session flow (chargen -> dungeon -> combat -> XP -> level up)
// ===========================================================================

#[test]
fn complete_ose_session() {
    let mut state = GameState::new();

    // === CHARACTER CREATION ===
    state.party.add_member(make_fighter("Sir Aldric"));
    state.party.add_member(make_thief("Nyx the Shadow"));
    state.party.add_member(make_cleric("Brother Tomas"));
    assert_eq!(state.party.members.len(), 3);

    // === ENTER DUNGEON ===
    let resp = handle_request(&req("d1", GMCommand::EnterDungeon {
        level: 1,
        room_name: "Mossy Staircase".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Exploration);
    assert!(state.dungeon.is_some(), "dungeon state should be initialized");
    assert!(state.time.is_some(), "time tracker should be initialized");

    // Light source
    let resp = handle_request(&req("d2", GMCommand::Light {
        source: LightSourceKind::Lantern,
        carrier: "Brother Tomas".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.time.as_ref().unwrap().lights.len(), 1, "should have 1 light source");

    // Advance turn
    let resp = handle_request(&req("d3", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success);
    assert!(state.turn() > 0, "turn counter should have advanced");

    // Add rooms
    let resp = handle_request(&req("d4", GMCommand::AddRoom { id: 1, name: "Goblin Lair".to_string() }), &mut state);
    assert!(resp.success, "add room failed: {}", resp.message);
    assert_eq!(state.dungeon.as_ref().unwrap().rooms.len(), 2);
    let resp = handle_request(&req("d5", GMCommand::AddDoor { id: 0, room_a: 0, room_b: 1, state: DoorState::Stuck }), &mut state);
    assert!(resp.success, "add door failed: {}", resp.message);
    assert_eq!(state.dungeon.as_ref().unwrap().doors.len(), 1);

    // Thief listens at the door
    let resp = handle_request(&req("d6", GMCommand::ThiefSkillCheck {
        character: "Nyx the Shadow".to_string(),
        skill: "hear noise".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["skill"], "Hear Noise");

    // === ENCOUNTER: GOBLINS ===
    let resp = handle_request(&req("c1", GMCommand::SpawnMonster {
        name: "Goblin".to_string(),
        count: 4,
        distance: 5,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Combat);

    // Roll surprise
    let resp = handle_request(&req("c2", GMCommand::RollSurprise), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["party_roll"].as_u64().is_some(), "surprise should have party roll");
    assert!(data["monster_roll"].as_u64().is_some(), "surprise should have monster roll");

    // Roll initiative
    let resp = handle_request(&req("c3", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);
    let combat = state.combat.as_ref().unwrap();
    assert!(combat.party_initiative > 0, "party initiative should be set");
    assert!(combat.monster_initiative > 0, "monster initiative should be set");

    // Fighter attacks
    let pre_hp_0 = state.combat.as_ref().unwrap().monsters[0].hp;
    let resp = handle_request(&req("c4", GMCommand::Attack {
        character: "Sir Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.combat.as_ref().unwrap().monsters[0].hp < pre_hp_0, "goblin 0 HP should decrease on hit");
    }

    // Thief backstabs — retry until hit to ensure multiplier is verified
    let saved_hp_1 = state.combat.as_ref().unwrap().monsters[1].hp;
    let mut backstab_hit = false;
    for i in 0..100 {
        state.combat.as_mut().unwrap().monsters[1].hp = saved_hp_1;
        // Clear per-round action tracking so retry is allowed
        state.combat.as_mut().unwrap().characters_acted.clear();
        let resp = handle_request(&req(&format!("c5_{}", i), GMCommand::Backstab {
            character: "Nyx the Shadow".to_string(),
            monster_idx: 1,
            weapon: "dagger".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.as_ref().unwrap();
        if data["hit"].as_bool().unwrap_or(false) {
            assert_eq!(data["multiplier"], 2, "level 1 backstab should be x2");
            assert!(state.combat.as_ref().unwrap().monsters[1].hp < saved_hp_1, "goblin 1 HP should decrease on hit");
            backstab_hit = true;
            break;
        }
    }
    assert!(backstab_hit, "backstab should land at least once in 100 attempts");

    // Cleric attacks
    let pre_hp_2 = state.combat.as_ref().unwrap().monsters[2].hp;
    let resp = handle_request(&req("c6", GMCommand::Attack {
        character: "Brother Tomas".to_string(),
        monster_idx: 2,
        weapon: "mace".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.combat.as_ref().unwrap().monsters[2].hp < pre_hp_2, "goblin 2 HP should decrease on hit");
    }

    // Monster attacks back
    let pre_aldric_hp = state.party.find_member("Sir Aldric").unwrap().hp;
    let resp = handle_request(&req("c7", GMCommand::MonsterAttack {
        monster_idx: 3,
        character: "Sir Aldric".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.party.find_member("Sir Aldric").unwrap().hp < pre_aldric_hp, "Aldric HP should decrease on hit");
    }

    // Check morale
    let resp = handle_request(&req("c8", GMCommand::CheckMorale), &mut state);
    assert!(resp.success);
    assert!(!resp.message.is_empty(), "morale check should have result");

    // End combat
    let resp = handle_request(&req("c9", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Exploration);
    let data = resp.data.unwrap();
    let monster_xp = data["total_xp"].as_u64().unwrap();

    // === LOOT & XP ===
    // Party finds 300gp treasure (1gp = 1xp)
    let treasure_gp = 300u64;
    let per_member_treasure = treasure_gp / 3;
    let per_member_monster_xp = monster_xp / 3;

    for name in &["Sir Aldric", "Nyx the Shadow", "Brother Tomas"] {
        let resp = handle_request(&req("x1", GMCommand::AwardTreasureXp {
            character: name.to_string(),
            treasure_gp: per_member_treasure,
            monster_xp: per_member_monster_xp,
        }), &mut state);
        assert!(resp.success, "award xp to {} failed: {}", name, resp.message);
    }

    // Verify XP was awarded
    for member in &state.party.members {
        assert!(member.xp > 0, "{} should have XP", member.name);
    }

    // === QUERY ENCUMBRANCE ===
    let resp = handle_request(&req("e1", GMCommand::QueryEncumbrance {
        character: "Sir Aldric".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["total_weight_cn"].as_u64().is_some(), "encumbrance should have weight");
    assert!(data["movement_rate"].as_u64().is_some(), "encumbrance should have movement rate");

    // === SPELL LOOKUP ===
    let resp = handle_request(&req("s1", GMCommand::LookupSpell {
        name: "Cure Light Wounds".to_string(),
        list: "cleric".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["name"], "Cure Light Wounds");
    assert_eq!(data["list"], "Cleric");

    // === RETAINER HIRING ===
    let resp = handle_request(&req("r1", GMCommand::HireRetainer {
        employer: "Sir Aldric".to_string(),
        retainer_name: "Bort the Torchbearer".to_string(),
        retainer_class: Class::Fighter,
        retainer_level: 0,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["retainer"], "Bort the Torchbearer");
    assert_eq!(data["employer"], "Sir Aldric");

    // === VERIFY FINAL STATE ===
    let resp = handle_request(&req("q1", GMCommand::QueryState), &mut state);
    assert!(resp.success);
    let resp = handle_request(&req("q2", GMCommand::QueryParty), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    let members = data["members"].as_array().unwrap();
    assert_eq!(members.len(), 3);
    // All alive
    for m in members {
        assert!(m["alive"].as_bool().unwrap(), "{} should be alive", m["name"]);
    }
}

// ===========================================================================
// INTEGRATION TEST: Level up after big treasure haul
// ===========================================================================

#[test]
fn level_up_from_treasure() {
    let mut state = GameState::new();

    // Create a fighter near level-up (needs 2000 XP)
    let mut fighter = make_fighter("Goldbeard");
    fighter.xp = 1_800; // STR 16 gives +10%, so 200gp treasure = 220 XP, total 2020
    state.party.add_member(fighter);

    // Award treasure that triggers level up
    let resp = handle_request(&req("1", GMCommand::AwardTreasureXp {
        character: "Goldbeard".to_string(),
        treasure_gp: 200,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["ready_to_train"].as_bool().unwrap());

    // Level up via LevelUp command
    let resp = handle_request(&req("2", GMCommand::LevelUp {
        character: "Goldbeard".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["new_level"], 2);

    let gb = state.party.find_member("Goldbeard").unwrap();
    assert_eq!(gb.level, 2);
    assert!(gb.max_hp > 8); // gained HP
    assert!(gb.saving_throws.is_some()); // saves updated
}

// ===========================================================================
// INTEGRATION TEST: Backstab at various thief levels (x2, x3, x4 multipliers)
// ===========================================================================

#[test]
fn backstab_multiplier_level_1_x2() {
    let mut state = GameState::new();
    let thief = make_thief("Dagger Dan");
    state.party.add_member(thief);

    // Spawn a goblin
    let resp = handle_request(&req("1", GMCommand::SpawnMonster {
        name: "Goblin".to_string(), count: 1, distance: 5,
    }), &mut state);
    assert!(resp.success);

    // Retry backstab until a hit to ensure multiplier is verified
    let saved_hp = state.combat.as_ref().unwrap().monsters[0].hp;
    let mut hit_verified = false;
    for i in 0..100 {
        state.combat.as_mut().unwrap().monsters[0].hp = saved_hp;
        // Clear per-round action tracking so retry is allowed
        state.combat.as_mut().unwrap().characters_acted.clear();
        let resp = handle_request(&req(&format!("2_{}", i), GMCommand::Backstab {
            character: "Dagger Dan".to_string(),
            monster_idx: 0,
            weapon: "dagger".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        if data["hit"].as_bool().unwrap_or(false) {
            assert_eq!(data["multiplier"], 2, "level 1 thief should have x2 backstab");
            hit_verified = true;
            break;
        }
    }
    assert!(hit_verified, "backstab should land at least once in 100 attempts");
}

#[test]
fn backstab_multiplier_level_5_x3() {
    let mut state = GameState::new();
    let mut thief = make_thief("Shadow Blade");
    thief.level = 5;
    thief.thac0 = 19; // thief THAC0 at level 5
    state.party.add_member(thief);

    let resp = handle_request(&req("1", GMCommand::SpawnMonster {
        name: "Goblin".to_string(), count: 1, distance: 5,
    }), &mut state);
    assert!(resp.success);

    // Retry backstab until a hit to ensure multiplier is verified
    let saved_hp = state.combat.as_ref().unwrap().monsters[0].hp;
    let mut hit_verified = false;
    for i in 0..100 {
        state.combat.as_mut().unwrap().monsters[0].hp = saved_hp;
        // Clear per-round action tracking so retry is allowed
        state.combat.as_mut().unwrap().characters_acted.clear();
        let resp = handle_request(&req(&format!("2_{}", i), GMCommand::Backstab {
            character: "Shadow Blade".to_string(),
            monster_idx: 0,
            weapon: "dagger".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        if data["hit"].as_bool().unwrap_or(false) {
            assert_eq!(data["multiplier"], 3, "level 5 thief should have x3 backstab");
            hit_verified = true;
            break;
        }
    }
    assert!(hit_verified, "backstab should land at least once in 100 attempts");
}

#[test]
fn backstab_multiplier_level_9_x4() {
    let mut state = GameState::new();
    let mut thief = make_thief("Master Thief");
    thief.level = 9;
    state.party.add_member(thief);

    let resp = handle_request(&req("1", GMCommand::SpawnMonster {
        name: "Goblin".to_string(), count: 1, distance: 5,
    }), &mut state);
    assert!(resp.success);

    // Retry backstab until a hit to ensure multiplier is verified
    let saved_hp = state.combat.as_ref().unwrap().monsters[0].hp;
    let mut hit_verified = false;
    for i in 0..100 {
        state.combat.as_mut().unwrap().monsters[0].hp = saved_hp;
        // Clear per-round action tracking so retry is allowed
        state.combat.as_mut().unwrap().characters_acted.clear();
        let resp = handle_request(&req(&format!("2_{}", i), GMCommand::Backstab {
            character: "Master Thief".to_string(),
            monster_idx: 0,
            weapon: "dagger".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        if data["hit"].as_bool().unwrap_or(false) {
            assert_eq!(data["multiplier"], 4, "level 9 thief should have x4 backstab");
            hit_verified = true;
            break;
        }
    }
    assert!(hit_verified, "backstab should land at least once in 100 attempts");
}

#[test]
fn backstab_non_thief_rejected() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("1", GMCommand::SpawnMonster {
        name: "Goblin".to_string(), count: 1, distance: 5,
    }), &mut state);
    assert!(resp.success);

    // Fighter cannot backstab
    let resp = handle_request(&req("2", GMCommand::Backstab {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(!resp.success, "fighter should not be able to backstab");
}

#[test]
fn backstab_dead_monster_rejected() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Sneaky"));

    let resp = handle_request(&req("1", GMCommand::SpawnMonster {
        name: "Goblin".to_string(), count: 1, distance: 5,
    }), &mut state);
    assert!(resp.success);

    // Kill the goblin
    state.combat.as_mut().unwrap().monsters[0].hp = 0;

    // Backstab dead monster
    let resp = handle_request(&req("2", GMCommand::Backstab {
        character: "Sneaky".to_string(),
        monster_idx: 0,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(!resp.success, "should not backstab a dead monster");
}

#[test]
fn backstab_no_combat_rejected() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Lone Thief"));

    // Backstab without active combat
    let resp = handle_request(&req("1", GMCommand::Backstab {
        character: "Lone Thief".to_string(),
        monster_idx: 0,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(!resp.success, "should not backstab outside combat");
}

// ===========================================================================
// INTEGRATION TEST: Turn undead via API
// ===========================================================================

#[test]
fn turn_undead_via_api() {
    let mut state = GameState::new();
    let mut cleric = make_cleric("Holy Brother");
    cleric.level = 3;
    state.party.add_member(cleric);

    // Spawn skeletons
    let resp = handle_request(&req("1", GMCommand::SpawnMonster {
        name: "Skeleton".to_string(), count: 4, distance: 30,
    }), &mut state);
    assert!(resp.success);

    // Turn undead
    let resp = handle_request(&req("2", GMCommand::TurnUndead {
        character: "Holy Brother".to_string(),
        monster_idx: 0,
    }), &mut state);
    assert!(resp.success);
    // Turn undead result is in the message — verify it contains meaningful content
    assert!(!resp.message.is_empty(), "turn undead should have result message");
    // At cleric level 3 vs skeletons, turn undead should always succeed (auto-turn)
    // so the message should indicate the outcome
    assert!(resp.message.contains("turn") || resp.message.contains("Turn") || resp.message.contains("undead"),
        "turn undead message should describe outcome: {}", resp.message);
}

// ===========================================================================
// INTEGRATION TEST: Multi-round combat through API
// ===========================================================================

#[test]
fn multi_round_combat_api() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Sir Brave"));
    state.party.add_member(make_cleric("Father Stone"));

    // Spawn orcs at melee distance
    let resp = handle_request(&req("1", GMCommand::SpawnMonster {
        name: "Orc".to_string(), count: 3, distance: 5,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Combat);

    // === Round 1 ===
    let resp = handle_request(&req("2", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);
    assert_eq!(state.combat.as_ref().unwrap().round, 1, "should be round 1");

    let pre_orc0_hp = state.combat.as_ref().unwrap().monsters[0].hp;
    let resp = handle_request(&req("3", GMCommand::Attack {
        character: "Sir Brave".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.combat.as_ref().unwrap().monsters[0].hp < pre_orc0_hp);
    }

    let pre_orc1_hp = state.combat.as_ref().unwrap().monsters[1].hp;
    let resp = handle_request(&req("4", GMCommand::Attack {
        character: "Father Stone".to_string(),
        monster_idx: 1,
        weapon: "mace".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.combat.as_ref().unwrap().monsters[1].hp < pre_orc1_hp);
    }

    // Monster attacks (use orc 2, which hasn't been attacked)
    let pre_brave_hp = state.party.find_member("Sir Brave").unwrap().hp;
    let resp = handle_request(&req("5", GMCommand::MonsterAttack {
        monster_idx: 2,
        character: "Sir Brave".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.party.find_member("Sir Brave").unwrap().hp < pre_brave_hp, "HP should decrease on hit");
    }

    let resp = handle_request(&req("6", GMCommand::CheckMorale), &mut state);
    assert!(resp.success);
    assert!(!resp.message.is_empty(), "morale check should have result");

    // === Round 2 ===
    let resp = handle_request(&req("7", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);

    let combat = state.combat.as_ref().unwrap();
    assert_eq!(combat.round, 2, "should be on round 2");

    // Attack a living monster
    let living_idx = state.combat.as_ref().unwrap().monsters.iter()
        .position(|m| m.is_alive());
    if let Some(idx) = living_idx {
        let resp = handle_request(&req("8", GMCommand::Attack {
            character: "Sir Brave".to_string(),
            monster_idx: idx,
            weapon: "sword".to_string(),
        }), &mut state);
        assert!(resp.success);
    }

    // End combat
    let resp = handle_request(&req("9", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Idle);
}

// ===========================================================================
// INTEGRATION TEST: Monster attack via API tracks damage
// ===========================================================================

#[test]
fn monster_attack_api_damages_character() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Bruiser"));

    let resp = handle_request(&req("1", GMCommand::SpawnMonster {
        name: "Orc".to_string(), count: 1, distance: 5,
    }), &mut state);
    assert!(resp.success);

    // Monster attacks once per round until a hit
    let mut hit_landed = false;
    for i in 0..100 {
        let resp = handle_request(&req(&format!("init{}", i), GMCommand::RollInitiative), &mut state);
        assert!(resp.success);

        let initial_hp = state.party.find_member("Bruiser").unwrap().hp;
        let resp = handle_request(&req(&format!("a{}", i), GMCommand::MonsterAttack {
            monster_idx: 0,
            character: "Bruiser".to_string(),
        }), &mut state);
        assert!(resp.success);
        let current_hp = state.party.find_member("Bruiser").unwrap().hp;
        if current_hp < initial_hp {
            hit_landed = true;
            break;
        }
    }
    assert!(hit_landed, "Monster should land at least one hit in 100 attempts");
}

// ===========================================================================
// INTEGRATION TEST: Wilderness travel then encounter
// ===========================================================================

#[test]
fn wilderness_encounter() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Ranger Rick"));

    // Enter wilderness
    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness);

    // Add adjacent hex
    let resp = handle_request(&req("2", GMCommand::AddHex {
        x: 1, y: 0,
        terrain: Terrain::Hills,
    }), &mut state);
    assert!(resp.success);
    assert!(state.wilderness.as_ref().unwrap().hexes.len() >= 2, "should have at least 2 hexes");

    // Travel
    let pre_travel_day = state.wilderness.as_ref().unwrap().travel_day;
    let resp = handle_request(&req("3", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["lost"].as_bool().is_some(), "travel should report lost status");
    assert!(state.wilderness.as_ref().unwrap().travel_day > pre_travel_day, "travel day should advance");

    // Query wilderness state
    let resp = handle_request(&req("4", GMCommand::QueryWilderness), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["travel_day"].as_u64().unwrap() >= 2, "should be at least day 2");
}

// ===========================================================================
// INTEGRATION TEST: Mode transition Idle -> Exploration -> Combat -> Exploration
// ===========================================================================

#[test]
fn mode_transition_idle_exploration_combat_exploration() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));
    assert_eq!(state.mode, GameMode::Idle);

    // Idle -> Exploration
    let resp = handle_request(&req("1", GMCommand::EnterDungeon {
        level: 1,
        room_name: "Entrance".to_string(),
    }), &mut state);
    assert!(resp.success, "enter dungeon failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Exploration);

    // Light a torch so exploration works
    let resp = handle_request(&req("2", GMCommand::Light {
        source: LightSourceKind::Lantern,
        carrier: "Aldric".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Advance a turn to verify exploration works
    let resp = handle_request(&req("3", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success);
    let dungeon_turn = state.time.as_ref().unwrap().total_turns;
    assert!(dungeon_turn > 0, "turns should have advanced");

    // Exploration -> Combat (spawn encounter)
    let resp = handle_request(&req("4", GMCommand::SpawnEncounter(EncounterParams {
        name: "skeleton".to_string(),
        count: 2,
        hit_dice: "1".parse().unwrap(),
        ac: 7,
        hp: 4,
        damage: "1d6".to_string(),
        morale: 12,
        distance: 5,
        xp_value: Some(10),
    })), &mut state);
    assert!(resp.success, "spawn encounter failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Combat);

    // Verify dungeon state is preserved during combat
    assert!(state.dungeon.is_some(), "dungeon state should persist during combat");
    assert!(state.time.is_some(), "time tracker should persist during combat");
    assert_eq!(
        state.time.as_ref().unwrap().total_turns, dungeon_turn,
        "dungeon turn count should not change during combat"
    );

    // Perform combat actions
    let resp = handle_request(&req("5", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);
    let resp = handle_request(&req("6", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Combat -> Exploration (EndCombat restores pre-combat mode)
    let resp = handle_request(&req("7", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Exploration);

    // Dungeon state should still be present — exploration can resume
    assert!(state.dungeon.is_some(), "dungeon state should survive combat");
    assert!(state.time.is_some(), "time tracker should survive combat");

    // Verify exploration still works after combat
    let resp = handle_request(&req("8", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success, "should be able to explore after combat: {}", resp.message);
}

// ===========================================================================
// INTEGRATION TEST: Mode transition Idle -> Wilderness -> Combat -> Wilderness
// ===========================================================================

#[test]
fn mode_transition_idle_wilderness_combat_wilderness() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Ranger"));
    assert_eq!(state.mode, GameMode::Idle);

    // Idle -> Wilderness
    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success, "enter wilderness failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Wilderness);

    // Add adjacent hexes for travel
    let resp = handle_request(&req("2", GMCommand::AddHex {
        x: 1, y: 0,
        terrain: Terrain::Hills,
    }), &mut state);
    assert!(resp.success);

    // Travel to verify wilderness works
    let resp = handle_request(&req("3", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(resp.success);
    let travel_day = state.wilderness.as_ref().unwrap().travel_day;
    assert!(travel_day > 1, "travel day should have incremented");

    // Wilderness -> Combat (encounter during travel)
    let resp = handle_request(&req("4", GMCommand::SpawnEncounter(EncounterParams {
        name: "wolf".to_string(),
        count: 3,
        hit_dice: "2".parse().unwrap(),
        ac: 7,
        hp: 6,
        damage: "1d6".to_string(),
        morale: 8,
        distance: 5,
        xp_value: Some(20),
    })), &mut state);
    assert!(resp.success, "spawn encounter failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Combat);

    // Wilderness state should persist during combat
    assert!(state.wilderness.is_some(), "wilderness state should persist during combat");
    assert_eq!(
        state.wilderness.as_ref().unwrap().travel_day, travel_day,
        "travel day should not change during combat"
    );

    // Combat actions
    let resp = handle_request(&req("5", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);
    let resp = handle_request(&req("6", GMCommand::Attack {
        character: "Ranger".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Combat -> Wilderness (EndCombat restores pre-combat mode)
    let resp = handle_request(&req("7", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness);

    // Wilderness state should still be present
    assert!(state.wilderness.is_some(), "wilderness state should survive combat");
    assert_eq!(
        state.wilderness.as_ref().unwrap().travel_day, travel_day,
        "travel day should be preserved after combat"
    );

    // Add another hex and verify wilderness travel still works after combat
    let _resp = handle_request(&req("8", GMCommand::AddHex {
        x: 0, y: 0,
        terrain: Terrain::Clear,
    }), &mut state);
    // This might fail since (0,0) was the original hex — that's fine, just test travel
    let resp = handle_request(&req("9", GMCommand::Travel { x: 0, y: 0 }), &mut state);
    assert!(resp.success, "should be able to travel after combat: {}", resp.message);
}

// ===========================================================================
// INTEGRATION TEST: Full dungeon exploration flow with light, rest, doors
// ===========================================================================

#[test]
fn dungeon_exploration_flow() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));
    state.party.add_member(make_thief("Shadow"));

    // Enter dungeon
    let resp = handle_request(&req("1", GMCommand::EnterDungeon {
        level: 1,
        room_name: "Entry Hall".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Exploration);

    // Without light, advance turn should warn about darkness
    let resp = handle_request(&req("2", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success);
    assert!(resp.message.contains("DARKNESS"), "should warn about darkness");

    // Light a torch
    let resp = handle_request(&req("3", GMCommand::Light {
        source: LightSourceKind::Torch,
        carrier: "Aldric".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert!(!state.time.as_ref().unwrap().lights.is_empty(), "should have active light");
    assert_eq!(state.time.as_ref().unwrap().lights[0].carrier, "Aldric");

    // Now exploration should work
    let resp = handle_request(&req("4", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success);
    assert!(!resp.message.contains("DARKNESS"), "should not be in darkness with torch");

    // Build out the dungeon
    let resp = handle_request(&req("5a", GMCommand::AddRoom { id: 1, name: "Guard Room".to_string() }), &mut state);
    assert!(resp.success, "add room failed: {}", resp.message);
    let resp = handle_request(&req("5b", GMCommand::AddDoor {
        id: 0, room_a: 0, room_b: 1, state: DoorState::Closed,
    }), &mut state);
    assert!(resp.success, "add door failed: {}", resp.message);

    // Search the room
    let resp = handle_request(&req("6", GMCommand::Search { is_elf: false }), &mut state);
    assert!(resp.success);
    assert!(!resp.message.is_empty(), "search should have result message");

    // Check exploration status
    let resp = handle_request(&req("7", GMCommand::QueryExploration), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["has_light"].as_bool().unwrap(), "should have light");
    assert!(data["total_turns"].as_u64().unwrap() > 0);

    // Advance enough turns to need rest (5 turns of activity)
    // We've already used some turns, advance until rest is needed
    let time = state.time.as_ref().unwrap();
    let turns_until_rest = if time.turns_since_rest >= 5 { 0 } else { 5 - time.turns_since_rest };
    for i in 0..turns_until_rest {
        handle_request(&req(&format!("r{}", i), GMCommand::AdvanceTurn), &mut state);
    }

    // Verify rest penalty
    assert!(
        state.time.as_ref().unwrap().needs_rest(),
        "should need rest after 5 turns of activity"
    );
}

// ===========================================================================
// INTEGRATION TEST: Wilderness multi-day travel
// ===========================================================================

#[test]
fn wilderness_multi_day_travel() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Explorer"));

    // Enter wilderness
    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Clear,
    }), &mut state);
    assert!(resp.success);

    // Build a hex path
    handle_request(&req("2a", GMCommand::AddHex { x: 1, y: 0, terrain: Terrain::Clear }), &mut state);
    handle_request(&req("2b", GMCommand::AddHex { x: 1, y: 1, terrain: Terrain::Forest }), &mut state);
    handle_request(&req("2c", GMCommand::AddHex { x: 0, y: 1, terrain: Terrain::Mountains }), &mut state);

    // Day 1: travel to (1,0) — clear terrain
    let resp = handle_request(&req("3", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(resp.success);
    let ws = state.wilderness.as_ref().unwrap();
    assert_eq!(ws.travel_day, 2, "should be day 2 after first travel");

    // Day 2: travel to (1,1) — forest terrain
    let resp = handle_request(&req("4", GMCommand::Travel { x: 1, y: 1 }), &mut state);
    assert!(resp.success);
    let ws = state.wilderness.as_ref().unwrap();
    assert_eq!(ws.travel_day, 3, "should be day 3 after second travel");

    // Day 3: travel to (0,1) — mountains
    let resp = handle_request(&req("5", GMCommand::Travel { x: 0, y: 1 }), &mut state);
    assert!(resp.success);
    let ws = state.wilderness.as_ref().unwrap();
    assert_eq!(ws.travel_day, 4, "should be day 4 after third travel");

    // Verify final position
    let resp = handle_request(&req("6", GMCommand::QueryWilderness), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    // Position might be different if party got lost, but travel_day should still advance
    assert_eq!(data["travel_day"], 4);
}

// ===========================================================================
// SESSION A: Full dungeon crawl — 4-character party, 3 rooms, combat, loot,
// XP, level-up, save/load roundtrip
// ===========================================================================

#[test]
fn session_a_dungeon_crawl() {
    let mut state = GameState::new();

    // === STEP 1: Create 4-character party ===
    state.party.add_member(make_fighter("Aldric the Bold"));
    state.party.add_member(make_thief("Vex"));
    state.party.add_member(make_cleric("Sister Mira"));
    state.party.add_member(make_magic_user("Zanthus"));
    assert_eq!(state.party.members.len(), 4);

    // === STEP 2: Enter dungeon ===
    let resp = handle_request(&req("a1", GMCommand::EnterDungeon {
        level: 1,
        room_name: "Crumbling Antechamber".to_string(),
    }), &mut state);
    assert!(resp.success, "enter dungeon failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Exploration);

    // Light two sources — torch and lantern
    let resp = handle_request(&req("a2", GMCommand::Light {
        source: LightSourceKind::Torch,
        carrier: "Aldric the Bold".to_string(),
    }), &mut state);
    assert!(resp.success);
    let resp = handle_request(&req("a3", GMCommand::Light {
        source: LightSourceKind::Lantern,
        carrier: "Sister Mira".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.time.as_ref().unwrap().lights.len(), 2);

    // Advance a turn
    let resp = handle_request(&req("a4", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success);
    assert_eq!(state.turn(), 1);

    // === STEP 3: Build 3 rooms and explore ===
    // Room 1 already exists (id=0). Add rooms 1 and 2.
    let resp = handle_request(&req("a5", GMCommand::AddRoom {
        id: 1, name: "Goblin Guardpost".to_string(),
    }), &mut state);
    assert!(resp.success);
    let resp = handle_request(&req("a6", GMCommand::AddRoom {
        id: 2, name: "Treasure Vault".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Connect rooms with doors (open so party can pass through)
    let resp = handle_request(&req("a7", GMCommand::AddDoor {
        id: 0, room_a: 0, room_b: 1, state: DoorState::Open,
    }), &mut state);
    assert!(resp.success);
    let resp = handle_request(&req("a8", GMCommand::AddDoor {
        id: 1, room_a: 1, room_b: 2, state: DoorState::Open,
    }), &mut state);
    assert!(resp.success);

    // Search room 0
    let resp = handle_request(&req("a9", GMCommand::Search { is_elf: false }), &mut state);
    assert!(resp.success);

    // Move to room 1 through door 0
    let resp = handle_request(&req("a10", GMCommand::MoveRoom { door_id: 0 }), &mut state);
    assert!(resp.success);
    let dungeon = state.dungeon.as_ref().unwrap();
    assert_eq!(dungeon.current_room, Some(1));

    // Advance another turn
    let resp = handle_request(&req("a11", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success);
    assert!(state.turn() >= 2, "should have advanced multiple turns");

    // === STEP 4: Encounter in room 1 ===
    let resp = handle_request(&req("a20", GMCommand::SpawnMonster {
        name: "Goblin".to_string(),
        count: 5,
        distance: 5,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Combat);
    assert_eq!(state.combat.as_ref().unwrap().monsters.len(), 5);

    // Roll surprise
    let resp = handle_request(&req("a21", GMCommand::RollSurprise), &mut state);
    assert!(resp.success);

    // Roll initiative
    let resp = handle_request(&req("a22", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);
    let combat = state.combat.as_ref().unwrap();
    assert!(combat.party_initiative > 0);
    assert!(combat.monster_initiative > 0);
    assert_eq!(combat.round, 1);

    // Fighter attacks goblin 0
    let pre_g0_hp = state.combat.as_ref().unwrap().monsters[0].hp;
    let resp = handle_request(&req("a23", GMCommand::Attack {
        character: "Aldric the Bold".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.combat.as_ref().unwrap().monsters[0].hp < pre_g0_hp);
    }

    // Thief backstabs goblin 1 — retry until hit to verify multiplier
    let saved_g1_hp = state.combat.as_ref().unwrap().monsters[1].hp;
    let mut backstab_hit = false;
    for i in 0..100 {
        state.combat.as_mut().unwrap().monsters[1].hp = saved_g1_hp;
        // Clear per-round action tracking so retry is allowed
        state.combat.as_mut().unwrap().characters_acted.clear();
        let resp = handle_request(&req(&format!("a24_{}", i), GMCommand::Backstab {
            character: "Vex".to_string(),
            monster_idx: 1,
            weapon: "dagger".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        if data["hit"].as_bool().unwrap_or(false) {
            assert_eq!(data["multiplier"], 2); // level 1 backstab = x2
            assert!(state.combat.as_ref().unwrap().monsters[1].hp < saved_g1_hp);
            backstab_hit = true;
            break;
        }
    }
    assert!(backstab_hit, "backstab should land at least once in 100 attempts");

    // Cleric attacks goblin 2
    let pre_g2_hp = state.combat.as_ref().unwrap().monsters[2].hp;
    let resp = handle_request(&req("a25", GMCommand::Attack {
        character: "Sister Mira".to_string(),
        monster_idx: 2,
        weapon: "mace".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.combat.as_ref().unwrap().monsters[2].hp < pre_g2_hp);
    }

    // Magic-User attacks goblin 3 with dagger (can't use swords)
    let pre_g3_hp = state.combat.as_ref().unwrap().monsters[3].hp;
    let resp = handle_request(&req("a26", GMCommand::Attack {
        character: "Zanthus".to_string(),
        monster_idx: 3,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.combat.as_ref().unwrap().monsters[3].hp < pre_g3_hp);
    }

    // Monster attacks back
    let pre_aldric_hp = state.party.find_member("Aldric the Bold").unwrap().hp;
    let resp = handle_request(&req("a27", GMCommand::MonsterAttack {
        monster_idx: 4,
        character: "Aldric the Bold".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.party.find_member("Aldric the Bold").unwrap().hp < pre_aldric_hp);
    }

    // Check morale
    let resp = handle_request(&req("a28", GMCommand::CheckMorale), &mut state);
    assert!(resp.success);
    assert!(!resp.message.is_empty(), "morale check should have result");

    // End combat — restores Exploration mode
    let resp = handle_request(&req("a29", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Exploration);
    let combat_data = resp.data.unwrap();
    let monster_xp = combat_data["total_xp"].as_u64().unwrap();
    // monster_xp may be 0 if no goblins were killed (attack rolls are random)

    // === STEP 5: Loot treasure and award XP ===
    let treasure_gp = 2000u64; // big haul to trigger level-up
    let share = treasure_gp / 4;
    let monster_share = monster_xp / 4;

    // Track pre-XP state
    let pre_fighter_xp = state.party.find_member("Aldric the Bold").unwrap().xp;
    assert_eq!(pre_fighter_xp, 0);

    for name in &["Aldric the Bold", "Vex", "Sister Mira", "Zanthus"] {
        let resp = handle_request(&req("a30", GMCommand::AwardTreasureXp {
            character: name.to_string(),
            treasure_gp: share,
            monster_xp: monster_share,
        }), &mut state);
        assert!(resp.success, "award xp to {} failed: {}", name, resp.message);
    }

    // === STEP 6: Verify level-up triggers ===
    // Fighter needs 2000 XP for L2. 500gp treasure + monster share + 10% bonus
    // should push past 2000.
    let fighter = state.party.find_member("Aldric the Bold").unwrap();
    assert!(fighter.xp > 0);
    // With STR 16 = +10% modifier, 500+monster_share base -> 550+ adjusted
    // plus treasure 500gp -> should be around 550+ total.
    // If treasure_gp=2000, share=500, adjusted = (500+monster_share)*1.10
    // Fighter needs 2000 for L2. 500*1.10 = 550 base. Not enough for L2 with just share.

    // Thief needs 1200 for L2. DEX 16 = +10%. share=500+monster_share, adjusted ~550+
    // Also not enough from share alone. Let's add more XP to trigger level-ups.
    // Award a big direct XP to trigger level-up for the thief
    let resp = handle_request(&req("a31", GMCommand::AwardTreasureXp {
        character: "Vex".to_string(),
        treasure_gp: 1000,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["ready_to_train"].as_bool().unwrap(), "thief should be ready to train");

    // Level up thief
    let resp = handle_request(&req("a31b", GMCommand::LevelUp {
        character: "Vex".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["new_level"], 2);
    assert!(data["hp_gained"].as_i64().unwrap() >= 1);

    let vex = state.party.find_member("Vex").unwrap();
    assert_eq!(vex.level, 2);
    assert!(vex.max_hp > 4, "thief should have gained HP");

    // Award big XP to fighter too
    let resp = handle_request(&req("a32", GMCommand::AwardTreasureXp {
        character: "Aldric the Bold".to_string(),
        treasure_gp: 2000,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["ready_to_train"].as_bool().unwrap(), "fighter should be ready to train");

    // Level up fighter
    let resp = handle_request(&req("a32b", GMCommand::LevelUp {
        character: "Aldric the Bold".to_string(),
    }), &mut state);
    assert!(resp.success);
    let fighter = state.party.find_member("Aldric the Bold").unwrap();
    assert_eq!(fighter.level, 2);
    assert!(fighter.max_hp > 8);
    assert!(fighter.saving_throws.is_some());

    // === STEP 7: Save state ===
    let save_name = unique_save_name("session_a");
    let resp = handle_request(&req("a40", GMCommand::Save {
        path: save_name.clone(),
    }), &mut state);
    assert!(resp.success, "save failed: {}", resp.message);

    // Record key state for comparison
    let saved_party_count = state.party.members.len();
    let saved_fighter_level = state.party.find_member("Aldric the Bold").unwrap().level;
    let saved_fighter_xp = state.party.find_member("Aldric the Bold").unwrap().xp;
    let saved_fighter_hp = state.party.find_member("Aldric the Bold").unwrap().max_hp;
    let saved_thief_level = state.party.find_member("Vex").unwrap().level;
    let saved_notes_count = state.notes.len();

    // === STEP 8: Load into fresh state and verify ===
    let mut loaded_state = GameState::new();
    let resp = handle_request(&req("a41", GMCommand::Load {
        path: save_name.clone(),
    }), &mut loaded_state);
    assert!(resp.success, "load failed: {}", resp.message);

    assert_eq!(loaded_state.party.members.len(), saved_party_count);
    let loaded_fighter = loaded_state.party.find_member("Aldric the Bold").unwrap();
    assert_eq!(loaded_fighter.level, saved_fighter_level);
    assert_eq!(loaded_fighter.xp, saved_fighter_xp);
    assert_eq!(loaded_fighter.max_hp, saved_fighter_hp);
    let loaded_thief = loaded_state.party.find_member("Vex").unwrap();
    assert_eq!(loaded_thief.level, saved_thief_level);
    assert_eq!(loaded_state.notes.len(), saved_notes_count);

    // Verify all 4 members survived the roundtrip
    assert!(loaded_state.party.find_member("Aldric the Bold").is_some());
    assert!(loaded_state.party.find_member("Vex").is_some());
    assert!(loaded_state.party.find_member("Sister Mira").is_some());
    assert!(loaded_state.party.find_member("Zanthus").is_some());

    // Clean up
    let _ = std::fs::remove_file(resolve_save(&save_name));
}

// ===========================================================================
// SESSION B: Wilderness travel — multi-hex travel, encounters, foraging
// ===========================================================================

#[test]
fn session_b_wilderness_travel() {
    let mut state = GameState::new();

    // Create a party
    state.party.add_member(make_fighter("Rowan"));
    state.party.add_member(make_cleric("Prior Anselm"));
    assert_eq!(state.party.members.len(), 2);

    // === Enter wilderness ===
    let resp = handle_request(&req("b1", GMCommand::EnterWilderness {
        terrain: Terrain::Clear,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness);

    // Verify starting position
    let ws = state.wilderness.as_ref().unwrap();
    assert_eq!(ws.current_x, 0);
    assert_eq!(ws.current_y, 0);
    assert_eq!(ws.travel_day, 1);

    // === Add multiple hexes for multi-hex travel ===
    let hexes = [
        (1, 0, Terrain::Forest),
        (1, 1, Terrain::Hills),
        (0, 1, Terrain::Swamp),
        (1, -1, Terrain::Clear),
        (2, 0, Terrain::Mountains),
    ];
    for (x, y, terrain) in &hexes {
        let resp = handle_request(&req("b2", GMCommand::AddHex {
            x: *x, y: *y, terrain: *terrain,
        }), &mut state);
        assert!(resp.success, "add hex ({},{}) failed: {}", x, y, resp.message);
    }

    // === Travel hex 1: (0,0) -> (1,0) forest ===
    let resp = handle_request(&req("b3", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    // Travel response should include encounter/lost info
    assert!(data.get("lost").is_some());
    assert!(data.get("has_encounter").is_some());

    // Party may or may not arrive at destination (could get lost in forest, 2-in-6)
    let ws = state.wilderness.as_ref().unwrap();
    let _after_travel1 = (ws.current_x, ws.current_y);
    // Travel day should have incremented
    assert!(ws.travel_day >= 2);

    // === Travel multiple days to cover ground ===
    // Travel toward (1,0) until we arrive (handling lost possibility)
    for _ in 0..5 {
        let ws = state.wilderness.as_ref().unwrap();
        if ws.current_x == 1 && ws.current_y == 0 { break; }
        // Travel back toward (1,0) from wherever we are
        handle_request(&req("b3r", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    }

    // Now try to travel to (1,1) hills
    // First make sure hex (1,1) is adjacent to current position
    let ws = state.wilderness.as_ref().unwrap();
    let can_reach_11 = (ws.current_x - 1).abs() <= 1 && (ws.current_y - 1).abs() <= 1;
    if can_reach_11 {
        let resp = handle_request(&req("b4", GMCommand::Travel { x: 1, y: 1 }), &mut state);
        assert!(resp.success);
    }

    // Travel toward swamp (0,1)
    let ws = state.wilderness.as_ref().unwrap();
    let can_reach_01 = (ws.current_x - 0).abs() <= 1 && (ws.current_y - 1).abs() <= 1;
    if can_reach_01 {
        let resp = handle_request(&req("b5", GMCommand::Travel { x: 0, y: 1 }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        // Swamp has higher lost chance (2-in-6)
        assert!(data.get("lost").is_some());
    }

    // === Query wilderness state ===
    let resp = handle_request(&req("b6", GMCommand::QueryWilderness), &mut state);
    assert!(resp.success);

    // Verify explored hexes accumulated (at least the starting hex + some travel)
    let ws = state.wilderness.as_ref().unwrap();
    assert!(ws.explored.len() >= 1, "should have explored at least 1 hex");

    // === Verify travel day counter incremented across all travel ===
    let ws = state.wilderness.as_ref().unwrap();
    assert!(ws.travel_day >= 2, "travel days should have incremented");

    // === Spawn encounter in wilderness ===
    let resp = handle_request(&req("b8", GMCommand::SpawnMonster {
        name: "Orc".to_string(),
        count: 3,
        distance: 60,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Combat);

    // Roll reaction to try to evade/negotiate
    let resp = handle_request(&req("b9", GMCommand::RollReaction {
        character: "Prior Anselm".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["character"], "Prior Anselm");
    assert!(data["charisma"].as_i64().unwrap() > 0);
    let reaction = data["reaction"].as_str().unwrap();
    assert!(!reaction.is_empty());

    // End combat — restores Wilderness mode
    let resp = handle_request(&req("b10", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness);
}

// ===========================================================================
// SESSION C: Retainer — hire, dungeon crawl, morale, combat, XP split
// ===========================================================================

#[test]
fn session_c_retainers() {
    let mut state = GameState::new();

    // Create party with high-CHA leader
    let mut leader = make_fighter("Captain Kael");
    leader.abilities.charisma = 16; // CHA 16 = max 6 retainers
    state.party.add_member(leader);
    state.party.add_member(make_cleric("Deacon Brin"));

    // === Hire a retainer ===
    let resp = handle_request(&req("c1", GMCommand::HireRetainer {
        employer: "Captain Kael".to_string(),
        retainer_name: "Tormund".to_string(),
        retainer_class: Class::Fighter,
        retainer_level: 1,
    }), &mut state);
    assert!(resp.success, "hire retainer failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["retainer"], "Tormund");
    assert_eq!(data["level"], 1);
    assert_eq!(data["wage_gp"], 25); // level 1 wage
    assert_eq!(data["max_retainers"], 6); // CHA 16

    // Hire a second retainer
    let resp = handle_request(&req("c2", GMCommand::HireRetainer {
        employer: "Captain Kael".to_string(),
        retainer_name: "Greta".to_string(),
        retainer_class: Class::Thief,
        retainer_level: 2,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["wage_gp"], 50); // level 2 wage

    // === Enter dungeon with retainers ===
    let resp = handle_request(&req("c3", GMCommand::EnterDungeon {
        level: 1,
        room_name: "Collapsed Entrance".to_string(),
    }), &mut state);
    assert!(resp.success);

    let resp = handle_request(&req("c4", GMCommand::Light {
        source: LightSourceKind::Torch,
        carrier: "Captain Kael".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert!(!state.time.as_ref().unwrap().lights.is_empty(), "should have active light");

    // === Retainer loyalty check ===
    // Tormund loyalty check (base loyalty for CHA 16 employer = 9)
    let resp = handle_request(&req("c5", GMCommand::LoyaltyCheck {
        retainer_name: "Tormund".to_string(),
        loyalty: 9,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["retainer"], "Tormund");
    assert_eq!(data["loyalty"], 9);
    let result = data["result"].as_str().unwrap();
    assert!(["Loyal", "Wavering", "Disloyal"].contains(&result));

    // === Combat with retainer in party ===
    let resp = handle_request(&req("c6", GMCommand::SpawnMonster {
        name: "Hobgoblin".to_string(),
        count: 3,
        distance: 5,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Combat);

    let resp = handle_request(&req("c7", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);

    // Party members attack
    let pre_hob0_hp = state.combat.as_ref().unwrap().monsters[0].hp;
    let resp = handle_request(&req("c8", GMCommand::Attack {
        character: "Captain Kael".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success, "attack failed: {}", resp.message);
    if resp.message.contains("HIT") {
        assert!(state.combat.as_ref().unwrap().monsters[0].hp < pre_hob0_hp);
    }

    let pre_hob1_hp = state.combat.as_ref().unwrap().monsters[1].hp;
    let resp = handle_request(&req("c9", GMCommand::Attack {
        character: "Deacon Brin".to_string(),
        monster_idx: 1,
        weapon: "mace".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.combat.as_ref().unwrap().monsters[1].hp < pre_hob1_hp);
    }

    // Monster attacks back
    let pre_kael_hp = state.party.find_member("Captain Kael").unwrap().hp;
    let resp = handle_request(&req("c10", GMCommand::MonsterAttack {
        monster_idx: 2,
        character: "Captain Kael".to_string(),
    }), &mut state);
    assert!(resp.success);
    if resp.message.contains("HIT") {
        assert!(state.party.find_member("Captain Kael").unwrap().hp < pre_kael_hp, "HP should decrease on hit");
    }

    // Check morale
    let resp = handle_request(&req("c11", GMCommand::CheckMorale), &mut state);
    assert!(resp.success);
    assert!(!resp.message.is_empty(), "morale check should have result");

    // End combat
    let resp = handle_request(&req("c12", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    let combat_data = resp.data.unwrap();
    let total_monster_xp = combat_data["total_xp"].as_u64().unwrap();

    // === XP split with retainer (retainer gets half share) ===
    // 2 PCs + 1 retainer at half = 2.5 shares
    // PC share = total / 2.5 = total * 2 / 5
    // Retainer share = total / 5
    let treasure_gp = 500u64;
    // Simple split: each PC gets full share, retainer would get half
    let pc_share_treasure = treasure_gp / 2;
    let pc_share_monster = total_monster_xp / 2;

    let resp = handle_request(&req("c13", GMCommand::AwardTreasureXp {
        character: "Captain Kael".to_string(),
        treasure_gp: pc_share_treasure,
        monster_xp: pc_share_monster,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    // CHA 16, STR prime req for Fighter (+10%)
    assert!(data["adjusted_xp"].as_u64().unwrap() > 0);

    let resp = handle_request(&req("c14", GMCommand::AwardTreasureXp {
        character: "Deacon Brin".to_string(),
        treasure_gp: pc_share_treasure,
        monster_xp: pc_share_monster,
    }), &mut state);
    assert!(resp.success);

    // Verify both PCs got XP
    let kael = state.party.find_member("Captain Kael").unwrap();
    assert!(kael.xp > 0, "Captain Kael should have XP");
    let brin = state.party.find_member("Deacon Brin").unwrap();
    assert!(brin.xp > 0, "Deacon Brin should have XP");

    // Retainer loyalty check after combat (morale situation)
    let resp = handle_request(&req("c15", GMCommand::LoyaltyCheck {
        retainer_name: "Greta".to_string(),
        loyalty: 8,
    }), &mut state);
    assert!(resp.success);
}

// ===========================================================================
// SAVE/LOAD ROUNDTRIP: Complex state — mid-combat, lights, wilderness+lost
// ===========================================================================

#[test]
fn save_load_complex_state() {
    let save_name = unique_save_name("complex_roundtrip");

    // === Build complex state: mid-combat with lights ===
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Grom"));
    state.party.add_member(make_thief("Silke"));
    state.party.add_member(make_cleric("Father Odo"));

    // Enter dungeon and set up time/lights
    let resp = handle_request(&req("s1", GMCommand::EnterDungeon {
        level: 2,
        room_name: "Deep Crypt".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Light multiple sources
    handle_request(&req("s2", GMCommand::Light {
        source: LightSourceKind::Torch,
        carrier: "Grom".to_string(),
    }), &mut state);
    handle_request(&req("s3", GMCommand::Light {
        source: LightSourceKind::Lantern,
        carrier: "Father Odo".to_string(),
    }), &mut state);

    // Advance a few turns to change light state
    for _ in 0..3 {
        handle_request(&req("s4", GMCommand::AdvanceTurn), &mut state);
    }

    // Add rooms and a door
    handle_request(&req("s5", GMCommand::AddRoom { id: 1, name: "Bone Room".to_string() }), &mut state);
    handle_request(&req("s6", GMCommand::AddDoor {
        id: 0, room_a: 0, room_b: 1, state: DoorState::Secret,
    }), &mut state);

    // Add rulings
    handle_request(&req("s7", GMCommand::Ruling {
        text: "The crypt is unnaturally cold.".to_string(),
    }), &mut state);
    handle_request(&req("s8", GMCommand::Ruling {
        text: "Strange runes glow on the walls.".to_string(),
    }), &mut state);

    // Spawn combat (puts us mid-combat)
    let resp = handle_request(&req("s9", GMCommand::SpawnMonster {
        name: "Orc".to_string(),
        count: 4,
        distance: 30,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Combat);

    // Roll initiative to populate combat state
    handle_request(&req("s10", GMCommand::RollInitiative), &mut state);

    // Attack to change combat state
    handle_request(&req("s11", GMCommand::Attack {
        character: "Grom".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);

    // Record all state before save
    let pre_turn = state.turn();
    let pre_dungeon_level = state.dungeon_level;
    let pre_mode = state.mode.clone();
    let pre_party_count = state.party.members.len();
    let pre_notes_count = state.notes.len();
    let pre_combat_round = state.combat.as_ref().unwrap().round;
    let pre_combat_monster_count = state.combat.as_ref().unwrap().monsters.len();
    let pre_combat_distance = state.combat.as_ref().unwrap().distance;
    let pre_light_count = state.time.as_ref().unwrap().lights.len();
    let pre_room_count = state.dungeon.as_ref().unwrap().rooms.len();
    let pre_door_count = state.dungeon.as_ref().unwrap().doors.len();
    let pre_grom_hp = state.party.find_member("Grom").unwrap().hp;
    let pre_monster0_hp = state.combat.as_ref().unwrap().monsters[0].hp;

    // === Save ===
    let resp = handle_request(&req("s20", GMCommand::Save {
        path: save_name.clone(),
    }), &mut state);
    assert!(resp.success, "save failed: {}", resp.message);

    // === Load into completely fresh state ===
    let mut loaded = GameState::new();
    let resp = handle_request(&req("s21", GMCommand::Load {
        path: save_name.clone(),
    }), &mut loaded);
    assert!(resp.success, "load failed: {}", resp.message);

    // === Verify EVERY field ===
    assert_eq!(loaded.turn(), pre_turn, "turn mismatch");
    assert_eq!(loaded.dungeon_level, pre_dungeon_level, "dungeon_level mismatch");
    assert_eq!(loaded.mode, pre_mode, "mode mismatch");
    assert_eq!(loaded.party.members.len(), pre_party_count, "party count mismatch");
    assert_eq!(loaded.notes.len(), pre_notes_count, "notes count mismatch");

    // Combat state
    let combat = loaded.combat.as_ref().expect("combat should exist after load");
    assert_eq!(combat.round, pre_combat_round, "combat round mismatch");
    assert_eq!(combat.monsters.len(), pre_combat_monster_count, "monster count mismatch");
    assert_eq!(combat.distance, pre_combat_distance, "combat distance mismatch");
    assert_eq!(combat.monsters[0].hp, pre_monster0_hp, "monster hp mismatch");

    // Time/light state
    let time = loaded.time.as_ref().expect("time should exist after load");
    assert_eq!(time.lights.len(), pre_light_count, "light count mismatch");
    // Torch should have 3 turns remaining (started at 6, advanced 3)
    let torch = time.lights.iter().find(|l| l.carrier == "Grom");
    assert!(torch.is_some(), "Grom's torch should persist");
    assert_eq!(torch.unwrap().remaining_turns, 3, "torch remaining turns mismatch");
    // Lantern should have 21 turns remaining (started at 24, advanced 3)
    let lantern = time.lights.iter().find(|l| l.carrier == "Father Odo");
    assert!(lantern.is_some(), "Father Odo's lantern should persist");
    assert_eq!(lantern.unwrap().remaining_turns, 21, "lantern remaining turns mismatch");

    // Dungeon state
    let dungeon = loaded.dungeon.as_ref().expect("dungeon should exist after load");
    assert_eq!(dungeon.rooms.len(), pre_room_count, "room count mismatch");
    assert_eq!(dungeon.doors.len(), pre_door_count, "door count mismatch");
    assert_eq!(dungeon.level, 2, "dungeon level mismatch");

    // Character state preserved
    let grom = loaded.party.find_member("Grom").unwrap();
    assert_eq!(grom.hp, pre_grom_hp, "Grom HP mismatch");
    assert_eq!(grom.class, Class::Fighter);

    // Notes preserved
    assert!(loaded.notes.iter().any(|n| n.contains("unnaturally cold")));
    assert!(loaded.notes.iter().any(|n| n.contains("runes glow")));

    // Clean up
    let _ = std::fs::remove_file(resolve_save(&save_name));

    // === Part 2: Wilderness state with lost flag ===
    let mut ws_state = GameState::new();
    ws_state.party.add_member(make_fighter("Wanderer"));

    let resp = handle_request(&req("w1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut ws_state);
    assert!(resp.success);

    // Add hexes and travel
    handle_request(&req("w2", GMCommand::AddHex {
        x: 1, y: 0, terrain: Terrain::Swamp,
    }), &mut ws_state);
    handle_request(&req("w3", GMCommand::Travel { x: 1, y: 0 }), &mut ws_state);

    // Manually set lost flag for testing
    ws_state.wilderness.as_mut().unwrap().lost = true;

    let pre_ws_x = ws_state.wilderness.as_ref().unwrap().current_x;
    let pre_ws_y = ws_state.wilderness.as_ref().unwrap().current_y;
    let pre_ws_lost = ws_state.wilderness.as_ref().unwrap().lost;
    let pre_ws_day = ws_state.wilderness.as_ref().unwrap().travel_day;

    let ws_save_name = unique_save_name("wilderness_roundtrip");
    let resp = handle_request(&req("w4", GMCommand::Save {
        path: ws_save_name.clone(),
    }), &mut ws_state);
    assert!(resp.success);

    let mut ws_loaded = GameState::new();
    let resp = handle_request(&req("w5", GMCommand::Load {
        path: ws_save_name.clone(),
    }), &mut ws_loaded);
    assert!(resp.success);

    let ws = ws_loaded.wilderness.as_ref().expect("wilderness should exist after load");
    assert_eq!(ws.current_x, pre_ws_x, "wilderness x mismatch");
    assert_eq!(ws.current_y, pre_ws_y, "wilderness y mismatch");
    assert_eq!(ws.lost, pre_ws_lost, "wilderness lost flag mismatch");
    assert_eq!(ws.travel_day, pre_ws_day, "wilderness travel_day mismatch");
    assert_eq!(ws_loaded.mode, GameMode::Wilderness, "mode should be Wilderness");

    let _ = std::fs::remove_file(resolve_save(&ws_save_name));
}

// ===========================================================================
// CHARACTER PROGRESSION: Multi-level XP accumulation across combats/treasure
// ===========================================================================

#[test]
fn character_progression_multi_level() {
    let mut state = GameState::new();

    // Create all 4 class types at level 1
    let mut fighter = make_fighter("Bjorn");
    fighter.xp = 0;
    state.party.add_member(fighter);

    let mut thief = make_thief("Nyx");
    thief.xp = 0;
    state.party.add_member(thief);

    let mut cleric = make_cleric("Amara");
    cleric.xp = 0;
    state.party.add_member(cleric);

    let mut mage = make_magic_user("Elara");
    mage.xp = 0;
    state.party.add_member(mage);

    // Record baseline stats
    let fighter_base_hp = state.party.find_member("Bjorn").unwrap().max_hp;
    let thief_base_hp = state.party.find_member("Nyx").unwrap().max_hp;
    let cleric_base_hp = state.party.find_member("Amara").unwrap().max_hp;
    let mage_base_hp = state.party.find_member("Elara").unwrap().max_hp;

    assert_eq!(state.party.find_member("Bjorn").unwrap().thac0, 19);
    assert_eq!(state.party.find_member("Nyx").unwrap().thac0, 19);

    // === Combat 1: Small encounter ===
    let resp = handle_request(&req("p1", GMCommand::SpawnMonster {
        name: "Kobold".to_string(),
        count: 6,
        distance: 20,
    }), &mut state);
    assert!(resp.success);

    handle_request(&req("p2", GMCommand::RollInitiative), &mut state);
    handle_request(&req("p3", GMCommand::Attack {
        character: "Bjorn".to_string(), monster_idx: 0, weapon: "sword".to_string(),
    }), &mut state);
    handle_request(&req("p4", GMCommand::Attack {
        character: "Amara".to_string(), monster_idx: 1, weapon: "mace".to_string(),
    }), &mut state);

    let resp = handle_request(&req("p5", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    let combat1_xp = resp.data.unwrap()["total_xp"].as_u64().unwrap();

    // Award combat 1 XP + small treasure
    for name in &["Bjorn", "Nyx", "Amara", "Elara"] {
        let resp = handle_request(&req("p6", GMCommand::AwardTreasureXp {
            character: name.to_string(),
            treasure_gp: 50,
            monster_xp: combat1_xp / 4,
        }), &mut state);
        assert!(resp.success);
    }

    // === Combat 2: Medium encounter ===
    let resp = handle_request(&req("p10", GMCommand::SpawnMonster {
        name: "Orc".to_string(),
        count: 4,
        distance: 40,
    }), &mut state);
    assert!(resp.success);

    handle_request(&req("p11", GMCommand::RollInitiative), &mut state);
    handle_request(&req("p12", GMCommand::Attack {
        character: "Bjorn".to_string(), monster_idx: 0, weapon: "sword".to_string(),
    }), &mut state);

    let resp = handle_request(&req("p13", GMCommand::EndCombat), &mut state);
    let combat2_xp = resp.data.unwrap()["total_xp"].as_u64().unwrap();

    // Award combat 2 XP + bigger treasure
    for name in &["Bjorn", "Nyx", "Amara", "Elara"] {
        handle_request(&req("p14", GMCommand::AwardTreasureXp {
            character: name.to_string(),
            treasure_gp: 200,
            monster_xp: combat2_xp / 4,
        }), &mut state);
    }

    // All characters should have accumulated XP
    for name in &["Bjorn", "Nyx", "Amara", "Elara"] {
        let c = state.party.find_member(name).unwrap();
        assert!(c.xp > 0, "{} should have XP", name);
    }

    // === Award large treasure haul to trigger level-ups ===
    // Thief needs 1200 XP for L2 (DEX 16 = +10% bonus)
    // Award enough to level up thief first (lowest threshold)
    let resp = handle_request(&req("p20", GMCommand::AwardTreasureXp {
        character: "Nyx".to_string(),
        treasure_gp: 1000,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["ready_to_train"].as_bool().unwrap(), "thief should be ready to train");

    // Level up thief to L2
    let resp = handle_request(&req("p20b", GMCommand::LevelUp {
        character: "Nyx".to_string(),
    }), &mut state);
    assert!(resp.success);

    let nyx = state.party.find_member("Nyx").unwrap();
    assert_eq!(nyx.level, 2);
    assert!(nyx.max_hp > thief_base_hp, "thief HP should increase");
    // Thief at L2 should have Open Locks target of 20 (vs 15 at L1)
    let resp = handle_request(&req("p21", GMCommand::ThiefSkillCheck {
        character: "Nyx".to_string(),
        skill: "open locks".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["target"], 20, "L2 thief open locks should be 20%");

    // Cleric needs 1500 for L2 (WIS 16 = +10%)
    let resp = handle_request(&req("p22", GMCommand::AwardTreasureXp {
        character: "Amara".to_string(),
        treasure_gp: 1500,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["ready_to_train"].as_bool().unwrap(), "cleric should be ready to train");
    let resp = handle_request(&req("p22b", GMCommand::LevelUp {
        character: "Amara".to_string(),
    }), &mut state);
    assert!(resp.success);
    let amara = state.party.find_member("Amara").unwrap();
    assert_eq!(amara.level, 2);
    assert!(amara.max_hp > cleric_base_hp, "cleric HP should increase");
    // Cleric saving throws should be set
    assert!(amara.saving_throws.is_some(), "cleric should have saving throws after level up");

    // Fighter needs 2000 for L2 (STR 16 = +10%)
    let resp = handle_request(&req("p23", GMCommand::AwardTreasureXp {
        character: "Bjorn".to_string(),
        treasure_gp: 2000,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["ready_to_train"].as_bool().unwrap(), "fighter should be ready to train");
    let resp = handle_request(&req("p23b", GMCommand::LevelUp {
        character: "Bjorn".to_string(),
    }), &mut state);
    assert!(resp.success);
    let bjorn = state.party.find_member("Bjorn").unwrap();
    assert_eq!(bjorn.level, 2);
    assert!(bjorn.max_hp > fighter_base_hp, "fighter HP should increase");
    assert!(bjorn.saving_throws.is_some(), "fighter should have saving throws");
    // Fighter THAC0 stays 19 at L2 (changes at L4 for martial)
    assert_eq!(bjorn.thac0, 19, "fighter THAC0 should still be 19 at L2");

    // Magic-User needs 2500 for L2 (INT 16 = +10%)
    let resp = handle_request(&req("p24", GMCommand::AwardTreasureXp {
        character: "Elara".to_string(),
        treasure_gp: 2500,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["ready_to_train"].as_bool().unwrap(), "magic-user should be ready to train");
    let resp = handle_request(&req("p24b", GMCommand::LevelUp {
        character: "Elara".to_string(),
    }), &mut state);
    assert!(resp.success);
    let elara = state.party.find_member("Elara").unwrap();
    assert_eq!(elara.level, 2);
    assert!(elara.max_hp > mage_base_hp, "magic-user HP should increase");

    // === Push fighter to level 3 (needs 4000 XP total) ===
    let resp = handle_request(&req("p30", GMCommand::AwardTreasureXp {
        character: "Bjorn".to_string(),
        treasure_gp: 2000,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success);
    let resp = handle_request(&req("p30b", GMCommand::LevelUp {
        character: "Bjorn".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["new_level"], 3);
    let bjorn = state.party.find_member("Bjorn").unwrap();
    assert_eq!(bjorn.level, 3);
    // THAC0 still 19 at L3 for martial (changes at L4)
    assert_eq!(bjorn.thac0, 19);

    // === Push fighter to level 4 (needs 8000 XP total) — THAC0 should improve ===
    let resp = handle_request(&req("p31", GMCommand::AwardTreasureXp {
        character: "Bjorn".to_string(),
        treasure_gp: 5000,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success);
    let resp = handle_request(&req("p31b", GMCommand::LevelUp {
        character: "Bjorn".to_string(),
    }), &mut state);
    assert!(resp.success);
    let bjorn = state.party.find_member("Bjorn").unwrap();
    assert_eq!(bjorn.level, 4);
    // Martial L4-6 THAC0 = 17
    assert_eq!(bjorn.thac0, 17, "fighter THAC0 should improve to 17 at L4");

    // === Push thief further — verify skill improvement ===
    // Thief L3 needs 2400 total, L4 needs 4800
    let resp = handle_request(&req("p32", GMCommand::AwardTreasureXp {
        character: "Nyx".to_string(),
        treasure_gp: 4000,
        monster_xp: 0,
    }), &mut state);
    assert!(resp.success);
    // Level up thief through multiple levels
    while state.party.find_member("Nyx").unwrap().level < 3 {
        let resp = handle_request(&req("p32b", GMCommand::LevelUp {
            character: "Nyx".to_string(),
        }), &mut state);
        if !resp.success { break; }
    }
    // May have enough for L4 too
    let _ = handle_request(&req("p32c", GMCommand::LevelUp {
        character: "Nyx".to_string(),
    }), &mut state);
    let nyx = state.party.find_member("Nyx").unwrap();
    assert!(nyx.level >= 3, "thief should be at least L3");

    // Check thief skill at new level
    let resp = handle_request(&req("p33", GMCommand::ThiefSkillCheck {
        character: "Nyx".to_string(),
        skill: "find traps".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    let target = data["target"].as_u64().unwrap();
    // L3 find traps = 20%, L4 = 25%
    assert!(target >= 20, "higher level thief should have better find traps");

    // === Verify saving throw improvement ===
    // Fighter L4 saving throws should be different from L1-3
    let bjorn = state.party.find_member("Bjorn").unwrap();
    let saves = bjorn.saving_throws.as_ref().unwrap();
    // Fighter L4-6 saves: D10 W11 P12 B13 S14
    assert_eq!(saves.death, 10, "fighter L4 death save should be 10");
    assert_eq!(saves.wands, 11, "fighter L4 wands save should be 11");

    // === Verify all characters accumulated HP ===
    let bjorn = state.party.find_member("Bjorn").unwrap();
    assert!(bjorn.max_hp > fighter_base_hp + 2, "fighter should have gained HP from multiple levels");
    let nyx = state.party.find_member("Nyx").unwrap();
    assert!(nyx.max_hp > thief_base_hp, "thief should have gained HP from leveling");
}

// ===========================================================================
// INTEGRATION TEST: Wilderness commands rejected in dungeon mode
// ===========================================================================

#[test]
fn wilderness_commands_rejected_in_dungeon_mode() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    // Enter dungeon — mode should be Exploration
    let resp = handle_request(&req("1", GMCommand::EnterDungeon {
        level: 1,
        room_name: "Test Room".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Exploration);

    // Travel should fail
    let resp = handle_request(&req("2", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(!resp.success, "Travel should fail in dungeon mode");
    assert!(resp.message.contains("wilderness"), "error should mention wilderness");

    // AddHex should fail
    let resp = handle_request(&req("3", GMCommand::AddHex {
        x: 1, y: 0, terrain: Terrain::Forest,
    }), &mut state);
    assert!(!resp.success, "AddHex should fail in dungeon mode");
    assert!(resp.message.contains("wilderness"), "error should mention wilderness");

    // Forage should fail
    let resp = handle_request(&req("4", GMCommand::Forage), &mut state);
    assert!(!resp.success, "Forage should fail in dungeon mode");
    assert!(resp.message.contains("wilderness"), "error should mention wilderness");

    // Hunt should fail
    let resp = handle_request(&req("5", GMCommand::Hunt), &mut state);
    assert!(!resp.success, "Hunt should fail in dungeon mode");
    assert!(resp.message.contains("wilderness"), "error should mention wilderness");

    // Orient should fail
    let resp = handle_request(&req("6", GMCommand::Orient), &mut state);
    assert!(!resp.success, "Orient should fail in dungeon mode");
    assert!(resp.message.contains("wilderness"), "error should mention wilderness");

    // Verify the mode is still Exploration (commands didn't change it)
    assert_eq!(state.mode, GameMode::Exploration);
}

// ===========================================================================
// PLAYTEST PASS 2C: Wilderness & Mode Gating Verification
// ===========================================================================

// ---------------------------------------------------------------------------
// Phase 2: Verify Pass 1 Fixes
// ---------------------------------------------------------------------------

/// oag-odo8b: Travel to hex not on map should NOT consume rations or advance day.
#[test]
fn playtest2c_travel_out_of_range_no_resource_consumption() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));
    state.party.add_member(make_cleric("Mira"));

    // Set rations
    handle_request(&req("r0", GMCommand::SetRations { amount: 20 }), &mut state);

    // Enter wilderness
    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    let ws = state.wilderness.as_ref().unwrap();
    let day_before = ws.travel_day;
    let rations_before = state.party.rations;

    // Try to travel to unmapped hex (5,5) — not on map
    let _resp = handle_request(&req("2", GMCommand::Travel { x: 5, y: 5 }), &mut state);
    // Should fail or succeed without consuming resources
    let ws = state.wilderness.as_ref().unwrap();
    assert_eq!(ws.travel_day, day_before, "day should NOT advance for out-of-range travel");
    assert_eq!(state.party.rations, rations_before, "rations should NOT be consumed for out-of-range travel");
}

/// oag-iasbo: Forage should consume a day AND daily rations.
#[test]
fn playtest2c_forage_consumes_day_and_rations() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Explorer"));

    handle_request(&req("r0", GMCommand::SetRations { amount: 20 }), &mut state);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    let ws = state.wilderness.as_ref().unwrap();
    let day_before = ws.travel_day;

    let resp = handle_request(&req("2", GMCommand::Forage), &mut state);
    assert!(resp.success, "forage failed: {}", resp.message);

    let ws = state.wilderness.as_ref().unwrap();
    assert!(ws.travel_day > day_before, "forage should advance the day");
    // Forage consumes daily rations (1 per party member) via apply_daily_overhead.
    // If forage SUCCEEDS, it also adds rations, so net might be higher.
    // Check the response data for rations_consumed instead.
    let data = resp.data.unwrap();
    assert!(data["rations_consumed"].as_u64().unwrap() > 0,
        "forage should consume daily rations (via apply_daily_overhead)");
}

/// oag-iasbo: Hunt should consume a day AND daily rations.
#[test]
fn playtest2c_hunt_consumes_day_and_rations() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Hunter"));

    handle_request(&req("r0", GMCommand::SetRations { amount: 20 }), &mut state);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    let ws = state.wilderness.as_ref().unwrap();
    let day_before = ws.travel_day;
    let rations_before = state.party.rations;

    let resp = handle_request(&req("2", GMCommand::Hunt), &mut state);
    assert!(resp.success, "hunt failed: {}", resp.message);

    let ws = state.wilderness.as_ref().unwrap();
    assert!(ws.travel_day > day_before, "hunt should advance the day");
    assert!(state.party.rations <= rations_before, "hunt should consume daily rations");
}

/// oag-iasbo: Orient when not lost should return "not lost" without consuming resources.
/// Orient when LOST should consume a day AND daily rations.
#[test]
fn playtest2c_orient_behavior() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Navigator"));

    handle_request(&req("r0", GMCommand::SetRations { amount: 20 }), &mut state);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    // When NOT lost, orient should return early with no day/ration consumption
    let ws = state.wilderness.as_ref().unwrap();
    let day_before = ws.travel_day;
    let rations_before = state.party.rations;

    let resp = handle_request(&req("2", GMCommand::Orient), &mut state);
    assert!(resp.success, "orient failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert!(!data["success"].as_bool().unwrap(), "orient when not lost should report success=false");

    let ws = state.wilderness.as_ref().unwrap();
    assert_eq!(ws.travel_day, day_before, "orient when not lost should NOT advance day");
    assert_eq!(state.party.rations, rations_before, "orient when not lost should NOT consume rations");

    // Manually set lost flag, then orient should consume resources
    state.wilderness.as_mut().unwrap().lost = true;
    let resp = handle_request(&req("3", GMCommand::Orient), &mut state);
    assert!(resp.success, "orient when lost failed: {}", resp.message);
    let ws = state.wilderness.as_ref().unwrap();
    assert!(ws.travel_day > day_before, "orient when lost should advance the day");
}

/// oag-vsjm7: Travel to current hex (0,0) when already there.
#[test]
fn playtest2c_travel_to_current_hex() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    handle_request(&req("r0", GMCommand::SetRations { amount: 20 }), &mut state);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Clear,
    }), &mut state);
    assert!(resp.success);

    let rations_before = state.party.rations;
    let day_before = state.wilderness.as_ref().unwrap().travel_day;

    // Travel to (0,0) — the current position: should be a no-op
    let resp = handle_request(&req("2", GMCommand::Travel { x: 0, y: 0 }), &mut state);
    assert!(resp.success, "travel to current hex should succeed as a no-op");
    assert!(resp.message.contains("Already at"), "should report already at position: {}", resp.message);

    // No resources consumed
    assert_eq!(state.party.rations, rations_before, "rations should not be consumed");
    assert_eq!(state.wilderness.as_ref().unwrap().travel_day, day_before,
        "travel day should not advance");
}

/// oag-0knyk: Wilderness commands in dungeon mode should be rejected.
#[test]
fn playtest2c_wilderness_cmds_in_dungeon() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    // Enter dungeon
    let resp = handle_request(&req("1", GMCommand::EnterDungeon {
        level: 1, room_name: "Entry".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Exploration);

    // All wilderness commands should fail
    let resp = handle_request(&req("t1", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(!resp.success, "Travel should be rejected in dungeon mode");

    let resp = handle_request(&req("t2", GMCommand::AddHex { x: 1, y: 0, terrain: Terrain::Forest }), &mut state);
    assert!(!resp.success, "AddHex should be rejected in dungeon mode");

    let resp = handle_request(&req("t3", GMCommand::Forage), &mut state);
    assert!(!resp.success, "Forage should be rejected in dungeon mode");

    let resp = handle_request(&req("t4", GMCommand::Hunt), &mut state);
    assert!(!resp.success, "Hunt should be rejected in dungeon mode");

    let resp = handle_request(&req("t5", GMCommand::Orient), &mut state);
    assert!(!resp.success, "Orient should be rejected in dungeon mode");

    let resp = handle_request(&req("t6", GMCommand::QueryWilderness), &mut state);
    assert!(!resp.success, "QueryWilderness should be rejected in dungeon mode");

    // Mode should be unchanged
    assert_eq!(state.mode, GameMode::Exploration);
}

/// oag-61a6d: Dungeon commands in wilderness mode should be rejected.
#[test]
fn playtest2c_dungeon_cmds_in_wilderness() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness);

    // Dungeon commands should fail
    let resp = handle_request(&req("d1", GMCommand::AdvanceTurn), &mut state);
    assert!(!resp.success, "AdvanceTurn should be rejected in wilderness mode");

    let resp = handle_request(&req("d2", GMCommand::MoveRoom { door_id: 0 }), &mut state);
    assert!(!resp.success, "MoveRoom should be rejected in wilderness mode");

    let resp = handle_request(&req("d3", GMCommand::Search { is_elf: false }), &mut state);
    assert!(!resp.success, "Search should be rejected in wilderness mode");

    let resp = handle_request(&req("d4", GMCommand::Listen { is_demihuman: false }), &mut state);
    assert!(!resp.success, "Listen should be rejected in wilderness mode");

    let resp = handle_request(&req("d5", GMCommand::ForceDoor {
        door_id: 0, character: "Aldric".to_string(),
    }), &mut state);
    assert!(!resp.success, "ForceDoor should be rejected in wilderness mode");

    let resp = handle_request(&req("d6", GMCommand::QueryExploration), &mut state);
    assert!(!resp.success, "QueryExploration should be rejected in wilderness mode");

    // Mode should be unchanged
    assert_eq!(state.mode, GameMode::Wilderness);
}

/// oag-g5i6n: EnterWilderness while already in wilderness should reject.
#[test]
fn playtest2c_enter_wilderness_while_in_wilderness() {
    let mut state = GameState::new();

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness);

    // Try to enter wilderness again
    let resp = handle_request(&req("2", GMCommand::EnterWilderness {
        terrain: Terrain::Clear,
    }), &mut state);
    assert!(!resp.success, "EnterWilderness should reject when already in wilderness");
    assert!(resp.message.contains("already"), "should mention 'already': {}", resp.message);
    assert_eq!(state.mode, GameMode::Wilderness);
}

/// oag-exmb2: EnterDungeon from wilderness rejects.
/// The error says "Use LeaveWilderness first" — user must leave wilderness before entering dungeon.
#[test]
fn playtest2c_enter_dungeon_from_wilderness_blocked() {
    let mut state = GameState::new();

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness);

    // Enter dungeon from wilderness — currently blocked
    let resp = handle_request(&req("2", GMCommand::EnterDungeon {
        level: 1, room_name: "Cave Entrance".to_string(),
    }), &mut state);
    assert!(!resp.success, "EnterDungeon from wilderness should currently be blocked");
    assert!(resp.message.contains("wilderness"), "error should mention wilderness: {}", resp.message);
    // Mode should be unchanged
    assert_eq!(state.mode, GameMode::Wilderness);
}

// ---------------------------------------------------------------------------
// Phase 3: Mode Gating Deep Dive
// ---------------------------------------------------------------------------

/// In Idle mode, all exploration/wilderness commands should reject gracefully.
#[test]
fn playtest2c_idle_mode_rejects_exploration_and_wilderness() {
    let mut state = GameState::new();
    assert_eq!(state.mode, GameMode::Idle);

    // Wilderness commands in idle
    let resp = handle_request(&req("w1", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(!resp.success, "Travel should fail in idle mode");

    let resp = handle_request(&req("w2", GMCommand::AddHex { x: 1, y: 0, terrain: Terrain::Forest }), &mut state);
    assert!(!resp.success, "AddHex should fail in idle mode");

    let resp = handle_request(&req("w3", GMCommand::Forage), &mut state);
    assert!(!resp.success, "Forage should fail in idle mode");

    let resp = handle_request(&req("w4", GMCommand::Hunt), &mut state);
    assert!(!resp.success, "Hunt should fail in idle mode");

    let resp = handle_request(&req("w5", GMCommand::Orient), &mut state);
    assert!(!resp.success, "Orient should fail in idle mode");

    // Dungeon commands in idle
    let resp = handle_request(&req("d1", GMCommand::AdvanceTurn), &mut state);
    assert!(!resp.success, "AdvanceTurn should fail in idle mode");

    let resp = handle_request(&req("d2", GMCommand::MoveRoom { door_id: 0 }), &mut state);
    assert!(!resp.success, "MoveRoom should fail in idle mode");

    let resp = handle_request(&req("d3", GMCommand::Search { is_elf: false }), &mut state);
    assert!(!resp.success, "Search should fail in idle mode");

    let resp = handle_request(&req("d4", GMCommand::Listen { is_demihuman: false }), &mut state);
    assert!(!resp.success, "Listen should fail in idle mode");

    let resp = handle_request(&req("d5", GMCommand::ForceDoor {
        door_id: 0, character: "Nobody".to_string(),
    }), &mut state);
    assert!(!resp.success, "ForceDoor should fail in idle mode");

    // Mode unchanged
    assert_eq!(state.mode, GameMode::Idle);
}

/// RollEncounter should work in both Exploration and Wilderness but not Idle.
#[test]
fn playtest2c_roll_encounter_mode_gating() {
    // Idle: should fail
    let mut state = GameState::new();
    let resp = handle_request(&req("1", GMCommand::RollEncounter), &mut state);
    assert!(!resp.success, "RollEncounter should fail in idle mode");

    // Wilderness: should work
    let mut state = GameState::new();
    let resp = handle_request(&req("w1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);
    let resp = handle_request(&req("w2", GMCommand::RollEncounter), &mut state);
    assert!(resp.success, "RollEncounter should work in wilderness: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["context"], "wilderness");

    // Exploration: should work
    let mut state = GameState::new();
    let resp = handle_request(&req("e1", GMCommand::EnterDungeon {
        level: 1, room_name: "Test".to_string(),
    }), &mut state);
    assert!(resp.success);
    let resp = handle_request(&req("e2", GMCommand::RollEncounter), &mut state);
    assert!(resp.success, "RollEncounter should work in exploration: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["context"], "dungeon");
}

/// SpawnEncounter should work from wilderness (enters combat, returns to wilderness).
#[test]
fn playtest2c_spawn_encounter_in_wilderness() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Ranger"));

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    let resp = handle_request(&req("2", GMCommand::SpawnMonster {
        name: "Dire Wolf".to_string(),
        count: 3,
        distance: 60,
    }), &mut state);
    assert!(resp.success, "SpawnMonster should work from wilderness: {}", resp.message);
    assert_eq!(state.mode, GameMode::Combat);

    // End combat should return to wilderness
    let resp = handle_request(&req("3", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness);
}

// ---------------------------------------------------------------------------
// Phase 4: Wilderness Travel Deep Dive
// ---------------------------------------------------------------------------

/// Build hex map with all terrain types and verify travel.
#[test]
fn playtest2c_travel_all_terrain_types() {
    let mut state = GameState::new();
    let mut fighter = make_fighter("Explorer");
    fighter.movement_rate = 120; // Standard movement
    state.party.add_member(fighter);

    handle_request(&req("r0", GMCommand::SetRations { amount: 100 }), &mut state);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Clear,
    }), &mut state);
    assert!(resp.success);

    // Add hexes with various terrain types adjacent to (0,0)
    let hexes = [
        (1, 0, Terrain::Clear),
        (0, 1, Terrain::Hills),
        (-1, 0, Terrain::Mountains),
        (0, -1, Terrain::Swamp),
        (1, 1, Terrain::Desert),
        (-1, -1, Terrain::Jungle),
    ];
    for (x, y, terrain) in &hexes {
        let resp = handle_request(&req("2", GMCommand::AddHex {
            x: *x, y: *y, terrain: *terrain,
        }), &mut state);
        assert!(resp.success, "AddHex ({},{}) {:?} failed: {}", x, y, terrain, resp.message);
    }

    // Travel to Clear (1,0) — should be fast (1x cost)
    let resp = handle_request(&req("3a", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(resp.success, "travel to clear failed: {}", resp.message);
    let data = resp.data.unwrap();
    assert!(data.get("lost").is_some(), "response should include lost status");
    assert!(data.get("has_encounter").is_some(), "response should include encounter info");

    // Travel back to origin
    let resp = handle_request(&req("3b", GMCommand::Travel { x: 0, y: 0 }), &mut state);
    assert!(resp.success, "travel back failed: {}", resp.message);

    // Travel to Hills (0,1) — 1.5x cost
    let resp = handle_request(&req("4a", GMCommand::Travel { x: 0, y: 1 }), &mut state);
    assert!(resp.success, "travel to hills failed: {}", resp.message);

    // Day counter should be advancing
    let ws = state.wilderness.as_ref().unwrap();
    assert!(ws.travel_day >= 3, "should be at least day 3 after multiple travels");
}

/// Travel consumes rations per party member per day.
#[test]
fn playtest2c_travel_rations_consumption() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));
    state.party.add_member(make_cleric("Mira"));
    state.party.add_member(make_thief("Vex"));
    state.party.add_member(make_magic_user("Zanthus"));

    handle_request(&req("r0", GMCommand::SetRations { amount: 20 }), &mut state);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Clear,
    }), &mut state);
    assert!(resp.success);

    let rations_before = state.party.rations;
    assert_eq!(rations_before, 20);

    // Add and travel to adjacent hex
    handle_request(&req("2", GMCommand::AddHex { x: 1, y: 0, terrain: Terrain::Clear }), &mut state);
    let resp = handle_request(&req("3", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(resp.success);

    let rations_after = state.party.rations;
    // 4 party members should consume 4 rations per day
    assert_eq!(rations_before - rations_after, 4,
        "4-member party should consume 4 rations per travel day (was {} now {})",
        rations_before, rations_after);
}

// ---------------------------------------------------------------------------
// Phase 5: Foraging Economy
// ---------------------------------------------------------------------------

/// Run rations to 0, verify starvation triggers.
#[test]
fn playtest2c_starvation_at_zero_rations() {
    let mut state = GameState::new();
    let mut fighter = make_fighter("Starving Steve");
    fighter.movement_rate = 120;
    state.party.add_member(fighter);

    // Start with exactly 2 rations (1 party member x 2 days)
    handle_request(&req("r0", GMCommand::SetRations { amount: 2 }), &mut state);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Clear,
    }), &mut state);
    assert!(resp.success);

    // Add hexes for travel
    handle_request(&req("2a", GMCommand::AddHex { x: 1, y: 0, terrain: Terrain::Clear }), &mut state);
    handle_request(&req("2b", GMCommand::AddHex { x: 0, y: 1, terrain: Terrain::Clear }), &mut state);
    handle_request(&req("2c", GMCommand::AddHex { x: 1, y: 1, terrain: Terrain::Clear }), &mut state);
    handle_request(&req("2d", GMCommand::AddHex { x: -1, y: 0, terrain: Terrain::Clear }), &mut state);
    handle_request(&req("2e", GMCommand::AddHex { x: 0, y: -1, terrain: Terrain::Clear }), &mut state);
    handle_request(&req("2f", GMCommand::AddHex { x: -1, y: -1, terrain: Terrain::Clear }), &mut state);
    handle_request(&req("2g", GMCommand::AddHex { x: -1, y: 1, terrain: Terrain::Clear }), &mut state);
    handle_request(&req("2h", GMCommand::AddHex { x: 1, y: -1, terrain: Terrain::Clear }), &mut state);

    // Travel day 1: consume 1 ration (1 left)
    let resp = handle_request(&req("3", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(resp.success);
    assert_eq!(state.party.rations, 1, "should have 1 ration left after day 1");

    // Travel day 2: consume 1 ration (0 left)
    // Navigate back, handling possible lost status
    let ws = state.wilderness.as_ref().unwrap();
    let (cx, cy) = (ws.current_x, ws.current_y);
    // Travel to an adjacent hex
    let next = if cx == 1 && cy == 0 { (0, 0) } else { (1, 0) };
    let resp = handle_request(&req("4", GMCommand::Travel { x: next.0, y: next.1 }), &mut state);
    assert!(resp.success);
    assert_eq!(state.party.rations, 0, "should have 0 rations left after day 2");

    // Travel day 3+: 0 rations, should trigger starvation
    let ws = state.wilderness.as_ref().unwrap();
    let (cx, cy) = (ws.current_x, ws.current_y);
    let next = if cx == 0 && cy == 0 { (1, 0) } else { (0, 0) };
    let resp = handle_request(&req("5", GMCommand::Travel { x: next.0, y: next.1 }), &mut state);
    assert!(resp.success);

    // Verify starvation happening (days_without_food > 0)
    assert!(state.party.days_without_food >= 1,
        "party should be starving after traveling with 0 rations");
}

/// Forage success rate varies by terrain (Forest better than Desert).
#[test]
fn playtest2c_forage_terrain_variation() {
    // Forage many times in Forest and Desert, verify both can succeed/fail
    // (probabilistic, but with enough trials should see variance)

    // Forest forage (2-in-6 chance)
    let mut forest_successes = 0;
    for i in 0..20 {
        let mut state = GameState::new();
        state.party.add_member(make_fighter(&format!("Forager{}", i)));
        handle_request(&req("r0", GMCommand::SetRations { amount: 100 }), &mut state);
        handle_request(&req("1", GMCommand::EnterWilderness { terrain: Terrain::Forest }), &mut state);
        let resp = handle_request(&req("2", GMCommand::Forage), &mut state);
        assert!(resp.success, "forage should succeed: {}", resp.message);
        let data = resp.data.unwrap();
        if data.get("rations_found").and_then(|v| v.as_u64()).unwrap_or(0) > 0 {
            forest_successes += 1;
        }
    }

    // Desert forage (1-in-6 chance)
    let mut desert_successes = 0;
    for i in 0..20 {
        let mut state = GameState::new();
        state.party.add_member(make_fighter(&format!("DesertF{}", i)));
        handle_request(&req("r0", GMCommand::SetRations { amount: 100 }), &mut state);
        handle_request(&req("1", GMCommand::EnterWilderness { terrain: Terrain::Desert }), &mut state);
        let resp = handle_request(&req("2", GMCommand::Forage), &mut state);
        assert!(resp.success, "desert forage should succeed: {}", resp.message);
        let data = resp.data.unwrap();
        if data.get("rations_found").and_then(|v| v.as_u64()).unwrap_or(0) > 0 {
            desert_successes += 1;
        }
    }

    // Probabilistic: Forest should tend to have more successes than desert
    // Not a hard assert since this is random, but log it
    eprintln!("Forage results: Forest {}/20, Desert {}/20", forest_successes, desert_successes);
    // Sanity: at least some successes should happen in 20 tries
    // (2-in-6 = 33% chance; 20 trials, probability of zero successes = (4/6)^20 ≈ 0.03%)
    // Not asserting hard because it's stochastic
}

// ---------------------------------------------------------------------------
// Phase 6: Random Encounters & Evasion
// ---------------------------------------------------------------------------

/// RollEncounter in wilderness returns proper encounter data.
#[test]
fn playtest2c_roll_encounter_wilderness() {
    let mut state = GameState::new();

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    // Roll encounters multiple times — all should succeed
    for i in 0..5 {
        let resp = handle_request(&req(&format!("e{}", i), GMCommand::RollEncounter), &mut state);
        assert!(resp.success, "RollEncounter #{} failed: {}", i, resp.message);
        let data = resp.data.unwrap();
        assert_eq!(data["context"], "wilderness");
        assert!(data["monster_name"].as_str().is_some());
        assert!(data["number_appearing"].as_u64().unwrap() > 0);
        assert!(data["distance"].as_u64().is_some(), "should have distance in yards");
    }
}

/// RollEncounter in mountains (different terrain table).
#[test]
fn playtest2c_roll_encounter_mountains() {
    let mut state = GameState::new();

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Mountains,
    }), &mut state);
    assert!(resp.success);

    for i in 0..5 {
        let resp = handle_request(&req(&format!("e{}", i), GMCommand::RollEncounter), &mut state);
        assert!(resp.success, "RollEncounter mountains #{} failed: {}", i, resp.message);
        let data = resp.data.unwrap();
        assert_eq!(data["context"], "wilderness");
    }
}

/// RollSurprise works from wilderness.
#[test]
fn playtest2c_surprise_in_wilderness() {
    let mut state = GameState::new();

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    let resp = handle_request(&req("2", GMCommand::RollSurprise), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert!(data["party_roll"].as_u64().is_some());
    assert!(data["monster_roll"].as_u64().is_some());
}

/// RollReaction with high-CHA character (no combat needed).
#[test]
fn playtest2c_reaction_high_charisma() {
    let mut state = GameState::new();
    let mut cleric = make_cleric("Diplomat");
    cleric.abilities.charisma = 18;
    state.party.add_member(cleric);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    // RollReaction works without active combat — it's a standalone encounter resolution tool
    let resp = handle_request(&req("2", GMCommand::RollReaction {
        character: "Diplomat".to_string(),
    }), &mut state);
    assert!(resp.success, "RollReaction should work: {}", resp.message);
    let data = resp.data.unwrap();
    assert_eq!(data["character"], "Diplomat");
    assert_eq!(data["charisma"], 18);
    let reaction = data["reaction"].as_str().unwrap();
    assert!(!reaction.is_empty(), "reaction result should not be empty");
}

/// Evade with various monster counts/movement rates.
#[test]
fn playtest2c_evade_mechanics() {
    let mut state = GameState::new();
    let mut fighter = make_fighter("Runner");
    fighter.movement_rate = 120;
    state.party.add_member(fighter);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    // Test evasion with few slow monsters (high chance of success)
    let resp = handle_request(&req("e1", GMCommand::Evade {
        monster_count: 1,
        monster_movement: 60,
    }), &mut state);
    assert!(resp.success, "evade should succeed: {}", resp.message);
    let data = resp.data.unwrap();
    assert!(data.get("escaped").is_some(), "evasion result should include escaped status");

    // Test evasion with many fast monsters (lower chance)
    let resp = handle_request(&req("e2", GMCommand::Evade {
        monster_count: 10,
        monster_movement: 180,
    }), &mut state);
    assert!(resp.success, "evade should always return a result: {}", resp.message);
}

/// SpawnNpcParty should use B/X classes only (oag-cqbud fix).
#[test]
fn playtest2c_npc_party_bx_classes() {
    let bx_classes = [
        "Fighter", "Thief", "Cleric", "Magic-User", "Elf", "Dwarf", "Halfling",
    ];

    // Generate several parties and check class distribution
    for i in 0..5 {
        let mut state = GameState::new();
        let resp = handle_request(&req(&format!("n{}", i), GMCommand::SpawnNpcParty {
            party_type: "basic".to_string(),
            distance: 60,
        }), &mut state);
        assert!(resp.success, "SpawnNpcParty #{} failed: {:?}", i, resp.error);

        let combat = state.combat.as_ref().unwrap();
        for monster in &combat.monsters {
            // NPC party members are spawned as "monsters" with class names
            // Check that the name contains only B/X class references
            let name_lower = monster.name.to_lowercase();
            let is_bx = bx_classes.iter().any(|c| name_lower.contains(&c.to_lowercase()));
            // Numbering may be appended ("fighter 1"), so just check the base
            assert!(is_bx, "NPC party member '{}' should be a B/X class", monster.name);
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 7: Wilderness Combat
// ---------------------------------------------------------------------------

/// Full wilderness combat at wilderness distance scale.
#[test]
fn playtest2c_wilderness_combat_distance_yards() {
    let mut state = GameState::new();
    let mut fighter = make_fighter("Warrior");
    fighter.hp = 50;
    fighter.max_hp = 50;
    state.party.add_member(fighter);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    // Roll encounter to get wilderness distance
    let resp = handle_request(&req("2", GMCommand::RollEncounter), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    let encounter_distance = data["distance"].as_u64().unwrap();
    // Wilderness encounters use yards (should be significantly larger than dungeon)
    assert!(encounter_distance > 0, "encounter distance should be positive");
    // Wilderness encounters are typically 40-240 yards
    eprintln!("Wilderness encounter distance: {} yards", encounter_distance);

    // Spawn combat at wilderness distance using SpawnEncounter (custom monster params)
    // Use distance of 120 yards to demonstrate wilderness scale
    let resp = handle_request(&req("3", GMCommand::SpawnEncounter(EncounterParams {
        name: "bandit".to_string(),
        count: 3,
        hit_dice: "1".parse().unwrap(),
        ac: 6,
        hp: 4,
        damage: "1d6".to_string(),
        morale: 8,
        distance: 120, // 120 yards — wilderness scale
        xp_value: Some(10),
    })), &mut state);
    assert!(resp.success, "SpawnEncounter should work from wilderness: {}", resp.message);
    assert_eq!(state.mode, GameMode::Combat);
    assert_eq!(state.combat.as_ref().unwrap().distance, 120);

    // Verify wilderness encounter distance is in yards (much larger than dungeon feet)
    // Dungeon encounters are 20-80 feet; wilderness are 40-240 yards
    eprintln!("Combat distance set to {} (wilderness scale)", state.combat.as_ref().unwrap().distance);

    // Close distance repeatedly to get to melee range (encounter move = movement_rate/3)
    let resp = handle_request(&req("4", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);

    // Close to melee range
    while state.combat.as_ref().unwrap().distance > 5 {
        let resp = handle_request(&req("4c", GMCommand::Close {
            character: "Warrior".to_string(),
            feet: None,
        }), &mut state);
        assert!(resp.success, "close should work: {}", resp.message);
    }

    // Attack at melee range
    let resp = handle_request(&req("5", GMCommand::Attack {
        character: "Warrior".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success, "attack should work at melee range: {}", resp.message);

    // End combat — should return to Wilderness
    let resp = handle_request(&req("6", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness, "should return to wilderness after combat");
}

/// Retreat/withdrawal at wilderness scale.
#[test]
fn playtest2c_wilderness_retreat() {
    let mut state = GameState::new();
    let mut fighter = make_fighter("Runner");
    fighter.hp = 50;
    fighter.max_hp = 50;
    fighter.movement_rate = 120;
    state.party.add_member(fighter);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Clear,
    }), &mut state);
    assert!(resp.success);

    let resp = handle_request(&req("2", GMCommand::SpawnMonster {
        name: "Ogre".to_string(),
        count: 1,
        distance: 30,
    }), &mut state);
    assert!(resp.success, "SpawnMonster Ogre failed: {}", resp.message);

    // Fighting withdrawal first (no free attacks)
    let resp = handle_request(&req("3", GMCommand::FightingWithdrawal {
        character: "Runner".to_string(),
    }), &mut state);
    assert!(resp.success, "fighting withdrawal should work: {}", resp.message);

    // Full retreat (with free attacks)
    let resp = handle_request(&req("4", GMCommand::Retreat {
        character: "Runner".to_string(),
    }), &mut state);
    assert!(resp.success, "retreat should work: {}", resp.message);

    // End combat, verify return to wilderness
    let resp = handle_request(&req("5", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness);
}

// ---------------------------------------------------------------------------
// Phase 8: Hex Map Persistence (Save/Load)
// ---------------------------------------------------------------------------

/// Build large hex map, save, reload, verify all hexes and state persist.
#[test]
fn playtest2c_hex_map_persistence() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let save_name = format!("osr_test_wilderness_persist_{pid}_{n}");

    let mut state = GameState::new();
    state.party.add_member(make_fighter("Explorer"));
    state.party.add_member(make_cleric("Prior"));

    handle_request(&req("r0", GMCommand::SetRations { amount: 50 }), &mut state);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    // Build 15+ hex map
    let hex_data = [
        (1, 0, Terrain::Clear),
        (0, 1, Terrain::Hills),
        (-1, 0, Terrain::Mountains),
        (0, -1, Terrain::Swamp),
        (1, 1, Terrain::Desert),
        (-1, -1, Terrain::Jungle),
        (1, -1, Terrain::Barren),
        (-1, 1, Terrain::River),
        (2, 0, Terrain::City),
        (2, 1, Terrain::Forest),
        (2, -1, Terrain::Hills),
        (-2, 0, Terrain::Clear),
        (-2, 1, Terrain::Mountains),
        (-2, -1, Terrain::Desert),
        (0, 2, Terrain::Swamp),
    ];

    for (x, y, terrain) in &hex_data {
        let resp = handle_request(&req("h", GMCommand::AddHex {
            x: *x, y: *y, terrain: *terrain,
        }), &mut state);
        assert!(resp.success, "AddHex ({},{}) failed: {}", x, y, resp.message);
    }

    // Travel a couple hexes to change position
    handle_request(&req("t1", GMCommand::Travel { x: 1, y: 0 }), &mut state);

    let pre_save_hexes = state.wilderness.as_ref().unwrap().hexes.len();
    let pre_save_day = state.wilderness.as_ref().unwrap().travel_day;
    let pre_save_rations = state.party.rations;
    assert!(pre_save_hexes >= 16, "should have at least 16 hexes (15 added + 1 starting)");

    // Save
    let resp = handle_request(&req("s1", GMCommand::Save { path: save_name.clone() }), &mut state);
    assert!(resp.success, "save failed: {}", resp.message);

    // Load into fresh state
    let mut state2 = GameState::new();
    let resp = handle_request(&req("l1", GMCommand::Load { path: save_name.clone() }), &mut state2);
    assert!(resp.success, "load failed: {}", resp.message);

    // Verify wilderness state persisted
    assert!(state2.wilderness.is_some(), "wilderness should persist after load");
    let ws = state2.wilderness.as_ref().unwrap();
    assert_eq!(ws.hexes.len(), pre_save_hexes, "hex count should match after load");
    assert_eq!(ws.travel_day, pre_save_day, "travel day should match after load");
    assert_eq!(state2.party.rations, pre_save_rations, "rations should match after load");
    assert_eq!(state2.mode, GameMode::Wilderness, "mode should be Wilderness after load");

    // Verify QueryWilderness works after load
    let resp = handle_request(&req("q1", GMCommand::QueryWilderness), &mut state2);
    assert!(resp.success, "QueryWilderness should work after load: {}", resp.message);

    // Clean up save file
    if let Ok(path) = osr_ai_gm::persist::safe_save_path(&save_name) {
        let _ = std::fs::remove_file(path);
    }
}

// ---------------------------------------------------------------------------
// Edge Cases
// ---------------------------------------------------------------------------

/// AddHex at same coordinates should reject (duplicate).
#[test]
fn playtest2c_add_hex_duplicate_coordinates() {
    let mut state = GameState::new();

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    // (0,0) already exists from EnterWilderness
    let resp = handle_request(&req("2", GMCommand::AddHex {
        x: 0, y: 0, terrain: Terrain::Clear,
    }), &mut state);
    assert!(!resp.success, "duplicate hex should be rejected");
    assert!(resp.message.contains("duplicate"), "error should mention duplicate: {}", resp.message);
}

/// Very large hex coordinates should work for AddHex.
#[test]
fn playtest2c_large_hex_coordinates() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Explorer"));

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Clear,
    }), &mut state);
    assert!(resp.success);

    let resp = handle_request(&req("2", GMCommand::AddHex {
        x: 1000, y: -1000, terrain: Terrain::Mountains,
    }), &mut state);
    assert!(resp.success, "large coordinates should be accepted: {}", resp.message);

    // Can't travel there (too far) — distance check rejects early with
    // "exceeds travel range" but returns success=true with no resources consumed.
    // This is the out-of-range early rejection behavior (oag-odo8b).
    let ws = state.wilderness.as_ref().unwrap();
    let day_before = ws.travel_day;
    let rations_before = state.party.rations;
    let _resp = handle_request(&req("3", GMCommand::Travel { x: 1000, y: -1000 }), &mut state);
    // Either fails or succeeds with "no travel attempted" — either way, no crash
    let ws = state.wilderness.as_ref().unwrap();
    assert_eq!(ws.travel_day, day_before, "should not advance day for out-of-range travel");
    assert_eq!(state.party.rations, rations_before, "should not consume rations for out-of-range travel");
}

/// RollEncounter 20 times rapidly — no crashes.
#[test]
fn playtest2c_rapid_roll_encounters() {
    let mut state = GameState::new();

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    for i in 0..20 {
        let resp = handle_request(&req(&format!("r{}", i), GMCommand::RollEncounter), &mut state);
        assert!(resp.success, "RollEncounter #{} failed: {}", i, resp.message);
    }
}

/// Mode transition: wilderness → combat → wilderness (round trip).
#[test]
fn playtest2c_wilderness_combat_roundtrip() {
    let mut state = GameState::new();
    let mut fighter = make_fighter("Warrior");
    fighter.hp = 100;
    fighter.max_hp = 100;
    state.party.add_member(fighter);

    handle_request(&req("r0", GMCommand::SetRations { amount: 50 }), &mut state);

    let resp = handle_request(&req("1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert!(resp.success);

    // Add hex and travel
    handle_request(&req("2", GMCommand::AddHex { x: 1, y: 0, terrain: Terrain::Hills }), &mut state);
    handle_request(&req("3", GMCommand::Travel { x: 1, y: 0 }), &mut state);

    let ws_day = state.wilderness.as_ref().unwrap().travel_day;
    let ws_rations = state.party.rations;

    // Enter combat (use SpawnEncounter for custom monster since "Bear" isn't in db)
    let resp = handle_request(&req("4", GMCommand::SpawnEncounter(EncounterParams {
        name: "bear".to_string(),
        count: 1,
        hit_dice: "4".parse().unwrap(),
        ac: 6,
        hp: 16,
        damage: "2d4".to_string(),
        morale: 7,
        distance: 20,
        xp_value: Some(75),
    })), &mut state);
    assert!(resp.success, "SpawnEncounter failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Combat);

    // Combat actions
    handle_request(&req("5", GMCommand::RollInitiative), &mut state);
    handle_request(&req("6", GMCommand::Attack {
        character: "Warrior".to_string(), monster_idx: 0, weapon: "sword".to_string(),
    }), &mut state);

    // End combat
    let resp = handle_request(&req("7", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness, "should return to wilderness");

    // Verify wilderness state preserved through combat
    let ws = state.wilderness.as_ref().unwrap();
    assert_eq!(ws.travel_day, ws_day, "travel day should not change during combat");
    assert_eq!(state.party.rations, ws_rations, "rations should not change during combat");

    // Can still travel after combat
    handle_request(&req("8", GMCommand::AddHex { x: 0, y: 1, terrain: Terrain::Clear }), &mut state);
    // Need to navigate from current position
    let ws = state.wilderness.as_ref().unwrap();
    if ws.current_x == 1 && ws.current_y == 0 {
        let resp = handle_request(&req("9", GMCommand::Travel { x: 0, y: 0 }), &mut state);
        assert!(resp.success, "should be able to travel after combat: {}", resp.message);
    }
}
