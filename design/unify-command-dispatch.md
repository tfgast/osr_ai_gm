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

## Command Parity Classification

Not all commands are identically implemented across CLI and API. Before
unifying, each command must be classified into one of three categories:

| Class | Description | Migration approach |
|-------|-------------|-------------------|
| **A: Shared** | CLI and API have identical validation + orchestration | Single `action_*` function, both adapters call it directly |
| **B: Shared core + adapter post-processing** | Same core logic, but one frontend adds extra output | Single `action_*` returns full result; adapter selectively uses fields |
| **C: Intentionally divergent** | Frontends implement different semantics | Separate `action_*` variants or adapter-level branching |

### Known Divergences (Class B/C)

| Command | CLI behavior | API behavior | Class | Resolution |
|---------|-------------|-------------|-------|------------|
| `attack` | Auto-kills helpless targets before normal attack (`combat_cmds.rs:151`) | Always calls `resolve_character_attack`, no helpless check (`combat_handlers.rs:94`) | C | Unify to CLI behavior (helpless auto-kill is correct game logic). API was missing it — this is a bug fix. |
| `morale` | Accepts optional monster selector, checks specific monster (`combat_cmds.rs:246`) | No selector, uses max morale among living monsters (`combat_handlers.rs:133`) | C | Engine action takes `Option<usize>` selector. `None` = max morale (API default). CLI adapter passes parsed index. Both behaviors preserved. |
| `end_combat` | Includes retainer XP-share and loyalty reporting (`combat_cmds.rs:468`) | Omits retainer reporting in response data (`combat_handlers.rs:294`) | B | Engine action computes retainer data. CLI adapter formats it in message. API adapter includes it in typed payload. |
| `retreat` | Basic distance output | Returns `free_attacks` and `new_distance` in structured data (`interface.rs:838`) | B | Engine action returns typed `RetreatResult` with all fields. CLI uses message only. API serializes full struct. |
| `enter_dungeon`, `add_room`, `add_door`, `rest` | Pre-migration CLI strings had specific casing/onboarding text | Unified action messages changed CLI text contract in `de7c964` | C | Track and restore CLI text parity in follow-up bug `oag-r428g` (API typed payload remains valid). |
| `load_module` | Pre-migration CLI output included onboarding guidance lines | Unified action returns short API-style summary only | C | Track and restore CLI onboarding text parity in follow-up bug `oag-dqj8x`. |

**Before migrating each command**, verify its parity class. Commands not listed
above are assumed Class A until proven otherwise during implementation. Any
newly discovered divergence must be classified and documented here before
proceeding.

### Parity Audit Gate

Phase 2 (combat) must not begin until all combat commands have been audited
and classified in this table. The implementer runs both CLI and API for each
command with identical inputs and compares:
1. State mutations (must be identical for Class A/B)
2. Output messages (may differ for Class B/C — document why)
3. Structured data fields (API-only — document expected shape)

## Design

### Core Idea

Introduce `EngineAction` functions in `src/engine/` that own all validation and
orchestration. Both CLI and API become thin adapters that parse input into typed
args, call the engine action, and format the result for their output channel.

### New Types: `EngineResult`, Typed Payloads

```rust
// src/engine/result.rs

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

Engine actions return typed `Result<T, EngineError>` where `T` is a
command-specific payload struct. This keeps the engine layer free of
transport concerns (no `serde_json::Value`) and preserves compile-time
guarantees on payload shapes.

```rust
// src/engine/combat/results.rs

/// Result of an attack action.
pub struct AttackResult {
    pub message: String,
    pub hit: bool,
    pub damage: u32,
    pub target_killed: bool,
}

/// Result of ending combat.
pub struct EndCombatResult {
    pub message: String,
    pub rounds: u32,
    pub monsters_defeated: usize,
    pub total_xp: u64,
    pub party_casualties: usize,
    pub retainer_xp_shares: Vec<RetainerXpShare>,
}

