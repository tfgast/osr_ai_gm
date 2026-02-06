# Design: Unify CLI and GM API Command Dispatch

**Issue:** oag-mol-snz
**Blocks:** oag-mol-08q (implementation)

## Problem

The codebase has two independent command dispatch paths that duplicate validation
and orchestration logic:

1. **CLI path** (`src/command/*.rs`): `Command` trait implementations parse
   `&[&str]` args, validate preconditions, call engine functions, wrap results
   in `CommandResult`.

2. **GM API path** (`src/gmapi/*.rs`): `handle_request()` matches on
   `GMCommand` enum variants, validates the same preconditions, calls the same
   engine functions, wraps results in `GMResponse`.

The core game logic in `src/engine/` is already shared. The duplication lives in
the **validation and orchestration layer** between the frontends and the engine:

```
CLI:  parse_args -> CommandImpl::execute() -> [validate, call engine] -> CommandResult
API:  serde JSON -> handle_request()       -> [validate, call engine] -> GMResponse
```

Both paths independently check: character exists, combat active, weapon valid,
monster index in range, game mode correct, etc.

## Scope of Duplication

| Category | CLI files | API files | Duplicated patterns |
|----------|-----------|-----------|---------------------|
| Combat | combat_cmds.rs (~700 loc) | combat_handlers.rs (593 loc) | 12 commands |
| Exploration | exploration_cmds.rs | exploration_handlers.rs | 12 commands |
| Inventory | inventory_cmds.rs | inventory_handlers.rs | 4 commands |
| Party/Chargen | party.rs | interface.rs (inline) | 4 commands |
| GM fiat | gm_cmds.rs | interface.rs (inline) | 11 commands |
| Lookup | lookup_cmds.rs | lookup_handlers.rs | 5 commands |
| System | system.rs | interface.rs (inline) | 5 commands |
| Retainers | retainer_cmds.rs | interface.rs (inline) | 4 commands |
| Wilderness | wilderness_cmds.rs | exploration_handlers.rs | 7 commands |
| Encounter | encounter_cmds.rs | combat_handlers.rs | 4 commands |
| Treasure | treasure_cmds.rs | lookup_handlers.rs | 2 commands |
| Module | module_cmds.rs | exploration_handlers.rs | 1 command |

**~70 commands** duplicated across **~5,200 LOC** (CLI) and **~3,900 LOC** (API).

## Design

### Core Idea

Introduce `EngineAction` functions in `src/engine/` that own all validation and
orchestration. Both CLI and API become thin adapters that parse input into typed
args, call the engine action, and format the result for their output channel.

### New Type: `EngineResult`

```rust
// src/engine/result.rs

/// Unified result from any engine action.
pub struct EngineResult {
    /// Human-readable message describing what happened.
    pub message: String,
    /// Structured data for API consumers (optional).
    pub data: Option<serde_json::Value>,
}

/// Unified error from any engine action.
pub enum EngineError {
    /// Invalid input (wrong args, missing character, bad index, etc.)
    InvalidInput(String),
    /// Wrong game state (no active combat, not in dungeon, etc.)
    WrongState(String),
    /// Engine-level failure (dice parse error, etc.)
    Internal(String),
}
```

Engine actions return `Result<EngineResult, EngineError>`.

### Engine Action Functions

Move validation + orchestration into `src/engine/` modules. Each action is a
plain function that takes `&mut GameState` plus typed parameters:

