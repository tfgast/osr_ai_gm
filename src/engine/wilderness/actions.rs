use crate::engine::result::EngineError;
use crate::engine::wilderness::results::{
    AddHexResult, EnterWildernessResult, ForageResult, HuntResult, OrientResult, TravelResult,
    WildernessStatusResult,
};
use crate::engine::wilderness_engine;
use crate::persist::GameState;
use crate::state::game::GameMode;
use crate::state::wilderness::{HexCell, Terrain, WildernessState};

fn not_in_wilderness() -> EngineError {
    EngineError::WrongState("not in wilderness mode.".to_string())
}

fn party_movement_rate(state: &GameState) -> u32 {
    state
        .party
        .members
        .iter()
        .filter(|c| c.is_alive())
        .map(|c| c.movement_rate)
        .min()
        .unwrap_or(120)
}

pub fn action_enter_wilderness(
    state: &mut GameState,
    terrain: Terrain,
) -> Result<EnterWildernessResult, EngineError> {
    if state.mode == GameMode::Wilderness {
        return Err(EngineError::WrongState(
            "already in wilderness mode.".to_string(),
        ));
    }

    let mut wilderness = WildernessState::new();
    wilderness
        .add_hex(HexCell::new(0, 0, terrain))
        .map_err(EngineError::Internal)?;
    state.enter_wilderness(wilderness);

    Ok(EnterWildernessResult {
        message: format!(
            "entered wilderness. starting hex: (0, 0) — {}.",
            terrain.name()
        ),
        terrain,
        x: 0,
        y: 0,
    })
}

pub fn action_add_hex(
    state: &mut GameState,
    x: i32,
    y: i32,
    terrain: Terrain,
) -> Result<AddHexResult, EngineError> {
    let wilderness = state.wilderness.as_mut().ok_or_else(not_in_wilderness)?;
    wilderness
        .add_hex(HexCell::new(x, y, terrain))
        .map_err(EngineError::InvalidInput)?;

    Ok(AddHexResult {
        message: format!("added hex ({x}, {y}) — {}.", terrain.name()),
        x,
        y,
        terrain,
    })
}

pub fn action_travel(state: &mut GameState, x: i32, y: i32) -> Result<TravelResult, EngineError> {
    let movement_rate = party_movement_rate(state);
    let wilderness = state.wilderness.as_mut().ok_or_else(not_in_wilderness)?;
    let core = wilderness_engine::travel_day(wilderness, &mut state.party, x, y, movement_rate);
    let message = core.to_string();
    let encounters = core
        .encounters
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    let has_encounter = !encounters.is_empty();

    Ok(TravelResult {
        message,
        messages: core.messages,
        lost: core.lost,
        has_encounter,
        encounters,
        foraged: core.foraged,
        rations_consumed: core.rations_consumed,
        starving: core.starving,
        starvation_damage: core.starvation_damage,
        rations_remaining: state.party.rations,
    })
}

pub fn action_orient(state: &mut GameState) -> Result<OrientResult, EngineError> {
    let wilderness = state.wilderness.as_mut().ok_or_else(not_in_wilderness)?;
    let core = wilderness_engine::orient(wilderness);

    Ok(OrientResult {
        message: core.message,
        success: core.success,
        terrain: core.terrain,
        lost: wilderness.lost,
        travel_day: wilderness.travel_day,
    })
}

pub fn action_forage(state: &mut GameState) -> Result<ForageResult, EngineError> {
    let wilderness = state.wilderness.as_ref().ok_or_else(not_in_wilderness)?;
    let core = wilderness_engine::forage(wilderness, &mut state.party);

    Ok(ForageResult {
        message: core.message,
        quantity: core.quantity,
        success: core.success,
        rations_remaining: state.party.rations,
    })
}

pub fn action_hunt(state: &mut GameState) -> Result<HuntResult, EngineError> {
    let wilderness = state.wilderness.as_ref().ok_or_else(not_in_wilderness)?;
    let core = wilderness_engine::hunt(wilderness, &mut state.party);

    Ok(HuntResult {
        message: core.message,
        quantity: core.quantity,
        success: core.success,
        rations_remaining: state.party.rations,
    })
}

pub fn action_wilderness_status(state: &GameState) -> Result<WildernessStatusResult, EngineError> {
    let wilderness = state.wilderness.as_ref().ok_or_else(not_in_wilderness)?;
    let movement_rate = party_movement_rate(state);
    let message = wilderness_engine::wilderness_status(wilderness, &state.party, movement_rate);

    Ok(WildernessStatusResult {
        message,
        movement_rate,
    })
}
