use ttrpg_interp::adapter::StateAdapter;
use ttrpg_interp::state::EntityRef;
use ttrpg_interp::value::Value;
use ttrpg_interp::{Interpreter, RuntimeError};

use osr_ai_gm::log_entry::LogEntry;
use osr_ai_gm::model::{Character, Monster};

use crate::handler::BridgeHandler;
use crate::registry::{character_ref, monster_ref};
use crate::state::BridgeState;

/// Result of executing a DSL action through the bridge.
pub struct BridgeResult {
    /// Characters after execution (may have modified HP, etc.).
    pub characters: Vec<Character>,
    /// Monsters after execution (may have modified HP, etc.).
    pub monsters: Vec<Monster>,
    /// New log entries generated during execution.
    pub log_entries: Vec<LogEntry>,
    /// Updated log sequence counter.
    pub log_seq: u64,
    /// Human-readable effect log from the handler.
    pub effect_log: Vec<String>,
    /// The return value from the interpreter.
    pub return_value: Value,
}

/// Integration engine that bridges ttrpg_interp with oag's game state.
///
/// Uses Layer 2 (StateAdapter) for auto-applied mutations, so the
/// BridgeState is automatically updated as the interpreter runs.
pub struct BridgedEngine<'p> {
    interpreter: &'p Interpreter<'p>,
}

impl<'p> BridgedEngine<'p> {
    pub fn new(interpreter: &'p Interpreter<'p>) -> Self {
        BridgedEngine { interpreter }
    }

    /// Execute a named action on behalf of a character.
    pub fn execute_action(
        &self,
        characters: Vec<Character>,
        monsters: Vec<Monster>,
        log: Vec<LogEntry>,
        log_seq: u64,
        combat_round: u32,
        action_name: &str,
        actor_index: usize,
        args: Vec<Value>,
    ) -> Result<BridgeResult, RuntimeError> {
        let actor = character_ref(actor_index);
        self.run(characters, monsters, log, log_seq, combat_round, |adapter, handler| {
            adapter.run(handler, |state, eff_handler| {
                self.interpreter
                    .execute_action(state, eff_handler, action_name, actor, args)
            })
        })
    }

    /// Execute a named action on behalf of a monster.
    pub fn execute_monster_action(
        &self,
        characters: Vec<Character>,
        monsters: Vec<Monster>,
        log: Vec<LogEntry>,
        log_seq: u64,
        combat_round: u32,
        action_name: &str,
        monster_index: usize,
        args: Vec<Value>,
    ) -> Result<BridgeResult, RuntimeError> {
        let actor = monster_ref(monster_index);
        self.run(characters, monsters, log, log_seq, combat_round, |adapter, handler| {
            adapter.run(handler, |state, eff_handler| {
                self.interpreter
                    .execute_action(state, eff_handler, action_name, actor, args)
            })
        })
    }

    /// Evaluate a named derive function.
    pub fn evaluate_derive(
        &self,
        characters: Vec<Character>,
        monsters: Vec<Monster>,
        derive_name: &str,
        args: Vec<Value>,
    ) -> Result<BridgeResult, RuntimeError> {
        self.run(characters, monsters, Vec::new(), 0, 0, |adapter, handler| {
            adapter.run(handler, |state, eff_handler| {
                self.interpreter
                    .evaluate_derive(state, eff_handler, derive_name, args)
            })
        })
    }

    /// Evaluate a named mechanic function.
    pub fn evaluate_mechanic(
        &self,
        characters: Vec<Character>,
        monsters: Vec<Monster>,
        log: Vec<LogEntry>,
        log_seq: u64,
        mechanic_name: &str,
        args: Vec<Value>,
    ) -> Result<BridgeResult, RuntimeError> {
        self.run(characters, monsters, log, log_seq, 0, |adapter, handler| {
            adapter.run(handler, |state, eff_handler| {
                self.interpreter
                    .evaluate_mechanic(state, eff_handler, mechanic_name, args)
            })
        })
    }

    /// Get EntityRef handles for all characters in the party.
    pub fn character_refs(count: usize) -> Vec<EntityRef> {
        (0..count).map(character_ref).collect()
    }

    /// Get EntityRef handles for all monsters in combat.
    pub fn monster_refs(count: usize) -> Vec<EntityRef> {
        (0..count).map(monster_ref).collect()
    }

    /// Get all entity refs (characters + monsters).
    pub fn all_refs(character_count: usize, monster_count: usize) -> Vec<EntityRef> {
        let mut refs = Self::character_refs(character_count);
        refs.extend(Self::monster_refs(monster_count));
        refs
    }

    // ── Internal ───────────────────────────────────────────────

    fn run<F>(
        &self,
        characters: Vec<Character>,
        monsters: Vec<Monster>,
        log: Vec<LogEntry>,
        log_seq: u64,
        combat_round: u32,
        f: F,
    ) -> Result<BridgeResult, RuntimeError>
    where
        F: FnOnce(
            &StateAdapter<BridgeState>,
            &mut BridgeHandler,
        ) -> Result<Value, RuntimeError>,
    {
        let bridge_state = BridgeState::new(characters, monsters, log, log_seq, combat_round);
        let adapter = StateAdapter::new(bridge_state);
        let mut handler = BridgeHandler::new();

        let return_value = f(&adapter, &mut handler)?;

        let final_state = adapter.into_inner();
        Ok(BridgeResult {
            characters: final_state.characters,
            monsters: final_state.monsters,
            log_entries: final_state.log,
            log_seq: final_state.log_seq,
            effect_log: handler.effect_log,
            return_value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_refs_generation() {
        let refs = BridgedEngine::character_refs(3);
        assert_eq!(refs.len(), 3);
        assert_eq!(refs[0], EntityRef(0));
        assert_eq!(refs[1], EntityRef(1));
        assert_eq!(refs[2], EntityRef(2));
    }

    #[test]
    fn monster_refs_generation() {
        let refs = BridgedEngine::monster_refs(2);
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0], EntityRef(10_000));
        assert_eq!(refs[1], EntityRef(10_001));
    }

    #[test]
    fn all_refs_combined() {
        let refs = BridgedEngine::all_refs(2, 3);
        assert_eq!(refs.len(), 5);
        // Characters first
        assert_eq!(refs[0], EntityRef(0));
        assert_eq!(refs[1], EntityRef(1));
        // Then monsters
        assert_eq!(refs[2], EntityRef(10_000));
        assert_eq!(refs[3], EntityRef(10_001));
        assert_eq!(refs[4], EntityRef(10_002));
    }
}
