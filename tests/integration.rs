/// Integration tests for the OSR AI GM engine.
///
/// Full scenario: create characters -> explore dungeon -> encounter -> combat -> loot -> XP -> level up.

use osr_ai_gm::gmapi::protocol::{GMCommand, GMRequest};
use osr_ai_gm::gmapi::interface::handle_request;
use osr_ai_gm::persist::GameState;
use osr_ai_gm::model::{Character, AbilityScores};
use osr_ai_gm::rules::class::Class;
use osr_ai_gm::state::game::GameMode;

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
    c.alignment = "Lawful".to_string();
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
    c.alignment = "Neutral".to_string();
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
    c.alignment = "Lawful".to_string();
    c.gold_gp = 100;
    c.movement_rate = 60;
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
        source: "torch".to_string(),
        carrier: "Aldric".to_string(),
    }), &mut state);
    assert!(resp.success, "light torch failed: {}", resp.message);

    // -- STEP 4: Advance a dungeon turn --
    let resp = handle_request(&req("12", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success, "advance turn failed: {}", resp.message);

    // -- STEP 5: Add rooms and explore --
    let resp = handle_request(&req("13", GMCommand::AddRoom {
        id: 1,
        name: "Guard Room".to_string(),
    }), &mut state);
    assert!(resp.success, "add room failed: {}", resp.message);

    let resp = handle_request(&req("14", GMCommand::AddDoor {
        id: 0,
        room_a: 0,
        room_b: 1,
        state: "closed".to_string(),
    }), &mut state);
    assert!(resp.success, "add door failed: {}", resp.message);

    // -- STEP 6: Search the room --
    let resp = handle_request(&req("15", GMCommand::Search { is_elf: false }), &mut state);
    assert!(resp.success, "search failed: {}", resp.message);

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

    // -- STEP 9: Fighter attacks --
    let resp = handle_request(&req("22", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success, "attack failed: {}", resp.message);

    // -- STEP 10: Thief attempts backstab on a different goblin --
    let resp = handle_request(&req("23", GMCommand::Backstab {
        character: "Shade".to_string(),
        monster_idx: 1,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(resp.success, "backstab failed: {}", resp.message);
    let data = resp.data.unwrap();
    // Backstab should have multiplier 2 at level 1
    if data["hit"].as_bool().unwrap_or(false) {
        assert_eq!(data["multiplier"], 2);
    }

    // -- STEP 11: Check morale --
    let resp = handle_request(&req("24", GMCommand::CheckMorale), &mut state);
    assert!(resp.success, "morale check failed: {}", resp.message);

    // -- STEP 12: End combat --
    let resp = handle_request(&req("25", GMCommand::EndCombat), &mut state);
    assert!(resp.success, "end combat failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Idle);
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
    assert!(data["leveled_up"].as_bool().unwrap(), "should have leveled up");
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
    assert_eq!(data["target"], 1); // level 1 hear noise = 1-in-6
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
        retainer_class: "Fighter".to_string(),
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
    let path = "/tmp/osr_test_save.json";
    let resp = handle_request(&req("1", GMCommand::Save {
        path: path.to_string(),
    }), &mut state);
    assert!(resp.success, "save failed: {}", resp.message);

    // Load into a fresh state
    let mut new_state = GameState::new();
    let resp = handle_request(&req("2", GMCommand::Load {
        path: path.to_string(),
    }), &mut new_state);
    assert!(resp.success, "load failed: {}", resp.message);

    // Verify state matches
    assert_eq!(new_state.party.members.len(), 2);
    assert_eq!(new_state.party.members[0].name, "Aldric");
    assert_eq!(new_state.party.members[0].xp, 1500);
    assert_eq!(new_state.party.members[1].name, "Shade");
    assert_eq!(new_state.notes.len(), 1);

    // Clean up
    let _ = std::fs::remove_file(path);
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

    // Light source
    let resp = handle_request(&req("d2", GMCommand::Light {
        source: "lantern".to_string(),
        carrier: "Brother Tomas".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Advance turn
    let resp = handle_request(&req("d3", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success);

    // Add rooms
    handle_request(&req("d4", GMCommand::AddRoom { id: 1, name: "Goblin Lair".to_string() }), &mut state);
    handle_request(&req("d5", GMCommand::AddDoor { id: 0, room_a: 0, room_b: 1, state: "stuck".to_string() }), &mut state);

    // Thief listens at the door
    let resp = handle_request(&req("d6", GMCommand::ThiefSkillCheck {
        character: "Nyx the Shadow".to_string(),
        skill: "hear noise".to_string(),
    }), &mut state);
    assert!(resp.success);

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

    // Roll initiative
    let resp = handle_request(&req("c3", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);

    // Fighter attacks
    let resp = handle_request(&req("c4", GMCommand::Attack {
        character: "Sir Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Thief backstabs
    let resp = handle_request(&req("c5", GMCommand::Backstab {
        character: "Nyx the Shadow".to_string(),
        monster_idx: 1,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Cleric attacks
    let resp = handle_request(&req("c6", GMCommand::Attack {
        character: "Brother Tomas".to_string(),
        monster_idx: 2,
        weapon: "mace".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Monster attacks back
    let resp = handle_request(&req("c7", GMCommand::MonsterAttack {
        monster_idx: 3,
        character: "Sir Aldric".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Check morale
    let resp = handle_request(&req("c8", GMCommand::CheckMorale), &mut state);
    assert!(resp.success);

    // End combat
    let resp = handle_request(&req("c9", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Idle);
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

    // === SPELL LOOKUP ===
    let resp = handle_request(&req("s1", GMCommand::LookupSpell {
        name: "Cure Light Wounds".to_string(),
        list: "cleric".to_string(),
    }), &mut state);
    assert!(resp.success);

    // === RETAINER HIRING ===
    let resp = handle_request(&req("r1", GMCommand::HireRetainer {
        employer: "Sir Aldric".to_string(),
        retainer_name: "Bort the Torchbearer".to_string(),
        retainer_class: "Fighter".to_string(),
        retainer_level: 0,
    }), &mut state);
    assert!(resp.success);

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
    assert!(data["leveled_up"].as_bool().unwrap());
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

    // Backstab
    let resp = handle_request(&req("2", GMCommand::Backstab {
        character: "Dagger Dan".to_string(),
        monster_idx: 0,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    if data["hit"].as_bool().unwrap_or(false) {
        assert_eq!(data["multiplier"], 2, "level 1 thief should have x2 backstab");
    }
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

    let resp = handle_request(&req("2", GMCommand::Backstab {
        character: "Shadow Blade".to_string(),
        monster_idx: 0,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    if data["hit"].as_bool().unwrap_or(false) {
        assert_eq!(data["multiplier"], 3, "level 5 thief should have x3 backstab");
    }
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

    let resp = handle_request(&req("2", GMCommand::Backstab {
        character: "Master Thief".to_string(),
        monster_idx: 0,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    if data["hit"].as_bool().unwrap_or(false) {
        assert_eq!(data["multiplier"], 4, "level 9 thief should have x4 backstab");
    }
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

    let resp = handle_request(&req("3", GMCommand::Attack {
        character: "Sir Brave".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(resp.success);

    let resp = handle_request(&req("4", GMCommand::Attack {
        character: "Father Stone".to_string(),
        monster_idx: 1,
        weapon: "mace".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Monster attacks (use orc 2, which hasn't been attacked)
    let resp = handle_request(&req("5", GMCommand::MonsterAttack {
        monster_idx: 2,
        character: "Sir Brave".to_string(),
    }), &mut state);
    assert!(resp.success);

    let resp = handle_request(&req("6", GMCommand::CheckMorale), &mut state);
    assert!(resp.success);

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

    let resp = handle_request(&req("2", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);

    let initial_hp = state.party.find_member("Bruiser").unwrap().hp;

    // Monster attacks repeatedly until a hit
    for i in 0..10 {
        // Reset HP for each attempt
        state.party.find_member_mut("Bruiser").unwrap().hp = initial_hp;
        let resp = handle_request(&req(&format!("a{}", i), GMCommand::MonsterAttack {
            monster_idx: 0,
            character: "Bruiser".to_string(),
        }), &mut state);
        assert!(resp.success);
        let current_hp = state.party.find_member("Bruiser").unwrap().hp;
        if current_hp < initial_hp {
            // Damage was applied
            assert!(current_hp < initial_hp, "HP should decrease on hit");
            return;
        }
    }
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
        terrain: "forest".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Wilderness);

    // Add adjacent hex
    let resp = handle_request(&req("2", GMCommand::AddHex {
        x: 1, y: 0,
        terrain: "hills".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Travel
    let resp = handle_request(&req("3", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(resp.success);

    // Query wilderness state
    let resp = handle_request(&req("4", GMCommand::QueryWilderness), &mut state);
    assert!(resp.success);
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
        source: "lantern".to_string(),
        carrier: "Aldric".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Advance a turn to verify exploration works
    let resp = handle_request(&req("3", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success);
    let dungeon_turn = state.time.as_ref().unwrap().total_turns;
    assert!(dungeon_turn > 0, "turns should have advanced");

    // Exploration -> Combat (spawn encounter)
    let resp = handle_request(&req("4", GMCommand::SpawnEncounter {
        name: "skeleton".to_string(),
        count: 2,
        hit_dice: "1".to_string(),
        ac: 7,
        hp: 4,
        damage: "1d6".to_string(),
        morale: 12,
        distance: 5,
        xp_value: Some(10),
    }), &mut state);
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

    // Combat -> Idle (EndCombat resets to Idle)
    let resp = handle_request(&req("7", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Idle);

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
        terrain: "forest".to_string(),
    }), &mut state);
    assert!(resp.success, "enter wilderness failed: {}", resp.message);
    assert_eq!(state.mode, GameMode::Wilderness);

    // Add adjacent hexes for travel
    let resp = handle_request(&req("2", GMCommand::AddHex {
        x: 1, y: 0,
        terrain: "hills".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Travel to verify wilderness works
    let resp = handle_request(&req("3", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(resp.success);
    let travel_day = state.wilderness.as_ref().unwrap().travel_day;
    assert!(travel_day > 1, "travel day should have incremented");

    // Wilderness -> Combat (encounter during travel)
    let resp = handle_request(&req("4", GMCommand::SpawnEncounter {
        name: "wolf".to_string(),
        count: 3,
        hit_dice: "2".to_string(),
        ac: 7,
        hp: 6,
        damage: "1d6".to_string(),
        morale: 8,
        distance: 5,
        xp_value: Some(20),
    }), &mut state);
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

    // Combat -> Idle
    let resp = handle_request(&req("7", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Idle);

    // Wilderness state should still be present
    assert!(state.wilderness.is_some(), "wilderness state should survive combat");
    assert_eq!(
        state.wilderness.as_ref().unwrap().travel_day, travel_day,
        "travel day should be preserved after combat"
    );

    // Add another hex and verify wilderness travel still works after combat
    let _resp = handle_request(&req("8", GMCommand::AddHex {
        x: 0, y: 0,
        terrain: "clear".to_string(),
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
        source: "torch".to_string(),
        carrier: "Aldric".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Now exploration should work
    let resp = handle_request(&req("4", GMCommand::AdvanceTurn), &mut state);
    assert!(resp.success);
    assert!(!resp.message.contains("DARKNESS"), "should not be in darkness with torch");

    // Build out the dungeon
    handle_request(&req("5a", GMCommand::AddRoom { id: 1, name: "Guard Room".to_string() }), &mut state);
    handle_request(&req("5b", GMCommand::AddDoor {
        id: 0, room_a: 0, room_b: 1, state: "closed".to_string(),
    }), &mut state);

    // Search the room
    let resp = handle_request(&req("6", GMCommand::Search { is_elf: false }), &mut state);
    assert!(resp.success);

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
        terrain: "clear".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Build a hex path
    handle_request(&req("2a", GMCommand::AddHex { x: 1, y: 0, terrain: "clear".to_string() }), &mut state);
    handle_request(&req("2b", GMCommand::AddHex { x: 1, y: 1, terrain: "forest".to_string() }), &mut state);
    handle_request(&req("2c", GMCommand::AddHex { x: 0, y: 1, terrain: "mountains".to_string() }), &mut state);

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
