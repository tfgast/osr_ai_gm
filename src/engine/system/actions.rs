use crate::dice;
use crate::engine::result::EngineError;
use crate::persist::{self, GameState};

use super::results::{HelpResult, LoadGameResult, QuitResult, RollDiceResult, SaveGameResult};

pub fn action_roll_dice(notation: &str) -> Result<RollDiceResult, EngineError> {
    let result = dice::roll_str(notation).map_err(|e| EngineError::InvalidInput(e.to_string()))?;
    Ok(RollDiceResult {
        rendered: result.to_string(),
        total: result.total,
    })
}

pub fn action_save_game(state: &GameState, path: &str) -> Result<SaveGameResult, EngineError> {
    let safe_path =
        persist::safe_save_path(path).map_err(|e| EngineError::InvalidInput(e.to_string()))?;
    persist::save(state, &safe_path).map_err(|e| EngineError::Internal(e.to_string()))?;
    Ok(SaveGameResult { path: safe_path })
}

pub fn action_load_game(state: &mut GameState, path: &str) -> Result<LoadGameResult, EngineError> {
    let safe_path =
        persist::safe_save_path(path).map_err(|e| EngineError::InvalidInput(e.to_string()))?;
    let loaded = persist::load(&safe_path).map_err(|e| EngineError::Internal(e.to_string()))?;
    let result = LoadGameResult {
        turn: loaded.turn(),
        dungeon_level: loaded.dungeon_level,
        party_members: loaded.party.members.len(),
        combat_active: loaded.combat.is_some(),
    };
    *state = loaded;
    Ok(result)
}

pub fn action_quit() -> Result<QuitResult, EngineError> {
    Ok(QuitResult)
}

pub fn action_help(commands: &[(&str, &str)]) -> Result<HelpResult, EngineError> {
    let mut output = String::from("Available commands:\n");
    for (command_name, help) in commands {
        output.push_str(&format!("  {:18} {}\n", command_name, help));
    }
    Ok(HelpResult { output })
}

#[cfg(test)]
mod tests {
    use super::{action_help, action_load_game, action_quit, action_roll_dice, action_save_game};
    use crate::persist::GameState;

    #[test]
    fn action_roll_dice_valid() {
        let result = action_roll_dice("2d6+3").unwrap();
        assert!((5..=15).contains(&result.total));
        assert!(result.rendered.contains("2d6+3"));
    }

    #[test]
    fn action_roll_dice_invalid() {
        let error = action_roll_dice("invalid").unwrap_err();
        assert!(error.to_string().contains("invalid"));
    }

    #[test]
    fn action_save_game_rejects_path_like_input() {
        let state = GameState::new();
        let error = action_save_game(&state, "../save").unwrap_err();
        assert!(error.to_string().contains("simple name"));
    }

    #[test]
    fn action_load_game_rejects_path_like_input() {
        let mut state = GameState::new();
        let error = action_load_game(&mut state, "../save").unwrap_err();
        assert!(error.to_string().contains("simple name"));
    }

    #[test]
    fn action_quit_success() {
        action_quit().unwrap();
    }

    #[test]
    fn action_help_lists_commands() {
        let result = action_help(&[("roll", "Roll dice"), ("save", "Save game")]).unwrap();
        assert!(result.output.contains("Available commands:"));
        assert!(result.output.contains("roll"));
        assert!(result.output.contains("save"));
    }
}
