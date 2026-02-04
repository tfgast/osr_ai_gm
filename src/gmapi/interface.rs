use crate::dice;
use crate::engine::{chargen, combat, encounter_engine, exploration, wilderness_engine};
use crate::gmapi::protocol::{GMCommand, GMRequest, GMResponse};
use crate::model::{CombatState, Monster};
use crate::persist::{self, GameState};
use crate::rules::{ability, class, equipment};
use crate::rules::class::Class;
use crate::state::dungeon::{Door, DoorState, DungeonState, Room};
use crate::state::game::GameMode;
use crate::state::time::{LightSourceKind, TimeTracker};
use crate::state::wilderness::{HexCell, Terrain, WildernessState};

/// Process a GMRequest against the current GameState and return a GMResponse.
pub fn handle_request(req: &GMRequest, state: &mut GameState) -> GMResponse {
    let id = &req.id;
    match &req.command {
        // -- State queries --
        GMCommand::QueryState => query_state(id, state),
        GMCommand::QueryMode => GMResponse::ok_with_data(
            id, format!("current mode: {}", state.mode), state.mode.clone(),
            serde_json::json!({ "mode": format!("{}", state.mode) }),
        ),
        GMCommand::QueryParty => query_party(id, state),
        GMCommand::QueryCombat => query_combat(id, state),
        GMCommand::QueryExploration => query_exploration(id, state),
        GMCommand::QueryWilderness => query_wilderness(id, state),

        // -- Character management --
        GMCommand::CreateCharacter { name, class, alignment } => {
            create_character(id, state, name, class, alignment)
        }

        // -- Combat --
        GMCommand::SpawnEncounter { name, count, hit_dice, ac, hp, damage, morale, distance } => {
            spawn_encounter(id, state, name, *count, hit_dice, *ac, *hp, damage, *morale, *distance)
        }
        GMCommand::RollInitiative => roll_initiative(id, state),
        GMCommand::Attack { character, monster_idx, weapon } => {
            attack(id, state, character, *monster_idx, weapon)
        }
        GMCommand::MonsterAttack { monster_idx, character } => {
            monster_attack(id, state, *monster_idx, character)
        }
        GMCommand::CheckMorale => check_morale(id, state),
        GMCommand::TurnUndead { character, monster_idx } => {
            turn_undead(id, state, character, *monster_idx)
        }
        GMCommand::EndCombat => end_combat(id, state),

        // -- Exploration --
        GMCommand::EnterDungeon { level, room_name } => {
            enter_dungeon(id, state, *level, room_name)
        }
        GMCommand::AdvanceTurn => advance_turn(id, state),
        GMCommand::AddRoom { id: room_id, name } => add_room(id, state, *room_id, name),
        GMCommand::AddDoor { id: door_id, room_a, room_b, state: door_state } => {
            add_door(id, state, *door_id, *room_a, *room_b, door_state)
        }
        GMCommand::MoveRoom { door_id } => move_room(id, state, *door_id),
        GMCommand::Search { is_elf } => search(id, state, *is_elf),
        GMCommand::Light { source, carrier } => light(id, state, source, carrier),

        // -- Wilderness --
        GMCommand::EnterWilderness { terrain } => enter_wilderness(id, state, terrain),
        GMCommand::AddHex { x, y, terrain } => add_hex(id, state, *x, *y, terrain),
        GMCommand::Travel { x, y } => travel(id, state, *x, *y),

        // -- Encounter resolution --
        GMCommand::RollSurprise => roll_surprise(id, state),
        GMCommand::RollReaction { character } => roll_reaction(id, state, character),

        // -- Management --
        GMCommand::AwardXp { character, xp } => award_xp(id, state, character, *xp),
        GMCommand::Ruling { text } => ruling(id, state, text),

        // -- System --
        GMCommand::Save { path } => save_game(id, state, path),
        GMCommand::Load { path } => load_game(id, state, path),
        GMCommand::Roll { notation } => roll_dice(id, state, notation),
        GMCommand::Quit => GMResponse::ok(id, "session ended.", state.mode.clone()),
    }
}

