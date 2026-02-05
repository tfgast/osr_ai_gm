use serde::{Deserialize, Serialize};

/// How a participant connects to the game session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerKind {
    /// Human at a terminal — reads/writes via stdin/stdout.
    HumanCli,
    /// AI player — sends/receives JSON over a pipe.
    AiPlayer,
    /// AI Game Master — sends/receives JSON over a pipe, has GM privileges.
    AiGm,
}

impl std::fmt::Display for PlayerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerKind::HumanCli => write!(f, "human-cli"),
            PlayerKind::AiPlayer => write!(f, "ai-player"),
            PlayerKind::AiGm => write!(f, "ai-gm"),
        }
    }
}

/// The role a participant plays in the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Game Master — can spawn encounters, issue rulings, award XP, etc.
    Gm,
    /// Player — controls one or more characters.
    Player,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Gm => write!(f, "gm"),
            Role::Player => write!(f, "player"),
        }
    }
}

/// A participant in a game session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    /// Unique identifier for this player within the session.
    pub id: String,
    /// Display name.
    pub name: String,
    /// How the player connects.
    pub kind: PlayerKind,
    /// Role in the game.
    pub role: Role,
    /// Names of characters this player controls (empty for GM).
    pub characters: Vec<String>,
}

impl Player {
    pub fn new(id: &str, name: &str, kind: PlayerKind, role: Role) -> Self {
        Player {
            id: id.to_string(),
            name: name.to_string(),
            kind,
            role,
            characters: Vec::new(),
        }
    }

    pub fn is_gm(&self) -> bool {
        self.role == Role::Gm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_player() {
        let p = Player::new("p1", "Alice", PlayerKind::HumanCli, Role::Player);
        assert_eq!(p.id, "p1");
        assert!(!p.is_gm());
        assert_eq!(p.kind.to_string(), "human-cli");
        assert_eq!(p.role.to_string(), "player");
    }

    #[test]
    fn ai_gm() {
        let p = Player::new("gm", "GPT-GM", PlayerKind::AiGm, Role::Gm);
        assert!(p.is_gm());
        assert_eq!(p.kind.to_string(), "ai-gm");
        assert_eq!(p.role.to_string(), "gm");
    }

    #[test]
    fn serialization_roundtrip() {
        let p = Player::new("p1", "Bob", PlayerKind::AiPlayer, Role::Player);
        let json = serde_json::to_string(&p).unwrap();
        let p2: Player = serde_json::from_str(&json).unwrap();
        assert_eq!(p.id, p2.id);
        assert_eq!(p.kind, p2.kind);
        assert_eq!(p.role, p2.role);
    }
}
