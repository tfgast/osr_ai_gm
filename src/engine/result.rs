use std::fmt;

/// Unified success payload returned by engine actions.
#[derive(Debug, Clone)]
pub struct EngineResult {
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl EngineResult {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Unified error classification for engine actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// Invalid user input (bad args, unknown entities, out-of-range indexes, etc.).
    InvalidInput(String),
    /// Command is valid but cannot run in the current game state.
    WrongState(String),
    /// Internal error while running engine logic.
    Internal(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "{msg}"),
            Self::WrongState(msg) => write!(f, "{msg}"),
            Self::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for EngineError {}

#[cfg(test)]
mod tests {
    use super::EngineError;

    #[test]
    fn display_uses_wrapped_message() {
        let invalid = EngineError::InvalidInput("bad input".to_string());
        let wrong_state = EngineError::WrongState("wrong mode".to_string());
        let internal = EngineError::Internal("dice parse failed".to_string());

        assert_eq!(invalid.to_string(), "bad input");
        assert_eq!(wrong_state.to_string(), "wrong mode");
        assert_eq!(internal.to_string(), "dice parse failed");
    }
}