// =============================================================================
// Query handlers
// =============================================================================

fn query_state(id: &str, state: &GameState) -> GMResponse {
    let data = serde_json::json!({
        "mode": format!("{}", state.mode),
        "turn": state.turn,
        "dungeon_level": state.dungeon_level,
        "party_size": state.party.members.len(),
        "has_combat": state.combat.is_some(),
        "has_dungeon": state.dungeon.is_some(),
        "has_wilderness": state.wilderness.is_some(),
        "notes": state.notes,
    });
    GMResponse::ok_with_data(id, "game state summary", state.mode.clone(), data)
}

fn query_party(id: &str, state: &GameState) -> GMResponse {
    if state.party.members.is_empty() {
        return GMResponse::ok_with_data(
            id, "no party members.", state.mode.clone(),
            serde_json::json!({ "members": [] }),
        );
    }
    let members: Vec<serde_json::Value> = state.party.members.iter().map(|c| {
        serde_json::json!({
            "name": c.name,
            "class": c.class,
            "level": c.level,
            "hp": c.hp,
            "max_hp": c.max_hp,
            "ac": c.ac,
            "thac0": c.thac0,
            "xp": c.xp,
            "alive": c.is_alive(),
            "alignment": c.alignment,
            "movement_rate": c.movement_rate,
        })
    }).collect();
    GMResponse::ok_with_data(
        id, format!("{} party members.", members.len()), state.mode.clone(),
        serde_json::json!({ "members": members }),
    )
}

fn query_combat(id: &str, state: &GameState) -> GMResponse {
    match &state.combat {
        Some(combat_state) => {
            let status = combat::combat_status(combat_state, &state.party.members);
            let monsters: Vec<serde_json::Value> = combat_state.monsters.iter().enumerate().map(|(i, m)| {
                serde_json::json!({
                    "index": i,
                    "name": m.name,
                    "hp": m.hp,
                    "max_hp": m.max_hp,
                    "ac": m.ac,
                    "alive": m.is_alive(),
                })
            }).collect();
            GMResponse::ok_with_data(
                id, status, state.mode.clone(),
                serde_json::json!({
                    "round": combat_state.round,
                    "distance": combat_state.distance,
                    "party_initiative": combat_state.party_initiative,
                    "monster_initiative": combat_state.monster_initiative,
                    "monsters": monsters,
                }),
            )
        }
        None => GMResponse::err(id, "no active combat.", state.mode.clone()),
    }
}