```rust
// src/engine/combat/mod.rs (extending existing module)

pub fn action_attack(
    state: &mut GameState,
    char_name: &str,
    monster_idx: usize,
    weapon_name: &str,
) -> Result<EngineResult, EngineError> {
    // 1. Validate weapon exists
    let weapon = equipment::find_weapon(weapon_name)
        .ok_or_else(|| EngineError::InvalidInput(
            format!("unknown weapon '{}'.", weapon_name)))?;

    // 2. Validate character exists
    let character = state.party.find_member(char_name)
        .ok_or_else(|| EngineError::InvalidInput(
            format!("no party member named '{}'.", char_name)))?
        .clone();

    // 3. Validate combat active
    let combat = state.combat.as_mut()
        .ok_or_else(|| EngineError::WrongState(
            "no active combat.".into()))?;

    // 4. Check helpless auto-kill
    if monster_idx < combat.monsters.len()
        && combat.monsters[monster_idx].is_alive()
        && combat.monsters[monster_idx].helpless
    {
        let result = coup_de_grace(combat, &character, monster_idx)
            .map_err(|e| EngineError::InvalidInput(e))?;
        return Ok(EngineResult {
            message: result.to_string(),
            data: None,
        });
    }

    // 5. Resolve normal attack
    let rest_penalty = state.time.as_ref()
        .map(|t| t.rest_penalty()).unwrap_or(0);
    let result = resolve_character_attack(
        combat, &character, monster_idx, weapon, rest_penalty,
    ).map_err(|e| EngineError::InvalidInput(e))?;

    Ok(EngineResult {
        message: result.to_string(),
        data: None,
    })
}
```

### Adapted CLI (thin adapter)

```rust
// src/command/combat_cmds.rs

pub struct AttackCommand;
impl Command for AttackCommand {
    fn name(&self) -> &str { "attack" }
    fn help(&self) -> &str { "Melee/missile attack ..." }
    fn execute(&self, args: &[&str], state: &mut GameState) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("usage: attack <character> <monster_idx> [weapon]");
        }
        let char_name = args[0];
        let monster_idx: usize = match args[1].parse() {
            Ok(n) => n,
            _ => return CommandResult::error("monster_index must be a number"),
        };
        let weapon = if args.len() >= 3 { args[2..].join(" ") }
                     else { "sword".to_string() };

        match engine::combat::action_attack(state, char_name, monster_idx, &weapon) {
            Ok(r) => CommandResult::ok(r.message),
            Err(e) => CommandResult::error(e.to_string()),
        }
    }
}
```

CLI responsibility shrinks to: parse `&[&str]` -> typed args, call engine, map
`EngineResult`/`EngineError` to `CommandResult`.

### Adapted GM API (thin adapter)

```rust
// src/gmapi/interface.rs

GMCommand::Attack { character, monster_idx, weapon } => {
    match engine::combat::action_attack(state, character, *monster_idx, weapon) {
        Ok(r) => GMResponse::ok_with_data(id, r.message, state.mode.clone(),
            r.data.unwrap_or(serde_json::json!({}))),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}
```

API responsibility shrinks to: destructure `GMCommand`, call engine, map result
to `GMResponse`.

### Structured Data

Some API consumers need structured data (JSON) that CLI consumers don't.
Two options:

**Option A (recommended): Engine always produces `data`.**
Engine actions populate `EngineResult.data` with structured JSON. CLI ignores it.
API passes it through. Simple, consistent, and forward-compatible (CLI could use
it for TUI features later).

**Option B: API adapter enriches.**
Engine returns only a message. API adapter queries state after the call to build
JSON. More complex, risks drift between message and data.

### Migration Strategy

This is a large refactor (~70 commands). Migrate incrementally by subsystem:

**Phase 1: Foundation**
- Add `src/engine/result.rs` with `EngineResult` and `EngineError`.
- Add `Display` impl for `EngineError`.

**Phase 2: Combat commands (highest duplication)**
- Create `action_*` functions in `src/engine/combat/`.
- Update CLI `combat_cmds.rs` to call engine actions.
- Update API `combat_handlers.rs` to call engine actions.
- Run tests after each command migration.

**Phase 3: Exploration commands**
- Same pattern for `exploration_cmds.rs` / `exploration_handlers.rs`.

**Phase 4: Remaining subsystems**
- GM fiat, inventory, party, retainers, wilderness, encounter, lookup, system,
  treasure, module commands.

**Phase 5: Cleanup**
- Remove empty handler files once all commands migrated.
- Consider whether `Command` trait is still needed or if CLI dispatch can
  use a simpler match.

### What Does NOT Change

