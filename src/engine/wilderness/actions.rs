use crate::engine::result::EngineError;
use crate::engine::wilderness::results::{
    AddHexResult, EnterWildernessResult, ForageResult, HuntResult, LeaveWildernessResult,
    OrientResult, TravelResult, WildernessStatusResult,
};
use crate::engine::wilderness_engine;
use crate::persist::GameState;
use crate::state::game::GameMode;
use crate::state::wilderness::{HexCell, Terrain, WildernessState};

fn not_in_wilderness() -> EngineError {
    EngineError::WrongState("not in wilderness mode.".to_string())
}

fn require_wilderness_mode(state: &GameState) -> Result<(), EngineError> {
    if state.mode != GameMode::Wilderness {
        return Err(not_in_wilderness());
    }
    Ok(())
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
    match state.mode {
        GameMode::Idle | GameMode::Downtime => {}
        GameMode::Wilderness => {
            return Err(EngineError::WrongState(
                "already in wilderness mode.".to_string(),
            ));
        }
        GameMode::Combat => {
            return Err(EngineError::WrongState(
                "cannot enter wilderness during combat. Use EndCombat first.".to_string(),
            ));
        }
        GameMode::Exploration => {
            return Err(EngineError::WrongState(
                "cannot enter wilderness while in exploration mode. Use LeaveDungeon first."
                    .to_string(),
            ));
        }
        GameMode::CharGen => {
            return Err(EngineError::WrongState(
                "cannot enter wilderness during character generation.".to_string(),
            ));
        }
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
    require_wilderness_mode(state)?;
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
    require_wilderness_mode(state)?;
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
    require_wilderness_mode(state)?;
    let wilderness = state.wilderness.as_mut().ok_or_else(not_in_wilderness)?;
    let core = wilderness_engine::orient(wilderness, &mut state.party);

    Ok(OrientResult {
        message: core.message,
        success: core.success,
        terrain: core.terrain,
        lost: wilderness.lost,
        travel_day: wilderness.travel_day,
        rations_consumed: core.overhead.rations_consumed,
        starving: core.overhead.starving,
        starvation_damage: core.overhead.starvation_damage,
    })
}

pub fn action_forage(state: &mut GameState) -> Result<ForageResult, EngineError> {
    require_wilderness_mode(state)?;
    let wilderness = state.wilderness.as_mut().ok_or_else(not_in_wilderness)?;
    let core = wilderness_engine::forage(wilderness, &mut state.party);

    Ok(ForageResult {
        message: core.message,
        quantity: core.quantity,
        success: core.success,
        rations_remaining: state.party.rations,
        rations_consumed: core.overhead.rations_consumed,
        starving: core.overhead.starving,
        starvation_damage: core.overhead.starvation_damage,
        travel_day: wilderness.travel_day,
    })
}

pub fn action_hunt(state: &mut GameState) -> Result<HuntResult, EngineError> {
    require_wilderness_mode(state)?;
    let wilderness = state.wilderness.as_mut().ok_or_else(not_in_wilderness)?;
    let core = wilderness_engine::hunt(wilderness, &mut state.party);

    Ok(HuntResult {
        message: core.message,
        quantity: core.quantity,
        success: core.success,
        rations_remaining: state.party.rations,
        rations_consumed: core.overhead.rations_consumed,
        starving: core.overhead.starving,
        starvation_damage: core.overhead.starvation_damage,
        travel_day: wilderness.travel_day,
    })
}

pub fn action_leave_wilderness(
    state: &mut GameState,
) -> Result<LeaveWildernessResult, EngineError> {
    require_wilderness_mode(state)?;
    state.exit_wilderness();

    Ok(LeaveWildernessResult {
        message: "left wilderness mode.".to_string(),
    })
}

pub fn action_wilderness_status(state: &GameState) -> Result<WildernessStatusResult, EngineError> {
    require_wilderness_mode(state)?;
    let wilderness = state.wilderness.as_ref().ok_or_else(not_in_wilderness)?;
    let movement_rate = party_movement_rate(state);
    let message = wilderness_engine::wilderness_status(wilderness, &state.party, movement_rate);

    Ok(WildernessStatusResult {
        message,
        movement_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CombatState;
    use crate::state::dungeon::DungeonState;

    #[test]
    fn enter_wilderness_from_idle() {
        let mut state = GameState::new();
        assert_eq!(state.mode, GameMode::Idle);
        let result = action_enter_wilderness(&mut state, Terrain::Forest);
        assert!(result.is_ok());
        assert_eq!(state.mode, GameMode::Wilderness);
    }

    #[test]
    fn enter_wilderness_from_downtime() {
        let mut state = GameState::new();
        state.mode = GameMode::Downtime;
        let result = action_enter_wilderness(&mut state, Terrain::Clear);
        assert!(result.is_ok());
        assert_eq!(state.mode, GameMode::Wilderness);
    }

    #[test]
    fn enter_wilderness_rejects_combat() {
        let mut state = GameState::new();
        state.enter_combat(CombatState::new(vec![], 60));
        assert_eq!(state.mode, GameMode::Combat);
        let result = action_enter_wilderness(&mut state, Terrain::Forest);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("combat"), "error should mention combat: {err}");
    }

    #[test]
    fn enter_wilderness_rejects_exploration() {
        let mut state = GameState::new();
        state.enter_exploration(DungeonState::new(1), 1);
        assert_eq!(state.mode, GameMode::Exploration);
        let result = action_enter_wilderness(&mut state, Terrain::Desert);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("exploration"), "error should mention exploration: {err}");
    }

    #[test]
    fn enter_wilderness_rejects_chargen() {
        let mut state = GameState::new();
        state.mode = GameMode::CharGen;
        let result = action_enter_wilderness(&mut state, Terrain::Mountains);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("character generation"), "error should mention chargen: {err}");
    }

    #[test]
    fn enter_wilderness_rejects_already_wilderness() {
        let mut state = GameState::new();
        state.enter_wilderness(WildernessState::new());
        assert_eq!(state.mode, GameMode::Wilderness);
        let result = action_enter_wilderness(&mut state, Terrain::Forest);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already"), "error should mention already in wilderness: {err}");
    }
}