fn query_exploration(id: &str, state: &GameState) -> GMResponse {
    let time = match &state.time {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match &state.dungeon {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    let status = exploration::exploration_status(time, dungeon);
    GMResponse::ok_with_data(
        id, status, state.mode.clone(),
        serde_json::json!({
            "dungeon_level": dungeon.level,
            "current_room": dungeon.current_room,
            "total_turns": time.total_turns,
            "has_light": time.has_light(),
        }),
    )
}

fn query_wilderness(id: &str, state: &GameState) -> GMResponse {
    let ws = match &state.wilderness {
        Some(w) => w,
        None => return GMResponse::err(id, "not in wilderness mode.", state.mode.clone()),
    };
    let party_movement = state.party.members.iter()
        .filter(|c| c.is_alive())
        .map(|c| c.movement_rate)
        .min()
        .unwrap_or(120);
    let status = wilderness_engine::wilderness_status(ws, party_movement);
    GMResponse::ok_with_data(
        id, status, state.mode.clone(),
        serde_json::json!({
            "current_x": ws.current_x,
            "current_y": ws.current_y,
            "travel_day": ws.travel_day,
            "lost": ws.lost,
        }),
    )
}

// =============================================================================
// Character management
// =============================================================================

fn create_character(id: &str, state: &mut GameState, name: &str, class_name: &str, alignment: &str) -> GMResponse {
    let class = match Class::parse(class_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("unknown class '{}'.", class_name), state.mode.clone()),
    };
    let alignment = match alignment.to_lowercase().as_str() {
        "lawful" | "l" => "Lawful",
        "neutral" | "n" => "Neutral",
        "chaotic" | "c" => "Chaotic",
        _ => return GMResponse::err(id, "alignment must be Lawful, Neutral, or Chaotic.", state.mode.clone()),
    };

    let mut abilities = chargen::roll_abilities();
    let def = class::class_def(class);
    if !def.racial_modifiers.is_empty() {
        class::apply_racial_modifiers(class, &mut abilities);
    }
    if !class::meets_requirements(class, &abilities) {
        return GMResponse::err(
            id,
            format!("rolled abilities do not meet requirements for {}.", class.name()),
            state.mode.clone(),
        );
    }

    let c = chargen::create_character(name, class, abilities, alignment);
    let sheet = chargen::character_sheet(&c);
    state.party.add_member(c);

    GMResponse::ok_with_data(
        id,
        format!("{} created and added to party.", name),
        state.mode.clone(),
        serde_json::json!({ "character_sheet": sheet }),
    )
}

// =============================================================================
// Combat
// =============================================================================

fn spawn_encounter(
    id: &str, state: &mut GameState,
    name: &str, count: u32, hit_dice: &str, ac: i32, hp: i32,
    damage: &str, morale: u32, distance: u32,
) -> GMResponse {
    if state.combat.is_some() {
        return GMResponse::err(id, "combat already active.", state.mode.clone());
    }
    if !(2..=12).contains(&morale) {
        return GMResponse::err(id, "morale must be 2-12.", state.mode.clone());
    }
    let mut monsters = Vec::new();
    for i in 0..count {
        let monster_name = if count > 1 {
            format!("{} {}", name, i + 1)
        } else {
            name.to_string()
        };
        let mut m = Monster::new(&monster_name, hit_dice);
        m.hp = hp;
        m.max_hp = hp;
        m.ac = ac;
        m.damage = damage.to_string();
        m.morale = morale;
        m.attacks = vec!["attack".to_string()];
        monsters.push(m);
    }

    let combat_state = CombatState::new(monsters, distance);
    let status = combat::combat_status(&combat_state, &state.party.members);
    state.combat = Some(combat_state);
    state.mode = GameMode::Combat;

    GMResponse::ok_with_data(
        id,
        format!("combat started: {} {}(s) at {}' distance.", count, name, distance),
        state.mode.clone(),
        serde_json::json!({ "status": status }),
    )
}

fn roll_initiative(id: &str, state: &mut GameState) -> GMResponse {
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    let (p, m) = combat::roll_initiative(combat_state);
    let winner = if p > m { "party" } else if m > p { "monsters" } else { "simultaneous" };
    GMResponse::ok_with_data(
        id,
        format!("round {} initiative: party {} vs monsters {} — {} acts first.",
            combat_state.round, p, m, winner),
        state.mode.clone(),
        serde_json::json!({
            "round": combat_state.round,
            "party_initiative": p,
            "monster_initiative": m,
            "winner": winner,
        }),
    )
}

fn attack(id: &str, state: &mut GameState, char_name: &str, monster_idx: usize, weapon_name: &str) -> GMResponse {
    let weapon = match equipment::find_weapon(weapon_name) {
        Some(w) => w,
        None => return GMResponse::err(id, format!("unknown weapon '{}'.", weapon_name), state.mode.clone()),
    };
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if monster_idx >= combat_state.monsters.len() {
        return GMResponse::err(id, format!("monster index {} out of range.", monster_idx), state.mode.clone());
    }
    if !combat_state.monsters[monster_idx].is_alive() {
        return GMResponse::err(id, format!("{} is already dead.", combat_state.monsters[monster_idx].name), state.mode.clone());
    }
    if !character.is_alive() {
        return GMResponse::err(id, format!("{} is dead.", character.name), state.mode.clone());
    }

    let result_msg = if weapon.qualities.missile && !weapon.qualities.melee {
        let dex_mod = ability::dex_missile_mod(character.abilities.dexterity);
        match combat::character_missile_attack(combat_state, &character, monster_idx, weapon.damage, dex_mod, weapon.range) {
            Ok(r) => format!("{}", r),
            Err(e) => return GMResponse::err(id, e, state.mode.clone()),
        }
    } else if weapon.qualities.missile && weapon.qualities.melee && combat_state.distance > 5 {
        let dex_mod = ability::dex_missile_mod(character.abilities.dexterity);
        match combat::character_missile_attack(combat_state, &character, monster_idx, weapon.damage, dex_mod, weapon.range) {
            Ok(r) => format!("{}", r),
            Err(e) => return GMResponse::err(id, e, state.mode.clone()),
        }
    } else if !weapon.qualities.missile && combat_state.distance > 5 {
        return GMResponse::err(id, format!("{} is a melee weapon but monsters are {}' away.", weapon.name, combat_state.distance), state.mode.clone());
    } else {
        let str_mod = ability::str_melee_mod(character.abilities.strength);
        let r = combat::character_melee_attack(combat_state, &character, monster_idx, weapon.damage, str_mod);
        format!("{}", r)
    };

    GMResponse::ok(id, result_msg, state.mode.clone())
}

fn monster_attack(id: &str, state: &mut GameState, monster_idx: usize, char_name: &str) -> GMResponse {
    {
        let combat_state = match state.combat.as_ref() {
            Some(c) => c,
            None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
        };
        if monster_idx >= combat_state.monsters.len() {
            return GMResponse::err(id, format!("monster index {} out of range.", monster_idx), state.mode.clone());
        }
        if !combat_state.monsters[monster_idx].is_alive() {
            return GMResponse::err(id, format!("{} is dead.", combat_state.monsters[monster_idx].name), state.mode.clone());
        }
    }
    let character = match state.party.find_member_mut(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    if !character.is_alive() {
        return GMResponse::err(id, format!("{} is already dead.", character.name), state.mode.clone());
    }
    let combat_state = state.combat.as_mut().unwrap();
    let result = combat::monster_attack(combat_state, monster_idx, character);
    GMResponse::ok(id, format!("{}", result), state.mode.clone())
}

fn check_morale(id: &str, state: &mut GameState) -> GMResponse {
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if combat_state.living_monster_count() == 0 {
        return GMResponse::err(id, "no living monsters.", state.mode.clone());
    }
    let result = combat::check_morale(combat_state);
    GMResponse::ok(id, format!("{}", result), state.mode.clone())
}

fn turn_undead(id: &str, state: &mut GameState, char_name: &str, monster_idx: usize) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c.clone(),
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let combat_state = match state.combat.as_mut() {
        Some(c) => c,
        None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
    };
    if monster_idx >= combat_state.monsters.len() {
        return GMResponse::err(id, format!("monster index {} out of range.", monster_idx), state.mode.clone());
    }
    if !combat_state.monsters[monster_idx].is_alive() {
        return GMResponse::err(id, "target is already dead.", state.mode.clone());
    }
    let result = combat::resolve_turn_undead(combat_state, &character, character.level, monster_idx);
    GMResponse::ok(id, format!("{}", result), state.mode.clone())
}

fn end_combat(id: &str, state: &mut GameState) -> GMResponse {
    if state.combat.is_none() {
        return GMResponse::err(id, "no active combat.", state.mode.clone());
    }
    let combat_state = state.combat.take().unwrap();
    let dead_monsters = combat_state.monsters.iter().filter(|m| !m.is_alive()).count();
    let total_xp: u64 = combat_state.monsters.iter()
        .filter(|m| !m.is_alive())
        .map(|m| m.xp_value)
        .sum();
    let dead_party = state.party.members.iter().filter(|c| !c.is_alive()).count();
    state.mode = GameMode::Idle;

    GMResponse::ok_with_data(
        id,
        format!("combat ended after {} rounds. {} of {} monsters defeated.",
            combat_state.round, dead_monsters, combat_state.monsters.len()),
        state.mode.clone(),
        serde_json::json!({
            "rounds": combat_state.round,
            "monsters_defeated": dead_monsters,
            "total_xp": total_xp,
            "party_casualties": dead_party,
        }),
    )
}

// =============================================================================
// Exploration
// =============================================================================

fn enter_dungeon(id: &str, state: &mut GameState, level: u32, room_name: &str) -> GMResponse {
    if level == 0 {
        return GMResponse::err(id, "level must be a positive integer.", state.mode.clone());
    }
    let mut dungeon = DungeonState::new(level);
    dungeon.add_room(Room::new(0, room_name));
    dungeon.explore_current();
    state.dungeon = Some(dungeon);
    state.time = Some(TimeTracker::new());
    state.dungeon_level = level;
    state.mode = GameMode::Exploration;

    GMResponse::ok(
        id,
        format!("entered dungeon level {}. starting room: {}.", level, room_name),
        state.mode.clone(),
    )
}

fn advance_turn(id: &str, state: &mut GameState) -> GMResponse {
    let level = state.dungeon_level;
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    let result = exploration::advance_dungeon_turn(time, dungeon, level);
    let has_encounter = result.encounter.is_some();
    let mut data = serde_json::json!({
        "messages": result.messages,
        "has_encounter": has_encounter,
    });
    if let Some(enc) = &result.encounter {
        data["encounter"] = serde_json::json!({
            "name": enc.name,
            "number": enc.number,
        });
    }
    GMResponse::ok_with_data(id, format!("{}", result), state.mode.clone(), data)
}

fn add_room(id: &str, state: &mut GameState, room_id: u32, name: &str) -> GMResponse {
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    dungeon.add_room(Room::new(room_id, name));
    GMResponse::ok(id, format!("added room {}: {}.", room_id, name), state.mode.clone())
}

fn add_door(id: &str, state: &mut GameState, door_id: u32, room_a: u32, room_b: u32, door_state_str: &str) -> GMResponse {
    let door_state = match door_state_str.to_lowercase().as_str() {
        "open" => DoorState::Open,
        "closed" => DoorState::Closed,
        "stuck" => DoorState::Stuck,
        "locked" => DoorState::Locked,
        "secret" => DoorState::Secret,
        _ => return GMResponse::err(id, "door state must be open, closed, stuck, locked, or secret.", state.mode.clone()),
    };
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    dungeon.add_door(Door::new(door_id, room_a, room_b, door_state));
    GMResponse::ok(
        id,
        format!("added door {} between rooms {} and {} ({:?}).", door_id, room_a, room_b, door_state),
        state.mode.clone(),
    )
}

fn move_room(id: &str, state: &mut GameState, door_id: u32) -> GMResponse {
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    match exploration::move_through_door(time, dungeon, door_id) {
        Ok(msg) => GMResponse::ok(id, msg, state.mode.clone()),
        Err(e) => GMResponse::err(id, e, state.mode.clone()),
    }
}

fn search(id: &str, state: &mut GameState, is_elf: bool) -> GMResponse {
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    let result = exploration::search_room(time, dungeon, is_elf);
    GMResponse::ok(id, result, state.mode.clone())
}

fn light(id: &str, state: &mut GameState, source: &str, carrier: &str) -> GMResponse {
    let kind = match source.to_lowercase().as_str() {
        "torch" => LightSourceKind::Torch,
        "lantern" => LightSourceKind::Lantern,
        _ => return GMResponse::err(id, "light source must be 'torch' or 'lantern'.", state.mode.clone()),
    };
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    time.light(kind, carrier);
    GMResponse::ok(id, format!("{} lights a {}.", carrier, source), state.mode.clone())
}

// =============================================================================
// Wilderness
// =============================================================================

fn parse_terrain(name: &str) -> Option<Terrain> {
    match name.to_lowercase().as_str() {
        "clear" => Some(Terrain::Clear),
        "forest" => Some(Terrain::Forest),
        "hills" => Some(Terrain::Hills),
        "mountains" => Some(Terrain::Mountains),
        "desert" => Some(Terrain::Desert),
        "swamp" => Some(Terrain::Swamp),
        "jungle" => Some(Terrain::Jungle),
        "ocean" => Some(Terrain::Ocean),
        "river" => Some(Terrain::River),
        "barren" => Some(Terrain::Barren),
        "city" => Some(Terrain::City),
        _ => None,
    }
}

fn enter_wilderness(id: &str, state: &mut GameState, terrain_name: &str) -> GMResponse {
    let terrain = match parse_terrain(terrain_name) {
        Some(t) => t,
        None => return GMResponse::err(id, "invalid terrain type.", state.mode.clone()),
    };
    let mut ws = WildernessState::new();
    ws.add_hex(HexCell::new(0, 0, terrain));
    state.wilderness = Some(ws);
    state.mode = GameMode::Wilderness;
    GMResponse::ok(
        id,
        format!("entered wilderness. starting hex: (0, 0) — {}.", terrain.name()),
        state.mode.clone(),
    )
}

fn add_hex(id: &str, state: &mut GameState, x: i32, y: i32, terrain_name: &str) -> GMResponse {
    let terrain = match parse_terrain(terrain_name) {
        Some(t) => t,
        None => return GMResponse::err(id, "invalid terrain type.", state.mode.clone()),
    };
    let ws = match state.wilderness.as_mut() {
        Some(w) => w,
        None => return GMResponse::err(id, "not in wilderness mode.", state.mode.clone()),
    };
    ws.add_hex(HexCell::new(x, y, terrain));
    GMResponse::ok(id, format!("added hex ({}, {}) — {}.", x, y, terrain.name()), state.mode.clone())
}

fn travel(id: &str, state: &mut GameState, x: i32, y: i32) -> GMResponse {
    let party_movement = state.party.members.iter()
        .filter(|c| c.is_alive())
        .map(|c| c.movement_rate)
        .min()
        .unwrap_or(120);
    let ws = match state.wilderness.as_mut() {
        Some(w) => w,
        None => return GMResponse::err(id, "not in wilderness mode.", state.mode.clone()),
    };
    let result = wilderness_engine::travel_day(ws, x, y, party_movement);
    let has_encounter = result.encounter.is_some();
    let mut data = serde_json::json!({
        "messages": result.messages,
        "lost": result.lost,
        "has_encounter": has_encounter,
    });
    if let Some(enc) = &result.encounter {
        data["encounter"] = serde_json::json!({
            "name": enc.name,
            "number": enc.number,
        });
    }
    GMResponse::ok_with_data(id, format!("{}", result), state.mode.clone(), data)
}

// =============================================================================
// Encounter resolution
// =============================================================================

fn roll_surprise(id: &str, state: &GameState) -> GMResponse {
    let (result, p, m) = encounter_engine::check_surprise();
    GMResponse::ok_with_data(
        id,
        format!("party roll: {} monster roll: {} — {}", p, m, result),
        state.mode.clone(),
        serde_json::json!({
            "party_roll": p,
            "monster_roll": m,
            "result": format!("{}", result),
        }),
    )
}

fn roll_reaction(id: &str, state: &GameState, char_name: &str) -> GMResponse {
    let character = match state.party.find_member(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    let cha = character.abilities.charisma;
    let (reaction, raw, modified) = encounter_engine::reaction_roll(cha);
    let cha_mod = ability::cha_reaction_mod(cha);
    GMResponse::ok_with_data(
        id,
        format!("{} speaks (CHA {}, modifier {:+}). reaction roll: {} {:+} = {} — {}",
            character.name, cha, cha_mod, raw, cha_mod, modified, reaction),
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "charisma": cha,
            "cha_modifier": cha_mod,
            "raw_roll": raw,
            "modified_roll": modified,
            "reaction": format!("{}", reaction),
        }),
    )
}

