/// GM API protocol QA — comprehensive tests for gmapi/interface.rs
///
/// Tests every GMCommand variant for:
/// - Happy path (correct response format: id, success, message, mode, data)
/// - At least one error path
/// - State validation (commands reject in wrong game mode)
/// - Response data field contains correct payload types
/// - mode field accurately reflects GameMode after each command

use osr_ai_gm::gmapi::protocol::{GMCommand, GMRequest, GMResponse, parse_request};
use osr_ai_gm::gmapi::interface::handle_request;
use osr_ai_gm::persist::GameState;
use osr_ai_gm::model::{AbilityScores, Character, CombatState, Monster};
use osr_ai_gm::rules::alignment::Alignment;
use osr_ai_gm::rules::class::Class;
use osr_ai_gm::state::dungeon::DoorState;
use osr_ai_gm::state::game::GameMode;
use osr_ai_gm::state::time::LightSourceKind;
use osr_ai_gm::state::wilderness::Terrain;

use std::sync::atomic::{AtomicU64, Ordering};

fn unique_tmp_path(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    format!("{}/osr_test_{}_{pid}_{n}.json", std::env::temp_dir().display(), prefix)
}

// ===========================================================================
// Test helpers
// ===========================================================================

fn req(id: &str, command: GMCommand) -> GMRequest {
    GMRequest { id: id.to_string(), command }
}

fn make_fighter(name: &str) -> Character {
    let mut c = Character::new(name, Class::Fighter);
    c.abilities = AbilityScores {
        strength: 16, intelligence: 10, wisdom: 10,
        dexterity: 12, constitution: 14, charisma: 12,
    };
    c.hp = 8;
    c.max_hp = 8;
    c.ac = 3;
    c.thac0 = 19;
    c.alignment = Alignment::Lawful;
    c.gold_gp = 120;
    c.movement_rate = 60;
    c
}

fn make_thief(name: &str) -> Character {
    let mut c = Character::new(name, Class::Thief);
    c.abilities = AbilityScores {
        strength: 10, intelligence: 12, wisdom: 10,
        dexterity: 16, constitution: 10, charisma: 10,
    };
    c.hp = 4;
    c.max_hp = 4;
    c.ac = 6;
    c.thac0 = 19;
    c.alignment = Alignment::Neutral;
    c.gold_gp = 80;
    c.movement_rate = 120;
    c
}

fn make_cleric(name: &str) -> Character {
    let mut c = Character::new(name, Class::Cleric);
    c.abilities = AbilityScores {
        strength: 12, intelligence: 10, wisdom: 16,
        dexterity: 10, constitution: 12, charisma: 14,
    };
    c.hp = 6;
    c.max_hp = 6;
    c.ac = 4;
    c.thac0 = 19;
    c.alignment = Alignment::Lawful;
    c.gold_gp = 100;
    c.movement_rate = 60;
    c
}

/// Verify all GMResponse fields are well-formed.
fn assert_response_format(resp: &GMResponse, expected_id: &str) {
    assert_eq!(resp.id, expected_id, "response ID mismatch");
    assert!(!resp.message.is_empty(), "message should not be empty");
    // Success responses should not have error field
    if resp.success {
        assert!(resp.error.is_none(), "success response should not have error field, got: {:?}", resp.error);
    }
    // Error responses should have error field
    if !resp.success {
        assert!(resp.error.is_some(), "error response should have error field");
        assert!(!resp.error.as_ref().unwrap().is_empty(), "error field should not be empty");
    }
}

/// Setup a state with combat active (goblins at 5' distance).
fn setup_combat(state: &mut GameState) {
    state.party.add_member(make_fighter("Aldric"));
    let mut monsters = Vec::new();
    for i in 0..3 {
        let mut m = Monster::new(&format!("Goblin {}", i + 1), "1");
        m.hp = 3;
        m.max_hp = 3;
        m.ac = 6;
        m.damage = "1d6".to_string();
        m.morale = 7;
        m.xp_value = 5;
        m.attacks = vec!["attack".to_string()];
        monsters.push(m);
    }
    state.combat = Some(CombatState::new(monsters, 5));
    state.mode = GameMode::Combat;
}

/// Setup a state with exploration active.
fn setup_exploration(state: &mut GameState) {
    state.party.add_member(make_fighter("Aldric"));
    let resp = handle_request(&req("setup", GMCommand::EnterDungeon {
        level: 1,
        room_name: "Entry Hall".to_string(),
    }), state);
    assert!(resp.success, "setup exploration failed: {}", resp.message);
}

/// Setup a state with wilderness active.
fn setup_wilderness(state: &mut GameState) {
    state.party.add_member(make_fighter("Aldric"));
    let resp = handle_request(&req("setup", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), state);
    assert!(resp.success, "setup wilderness failed: {}", resp.message);
}

// ===========================================================================
// 1. QueryState
// ===========================================================================

#[test]
fn query_state_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("qs1", GMCommand::QueryState), &mut state);
    assert_response_format(&resp, "qs1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Idle);

    let data = resp.data.expect("QueryState should have data");
    assert_eq!(data["party_size"], 1);
    assert_eq!(data["mode"], "idle");
    assert_eq!(data["has_combat"], false);
    assert_eq!(data["has_dungeon"], false);
    assert_eq!(data["has_wilderness"], false);
}

#[test]
fn query_state_during_combat() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("qs2", GMCommand::QueryState), &mut state);
    assert_response_format(&resp, "qs2");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Combat);

    let data = resp.data.unwrap();
    assert_eq!(data["has_combat"], true);
    assert_eq!(data["mode"], "combat");
}

#[test]
fn query_state_during_exploration() {
    let mut state = GameState::new();
    setup_exploration(&mut state);

    let resp = handle_request(&req("qs3", GMCommand::QueryState), &mut state);
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Exploration);
    let data = resp.data.unwrap();
    assert_eq!(data["has_dungeon"], true);
    assert_eq!(data["mode"], "exploration");
}

// ===========================================================================
// 2. QueryMode
// ===========================================================================

#[test]
fn query_mode_idle() {
    let mut state = GameState::new();
    let resp = handle_request(&req("qm1", GMCommand::QueryMode), &mut state);
    assert_response_format(&resp, "qm1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Idle);
    let data = resp.data.unwrap();
    assert_eq!(data["mode"], "idle");
}

#[test]
fn query_mode_combat() {
    let mut state = GameState::new();
    setup_combat(&mut state);
    let resp = handle_request(&req("qm2", GMCommand::QueryMode), &mut state);
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Combat);
    let data = resp.data.unwrap();
    assert_eq!(data["mode"], "combat");
}

#[test]
fn query_mode_exploration() {
    let mut state = GameState::new();
    setup_exploration(&mut state);
    let resp = handle_request(&req("qm3", GMCommand::QueryMode), &mut state);
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Exploration);
}

#[test]
fn query_mode_wilderness() {
    let mut state = GameState::new();
    setup_wilderness(&mut state);
    let resp = handle_request(&req("qm4", GMCommand::QueryMode), &mut state);
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Wilderness);
}

// ===========================================================================
// 3. QueryParty
// ===========================================================================

#[test]
fn query_party_empty() {
    let mut state = GameState::new();
    let resp = handle_request(&req("qp1", GMCommand::QueryParty), &mut state);
    assert_response_format(&resp, "qp1");
    assert!(resp.success);
    let data = resp.data.unwrap();
    let members = data["members"].as_array().unwrap();
    assert_eq!(members.len(), 0);
}

#[test]
fn query_party_with_members() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));
    state.party.add_member(make_thief("Shade"));

    let resp = handle_request(&req("qp2", GMCommand::QueryParty), &mut state);
    assert_response_format(&resp, "qp2");
    assert!(resp.success);

    let data = resp.data.unwrap();
    let members = data["members"].as_array().unwrap();
    assert_eq!(members.len(), 2);

    // Verify data field structure
    let aldric = &members[0];
    assert_eq!(aldric["name"], "Aldric");
    assert_eq!(aldric["class"], "Fighter");
    assert_eq!(aldric["level"], 1);
    assert_eq!(aldric["hp"], 8);
    assert_eq!(aldric["max_hp"], 8);
    assert_eq!(aldric["ac"], 3);
    assert_eq!(aldric["thac0"], 19);
    assert_eq!(aldric["xp"], 0);
    assert_eq!(aldric["alive"], true);
    assert_eq!(aldric["alignment"], "Lawful");
    assert_eq!(aldric["movement_rate"], 60);
}

