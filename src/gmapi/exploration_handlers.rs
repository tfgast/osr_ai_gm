use crate::engine::{exploration, wilderness_engine};
use crate::gmapi::protocol::GMResponse;
use crate::persist::GameState;
use crate::state::dungeon::{Door, DoorState, DungeonState, Room};
use crate::state::game::GameMode;
use crate::state::time::{LightSourceKind, TimeTracker};
use crate::state::wilderness::{HexCell, Terrain, WildernessState};

pub(super) fn enter_dungeon(id: &str, state: &mut GameState, level: u32, room_name: &str) -> GMResponse {
    if level == 0 {
        return GMResponse::err(id, "level must be a positive integer.", state.mode.clone());
    }
    let mut dungeon = DungeonState::new(level);
    dungeon.add_room(Room::new(0, room_name)).unwrap();
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

pub(super) fn advance_turn(id: &str, state: &mut GameState) -> GMResponse {
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
    GMResponse::ok_with_data(id, result.to_string(), state.mode.clone(), data)
}

pub(super) fn add_room(id: &str, state: &mut GameState, room_id: u32, name: &str) -> GMResponse {
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    if let Err(e) = dungeon.add_room(Room::new(room_id, name)) {
        return GMResponse::err(id, e, state.mode.clone());
    }
    GMResponse::ok(id, format!("added room {}: {}.", room_id, name), state.mode.clone())
}

pub(super) fn add_door(id: &str, state: &mut GameState, door_id: u32, room_a: u32, room_b: u32, door_state: DoorState) -> GMResponse {
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    let door = match Door::new(door_id, room_a, room_b, door_state) {
        Ok(d) => d,
        Err(e) => return GMResponse::err(id, e, state.mode.clone()),
    };
    if let Err(e) = dungeon.add_door(door) {
        return GMResponse::err(id, e, state.mode.clone());
    }
    GMResponse::ok(
        id,
        format!("added door {} between rooms {} and {} ({}).", door_id, room_a, room_b, door_state),
        state.mode.clone(),
    )
}

pub(super) fn move_room(id: &str, state: &mut GameState, door_id: u32) -> GMResponse {
    let dungeon_level = state.dungeon_level;
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    match exploration::move_through_door(time, dungeon, dungeon_level, door_id) {
        Ok(result) => GMResponse::ok(id, result.to_string(), state.mode.clone()),
        Err(e) => GMResponse::err(id, e, state.mode.clone()),
    }
}

pub(super) fn search(id: &str, state: &mut GameState, is_elf: bool) -> GMResponse {
    let dungeon_level = state.dungeon_level;
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    let dungeon = match state.dungeon.as_mut() {
        Some(d) => d,
        None => return GMResponse::err(id, "no dungeon state.", state.mode.clone()),
    };
    let result = exploration::search_room(time, dungeon, dungeon_level, is_elf);
    GMResponse::ok(id, result.to_string(), state.mode.clone())
}

pub(super) fn light(id: &str, state: &mut GameState, source: LightSourceKind, carrier: &str) -> GMResponse {
    let time = match state.time.as_mut() {
        Some(t) => t,
        None => return GMResponse::err(id, "not in exploration mode.", state.mode.clone()),
    };
    time.light(source, carrier);
    GMResponse::ok(id, format!("{} lights a {}.", carrier, source.name()), state.mode.clone())
}

pub(super) fn load_module(id: &str, state: &mut GameState, path: &str) -> GMResponse {
    use crate::command::module_cmds::module_to_dungeon;
    use crate::rules::module;

    let module_def = match module::load_module(path, module::DEFAULT_MODULES_DIR) {
        Ok(m) => m,
        Err(e) => return GMResponse::err(id, e, state.mode.clone()),
    };

    let dungeon = match module_to_dungeon(&module_def) {
        Ok(d) => d,
        Err(e) => return GMResponse::err(id, e, state.mode.clone()),
    };

    let module_name = module_def.name.clone();
    let level_range = module_def.level_range;
    let room_count = dungeon.rooms.len();

    state.dungeon = Some(dungeon);
    state.time = Some(TimeTracker::new());
    state.dungeon_level = level_range.0;
    state.mode = GameMode::Exploration;

    GMResponse::ok_with_data(
        id,
        format!("loaded module: {} (levels {}-{}). {} rooms.", module_name, level_range.0, level_range.1, room_count),
        state.mode.clone(),
        serde_json::json!({
            "module_name": module_name,
            "level_range": [level_range.0, level_range.1],
            "room_count": room_count,
        }),
    )
}

pub(super) fn enter_wilderness(id: &str, state: &mut GameState, terrain: Terrain) -> GMResponse {
    let mut ws = WildernessState::new();
    ws.add_hex(HexCell::new(0, 0, terrain)).unwrap();
    state.wilderness = Some(ws);
    state.mode = GameMode::Wilderness;
    GMResponse::ok(
        id,
        format!("entered wilderness. starting hex: (0, 0) — {}.", terrain.name()),
        state.mode.clone(),
    )
}

pub(super) fn add_hex(id: &str, state: &mut GameState, x: i32, y: i32, terrain: Terrain) -> GMResponse {
    let ws = match state.wilderness.as_mut() {
        Some(w) => w,
        None => return GMResponse::err(id, "not in wilderness mode.", state.mode.clone()),
    };
    if let Err(e) = ws.add_hex(HexCell::new(x, y, terrain)) {
        return GMResponse::err(id, e, state.mode.clone());
    }
    GMResponse::ok(id, format!("added hex ({}, {}) — {}.", x, y, terrain.name()), state.mode.clone())
}

pub(super) fn travel(id: &str, state: &mut GameState, x: i32, y: i32) -> GMResponse {
    let party_movement = state.party.members.iter()
        .filter(|c| c.is_alive())
        .map(|c| c.movement_rate)
        .min()
        .unwrap_or(120);
    if state.wilderness.is_none() {
        return GMResponse::err(id, "not in wilderness mode.", state.mode.clone());
    }
    let ws = state.wilderness.as_mut().unwrap();
    let result = wilderness_engine::travel_day(ws, &mut state.party, x, y, party_movement);
    let has_encounter = !result.encounters.is_empty();
    let encounters_json: Vec<serde_json::Value> = result.encounters.iter().map(|enc| {
        serde_json::json!({
            "name": enc.name,
            "number": enc.number,
        })
    }).collect();
    let data = serde_json::json!({
        "messages": result.messages,
        "lost": result.lost,
        "has_encounter": has_encounter,
        "encounters": encounters_json,
        "rations_consumed": result.rations_consumed,
        "starving": result.starving,
        "rations_remaining": state.party.rations,
    });
    GMResponse::ok_with_data(id, result.to_string(), state.mode.clone(), data)
}

pub(super) fn orient(id: &str, state: &mut GameState) -> GMResponse {
    let ws = match state.wilderness.as_mut() {
        Some(w) => w,
        None => return GMResponse::err(id, "not in wilderness mode.", state.mode.clone()),
    };
    let result = wilderness_engine::orient(ws);
    let data = serde_json::json!({
        "success": result.success,
        "terrain": result.terrain.name(),
        "lost": ws.lost,
        "travel_day": ws.travel_day,
    });
    GMResponse::ok_with_data(id, result.message, state.mode.clone(), data)
}