// =============================================================================
// Management
// =============================================================================

fn award_xp(id: &str, state: &mut GameState, char_name: &str, xp: u64) -> GMResponse {
    let character = match state.party.find_member_mut(char_name) {
        Some(c) => c,
        None => return GMResponse::err(id, format!("no party member named '{}'.", char_name), state.mode.clone()),
    };
    character.xp += xp;
    GMResponse::ok_with_data(
        id,
        format!("{} awarded {} XP (total: {}).", character.name, xp, character.xp),
        state.mode.clone(),
        serde_json::json!({
            "character": character.name,
            "xp_awarded": xp,
            "total_xp": character.xp,
        }),
    )
}

fn ruling(id: &str, state: &mut GameState, text: &str) -> GMResponse {
    state.notes.push(format!("[RULING] {}", text));
    GMResponse::ok(
        id,
        format!("ruling recorded: {}", text),
        state.mode.clone(),
    )
}

// =============================================================================
// System
// =============================================================================

fn save_game(id: &str, state: &GameState, path: &str) -> GMResponse {
    match persist::save(state, std::path::Path::new(path)) {
        Ok(()) => GMResponse::ok(id, format!("game saved to {}.", path), state.mode.clone()),
        Err(e) => GMResponse::err(id, format!("save failed: {}.", e), state.mode.clone()),
    }
}