// ===========================================================================
// 4. QueryCombat
// ===========================================================================

#[test]
fn query_combat_no_combat() {
    let mut state = GameState::new();
    let resp = handle_request(&req("qc1", GMCommand::QueryCombat), &mut state);
    assert_response_format(&resp, "qc1");
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no active combat"));
}

#[test]
fn query_combat_active() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("qc2", GMCommand::QueryCombat), &mut state);
    assert_response_format(&resp, "qc2");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Combat);

    let data = resp.data.unwrap();
    assert!(data["round"].as_u64().is_some());
    assert_eq!(data["distance"], 5);
    let monsters = data["monsters"].as_array().unwrap();
    assert_eq!(monsters.len(), 3);
    // Verify monster data structure
    assert_eq!(monsters[0]["name"], "Goblin 1");
    assert_eq!(monsters[0]["ac"], 6);
    assert_eq!(monsters[0]["alive"], true);
}

// ===========================================================================
// 5. QueryExploration
// ===========================================================================

#[test]
fn query_exploration_not_exploring() {
    let mut state = GameState::new();
    let resp = handle_request(&req("qe1", GMCommand::QueryExploration), &mut state);
    assert_response_format(&resp, "qe1");
    assert!(!resp.success);
}

#[test]
fn query_exploration_active() {
    let mut state = GameState::new();
    setup_exploration(&mut state);

    let resp = handle_request(&req("qe2", GMCommand::QueryExploration), &mut state);
    assert_response_format(&resp, "qe2");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Exploration);

    let data = resp.data.unwrap();
    assert_eq!(data["dungeon_level"], 1);
    assert_eq!(data["current_room"], 0);
    assert!(data["total_turns"].as_u64().is_some());
}

// ===========================================================================
// 6. QueryWilderness
// ===========================================================================

#[test]
fn query_wilderness_not_in_wilderness() {
    let mut state = GameState::new();
    let resp = handle_request(&req("qw1", GMCommand::QueryWilderness), &mut state);
    assert_response_format(&resp, "qw1");
    assert!(!resp.success);
}

#[test]
fn query_wilderness_active() {
    let mut state = GameState::new();
    setup_wilderness(&mut state);

    let resp = handle_request(&req("qw2", GMCommand::QueryWilderness), &mut state);
    assert_response_format(&resp, "qw2");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Wilderness);

    let data = resp.data.unwrap();
    assert_eq!(data["current_x"], 0);
    assert_eq!(data["current_y"], 0);
    assert_eq!(data["travel_day"], 1);
    assert_eq!(data["lost"], false);
}

// ===========================================================================
// 7. CreateCharacter
// ===========================================================================

#[test]
fn create_character_happy_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("cc1", GMCommand::CreateCharacter {
        name: "Aldric".to_string(),
        class: Class::Fighter,
        alignment: Alignment::Lawful,
    }), &mut state);
    assert_response_format(&resp, "cc1");
    // Fighter has no ability requirements, so creation always succeeds.
    assert!(resp.success, "Fighter creation should always succeed (no requirements)");
    assert!(resp.data.is_some(), "success should include character sheet");
    assert_eq!(state.party.members.len(), 1);
}

#[test]
fn create_character_invalid_class() {
    // Invalid class is now caught at JSON deserialization — the type system
    // prevents constructing a GMCommand with an invalid Class enum value.
    let json = r#"{"id":"cc2","command":{"type":"CreateCharacter","params":{"name":"BadClass","class":"Astronaut"}}}"#;
    let result = parse_request(json);
    assert!(result.is_err(), "parsing invalid class should fail");
}

#[test]
fn create_character_invalid_alignment() {
    // Invalid alignment is caught at JSON deserialization.
    let json = r#"{"id":"cc3","command":{"type":"CreateCharacter","params":{"name":"BadAlign","class":"Fighter","alignment":"Evil"}}}"#;
    let result = parse_request(json);
    assert!(result.is_err(), "parsing invalid alignment should fail");
}

#[test]
fn create_character_alignment_abbreviations() {
    // Alignment abbreviations are accepted via serde aliases at parse time.
    let json_l = r#"{"id":"cc4","command":{"type":"CreateCharacter","params":{"name":"Knight","class":"Fighter","alignment":"L"}}}"#;
    let req_l = parse_request(json_l).unwrap();
    match &req_l.command {
        GMCommand::CreateCharacter { alignment, .. } => assert_eq!(*alignment, Alignment::Lawful),
        _ => panic!("expected CreateCharacter"),
    }

    let json_c = r#"{"id":"cc5","command":{"type":"CreateCharacter","params":{"name":"Rogue","class":"Thief","alignment":"C"}}}"#;
    let req_c = parse_request(json_c).unwrap();
    match &req_c.command {
        GMCommand::CreateCharacter { alignment, .. } => assert_eq!(*alignment, Alignment::Chaotic),
        _ => panic!("expected CreateCharacter"),
    }

    let json_n = r#"{"id":"cc6","command":{"type":"CreateCharacter","params":{"name":"Druid","class":"Cleric","alignment":"N"}}}"#;
    let req_n = parse_request(json_n).unwrap();
    match &req_n.command {
        GMCommand::CreateCharacter { alignment, .. } => assert_eq!(*alignment, Alignment::Neutral),
        _ => panic!("expected CreateCharacter"),
    }
}

// ===========================================================================
// 8. SpawnEncounter
// ===========================================================================

#[test]
fn spawn_encounter_happy_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("se1", GMCommand::SpawnEncounter {
        name: "Orc".to_string(),
        count: 2,
        hit_dice: "1".parse().unwrap(),
        ac: 6,
        hp: 4,
        damage: "1d6".to_string(),
        morale: 8,
        distance: 30,
        xp_value: Some(10),
    }), &mut state);
    assert_response_format(&resp, "se1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Combat);
    assert_eq!(state.mode, GameMode::Combat);
    let combat = state.combat.as_ref().unwrap();
    assert_eq!(combat.monsters.len(), 2);
    assert_eq!(combat.distance, 30);
    assert_eq!(combat.monsters[0].xp_value, 10);
}

#[test]
fn spawn_encounter_combat_already_active() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("se2", GMCommand::SpawnEncounter {
        name: "Orc".to_string(),
        count: 1,
        hit_dice: "1".parse().unwrap(),
        ac: 6,
        hp: 4,
        damage: "1d6".to_string(),
        morale: 8,
        distance: 30,
        xp_value: None,
    }), &mut state);
    assert_response_format(&resp, "se2");
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("already active"));
}