/// Result of a retreat action.
pub struct RetreatResult {
    pub message: String,
    pub free_attacks: Vec<FreeAttackResult>,
    pub new_distance: u32,
}
```

**Serialization responsibility stays in adapters:**
- CLI adapter: uses `result.message` (ignores structured fields).
- API adapter: serializes the typed struct via `serde::Serialize` into
  `GMResponse.data`. Compile-time type checking ensures the JSON shape
  matches the struct definition — no drift possible.

```rust
// API adapter example
GMCommand::EndCombat => {
    match engine::combat::action_end_combat(state) {
        Ok(r) => GMResponse::ok_with_data(id, r.message, state.mode.clone(),
            serde_json::to_value(&r).unwrap()),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}
```

**Why not `serde_json::Value` in the engine?** (see oag-e57i4.3)
- Untyped JSON removes compile-time payload guarantees.
- Engine would own wire-shape concerns that belong to the transport layer.
- Existing API tests assert on specific keys (`free_attacks`, `new_distance`);
  typed structs make these assertions enforceable at compile time.

Each subsystem defines its result structs in a `results.rs` submodule.
Structs derive `Serialize` so adapters can convert them, but the engine
never constructs raw JSON.

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
) -> Result<AttackResult, EngineError> {
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

    // 4. Check helpless auto-kill (unified — was missing from API)
    if monster_idx < combat.monsters.len()
        && combat.monsters[monster_idx].is_alive()
        && combat.monsters[monster_idx].helpless
    {
        let result = coup_de_grace(combat, &character, monster_idx)
            .map_err(|e| EngineError::InvalidInput(e))?;
        return Ok(AttackResult {
            message: result.to_string(),
            hit: true,
            damage: result.damage,
            target_killed: true,
        });
    }

    // 5. Resolve normal attack
    let rest_penalty = state.time.as_ref()
        .map(|t| t.rest_penalty()).unwrap_or(0);
    let result = resolve_character_attack(
        combat, &character, monster_idx, weapon, rest_penalty,
    ).map_err(|e| EngineError::InvalidInput(e))?;

    Ok(AttackResult {
        message: result.to_string(),
        hit: result.hit,
        damage: result.damage,
        target_killed: !combat.monsters[monster_idx].is_alive(),
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
            Ok(r) => CommandResult::ok(&r.message),
            Err(e) => CommandResult::error(&e.to_string()),
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
        Ok(r) => GMResponse::ok_with_data(id, r.message.clone(), state.mode.clone(),
            serde_json::to_value(&r).unwrap()),
        Err(e) => GMResponse::err(id, e.to_string(), state.mode.clone()),
    }
}
```

API responsibility shrinks to: destructure `GMCommand`, call engine, serialize
typed result to JSON via `serde::Serialize`.

### Migration Strategy

This is a large refactor (~70 commands). Migrate incrementally by subsystem:

**Phase 0: Parity audit + compatibility gates**
- Audit all combat commands for parity class (A/B/C). Update the parity table.
- Write golden tests for each command being migrated (see Compatibility Gates).
- Golden tests must pass on the existing code BEFORE any migration begins.

**Phase 1: Foundation**
- Add `src/engine/result.rs` with `EngineError` and `Display` impl.
- Add `src/engine/combat/results.rs` with typed result structs (derive `Serialize`).
- Verify golden tests still pass (no behavioral change yet).

**Phase 2: Combat commands (highest duplication)**
- For each combat command, in order:
  1. Create `action_*` function in `src/engine/combat/`.
  2. Update CLI `combat_cmds.rs` to call engine action.
  3. Run golden test — CLI output must match snapshot.
  4. Update API `combat_handlers.rs` to call engine action.
  5. Run golden test — API response must match snapshot.
  6. Run full test suite. Commit only if all pass.
- Class B/C commands: implement per the parity table resolution. Document
  any intentional behavior changes as bug fixes (e.g., helpless auto-kill).

**Phase 3: Exploration commands**
- Parity audit for exploration commands first.
- Same per-command migration + golden test cycle.

**Phase 4: Remaining subsystems**
- GM fiat, inventory, party, retainers, wilderness, encounter, lookup, system,
  treasure, module commands.
- Each subsystem gets its own `results.rs` with typed structs.

**Phase 5: Cleanup**
- Remove empty handler files once all commands migrated.
- Consider whether `Command` trait is still needed or if CLI dispatch can
  use a simpler match.
- Archive golden test snapshots (they become the new regression tests).

### Compatibility Gates

Each migrated command must pass these gates before the commit lands:

**Gate 1: CLI output stability**
Golden test captures `CommandResult.output` for a set of representative inputs
(happy path, error cases, edge cases). Before and after migration, output must
be byte-identical. Any intentional change (Class C bug fix) must be explicitly
documented and the snapshot updated with a comment explaining why.

```rust
#[test]
fn golden_attack_cli() {
    let state = setup_combat_state();
    let before = old_attack_execute(&["fighter", "0", "sword"], &mut state.clone());
    let after  = new_attack_execute(&["fighter", "0", "sword"], &mut state.clone());
    assert_eq!(before.output, after.output, "CLI output changed for attack");
}
```

**Gate 2: API response contract**
Golden test captures the full `GMResponse` (message + data JSON) for each
command. Typed result structs are serialized via `serde_json::to_value()` and
compared field-by-field against the existing handler output. Missing or renamed
fields fail the test.

```rust
#[test]
fn golden_end_combat_api() {
    let state = setup_combat_state();
    let before = old_end_combat_handler(&request, &mut state.clone());
    let after  = new_end_combat_handler(&request, &mut state.clone());
    assert_eq!(before.message, after.message);
    assert_json_eq!(before.data, after.data);
}
```

**Gate 3: State mutation equivalence**
For each command, serialize `GameState` before/after via both old and new paths.
Diff must be empty (same state mutations). This catches subtle ordering or
side-effect differences.

**Rollback criterion:** If a golden test cannot be made to pass without
changing the snapshot, the command stays on the old path until the divergence
is resolved. Never force-update a snapshot to make CI green.

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
| New: `engine/result.rs` | N/A | `EngineError` |
| New: `engine/*/results.rs` | N/A | Typed per-command result structs (derive `Serialize`) |

### Risk Assessment

| Risk | Mitigation |
|------|------------|
| Behavioral regression | Golden tests (Gates 1-3) before and after each command migration |
| CLI error messages change | Map `EngineError` display to match existing format; Gate 1 catches drift |
| API response `data` changes | Typed result structs + `Serialize` ensure compile-time shape; Gate 2 catches field drift |
| CLI/API parity assumptions | Parity matrix (Phase 0 audit) classifies each command before migration |
| Large diff | Incremental phases, each phase independently shippable |
| Helpless auto-kill in attack | Class C divergence — unified to CLI behavior as bug fix, documented in parity table |
| Retainer XP in end_combat | Class B — engine computes retainer data, CLI formats in message, API includes in typed payload |

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

## Phase 0: Combat Command Parity Audit (2026-02-06)

Audit method:
- Executed CLI and GM API paths with identical pre-state + equivalent inputs.
- Captured paired snapshots in `src/engine/combat/golden_tests.rs`.
- Compared state mutation, human-readable output, and API structured data.

Classification:
- **A**: Shared behavior (same core logic + equivalent user-visible behavior).
- **B**: Shared core logic, but adapter-level output/data post-processing differs.
- **C**: Behavior diverges (different validation/orchestration or state effects).

| CLI command | GM API command | State mutation parity | Output/data parity | Class | Notes |
|-------------|----------------|------------------------|--------------------|-------|-------|
| `start_combat` | `SpawnEncounter` | Same combat creation + mode transition | CLI prints full status/warnings; API returns compact message + `data.status` | B | Same encounter setup semantics, different adapter formatting |
| `initiative` | `RollInitiative` | Same round/inits mutation | API adds structured initiative fields | B | Shared `combat::roll_initiative` core |
| `attack` | `Attack` | Divergent when target is helpless | CLI auto-kill via coup de grace; API resolves normal attack flow | C | Known divergence: helpless auto-kill |
| `monster_attack` | `MonsterAttack` | Same successful attack mutation | Error text/validation order differ | B | Shared `combat::monster_attack` core |
| `morale` | `CheckMorale` | Divergent morale score selection | CLI supports monster-name selector; API uses max living monster morale | C | Known divergence: monster selector |
| `turn_undead` | `TurnUndead` | Same mutation/logging path | Equivalent message semantics | A | Near-identical adapter logic over shared engine call |
| `close` | `Close` | Same distance mutation | API appends `data.distance` | B | Shared `combat::close` core |
| `retreat` | `Retreat` | Same retreat/free-attack engine mutation | API returns rich free-attack payload; CLI text-only | C | Known divergence: structured data contract |
| `withdrawal` | `FightingWithdrawal` | Same mutation path | Equivalent message semantics | A | Shared `combat::fighting_withdrawal` core |
| `combat_log` | `QueryCombatLog` | Read-only parity | API includes `data.log`; minor message formatting differences | B | Same log source, different response envelope |
| `declare_spell` | `DeclareSpell` | Same declaration mutation | Equivalent message semantics | A | Same `combat::declare_spell` orchestration |
| `end_combat` | `EndCombat` | Divergent post-processing side effects | CLI reports retainer XP/loyalty text; API returns structured totals and room-clear update | C | Known divergence: retainer XP + endpoint-specific post-processing |

## Phase 0b: Party/Chargen Command Parity Audit (2026-02-06)

Audit method:
- Executed CLI and GM API paths with identical pre-state + equivalent inputs.
- Captured paired snapshots in `src/engine/party/golden_tests.rs`.
- Compared state mutation, human-readable output, and API structured data.

| CLI command | GM API command | State mutation parity | Output/data parity | Class | Notes |
|-------------|----------------|------------------------|--------------------|-------|-------|
| `chargen` | `CreateCharacter` | Same (no party mutation) for ineligible-abilities case | Divergent success contract: CLI returns `CommandResult::ok`, API returns error response | C | Known divergence: ineligible abilities are soft-fail in CLI, hard-fail in API |
| `classes` | `ListClasses` | Read-only parity | Same class source, different response envelope (CLI text table vs API `data.classes`) | B | Shared `Class::ALL` and `class_def` data |
| `eligible` | `EligibleClasses` | Read-only parity | Same eligibility evaluation, different message/data formatting | B | Shared `class::eligible_classes` rules |
| `party` | `QueryParty` | Read-only parity | CLI includes readiness/starvation annotations; API emits normalized member payload | B | Same underlying party members, adapter-level presentation differs |

## Phase 0c: Treasure + Module Command Parity Audit (2026-02-06)

Audit method:
- Executed CLI and GM API paths with identical pre-state + equivalent inputs.
- Captured paired snapshots in `src/engine/gm/golden_tests.rs`.
- Compared state mutation, human-readable output, and API structured data.

Notes:
- `treasure_type` CLI lives in `src/command/lookup_cmds.rs` (not `treasure_cmds.rs`);
  this is the second treasure command paired with `LookupTreasureType`.
- `treasure`/`RollTreasure` outcomes are RNG-driven per-path, so parity checks
  compare shared header fields and state effects, not exact rolled quantities.

| CLI command | GM API command | State mutation parity | Output/data parity | Class | Notes |
|-------------|----------------|------------------------|--------------------|-------|-------|
| `treasure_type` | `LookupTreasureType` | Same state mutation (read-only) | Same treasure metadata with adapter-specific formatting (CLI prose vs API JSON) | B | API includes structured `entries`; CLI prints table text |
| `treasure` | `RollTreasure` | Same state mutation (read-only) | Same roll intent with adapter-specific contracts (CLI formatted haul vs API itemized payload) | B | Random rolls differ independently between paths; parity anchors on shared type/category fields |
| `load_module` | `LoadModule` | Same dungeon/time/mode mutation via shared action | CLI onboarding text regressed vs pre-migration (guidance lines removed); API includes typed payload fields | C | Follow-up bug: `oag-dqj8x` |

## Phase 0d: Exploration Command Post-Migration Review (2026-02-06)

Audit method:
- Reviewed migration diff `de7c964^..de7c964` command-by-command.
- Verified runtime behavior with `cargo test --test gmapi_protocol_qa` and
  `cargo test exploration_cmds::tests`.
- Checked API `data` shape against typed structs in
  `src/engine/exploration/results.rs`.

Scope:
- `enter_dungeon`, `advance_turn`, `add_room`, `add_door`, `move_room`,
  `search`, `light`, `open_door`, `force_door`, `listen`, `rest`, `load_module`
  (the exploration migration surface in `de7c964`).

| CLI command | GM API command | State mutation parity | Output/data parity | Class | Notes |
|-------------|----------------|------------------------|--------------------|-------|-------|
| `enter_dungeon` | `EnterDungeon` | Same dungeon/time/mode mutation | CLI output regressed vs pre-migration (lost onboarding guidance lines + casing drift); API now returns typed payload `{message, level, room_name}` | C | Follow-up bug: `oag-r428g` |
| `explore` | `AdvanceTurn` | Same turn/light/dungeon mutation | CLI text behavior preserved; API now uses typed `ExplorationActionResult` (`message`, `messages`, `has_encounter`, `encounter`, `placed_monsters`, `placed_treasure`) | B | Adapter contract divergence only |
| `add_room` | `AddRoom` | Same dungeon-room mutation | CLI success text regressed vs pre-migration (casing/punctuation); API typed payload `{message, room_id, name}` | C | Follow-up bug: `oag-r428g` |
| `add_door` | `AddDoor` | Same dungeon-door mutation | CLI success text regressed vs pre-migration (`{:?}`-style display replaced, casing drift); API typed payload `{message, door_id, room_a, room_b, door_state}` | C | Follow-up bug: `oag-r428g` |
| `move` | `MoveRoom` | Same movement/turn mutation | CLI text behavior preserved; API now uses typed `ExplorationActionResult` payload | B | Adapter contract divergence only |
| `search` | `Search` | Same search/turn mutation | CLI text behavior preserved (RNG-dependent content); API now uses typed `ExplorationActionResult` payload | B | Adapter contract divergence only |
| `listen` | `Listen` | Same listen/turn mutation | CLI text behavior preserved (RNG-dependent content); API now uses typed `ExplorationActionResult` payload | B | Adapter contract divergence only |
| `light` | `Light` | Same light-source mutation | CLI success text preserved; API now returns typed payload `{message, source, carrier, duration_turns}` | B | Adapter contract divergence only |
| `open` | `OpenDoor` | Same open/force/move mutation | CLI primary success path preserved; API now returns typed payload `{message, door_id, steps, forced, moved}` | B | Adapter contract divergence only |
| `force_door` | `ForceDoor` | Same force-door mutation | CLI behavior preserved; API now returns typed payload `{message, door_id, character, forced_open}` | B | Adapter contract divergence only |
| `rest` | `Rest` | Same rest/turn mutation | CLI success text regressed vs pre-migration (casing drift); API typed payload `{message, total_turns}` | C | Follow-up bug: `oag-r428g` |
| `load_module` | `LoadModule` | Same dungeon/time/mode mutation | CLI onboarding text regressed vs pre-migration (guidance lines removed); API typed payload `{message, module_name, level_range, room_count}` | C | Follow-up bug: `oag-dqj8x` |

## Phase 0d: Wilderness Command Parity Audit (2026-02-06)

Audit method:
- Compared pre-migration (`a7840e2^`) and post-migration (`a7840e2`) adapter code for all seven CLI wilderness commands.
- Executed runtime checks with `cargo test wilderness -- --nocapture`, including `gmapi_protocol_qa` wilderness command coverage.
- Verified API payload keys derive from typed structs in `src/engine/wilderness/results.rs` via adapter serialization (`ok_with_typed_data`).

Notes:
- CLI output text for all seven wilderness commands remains parity-equivalent with pre-migration behavior.
- `travel`/`forage`/`hunt`/`orient` include RNG-driven outcomes; parity checks anchor on shared orchestration, state effects, and payload schema rather than exact per-run rolled values.
- `wilderness_status` pairs with `QueryWilderness` (legacy query handler) and remains read-only parity over shared `wilderness_engine::wilderness_status` text generation.

| CLI command | GM API command | State mutation parity | Output/data parity | Class | Notes |
|-------------|----------------|------------------------|--------------------|-------|-------|
| `enter_wilderness` | `EnterWilderness` | Same wilderness init + mode transition (`state.wilderness`, `state.mode`) | CLI keeps instructional multiline text; API returns typed payload (`message`, `terrain`, `x`, `y`) | B | Core behavior unified through `wilderness::action_enter_wilderness`; adapter contracts intentionally differ |
| `add_hex` | `AddHex` | Same hex insertion and duplicate-hex rejection | CLI human string vs API typed payload (`message`, `x`, `y`, `terrain`) | B | Shared `wilderness::action_add_hex` validates identical preconditions |
| `travel` | `Travel` | Same travel-day mutation path (rations, starvation, lost state, travel day, movement) | Message sourced from shared `TravelResult::message`; API includes structured fields from typed result | B | API typed payload adds structured `foraged`, `starvation_damage`, and optional encounter `hd` metadata |
| `forage` | `Forage` | Same forage resolution + party ration mutation | CLI message parity; API exposes typed (`success`, `quantity`, `rations_remaining`) | B | Shared `wilderness::action_forage` orchestration |
| `hunt` | `Hunt` | Same hunt resolution + party ration mutation | CLI message parity; API exposes typed (`success`, `quantity`, `rations_remaining`) | B | Shared `wilderness::action_hunt` orchestration |
| `orient` | `Orient` | Same lost-state and travel-day mutation semantics | CLI message parity; API exposes typed (`success`, `terrain`, `lost`, `travel_day`) | B | Shared `wilderness::action_orient` orchestration |
| `wilderness_status` | `QueryWilderness` | Read-only parity | Same status text source; API adds structured position/day flags | B | `QueryWilderness` remains legacy query adapter, but uses same status engine function as CLI |

Class C findings:
- None. No follow-up Class C bug beads were filed from this audit.

## Phase 0e: GM Fiat Command Parity Audit (commit `6a5b889`, audited 2026-02-06)

Audit method:
- Compared `6a5b889^` vs `6a5b889` implementations in:
  - `src/command/gm_cmds.rs`
  - `src/gmapi/interface.rs`
  - `src/engine/gm/actions.rs`
  - `src/engine/gm/results.rs`
- Verified CLI output parity for migrated CLI adapters.
- Verified GM API `data` JSON shapes against typed result structs.
- Verified state mutation parity, including edge-case overflow behavior.

| CLI command | GM API command | State mutation parity | Output/data parity | Class | Notes |
|-------------|----------------|------------------------|--------------------|-------|-------|
| `award_xp` | `AwardXp` | **Divergent on overflow edge:** API path now saturates (`u64::saturating_add`) while pre-migration used `+=` | CLI text unchanged; API contract keys unchanged (`character`, `xp_awarded`, `total_xp`) | C | Follow-up bug: `oag-gyymh.1` |
| `ruling` | `Ruling` | Same note append semantics (`[RULING] <text>`) | CLI/API messages unchanged from pre-migration behavior | A | Shared `action_ruling` preserves prior behavior |
| `heal` | `Heal` | Same HP mutation semantics | Success payload unchanged; invalid-amount API error string lost trailing period via shared `EngineError` text | B | Formatting-only response drift |
| `damage` | `Damage` | Same HP mutation semantics | API payload now includes extra `status` field from typed struct (pre-migration payload omitted it) | B | Backward-compatible enrichment, but parity drift |
| `set_hp` | `SetHp` | Same HP assignment semantics | API payload now includes extra `status` field from typed struct (pre-migration payload omitted it) | B | Backward-compatible enrichment, but parity drift |
| `set_rations` | `SetRations` | Same rations assignment semantics | CLI/API output and API payload shape unchanged | A | Shared `action_set_rations` preserves behavior |
| `add_rations` | `AddRations` | **Divergent on overflow edge:** migrated action saturates (`u32::saturating_add`) vs pre-migration `+=` | Success payload unchanged; invalid-amount API error punctuation drift (period removed) | C | Follow-up bug: `oag-gyymh.1` |
| `notes` | `ListNotes` | Same read-only behavior | API message/data shape unchanged from pre-migration | A | No CLI adapter migration in `6a5b889`; API preserved |
| `note_delete` | `DeleteNote` | Same note deletion semantics | API message/data shape unchanged from pre-migration | A | No CLI adapter migration in `6a5b889`; API preserved |
| `retainers` | `ListRetainers` | Same read-only behavior | API message/data shape unchanged from pre-migration | A | No CLI adapter migration in `6a5b889`; API preserved |
| `dismiss` | `DismissRetainer` | Same retainer removal semantics | API message/data shape unchanged from pre-migration | A | No CLI adapter migration in `6a5b889`; API preserved |

## Phase 0f: Inventory Command Parity Audit (2026-02-06)

Audit method:
- Compared pre-migration (`873093c`) and migrated (`cf83f74`) command/handler code for all four inventory commands.
- Executed parity snapshots in `src/engine/inventory/golden_tests.rs` to run CLI and GM API with identical pre-state and equivalent inputs.
- Verified API payload keys match typed structs in `src/engine/inventory/results.rs` through adapter serialization (`ok_with_typed_data`).

Notes:
- CLI output for `buy`, `drop`, `equip`, and `loot` remains byte-identical to pre-migration success messages.
- All four API commands now serialize typed result structs; `data` includes `message` plus command-specific fields from the typed payload.
- No inventory command showed divergent state mutation between CLI and API under equivalent inputs.

| CLI command | GM API command | State mutation parity | Output/data parity | Class | Notes |
|-------------|----------------|------------------------|--------------------|-------|-------|
| `buy` | `Buy` | Same gold deduction + inventory insert via shared `inventory::action_buy` | CLI success text unchanged; API emits typed `BuyResult` payload (`message`, `character`, `item`, `cost_gp`, `gold_remaining`) | B | Shared orchestration; adapter contracts differ (text-only vs structured data) |
| `drop` | `Drop` | Same inventory removal via shared `inventory::action_drop` | CLI success text unchanged; API emits typed `DropResult` payload (`message`, `character`, `item`) | B | Shared orchestration; API keeps structured fields |
| `equip` | `Equip` | Same equipped toggle + AC recalculation via shared `inventory::action_equip` | CLI success text unchanged; API emits typed `EquipResult` payload (`message`, `character`, `item`, `action`, `ac`) | B | Shared orchestration; API exposes AC/action fields structurally |
| `loot` | `Loot` | Same room treasure `taken` mutation and inventory insert via shared `inventory::action_loot` | CLI success text unchanged; API emits typed `LootResult` payload (`message`, `character`, `item`, `value_gp`) | B | Shared orchestration; API exposes loot value field structurally |

Class C findings (inventory):
- None. No follow-up Class C bug beads were filed from this audit.

### Testing Strategy

1. **Golden tests (Phase 0):** Capture CLI output and API response snapshots
   for all commands being migrated. These run before AND after migration.
2. **Existing tests** pass unchanged (both CLI integration and API protocol tests).
3. **New unit tests** for each `action_*` function test validation logic directly
   against `GameState`, without going through CLI parsing or JSON serialization.
4. **Typed payload tests** verify that `serde_json::to_value(&result)` produces
   the expected JSON shape (field names, types, nesting). These replace the
   `serde_json::json!({})` assertions in current API tests with compile-time
   struct-based guarantees.
5. **State mutation tests** (Gate 3) serialize GameState before/after via both
   old and new paths to verify identical side effects.

### Example: Full Before/After for `end_combat`

**Before (CLI):** `command/combat_cmds.rs` EndCombatCommand::execute - 25 lines
of validation, state mutation, output formatting.

**Before (API):** `gmapi/combat_handlers.rs` end_combat - 35 lines of same
validation, state mutation, structured output formatting.

**After (engine):**
```rust
pub fn action_end_combat(state: &mut GameState) -> Result<EndCombatResult, EngineError> {
    let combat_state = state.combat.take()
        .ok_or_else(|| EngineError::WrongState("no active combat.".into()))?;

    let dead_monsters = combat_state.monsters.iter().filter(|m| !m.is_alive()).count();
    let total_xp: u64 = combat_state.monsters.iter()
        .filter(|m| !m.is_alive())
        .map(|m| m.xp_value).sum();
    let dead_party = state.party.members.iter().filter(|c| !c.is_alive()).count();
    state.mode = state.pre_combat_mode.take().unwrap_or(GameMode::Idle);

    // Compute retainer XP shares (Class B: CLI uses this, API includes in payload)
    let retainer_xp_shares = compute_retainer_xp_shares(&state.party, total_xp);

    if let Some(dungeon) = state.dungeon.as_mut() {
        if let Some(room_id) = dungeon.current_room {
            if let Some(room) = dungeon.find_room_mut(room_id) {
                room.monsters_cleared = true;
            }
        }
    }

    Ok(EndCombatResult {
        message: format!("combat ended after {} rounds. {} of {} monsters defeated.",
            combat_state.round, dead_monsters, combat_state.monsters.len()),
        rounds: combat_state.round,
        monsters_defeated: dead_monsters,
        total_xp,
        party_casualties: dead_party,
        retainer_xp_shares,
    })
}
```

**After (CLI):** Calls action, formats `message` + retainer XP lines from typed result.
**After (API):** Calls action, serializes `EndCombatResult` via `serde_json::to_value(&r)`.

## Review Findings Addressed

This design revision addresses three issues from Codex review (oag-e57i4):

- **oag-e57i4.2 (P1):** Added Command Parity Classification section with
  known divergences table and per-command audit gate before migration.
- **oag-e57i4.3 (P1):** Replaced `EngineResult.data: Option<serde_json::Value>`
  with typed per-command result structs. Serialization stays in API adapter.
- **oag-e57i4.1 (P2):** Added Phase 0 (parity audit + golden tests) and
  Compatibility Gates section with three concrete gates and rollback criterion.

## Summary

- Single source of truth for validation and orchestration per command.
- CLI and API become thin input/output adapters (~3-5 lines each).
- Typed result structs per command — compile-time payload guarantees, no raw JSON in engine.
- Command parity matrix classifies each command before migration begins.
- Golden tests (CLI output, API response, state mutation) gate every migration step.
- Incremental migration, one subsystem at a time.
- All existing tests continue passing.
- No protocol/wire format changes.