fn load_game(id: &str, state: &mut GameState, path: &str) -> GMResponse {
    match persist::load(std::path::Path::new(path)) {
        Ok(loaded) => {
            let msg = format!(
                "loaded: turn {}, dungeon level {}, {} party members.",
                loaded.turn, loaded.dungeon_level, loaded.party.members.len(),
            );
            *state = loaded;
            GMResponse::ok(id, msg, state.mode.clone())
        }
        Err(e) => GMResponse::err(id, format!("load failed: {}.", e), state.mode.clone()),
    }
}

fn roll_dice(id: &str, state: &GameState, notation: &str) -> GMResponse {
    match dice::roll_str(notation) {
        Ok(result) => GMResponse::ok_with_data(
            id,
            format!("{}", result),
            state.mode.clone(),
            serde_json::json!({ "total": result.total }),
        ),
        Err(e) => GMResponse::err(id, format!("{}", e), state.mode.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_req(id: &str, command: GMCommand) -> GMRequest {
        GMRequest { id: id.to_string(), command }
    }

    #[test]
    fn query_state_empty() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryState), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert_eq!(data["party_size"], 0);
        assert_eq!(data["mode"], "idle");
    }

    #[test]
    fn query_mode() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryMode), &mut state);
        assert!(resp.success);
        assert!(resp.message.contains("idle"));
    }

    #[test]
    fn query_party_empty() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryParty), &mut state);
        assert!(resp.success);
        assert!(resp.message.contains("no party"));
    }

    #[test]
    fn query_combat_no_combat() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryCombat), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn create_character_and_query() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::CreateCharacter {
            name: "Aldric".to_string(),
            class: "Fighter".to_string(),
            alignment: "Lawful".to_string(),
        }), &mut state);
        // Character creation might fail due to random ability rolls not meeting requirements.
        // So we just verify the response is well-formed.
        assert!(!resp.id.is_empty());

        // If it succeeded, verify party has a member.
        if resp.success {
            let resp2 = handle_request(&make_req("2", GMCommand::QueryParty), &mut state);
            assert!(resp2.success);
            let data = resp2.data.unwrap();
            let members = data["members"].as_array().unwrap();
            assert_eq!(members.len(), 1);
        }
    }

    #[test]
    fn spawn_encounter_and_query() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter {
            name: "goblin".to_string(),
            count: 3,
            hit_dice: "1".to_string(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 60,
        }), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Combat);

        let resp2 = handle_request(&make_req("2", GMCommand::QueryCombat), &mut state);
        assert!(resp2.success);
        let data = resp2.data.unwrap();
        assert_eq!(data["monsters"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn spawn_encounter_invalid_morale() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".to_string(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 13,
            distance: 60,
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn combat_already_active() {
        let mut state = GameState::new();
        state.combat = Some(CombatState::new(vec![], 0));
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter {
            name: "orc".to_string(),
            count: 1,
            hit_dice: "1".to_string(),
            ac: 6,
            hp: 4,
            damage: "1d6".to_string(),
            morale: 8,
            distance: 30,
        }), &mut state);
        assert!(!resp.success);
        assert!(resp.error.unwrap().contains("already active"));
    }

    #[test]
    fn initiative_no_combat() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::RollInitiative), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn end_combat_no_combat() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EndCombat), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn enter_dungeon_and_advance() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EnterDungeon {
            level: 1,
            room_name: "Entry Hall".to_string(),
        }), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Exploration);

        let resp2 = handle_request(&make_req("2", GMCommand::AdvanceTurn), &mut state);
        assert!(resp2.success);
    }

    #[test]
    fn enter_dungeon_level_zero() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EnterDungeon {
            level: 0,
            room_name: "Test".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn enter_wilderness_and_add_hex() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EnterWilderness {
            terrain: "forest".to_string(),
        }), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Wilderness);

        let resp2 = handle_request(&make_req("2", GMCommand::AddHex {
            x: 1,
            y: 0,
            terrain: "hills".to_string(),
        }), &mut state);
        assert!(resp2.success);
    }

    #[test]
    fn enter_wilderness_invalid_terrain() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::EnterWilderness {
            terrain: "lava".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn award_xp_no_character() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::AwardXp {
            character: "Nobody".to_string(),
            xp: 100,
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn ruling_recorded() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Ruling {
            text: "The bridge can hold 3 people at once.".to_string(),
        }), &mut state);
        assert!(resp.success);
        assert_eq!(state.notes.len(), 1);
        assert!(state.notes[0].contains("bridge"));
    }

    #[test]
    fn roll_dice_valid() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Roll {
            notation: "2d6+3".to_string(),
        }), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        let total = data["total"].as_i64().unwrap();
        assert!((5..=15).contains(&total));
    }

    #[test]
    fn roll_dice_invalid() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Roll {
            notation: "invalid".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn surprise_roll() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::RollSurprise), &mut state);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert!(data["party_roll"].as_u64().is_some());
        assert!(data["monster_roll"].as_u64().is_some());
    }

    #[test]
    fn query_exploration_not_exploring() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryExploration), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn query_wilderness_not_in_wilderness() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::QueryWilderness), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn quit_response() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Quit), &mut state);
        assert!(resp.success);
        assert!(resp.message.contains("ended"));
    }

    #[test]
    fn add_room_no_dungeon() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::AddRoom {
            id: 1,
            name: "Test".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn light_not_exploring() {
        let mut state = GameState::new();
        let resp = handle_request(&make_req("1", GMCommand::Light {
            source: "torch".to_string(),
            carrier: "Aldric".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn light_invalid_source() {
        let mut state = GameState::new();
        state.time = Some(TimeTracker::new());
        let resp = handle_request(&make_req("1", GMCommand::Light {
            source: "candle".to_string(),
            carrier: "Aldric".to_string(),
        }), &mut state);
        assert!(!resp.success);
    }

    #[test]
    fn full_combat_sequence() {
        let mut state = GameState::new();
        // Add a character
        let mut c = crate::model::Character::new("Aldric", "Fighter");
        c.hp = 10;
        c.max_hp = 10;
        c.thac0 = 19;
        c.abilities.strength = 12;
        state.party.add_member(c);

        // Spawn encounter
        let resp = handle_request(&make_req("1", GMCommand::SpawnEncounter {
            name: "goblin".to_string(),
            count: 1,
            hit_dice: "1".to_string(),
            ac: 6,
            hp: 3,
            damage: "1d6".to_string(),
            morale: 7,
            distance: 5,
        }), &mut state);
        assert!(resp.success);

        // Roll initiative
        let resp = handle_request(&make_req("2", GMCommand::RollInitiative), &mut state);
        assert!(resp.success);

        // Attack
        let resp = handle_request(&make_req("3", GMCommand::Attack {
            character: "Aldric".to_string(),
            monster_idx: 0,
            weapon: "sword".to_string(),
        }), &mut state);
        assert!(resp.success);

        // End combat
        let resp = handle_request(&make_req("4", GMCommand::EndCombat), &mut state);
        assert!(resp.success);
        assert_eq!(state.mode, GameMode::Idle);
    }
}