#[test]
fn spawn_encounter_invalid_morale_low() {
    let mut state = GameState::new();
    let resp = handle_request(&req("se3", GMCommand::SpawnEncounter {
        name: "Goblin".to_string(),
        count: 1,
        hit_dice: "1".parse().unwrap(),
        ac: 6,
        hp: 3,
        damage: "1d6".to_string(),
        morale: 1, // too low, must be 2-12
        distance: 60,
        xp_value: None,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("morale"));
}

#[test]
fn spawn_encounter_invalid_morale_high() {
    let mut state = GameState::new();
    let resp = handle_request(&req("se4", GMCommand::SpawnEncounter {
        name: "Goblin".to_string(),
        count: 1,
        hit_dice: "1".parse().unwrap(),
        ac: 6,
        hp: 3,
        damage: "1d6".to_string(),
        morale: 13, // too high
        distance: 60,
        xp_value: None,
    }), &mut state);
    assert!(!resp.success);
}

#[test]
fn spawn_encounter_morale_boundary_valid() {
    let mut state = GameState::new();
    // morale=2 (minimum valid)
    let resp = handle_request(&req("se5", GMCommand::SpawnEncounter {
        name: "Kobold".to_string(),
        count: 1,
        hit_dice: "1".parse().unwrap(),
        ac: 7,
        hp: 2,
        damage: "1d4".to_string(),
        morale: 2,
        distance: 60,
        xp_value: None,
    }), &mut state);
    assert!(resp.success, "morale=2 should be valid");

    // Reset
    let mut state2 = GameState::new();
    // morale=12 (maximum valid)
    let resp = handle_request(&req("se6", GMCommand::SpawnEncounter {
        name: "Dragon".to_string(),
        count: 1,
        hit_dice: "8".parse().unwrap(),
        ac: 2,
        hp: 36,
        damage: "2d8".to_string(),
        morale: 12,
        distance: 120,
        xp_value: Some(1200),
    }), &mut state2);
    assert!(resp.success, "morale=12 should be valid");
}

#[test]
fn spawn_encounter_explicit_xp_value() {
    let mut state = GameState::new();
    let resp = handle_request(&req("se7", GMCommand::SpawnEncounter {
        name: "Custom Creature".to_string(),
        count: 1,
        hit_dice: "3".parse().unwrap(),
        ac: 5,
        hp: 12,
        damage: "2d6".to_string(),
        morale: 9,
        distance: 40,
        xp_value: Some(500),
    }), &mut state);
    assert!(resp.success);
    let combat = state.combat.as_ref().unwrap();
    assert_eq!(combat.monsters[0].xp_value, 500);
}

#[test]
fn spawn_encounter_single_monster_no_numbering() {
    let mut state = GameState::new();
    let resp = handle_request(&req("se8", GMCommand::SpawnEncounter {
        name: "Troll".to_string(),
        count: 1,
        hit_dice: "6".parse().unwrap(),
        ac: 4,
        hp: 27,
        damage: "2d6".to_string(),
        morale: 10,
        distance: 60,
        xp_value: Some(275),
    }), &mut state);
    assert!(resp.success);
    let combat = state.combat.as_ref().unwrap();
    // Single monster should not have " 1" suffix
    assert_eq!(combat.monsters[0].name, "Troll");
}

// ===========================================================================
// 9. RollInitiative
// ===========================================================================

#[test]
fn roll_initiative_happy_path() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("ri1", GMCommand::RollInitiative), &mut state);
    assert_response_format(&resp, "ri1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Combat);

    let data = resp.data.unwrap();
    assert!(data["round"].as_u64().is_some());
    assert!(data["party_initiative"].as_i64().is_some());
    assert!(data["monster_initiative"].as_i64().is_some());
    let winner = data["winner"].as_str().unwrap();
    assert!(["party", "monsters", "simultaneous"].contains(&winner));
}

#[test]
fn roll_initiative_no_combat() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ri2", GMCommand::RollInitiative), &mut state);
    assert_response_format(&resp, "ri2");
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no active combat"));
}

// ===========================================================================
// 10. Attack
// ===========================================================================

#[test]
fn attack_happy_path() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("a1", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert_response_format(&resp, "a1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Combat);
}

#[test]
fn attack_unknown_weapon() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("a2", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "lightsaber".to_string(),
    }), &mut state);
    assert_response_format(&resp, "a2");
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("unknown weapon"));
}

#[test]
fn attack_unknown_character() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("a3", GMCommand::Attack {
        character: "Nobody".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

#[test]
fn attack_monster_idx_out_of_range() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("a4", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 99,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("out of range"));
}

#[test]
fn attack_dead_monster() {
    let mut state = GameState::new();
    setup_combat(&mut state);
    // Kill the first monster
    state.combat.as_mut().unwrap().monsters[0].hp = 0;

    let resp = handle_request(&req("a5", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("dead"));
}

#[test]
fn attack_dead_character() {
    let mut state = GameState::new();
    setup_combat(&mut state);
    // Kill the character
    state.party.find_member_mut("Aldric").unwrap().hp = 0;

    let resp = handle_request(&req("a6", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("dead"));
}

#[test]
fn attack_no_combat() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("a7", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no active combat"));
}

#[test]
fn attack_melee_at_distance() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));
    // Create combat at long distance
    let mut m = Monster::new("Orc", "1");
    m.hp = 4;
    m.max_hp = 4;
    m.ac = 6;
    m.damage = "1d6".to_string();
    m.morale = 8;
    m.attacks = vec!["attack".to_string()];
    state.combat = Some(CombatState::new(vec![m], 60));
    state.mode = GameMode::Combat;

    let resp = handle_request(&req("a8", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("melee weapon"));
}

// ===========================================================================
// 11. MonsterAttack
// ===========================================================================

#[test]
fn monster_attack_happy_path() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("ma1", GMCommand::MonsterAttack {
        monster_idx: 0,
        character: "Aldric".to_string(),
    }), &mut state);
    assert_response_format(&resp, "ma1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Combat);
}

