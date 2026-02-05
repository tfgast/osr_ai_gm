use serde::{Deserialize, Serialize};

/// The current game mode — determines which commands are valid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GameMode {
    /// No active game mode. Party management and setup only.
    #[default]
    Idle,
    /// Character creation in progress.
    CharGen,
    /// Dungeon exploration (turn-by-turn).
    Exploration,
    /// Overland/wilderness travel.
    Wilderness,
    /// Active combat encounter.
    Combat,
    /// Downtime activities (training, research, etc.).
    Downtime,
}

impl std::fmt::Display for GameMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameMode::Idle => write!(f, "idle"),
            GameMode::CharGen => write!(f, "chargen"),
            GameMode::Exploration => write!(f, "exploration"),
            GameMode::Wilderness => write!(f, "wilderness"),
            GameMode::Combat => write!(f, "combat"),
            GameMode::Downtime => write!(f, "downtime"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_idle() {
        assert_eq!(GameMode::default(), GameMode::Idle);
    }

    #[test]
    fn display_modes() {
        assert_eq!(format!("{}", GameMode::Idle), "idle");
        assert_eq!(format!("{}", GameMode::Combat), "combat");
        assert_eq!(format!("{}", GameMode::Exploration), "exploration");
        assert_eq!(format!("{}", GameMode::Wilderness), "wilderness");
        assert_eq!(format!("{}", GameMode::CharGen), "chargen");
        assert_eq!(format!("{}", GameMode::Downtime), "downtime");
    }

    #[test]
    fn serialization_roundtrip() {
        let mode = GameMode::Combat;
        let json = serde_json::to_string(&mode).unwrap();
        let mode2: GameMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, mode2);
    }
}