- `GMCommand` enum (protocol.rs): Unchanged. This is the API contract.
- `GMRequest`/`GMResponse`: Unchanged. These are the wire format.
- `CommandResult`: Unchanged. CLI output contract stays the same.
- `CommandRegistry` + `Command` trait: Kept for CLI dispatch.
  Commands become thin adapters.
- `parse_args()` in main.rs: Unchanged.
- All existing tests: Should continue passing. New engine action tests
  supplement existing coverage.
- `GameState`: Unchanged. Engine actions take `&mut GameState`.

### What Changes

| Component | Before | After |
|-----------|--------|-------|
| `command/*.rs` | Full validation + engine calls | Parse args, call `engine::action_*`, map result |
| `gmapi/*_handlers.rs` | Full validation + engine calls | Call `engine::action_*`, map result |
| `gmapi/interface.rs` | Delegates to handlers | Delegates to handlers (which call engine) |
| `engine/*.rs` | Pure game logic only | Game logic + validation + orchestration |
| New: `engine/result.rs` | N/A | `EngineResult`, `EngineError` |

### Risk Assessment

| Risk | Mitigation |
|------|------------|
| Behavioral regression | Migrate one command at a time, run full test suite after each |
| CLI error messages change | Map `EngineError` display to match existing `CommandResult::error` format |
| API response `data` changes | Engine produces same JSON structure, verify with `gmapi_protocol_qa` tests |
| Large diff | Incremental phases, each phase independently shippable |
| Helpless auto-kill in attack | Unified in engine action, not split across adapter layers |

### Naming Convention

Engine action functions: `action_<command_name>` in the appropriate engine module.

- `engine::combat::action_attack()`
- `engine::combat::action_spawn_encounter()`
- `engine::combat::action_roll_initiative()`
- `engine::exploration::action_enter_dungeon()`
- `engine::exploration::action_move_room()`
- etc.

This avoids colliding with existing pure-logic function names like
`resolve_character_attack()` which the action functions call internally.

### Testing Strategy

1. Existing tests pass unchanged (both CLI integration and API protocol tests).
2. New unit tests for each `action_*` function test validation logic directly
   against `GameState`, without going through CLI parsing or JSON serialization.
3. The `action_*` tests replace the duplicated validation tests that currently
   exist in both command and handler test suites.

### Example: Full Before/After for `end_combat`

**Before (CLI):** `command/combat_cmds.rs` EndCombatCommand::execute - 25 lines
of validation, state mutation, output formatting.

**Before (API):** `gmapi/combat_handlers.rs` end_combat - 35 lines of same
validation, state mutation, structured output formatting.

**After (engine):**
```rust
pub fn action_end_combat(state: &mut GameState) -> Result<EngineResult, EngineError> {
    let combat_state = state.combat.take()
        .ok_or_else(|| EngineError::WrongState("no active combat.".into()))?;

    let dead_monsters = combat_state.monsters.iter().filter(|m| !m.is_alive()).count();
    let total_xp: u64 = combat_state.monsters.iter()
        .filter(|m| !m.is_alive())
        .map(|m| m.xp_value).sum();
    let dead_party = state.party.members.iter().filter(|c| !c.is_alive()).count();
    state.mode = state.pre_combat_mode.take().unwrap_or(GameMode::Idle);

    if let Some(dungeon) = state.dungeon.as_mut() {
        if let Some(room_id) = dungeon.current_room {
            if let Some(room) = dungeon.find_room_mut(room_id) {
                room.monsters_cleared = true;
            }
        }
    }

    Ok(EngineResult {
        message: format!("combat ended after {} rounds. {} of {} monsters defeated.",
            combat_state.round, dead_monsters, combat_state.monsters.len()),
        data: Some(serde_json::json!({
            "rounds": combat_state.round,
            "monsters_defeated": dead_monsters,
            "total_xp": total_xp,
            "party_casualties": dead_party,
        })),
    })
}
```

**After (CLI):** 3 lines - call action, map result.
**After (API):** 3 lines - call action, map result.

## Summary

- Single source of truth for validation and orchestration per command.
- CLI and API become thin input/output adapters (~3-5 lines each).
- Incremental migration, one subsystem at a time.
- All existing tests continue passing.
- No protocol/wire format changes.