#[test]
fn monster_attack_no_combat() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("ma2", GMCommand::MonsterAttack {
        monster_idx: 0,
        character: "Aldric".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no active combat"));
}

#[test]
fn monster_attack_idx_out_of_range() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("ma3", GMCommand::MonsterAttack {
        monster_idx: 99,
        character: "Aldric".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("out of range"));
}

#[test]
fn monster_attack_dead_monster() {
    let mut state = GameState::new();
    setup_combat(&mut state);
    state.combat.as_mut().unwrap().monsters[0].hp = 0;

    let resp = handle_request(&req("ma4", GMCommand::MonsterAttack {
        monster_idx: 0,
        character: "Aldric".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("dead"));
}

#[test]
fn monster_attack_unknown_character() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("ma5", GMCommand::MonsterAttack {
        monster_idx: 0,
        character: "Nobody".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

#[test]
fn monster_attack_dead_character() {
    let mut state = GameState::new();
    setup_combat(&mut state);
    state.party.find_member_mut("Aldric").unwrap().hp = 0;

    let resp = handle_request(&req("ma6", GMCommand::MonsterAttack {
        monster_idx: 0,
        character: "Aldric".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("dead"));
}

// ===========================================================================
// 12. CheckMorale
// ===========================================================================

#[test]
fn check_morale_happy_path() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("cm1", GMCommand::CheckMorale), &mut state);
    assert_response_format(&resp, "cm1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Combat);
}

#[test]
fn check_morale_no_combat() {
    let mut state = GameState::new();
    let resp = handle_request(&req("cm2", GMCommand::CheckMorale), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no active combat"));
}

#[test]
fn check_morale_no_living_monsters() {
    let mut state = GameState::new();
    setup_combat(&mut state);
    // Kill all monsters
    for m in &mut state.combat.as_mut().unwrap().monsters {
        m.hp = 0;
    }

    let resp = handle_request(&req("cm3", GMCommand::CheckMorale), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no living monsters"));
}

// ===========================================================================
// 13. TurnUndead
// ===========================================================================

#[test]
fn turn_undead_cleric_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_cleric("Brother Marcus"));
    // Create combat with skeleton
    let mut m = Monster::new("Skeleton", "1");
    m.hp = 4;
    m.max_hp = 4;
    m.ac = 7;
    m.damage = "1d6".to_string();
    m.morale = 12;
    m.attacks = vec!["attack".to_string()];
    state.combat = Some(CombatState::new(vec![m], 5));
    state.mode = GameMode::Combat;

    let resp = handle_request(&req("tu1", GMCommand::TurnUndead {
        character: "Brother Marcus".to_string(),
        monster_idx: 0,
    }), &mut state);
    assert_response_format(&resp, "tu1");
    assert!(resp.success);
}

#[test]
fn turn_undead_non_cleric() {
    let mut state = GameState::new();
    setup_combat(&mut state); // Aldric is a Fighter

    let resp = handle_request(&req("tu2", GMCommand::TurnUndead {
        character: "Aldric".to_string(),
        monster_idx: 0,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("cannot turn undead"));
}

#[test]
fn turn_undead_no_combat() {
    let mut state = GameState::new();
    state.party.add_member(make_cleric("Brother Marcus"));

    let resp = handle_request(&req("tu3", GMCommand::TurnUndead {
        character: "Brother Marcus".to_string(),
        monster_idx: 0,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no active combat"));
}

#[test]
fn turn_undead_idx_out_of_range() {
    let mut state = GameState::new();
    state.party.add_member(make_cleric("Brother Marcus"));
    let m = Monster::new("Skeleton", "1");
    state.combat = Some(CombatState::new(vec![m], 5));
    state.mode = GameMode::Combat;

    let resp = handle_request(&req("tu4", GMCommand::TurnUndead {
        character: "Brother Marcus".to_string(),
        monster_idx: 99,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("out of range"));
}

#[test]
fn turn_undead_dead_monster() {
    let mut state = GameState::new();
    state.party.add_member(make_cleric("Brother Marcus"));
    let mut m = Monster::new("Skeleton", "1");
    m.hp = 0;
    state.combat = Some(CombatState::new(vec![m], 5));
    state.mode = GameMode::Combat;

    let resp = handle_request(&req("tu5", GMCommand::TurnUndead {
        character: "Brother Marcus".to_string(),
        monster_idx: 0,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("dead"));
}

#[test]
fn turn_undead_unknown_character() {
    let mut state = GameState::new();
    let m = Monster::new("Skeleton", "1");
    state.combat = Some(CombatState::new(vec![m], 5));
    state.mode = GameMode::Combat;

    let resp = handle_request(&req("tu6", GMCommand::TurnUndead {
        character: "Nobody".to_string(),
        monster_idx: 0,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

// ===========================================================================
// 14. EndCombat
// ===========================================================================

#[test]
fn end_combat_happy_path() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("ec1", GMCommand::EndCombat), &mut state);
    assert_response_format(&resp, "ec1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Idle);
    assert_eq!(state.mode, GameMode::Idle);
    assert!(state.combat.is_none());

    let data = resp.data.unwrap();
    assert!(data["rounds"].as_u64().is_some());
    assert!(data["monsters_defeated"].as_u64().is_some());
    assert!(data["total_xp"].as_u64().is_some());
    assert!(data["party_casualties"].as_u64().is_some());
}

#[test]
fn end_combat_no_combat() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ec2", GMCommand::EndCombat), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no active combat"));
}

#[test]
fn end_combat_xp_counts_dead_only() {
    let mut state = GameState::new();
    setup_combat(&mut state);
    // Kill one monster (5 XP each)
    state.combat.as_mut().unwrap().monsters[0].hp = 0;

    let resp = handle_request(&req("ec3", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["monsters_defeated"], 1);
    assert_eq!(data["total_xp"], 5);
}

// ===========================================================================
// 15. EnterDungeon
// ===========================================================================

#[test]
fn enter_dungeon_happy_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ed1", GMCommand::EnterDungeon {
        level: 3,
        room_name: "Dark Cave".to_string(),
    }), &mut state);
    assert_response_format(&resp, "ed1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Exploration);
    assert_eq!(state.mode, GameMode::Exploration);
    assert_eq!(state.dungeon_level, 3);
    assert!(state.dungeon.is_some());
    assert!(state.time.is_some());
}

#[test]
fn enter_dungeon_level_zero() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ed2", GMCommand::EnterDungeon {
        level: 0,
        room_name: "Test".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("positive integer"));
}

// ===========================================================================
// 16. AdvanceTurn
// ===========================================================================

#[test]
fn advance_turn_happy_path() {
    let mut state = GameState::new();
    setup_exploration(&mut state);

    let resp = handle_request(&req("at1", GMCommand::AdvanceTurn), &mut state);
    assert_response_format(&resp, "at1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert!(data["messages"].as_array().is_some());
    // has_encounter is a boolean
    assert!(data["has_encounter"].as_bool().is_some());
}

#[test]
fn advance_turn_not_exploring() {
    let mut state = GameState::new();
    let resp = handle_request(&req("at2", GMCommand::AdvanceTurn), &mut state);
    assert!(!resp.success);
}

// ===========================================================================
// 17. AddRoom
// ===========================================================================

#[test]
fn add_room_happy_path() {
    let mut state = GameState::new();
    setup_exploration(&mut state);

    let resp = handle_request(&req("ar1", GMCommand::AddRoom {
        id: 1,
        name: "Guard Room".to_string(),
    }), &mut state);
    assert_response_format(&resp, "ar1");
    assert!(resp.success);
    assert!(resp.message.contains("Guard Room"));
}

#[test]
fn add_room_no_dungeon() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ar2", GMCommand::AddRoom {
        id: 1,
        name: "Test".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no dungeon"));
}

#[test]
fn add_room_duplicate_id() {
    let mut state = GameState::new();
    setup_exploration(&mut state);
    // Room 0 already exists from setup

    let resp = handle_request(&req("ar3", GMCommand::AddRoom {
        id: 0,
        name: "Duplicate".to_string(),
    }), &mut state);
    assert!(!resp.success);
}

// ===========================================================================
// 18. AddDoor
// ===========================================================================

#[test]
fn add_door_happy_path() {
    let mut state = GameState::new();
    setup_exploration(&mut state);
    // Add room 1 first
    handle_request(&req("setup", GMCommand::AddRoom {
        id: 1,
        name: "Corridor".to_string(),
    }), &mut state);

    let resp = handle_request(&req("ad1", GMCommand::AddDoor {
        id: 0,
        room_a: 0,
        room_b: 1,
        state: DoorState::Closed,
    }), &mut state);
    assert_response_format(&resp, "ad1");
    assert!(resp.success);
}

#[test]
fn add_door_invalid_state() {
    // Invalid door state is now caught at JSON deserialization.
    let json = r#"{"id":"ad2","command":{"type":"AddDoor","params":{"id":0,"room_a":0,"room_b":1,"state":"broken"}}}"#;
    let result = parse_request(json);
    assert!(result.is_err(), "parsing invalid door state should fail");
}

#[test]
fn add_door_all_valid_states() {
    let mut state = GameState::new();
    setup_exploration(&mut state);
    // Add enough rooms
    for i in 1..=5 {
        handle_request(&req("setup", GMCommand::AddRoom {
            id: i,
            name: format!("Room {}", i),
        }), &mut state);
    }

    let states = [DoorState::Open, DoorState::Closed, DoorState::Stuck, DoorState::Locked, DoorState::Secret];
    for (i, door_state) in states.iter().enumerate() {
        let resp = handle_request(&req(&format!("ad{}", i), GMCommand::AddDoor {
            id: i as u32,
            room_a: i as u32,
            room_b: (i + 1) as u32,
            state: *door_state,
        }), &mut state);
        assert!(resp.success, "door state '{:?}' should be valid: {}", door_state, resp.message);
    }
}

#[test]
fn add_door_no_dungeon() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ad3", GMCommand::AddDoor {
        id: 0,
        room_a: 0,
        room_b: 1,
        state: DoorState::Closed,
    }), &mut state);
    assert!(!resp.success);
}

// ===========================================================================
// 19. MoveRoom
// ===========================================================================

#[test]
fn move_room_happy_path() {
    let mut state = GameState::new();
    setup_exploration(&mut state);
    // Need light to move through doors
    handle_request(&req("s", GMCommand::Light {
        source: LightSourceKind::Torch,
        carrier: "Aldric".to_string(),
    }), &mut state);
    handle_request(&req("s", GMCommand::AddRoom { id: 1, name: "Hall".to_string() }), &mut state);
    handle_request(&req("s", GMCommand::AddDoor {
        id: 0, room_a: 0, room_b: 1, state: DoorState::Open,
    }), &mut state);

    let resp = handle_request(&req("mr1", GMCommand::MoveRoom { door_id: 0 }), &mut state);
    assert_response_format(&resp, "mr1");
    assert!(resp.success, "move room failed: {}", resp.message);
}

#[test]
fn move_room_not_exploring() {
    let mut state = GameState::new();
    let resp = handle_request(&req("mr2", GMCommand::MoveRoom { door_id: 0 }), &mut state);
    assert!(!resp.success);
}

// ===========================================================================
// 20. Search
// ===========================================================================

#[test]
fn search_happy_path() {
    let mut state = GameState::new();
    setup_exploration(&mut state);

    let resp = handle_request(&req("s1", GMCommand::Search { is_elf: false }), &mut state);
    assert_response_format(&resp, "s1");
    assert!(resp.success);
}

#[test]
fn search_as_elf() {
    let mut state = GameState::new();
    setup_exploration(&mut state);

    let resp = handle_request(&req("s2", GMCommand::Search { is_elf: true }), &mut state);
    assert_response_format(&resp, "s2");
    assert!(resp.success);
}

#[test]
fn search_not_exploring() {
    let mut state = GameState::new();
    let resp = handle_request(&req("s3", GMCommand::Search { is_elf: false }), &mut state);
    assert!(!resp.success);
}

// ===========================================================================
// 21. Light
// ===========================================================================

#[test]
fn light_torch_happy_path() {
    let mut state = GameState::new();
    setup_exploration(&mut state);

    let resp = handle_request(&req("l1", GMCommand::Light {
        source: LightSourceKind::Torch,
        carrier: "Aldric".to_string(),
    }), &mut state);
    assert_response_format(&resp, "l1");
    assert!(resp.success);
    assert!(resp.message.contains("torch"));
}

#[test]
fn light_lantern_happy_path() {
    let mut state = GameState::new();
    setup_exploration(&mut state);

    let resp = handle_request(&req("l2", GMCommand::Light {
        source: LightSourceKind::Lantern,
        carrier: "Aldric".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert!(resp.message.contains("lantern"));
}

#[test]
fn light_invalid_source() {
    // Invalid light source is now caught at JSON deserialization.
    let json = r#"{"id":"l3","command":{"type":"Light","params":{"source":"candle","carrier":"Aldric"}}}"#;
    let result = parse_request(json);
    assert!(result.is_err(), "parsing invalid light source should fail");
}

#[test]
fn light_not_exploring() {
    let mut state = GameState::new();
    let resp = handle_request(&req("l4", GMCommand::Light {
        source: LightSourceKind::Torch,
        carrier: "Aldric".to_string(),
    }), &mut state);
    assert!(!resp.success);
}

// ===========================================================================
// 22. EnterWilderness
// ===========================================================================

#[test]
fn enter_wilderness_happy_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ew1", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert_response_format(&resp, "ew1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Wilderness);
    assert_eq!(state.mode, GameMode::Wilderness);
    assert!(state.wilderness.is_some());
}

#[test]
fn enter_wilderness_invalid_terrain() {
    // Invalid terrain is now caught at JSON deserialization.
    let json = r#"{"id":"ew2","command":{"type":"EnterWilderness","params":{"terrain":"lava"}}}"#;
    let result = parse_request(json);
    assert!(result.is_err(), "parsing invalid terrain should fail");
}

#[test]
fn enter_wilderness_all_terrain_types() {
    let terrains = [
        Terrain::Clear, Terrain::Forest, Terrain::Hills, Terrain::Mountains,
        Terrain::Desert, Terrain::Swamp, Terrain::Jungle, Terrain::Ocean,
        Terrain::River, Terrain::Barren, Terrain::City,
    ];
    for terrain in &terrains {
        let mut state = GameState::new();
        let resp = handle_request(&req("ew", GMCommand::EnterWilderness {
            terrain: *terrain,
        }), &mut state);
        assert!(resp.success, "terrain '{:?}' should be valid: {}", terrain, resp.message);
        assert_eq!(state.mode, GameMode::Wilderness);
    }
}

// ===========================================================================
// 23. AddHex
// ===========================================================================

#[test]
fn add_hex_happy_path() {
    let mut state = GameState::new();
    setup_wilderness(&mut state);

    let resp = handle_request(&req("ah1", GMCommand::AddHex {
        x: 1, y: 0, terrain: Terrain::Hills,
    }), &mut state);
    assert_response_format(&resp, "ah1");
    assert!(resp.success);
}

#[test]
fn add_hex_not_in_wilderness() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ah2", GMCommand::AddHex {
        x: 1, y: 0, terrain: Terrain::Hills,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("not in wilderness"));
}

#[test]
fn add_hex_invalid_terrain() {
    // Invalid terrain is now caught at JSON deserialization.
    let json = r#"{"id":"ah3","command":{"type":"AddHex","params":{"x":1,"y":0,"terrain":"lava"}}}"#;
    let result = parse_request(json);
    assert!(result.is_err(), "parsing invalid terrain should fail");
}

#[test]
fn add_hex_duplicate() {
    let mut state = GameState::new();
    setup_wilderness(&mut state);

    // (0,0) already exists from setup
    let resp = handle_request(&req("ah4", GMCommand::AddHex {
        x: 0, y: 0, terrain: Terrain::Hills,
    }), &mut state);
    assert!(!resp.success);
}

// ===========================================================================
// 24. Travel
// ===========================================================================

#[test]
fn travel_happy_path() {
    let mut state = GameState::new();
    setup_wilderness(&mut state);
    // Add destination hex
    handle_request(&req("s", GMCommand::AddHex {
        x: 1, y: 0, terrain: Terrain::Clear,
    }), &mut state);

    let resp = handle_request(&req("t1", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert_response_format(&resp, "t1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert!(data["messages"].as_array().is_some());
    assert!(data["lost"].as_bool().is_some());
    assert!(data["has_encounter"].as_bool().is_some());
    assert!(data["encounters"].as_array().is_some());
}

#[test]
fn travel_not_in_wilderness() {
    let mut state = GameState::new();
    let resp = handle_request(&req("t2", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("not in wilderness"));
}

// ===========================================================================
// 25. RollSurprise
// ===========================================================================

#[test]
fn roll_surprise_happy_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("rs1", GMCommand::RollSurprise), &mut state);
    assert_response_format(&resp, "rs1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    let party_roll = data["party_roll"].as_u64().unwrap();
    let monster_roll = data["monster_roll"].as_u64().unwrap();
    assert!((1..=6).contains(&party_roll));
    assert!((1..=6).contains(&monster_roll));
    let result = data["result"].as_str().unwrap();
    assert!(!result.is_empty());
}

// ===========================================================================
// 26. RollReaction
// ===========================================================================

#[test]
fn roll_reaction_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("rr1", GMCommand::RollReaction {
        character: "Aldric".to_string(),
    }), &mut state);
    assert_response_format(&resp, "rr1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert_eq!(data["character"], "Aldric");
    assert_eq!(data["charisma"], 12);
    assert!(data["raw_roll"].as_i64().is_some());
    assert!(data["modified_roll"].as_i64().is_some());
    assert!(data["reaction"].as_str().is_some());
    assert!(data["cha_modifier"].as_i64().is_some());
}

#[test]
fn roll_reaction_unknown_character() {
    let mut state = GameState::new();
    let resp = handle_request(&req("rr2", GMCommand::RollReaction {
        character: "Nobody".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

// ===========================================================================
// 27. AwardXp
// ===========================================================================

#[test]
fn award_xp_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("ax1", GMCommand::AwardXp {
        character: "Aldric".to_string(),
        xp: 100,
    }), &mut state);
    assert_response_format(&resp, "ax1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert_eq!(data["character"], "Aldric");
    assert_eq!(data["xp_awarded"], 100);
    assert_eq!(data["total_xp"], 100);

    // Verify state
    assert_eq!(state.party.find_member("Aldric").unwrap().xp, 100);
}

#[test]
fn award_xp_unknown_character() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ax2", GMCommand::AwardXp {
        character: "Nobody".to_string(),
        xp: 100,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

#[test]
fn award_xp_cumulative() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    handle_request(&req("1", GMCommand::AwardXp {
        character: "Aldric".to_string(), xp: 50,
    }), &mut state);
    let resp = handle_request(&req("2", GMCommand::AwardXp {
        character: "Aldric".to_string(), xp: 75,
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["total_xp"], 125);
}

// ===========================================================================
// 28. AwardTreasureXp
// ===========================================================================

#[test]
fn award_treasure_xp_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("atx1", GMCommand::AwardTreasureXp {
        character: "Aldric".to_string(),
        treasure_gp: 500,
        monster_xp: 100,
    }), &mut state);
    assert_response_format(&resp, "atx1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert_eq!(data["character"], "Aldric");
    assert!(data["base_xp"].as_u64().is_some());
    assert!(data["modifier_pct"].as_i64().is_some());
    assert!(data["adjusted_xp"].as_u64().is_some());
    assert!(data["total_xp"].as_u64().is_some());
    assert!(data["leveled_up"].as_bool().is_some());
    assert!(data["new_level"].as_u64().is_some());
    assert!(data["hp_gained"].as_i64().is_some());
    // Fighter with STR 16 gets +10%
    assert_eq!(data["modifier_pct"], 10);
}

#[test]
fn award_treasure_xp_unknown_character() {
    let mut state = GameState::new();
    let resp = handle_request(&req("atx2", GMCommand::AwardTreasureXp {
        character: "Nobody".to_string(),
        treasure_gp: 100,
        monster_xp: 50,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

// ===========================================================================
// 29. ThiefSkillCheck
// ===========================================================================

#[test]
fn thief_skill_check_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Shade"));

    let resp = handle_request(&req("tsc1", GMCommand::ThiefSkillCheck {
        character: "Shade".to_string(),
        skill: "open locks".to_string(),
    }), &mut state);
    assert_response_format(&resp, "tsc1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert_eq!(data["character"], "Shade");
    assert_eq!(data["skill"], "Open Locks");
    assert_eq!(data["target"], 15);
    assert!(data["roll"].as_u64().is_some());
    assert!(data["success"].as_bool().is_some());
}

#[test]
fn thief_skill_check_non_thief() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("tsc2", GMCommand::ThiefSkillCheck {
        character: "Aldric".to_string(),
        skill: "open locks".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("does not have thief skills"));
}

#[test]
fn thief_skill_check_unknown_skill() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Shade"));

    let resp = handle_request(&req("tsc3", GMCommand::ThiefSkillCheck {
        character: "Shade".to_string(),
        skill: "fly".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("unknown thief skill"));
}

#[test]
fn thief_skill_check_unknown_character() {
    let mut state = GameState::new();
    let resp = handle_request(&req("tsc4", GMCommand::ThiefSkillCheck {
        character: "Nobody".to_string(),
        skill: "open locks".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

#[test]
fn thief_skill_check_all_skills() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Shade"));

    let skills = [
        "climb walls", "find traps", "hear noise", "hide in shadows",
        "move silently", "open locks", "pick pockets", "read languages",
    ];
    for skill in &skills {
        let resp = handle_request(&req("tsc", GMCommand::ThiefSkillCheck {
            character: "Shade".to_string(),
            skill: skill.to_string(),
        }), &mut state);
        assert!(resp.success, "skill '{}' should work: {}", skill, resp.message);
    }
}

#[test]
fn thief_skill_check_aliases() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Shade"));

    // Test various aliases
    let aliases = [
        ("climb", "Climb Walls"),
        ("traps", "Find Traps"),
        ("listen", "Hear Noise"),
        ("hide", "Hide in Shadows"),
        ("sneak", "Move Silently"),
        ("stealth", "Move Silently"),
        ("pick", "Open Locks"),
        ("lockpick", "Open Locks"),
        ("steal", "Pick Pockets"),
        ("read", "Read Languages"),
    ];
    for (alias, expected_name) in &aliases {
        let resp = handle_request(&req("tsc", GMCommand::ThiefSkillCheck {
            character: "Shade".to_string(),
            skill: alias.to_string(),
        }), &mut state);
        assert!(resp.success, "alias '{}' should work: {}", alias, resp.message);
        let data = resp.data.unwrap();
        assert_eq!(data["skill"], *expected_name, "alias '{}' should map to '{}'", alias, expected_name);
    }
}

// ===========================================================================
// 30. Backstab
// ===========================================================================

#[test]
fn backstab_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Shade"));
    let mut m = Monster::new("Goblin", "1");
    m.hp = 3;
    m.max_hp = 3;
    m.ac = 6;
    m.damage = "1d6".to_string();
    m.morale = 7;
    m.attacks = vec!["attack".to_string()];
    state.combat = Some(CombatState::new(vec![m], 5));
    state.mode = GameMode::Combat;

    let resp = handle_request(&req("bs1", GMCommand::Backstab {
        character: "Shade".to_string(),
        monster_idx: 0,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert_response_format(&resp, "bs1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert!(data["hit"].as_bool().is_some());
    assert!(data["attack_roll"].as_i64().is_some());
    assert!(data["target_number"].as_i64().is_some());
    if data["hit"].as_bool().unwrap() {
        assert_eq!(data["multiplier"], 2); // Level 1 thief = x2
        assert!(data["damage"].as_i64().unwrap() > 0);
    }
}

#[test]
fn backstab_non_thief() {
    let mut state = GameState::new();
    setup_combat(&mut state); // Aldric is Fighter

    let resp = handle_request(&req("bs2", GMCommand::Backstab {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("cannot backstab"));
}

#[test]
fn backstab_no_combat() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Shade"));

    let resp = handle_request(&req("bs3", GMCommand::Backstab {
        character: "Shade".to_string(),
        monster_idx: 0,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no active combat"));
}

#[test]
fn backstab_idx_out_of_range() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Shade"));
    let m = Monster::new("Goblin", "1");
    state.combat = Some(CombatState::new(vec![m], 5));
    state.mode = GameMode::Combat;

    let resp = handle_request(&req("bs4", GMCommand::Backstab {
        character: "Shade".to_string(),
        monster_idx: 99,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("out of range"));
}

#[test]
fn backstab_dead_monster() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Shade"));
    let mut m = Monster::new("Goblin", "1");
    m.hp = 0;
    state.combat = Some(CombatState::new(vec![m], 5));
    state.mode = GameMode::Combat;

    let resp = handle_request(&req("bs5", GMCommand::Backstab {
        character: "Shade".to_string(),
        monster_idx: 0,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("dead"));
}

#[test]
fn backstab_unknown_weapon() {
    let mut state = GameState::new();
    state.party.add_member(make_thief("Shade"));
    let m = Monster::new("Goblin", "1");
    state.combat = Some(CombatState::new(vec![m], 5));
    state.mode = GameMode::Combat;

    let resp = handle_request(&req("bs6", GMCommand::Backstab {
        character: "Shade".to_string(),
        monster_idx: 0,
        weapon: "lightsaber".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("unknown weapon"));
}

#[test]
fn backstab_unknown_character() {
    let mut state = GameState::new();
    let m = Monster::new("Goblin", "1");
    state.combat = Some(CombatState::new(vec![m], 5));
    state.mode = GameMode::Combat;

    let resp = handle_request(&req("bs7", GMCommand::Backstab {
        character: "Nobody".to_string(),
        monster_idx: 0,
        weapon: "dagger".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

// ===========================================================================
// 31. QueryEncumbrance
// ===========================================================================

#[test]
fn query_encumbrance_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("qe1", GMCommand::QueryEncumbrance {
        character: "Aldric".to_string(),
    }), &mut state);
    assert_response_format(&resp, "qe1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert_eq!(data["character"], "Aldric");
    assert!(data["total_weight_cn"].as_u64().is_some());
    assert!(data["encumbrance_level"].as_str().is_some());
    assert!(data["movement_rate"].as_u64().is_some());
    assert!(data["max_capacity"].as_u64().is_some());
}

#[test]
fn query_encumbrance_unknown_character() {
    let mut state = GameState::new();
    let resp = handle_request(&req("qe2", GMCommand::QueryEncumbrance {
        character: "Nobody".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

// ===========================================================================
// 32. SpawnMonster
// ===========================================================================

#[test]
fn spawn_monster_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("sm1", GMCommand::SpawnMonster {
        name: "Goblin".to_string(),
        count: 3,
        distance: 30,
    }), &mut state);
    assert_response_format(&resp, "sm1");
    assert!(resp.success);
    assert_eq!(resp.mode, GameMode::Combat);

    let data = resp.data.unwrap();
    assert_eq!(data["monster"], "Goblin");
    assert!(data["hit_dice"].as_str().is_some());
    assert!(data["ac"].as_i64().is_some());
}

#[test]
fn spawn_monster_unknown() {
    let mut state = GameState::new();
    let resp = handle_request(&req("sm2", GMCommand::SpawnMonster {
        name: "Beholder".to_string(),
        count: 1,
        distance: 60,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("unknown monster"));
}

#[test]
fn spawn_monster_combat_already_active() {
    let mut state = GameState::new();
    setup_combat(&mut state);

    let resp = handle_request(&req("sm3", GMCommand::SpawnMonster {
        name: "Orc".to_string(),
        count: 1,
        distance: 60,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("already active"));
}

// ===========================================================================
// 33. LookupSpell
// ===========================================================================

#[test]
fn lookup_spell_happy_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ls1", GMCommand::LookupSpell {
        name: "Magic Missile".to_string(),
        list: String::new(),
    }), &mut state);
    assert_response_format(&resp, "ls1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert_eq!(data["name"], "Magic Missile");
    assert_eq!(data["level"], 1);
    assert_eq!(data["list"], "Magic-User");
    assert!(data["range"].as_str().is_some());
    assert!(data["duration"].as_str().is_some());
    assert!(data["description"].as_str().is_some());
}

#[test]
fn lookup_spell_not_found() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ls2", GMCommand::LookupSpell {
        name: "Nonexistent Spell".to_string(),
        list: String::new(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("not found"));
}

#[test]
fn lookup_spell_unknown_list() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ls3", GMCommand::LookupSpell {
        name: "Magic Missile".to_string(),
        list: "necromancer".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("unknown spell list"));
}

#[test]
fn lookup_spell_filter_by_cleric() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ls4", GMCommand::LookupSpell {
        name: "Cure Light Wounds".to_string(),
        list: "cleric".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    assert_eq!(data["list"], "Cleric");
}

#[test]
fn lookup_spell_list_aliases() {
    let mut state = GameState::new();
    // Test magic-user aliases
    let aliases = ["magicuser", "magic-user", "magic_user", "mu", "mage"];
    for alias in &aliases {
        let resp = handle_request(&req("ls", GMCommand::LookupSpell {
            name: "Magic Missile".to_string(),
            list: alias.to_string(),
        }), &mut state);
        assert!(resp.success, "spell list alias '{}' should work: {}", alias, resp.message);
    }
}

// ===========================================================================
// 34. HireRetainer
// ===========================================================================

#[test]
fn hire_retainer_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_cleric("Father Gregory"));

    let resp = handle_request(&req("hr1", GMCommand::HireRetainer {
        employer: "Father Gregory".to_string(),
        retainer_name: "Hrothgar".to_string(),
        retainer_class: Class::Fighter,
        retainer_level: 1,
    }), &mut state);
    assert_response_format(&resp, "hr1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert_eq!(data["employer"], "Father Gregory");
    assert_eq!(data["retainer"], "Hrothgar");
    assert_eq!(data["class"], "Fighter");
    assert_eq!(data["level"], 1);
    assert!(data["hired"].as_bool().is_some());
    assert!(data["loyalty"].as_u64().is_some());
    assert!(data["wage_gp"].as_u64().is_some());
    assert!(data["max_retainers"].as_u64().is_some());
    assert!(data["reaction"].as_str().is_some());
}

#[test]
fn hire_retainer_unknown_employer() {
    let mut state = GameState::new();
    let resp = handle_request(&req("hr2", GMCommand::HireRetainer {
        employer: "Nobody".to_string(),
        retainer_name: "Hrothgar".to_string(),
        retainer_class: Class::Fighter,
        retainer_level: 1,
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

// ===========================================================================
// 35. LoyaltyCheck
// ===========================================================================

#[test]
fn loyalty_check_happy_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("lc1", GMCommand::LoyaltyCheck {
        retainer_name: "Hrothgar".to_string(),
        loyalty: 8,
    }), &mut state);
    assert_response_format(&resp, "lc1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert_eq!(data["retainer"], "Hrothgar");
    assert_eq!(data["loyalty"], 8);
    let result = data["result"].as_str().unwrap();
    assert!(["Loyal", "Wavering", "Disloyal"].contains(&result));
}

// ===========================================================================
// 36. LevelUp
// ===========================================================================

#[test]
fn level_up_happy_path() {
    let mut state = GameState::new();
    let mut fighter = make_fighter("Veteran");
    fighter.xp = 2500;
    state.party.add_member(fighter);

    let resp = handle_request(&req("lu1", GMCommand::LevelUp {
        character: "Veteran".to_string(),
    }), &mut state);
    assert_response_format(&resp, "lu1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    assert_eq!(data["new_level"], 2);
    assert!(data["hp_gained"].as_i64().unwrap() > 0);
    assert_eq!(state.party.find_member("Veteran").unwrap().level, 2);
}

#[test]
fn level_up_not_enough_xp() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Rookie"));

    let resp = handle_request(&req("lu2", GMCommand::LevelUp {
        character: "Rookie".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("needs"));
}

#[test]
fn level_up_unknown_character() {
    let mut state = GameState::new();
    let resp = handle_request(&req("lu3", GMCommand::LevelUp {
        character: "Nobody".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("no party member"));
}

#[test]
fn level_up_double() {
    let mut state = GameState::new();
    let mut fighter = make_fighter("Veteran");
    fighter.xp = 2500;
    state.party.add_member(fighter);

    // First level up succeeds
    let resp = handle_request(&req("lu4a", GMCommand::LevelUp {
        character: "Veteran".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Second level up fails (not enough XP for level 3)
    let resp = handle_request(&req("lu4b", GMCommand::LevelUp {
        character: "Veteran".to_string(),
    }), &mut state);
    assert!(!resp.success);
}

// ===========================================================================
// 37. Ruling
// ===========================================================================

#[test]
fn ruling_happy_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("r1", GMCommand::Ruling {
        text: "The bridge can hold 3 people at once.".to_string(),
    }), &mut state);
    assert_response_format(&resp, "r1");
    assert!(resp.success);
    assert!(resp.message.contains("ruling recorded"));
    assert_eq!(state.notes.len(), 1);
    assert!(state.notes[0].contains("[RULING]"));
    assert!(state.notes[0].contains("bridge"));
}

#[test]
fn ruling_multiple() {
    let mut state = GameState::new();
    handle_request(&req("r1", GMCommand::Ruling {
        text: "First ruling.".to_string(),
    }), &mut state);
    handle_request(&req("r2", GMCommand::Ruling {
        text: "Second ruling.".to_string(),
    }), &mut state);
    assert_eq!(state.notes.len(), 2);
}

// ===========================================================================
// 38. Save
// ===========================================================================

#[test]
fn save_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let path = unique_tmp_path("qa_save");
    let resp = handle_request(&req("sv1", GMCommand::Save {
        path: path.to_string(),
    }), &mut state);
    assert_response_format(&resp, "sv1");
    assert!(resp.success);
    assert!(resp.message.contains("saved"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn save_invalid_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("sv2", GMCommand::Save {
        path: "/nonexistent/dir/save.json".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("save failed"));
}

// ===========================================================================
// 39. Load
// ===========================================================================

#[test]
fn load_happy_path() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let path = unique_tmp_path("qa_load");
    let resp = handle_request(&req("1", GMCommand::Save {
        path: path.to_string(),
    }), &mut state);
    assert!(resp.success);

    let mut new_state = GameState::new();
    let resp = handle_request(&req("ld1", GMCommand::Load {
        path: path.to_string(),
    }), &mut new_state);
    assert_response_format(&resp, "ld1");
    assert!(resp.success);
    assert_eq!(new_state.party.members.len(), 1);
    assert_eq!(new_state.party.members[0].name, "Aldric");

    let _ = std::fs::remove_file(path);
}

#[test]
fn load_invalid_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ld2", GMCommand::Load {
        path: "/nonexistent/save.json".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.unwrap().contains("load failed"));
}

// ===========================================================================
// 40. Roll
// ===========================================================================

#[test]
fn roll_valid() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ro1", GMCommand::Roll {
        notation: "3d6".to_string(),
    }), &mut state);
    assert_response_format(&resp, "ro1");
    assert!(resp.success);

    let data = resp.data.unwrap();
    let total = data["total"].as_i64().unwrap();
    assert!((3..=18).contains(&total));
}

#[test]
fn roll_with_modifier() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ro2", GMCommand::Roll {
        notation: "1d20+5".to_string(),
    }), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    let total = data["total"].as_i64().unwrap();
    assert!((6..=25).contains(&total));
}

#[test]
fn roll_invalid_notation() {
    let mut state = GameState::new();
    let resp = handle_request(&req("ro3", GMCommand::Roll {
        notation: "abc".to_string(),
    }), &mut state);
    assert!(!resp.success);
    assert!(resp.error.is_some());
}

// ===========================================================================
// 41. Quit
// ===========================================================================

#[test]
fn quit_happy_path() {
    let mut state = GameState::new();
    let resp = handle_request(&req("q1", GMCommand::Quit), &mut state);
    assert_response_format(&resp, "q1");
    assert!(resp.success);
    assert!(resp.message.contains("ended"));
}

// ===========================================================================
// Cross-cutting: Invalid command sequences
// ===========================================================================

#[test]
fn attack_outside_combat() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));
    assert_eq!(state.mode, GameMode::Idle);

    let resp = handle_request(&req("x1", GMCommand::Attack {
        character: "Aldric".to_string(),
        monster_idx: 0,
        weapon: "sword".to_string(),
    }), &mut state);
    assert!(!resp.success);
}

#[test]
fn advance_turn_outside_exploration() {
    let mut state = GameState::new();
    assert_eq!(state.mode, GameMode::Idle);

    let resp = handle_request(&req("x2", GMCommand::AdvanceTurn), &mut state);
    assert!(!resp.success);
}

#[test]
fn travel_outside_wilderness() {
    let mut state = GameState::new();
    assert_eq!(state.mode, GameMode::Idle);

    let resp = handle_request(&req("x3", GMCommand::Travel { x: 1, y: 0 }), &mut state);
    assert!(!resp.success);
}

#[test]
fn search_outside_exploration() {
    let mut state = GameState::new();
    assert_eq!(state.mode, GameMode::Idle);

    let resp = handle_request(&req("x4", GMCommand::Search { is_elf: false }), &mut state);
    assert!(!resp.success);
}

#[test]
fn move_room_outside_exploration() {
    let mut state = GameState::new();
    let resp = handle_request(&req("x5", GMCommand::MoveRoom { door_id: 0 }), &mut state);
    assert!(!resp.success);
}

// ===========================================================================
// Cross-cutting: Command idempotency
// ===========================================================================

#[test]
fn double_quit() {
    let mut state = GameState::new();
    let resp1 = handle_request(&req("dq1", GMCommand::Quit), &mut state);
    let resp2 = handle_request(&req("dq2", GMCommand::Quit), &mut state);
    assert!(resp1.success);
    assert!(resp2.success);
}

#[test]
fn double_save() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let path = unique_tmp_path("qa_double_save");
    let resp1 = handle_request(&req("ds1", GMCommand::Save {
        path: path.to_string(),
    }), &mut state);
    let resp2 = handle_request(&req("ds2", GMCommand::Save {
        path: path.to_string(),
    }), &mut state);
    assert!(resp1.success);
    assert!(resp2.success);

    let _ = std::fs::remove_file(path);
}

#[test]
fn double_query_state() {
    let mut state = GameState::new();
    let resp1 = handle_request(&req("1", GMCommand::QueryState), &mut state);
    let resp2 = handle_request(&req("2", GMCommand::QueryState), &mut state);
    assert!(resp1.success);
    assert!(resp2.success);
    // Data should be identical
    assert_eq!(resp1.data, resp2.data);
}

// ===========================================================================
// Cross-cutting: Rapid command sequences and state consistency
// ===========================================================================

#[test]
fn rapid_exploration_sequence() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    // Enter dungeon
    let resp = handle_request(&req("1", GMCommand::EnterDungeon {
        level: 2,
        room_name: "Start".to_string(),
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Exploration);

    // Light torch
    let resp = handle_request(&req("2", GMCommand::Light {
        source: LightSourceKind::Torch,
        carrier: "Aldric".to_string(),
    }), &mut state);
    assert!(resp.success);

    // Add rooms and doors rapidly
    for i in 1..=5 {
        let resp = handle_request(&req(&format!("r{}", i), GMCommand::AddRoom {
            id: i,
            name: format!("Room {}", i),
        }), &mut state);
        assert!(resp.success, "add room {} failed: {}", i, resp.message);
    }

    for i in 0..4 {
        let resp = handle_request(&req(&format!("d{}", i), GMCommand::AddDoor {
            id: i,
            room_a: i,
            room_b: i + 1,
            state: DoorState::Open,
        }), &mut state);
        assert!(resp.success, "add door {} failed: {}", i, resp.message);
    }

    // Move through doors
    for i in 0..4 {
        let resp = handle_request(&req(&format!("m{}", i), GMCommand::MoveRoom {
            door_id: i,
        }), &mut state);
        assert!(resp.success, "move through door {} failed: {}", i, resp.message);
    }

    // Search each room
    let resp = handle_request(&req("s", GMCommand::Search { is_elf: false }), &mut state);
    assert!(resp.success);

    // Advance turns
    for i in 0..5 {
        let resp = handle_request(&req(&format!("t{}", i), GMCommand::AdvanceTurn), &mut state);
        assert!(resp.success, "advance turn {} failed: {}", i, resp.message);
    }

    // State should still be consistent
    assert_eq!(state.mode, GameMode::Exploration);
    assert_eq!(state.dungeon_level, 2);
}

#[test]
fn rapid_combat_sequence() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));
    state.party.add_member(make_thief("Shade"));
    state.party.add_member(make_cleric("Marcus"));

    // Spawn combat
    let resp = handle_request(&req("1", GMCommand::SpawnMonster {
        name: "Goblin".to_string(),
        count: 4,
        distance: 5,
    }), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Combat);

    // Roll initiative
    let resp = handle_request(&req("2", GMCommand::RollInitiative), &mut state);
    assert!(resp.success);

    // Rapid attacks
    for i in 0..4 {
        let idx = i % state.combat.as_ref().unwrap().monsters.len();
        if state.combat.as_ref().unwrap().monsters[idx].is_alive() {
            let resp = handle_request(&req(&format!("a{}", i), GMCommand::Attack {
                character: "Aldric".to_string(),
                monster_idx: idx,
                weapon: "sword".to_string(),
            }), &mut state);
            assert!(resp.success, "attack {} failed: {}", i, resp.message);
        }
    }

    // Query combat state to verify consistency
    let resp = handle_request(&req("qc", GMCommand::QueryCombat), &mut state);
    assert!(resp.success);
    let data = resp.data.unwrap();
    let monsters = data["monsters"].as_array().unwrap();
    assert_eq!(monsters.len(), 4);

    // End combat
    let resp = handle_request(&req("end", GMCommand::EndCombat), &mut state);
    assert!(resp.success);
    assert_eq!(state.mode, GameMode::Idle);
    assert!(state.combat.is_none());
}

// ===========================================================================
// Response format: JSON serialization roundtrip
// ===========================================================================

#[test]
fn response_serializes_correctly() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    let resp = handle_request(&req("json1", GMCommand::QueryParty), &mut state);
    // Serialize to JSON and back
    let json = serde_json::to_string(&resp).unwrap();
    let deser: GMResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(resp.id, deser.id);
    assert_eq!(resp.success, deser.success);
    assert_eq!(resp.mode, deser.mode);
    assert_eq!(resp.message, deser.message);
    assert_eq!(resp.data, deser.data);
    assert_eq!(resp.error, deser.error);
}

#[test]
fn error_response_serializes_correctly() {
    let mut state = GameState::new();
    let resp = handle_request(&req("json2", GMCommand::QueryCombat), &mut state);
    assert!(!resp.success);

    let json = serde_json::to_string(&resp).unwrap();
    let deser: GMResponse = serde_json::from_str(&json).unwrap();
    assert!(!deser.success);
    assert!(deser.error.is_some());
}

// ===========================================================================
// Response ID correlation
// ===========================================================================

#[test]
fn response_id_matches_request() {
    let mut state = GameState::new();
    let commands: Vec<(&str, GMCommand)> = vec![
        ("id-qs", GMCommand::QueryState),
        ("id-qm", GMCommand::QueryMode),
        ("id-qp", GMCommand::QueryParty),
        ("id-rs", GMCommand::RollSurprise),
        ("id-quit", GMCommand::Quit),
        ("id-roll", GMCommand::Roll { notation: "1d6".to_string() }),
    ];

    for (id, cmd) in commands {
        let resp = handle_request(&req(id, cmd), &mut state);
        assert_eq!(resp.id, id, "response ID should match request ID");
    }
}

// ===========================================================================
// Mode field accuracy after state transitions
// ===========================================================================

#[test]
fn mode_transitions() {
    let mut state = GameState::new();
    state.party.add_member(make_fighter("Aldric"));

    // Idle -> Exploration
    let resp = handle_request(&req("1", GMCommand::EnterDungeon {
        level: 1, room_name: "Start".to_string(),
    }), &mut state);
    assert_eq!(resp.mode, GameMode::Exploration);

    // Still Exploration after queries
    let resp = handle_request(&req("2", GMCommand::QueryMode), &mut state);
    assert_eq!(resp.mode, GameMode::Exploration);

    // Exploration -> Combat (via SpawnMonster)
    let resp = handle_request(&req("3", GMCommand::SpawnMonster {
        name: "Goblin".to_string(), count: 1, distance: 5,
    }), &mut state);
    assert_eq!(resp.mode, GameMode::Combat);

    // Combat -> Idle (via EndCombat)
    let resp = handle_request(&req("4", GMCommand::EndCombat), &mut state);
    assert_eq!(resp.mode, GameMode::Idle);

    // Idle -> Wilderness
    let resp = handle_request(&req("5", GMCommand::EnterWilderness {
        terrain: Terrain::Forest,
    }), &mut state);
    assert_eq!(resp.mode, GameMode::Wilderness);
}
