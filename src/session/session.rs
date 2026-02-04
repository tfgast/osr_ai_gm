use serde::{Deserialize, Serialize};
use crate::persist::GameState;
use crate::session::player::{Player, Role};

/// A game session tracks participants, turn order, and permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier.
    pub id: String,
    /// All participants in the session.
    pub players: Vec<Player>,
    /// Index into `players` for whose turn it is (None if not tracking turns).
    pub current_turn: Option<usize>,
    /// Session-level log of events.
    pub log: Vec<String>,
}

impl Session {
    pub fn new(id: &str) -> Self {
        Session {
            id: id.to_string(),
            players: Vec::new(),
            current_turn: None,
            log: Vec::new(),
        }
    }

    /// Add a player to the session.
    pub fn add_player(&mut self, player: Player) {
        self.log.push(format!("{} ({}) joined as {}.", player.name, player.kind, player.role));
        self.players.push(player);
    }

    /// Find the GM player, if any.
    pub fn gm(&self) -> Option<&Player> {
        self.players.iter().find(|p| p.role == Role::Gm)
    }

    /// Find a player by id.
    pub fn find_player(&self, id: &str) -> Option<&Player> {
        self.players.iter().find(|p| p.id == id)
    }

    /// Check if a player id has GM privileges.
    pub fn is_gm(&self, player_id: &str) -> bool {
        self.find_player(player_id).map(|p| p.is_gm()).unwrap_or(false)
    }

    /// Advance to the next player's turn. Wraps around.
    pub fn advance_turn(&mut self) {
        if self.players.is_empty() {
            return;
        }
        self.current_turn = Some(match self.current_turn {
            Some(i) => (i + 1) % self.players.len(),
            None => 0,
        });
    }

    /// Get the player whose turn it currently is.
    pub fn current_player(&self) -> Option<&Player> {
        self.current_turn.and_then(|i| self.players.get(i))
    }

    /// Validate that a command is allowed for the given role.
    /// GM-only commands require Role::Gm.
    pub fn check_permission(role: &Role, gm_only: bool) -> Result<(), String> {
        if gm_only && *role != Role::Gm {
            return Err("this command requires GM privileges.".to_string());
        }
        Ok(())
    }
}

/// Process a command through the session layer, checking permissions.
pub fn dispatch_with_session(
    session: &Session,
    player_id: &str,
    command: &str,
    args: &[&str],
    state: &mut GameState,
    registry: &crate::command::CommandRegistry,
    gm_commands: &[&str],
) -> crate::command::CommandResult {
    let player = match session.find_player(player_id) {
        Some(p) => p,
        None => return crate::command::CommandResult::error(
            format!("unknown player '{}'.", player_id)
        ),
    };

    let is_gm_cmd = gm_commands.contains(&command);
    if let Err(e) = Session::check_permission(&player.role, is_gm_cmd) {
        return crate::command::CommandResult::error(e);
    }

    registry.dispatch(command, args, state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::player::{PlayerKind, Role};

    fn test_session() -> Session {
        let mut s = Session::new("test-1");
        s.add_player(Player::new("gm", "The GM", PlayerKind::AiGm, Role::Gm));
        s.add_player(Player::new("p1", "Alice", PlayerKind::HumanCli, Role::Player));
        s.add_player(Player::new("p2", "Bob", PlayerKind::AiPlayer, Role::Player));
        s
    }

    #[test]
    fn session_creation() {
        let s = test_session();
        assert_eq!(s.players.len(), 3);
        assert_eq!(s.log.len(), 3);
    }

    #[test]
    fn find_gm() {
        let s = test_session();
        let gm = s.gm().unwrap();
        assert_eq!(gm.id, "gm");
        assert!(gm.is_gm());
    }

    #[test]
    fn find_player() {
        let s = test_session();
        assert!(s.find_player("p1").is_some());
        assert!(s.find_player("nobody").is_none());
    }

    #[test]
    fn is_gm_check() {
        let s = test_session();
        assert!(s.is_gm("gm"));
        assert!(!s.is_gm("p1"));
        assert!(!s.is_gm("nobody"));
    }

    #[test]
    fn turn_order() {
        let mut s = test_session();
        assert!(s.current_player().is_none());

        s.advance_turn();
        assert_eq!(s.current_player().unwrap().id, "gm");

        s.advance_turn();
        assert_eq!(s.current_player().unwrap().id, "p1");

        s.advance_turn();
        assert_eq!(s.current_player().unwrap().id, "p2");

        s.advance_turn(); // wraps
        assert_eq!(s.current_player().unwrap().id, "gm");
    }

    #[test]
    fn permission_gm_only() {
        assert!(Session::check_permission(&Role::Gm, true).is_ok());
        assert!(Session::check_permission(&Role::Player, true).is_err());
        assert!(Session::check_permission(&Role::Player, false).is_ok());
    }

    #[test]
    fn serialization_roundtrip() {
        let s = test_session();
        let json = serde_json::to_string(&s).unwrap();
        let s2: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(s.id, s2.id);
        assert_eq!(s.players.len(), s2.players.len());
    }
}
