# Code Smells Review

## Summary

The codebase is a well-structured OSE/B/X tabletop RPG engine in Rust with good type safety and comprehensive test coverage. However, four files exceed 1,000 lines each (combat.rs at 2,256, interface.rs at 1,805, exploration.rs at 1,265, wilderness_engine.rs at 1,203), forming "god files" that concentrate too much logic. The most pervasive code smell is repeated boilerplate in `interface.rs`: party member lookup (15 occurrences), combat state validation (13 occurrences), and `GMResponse::err()` calls (76 occurrences) follow identical patterns that could be extracted into helpers. The `_with<R: Rng>` testability pattern, while sound in principle, is applied to 33 functions and doubles the API surface area. Overall, technical debt is being managed well (only 2 TODOs in the entire codebase), but the next significant change to combat or the GM API will be painful without decomposition.

## Critical Issues
(P0 - Must fix before merge)

None. The code smells are structural and do not affect correctness or safety.

## Major Issues
(P1 - Should fix before merge)

- **God file: `src/engine/combat.rs` (2,256 lines)**. Contains initiative, melee attacks, missile attacks, monster attacks, morale, turn undead, retreat, fighting withdrawal, and all associated tests. A change to attack resolution requires navigating 500+ lines of neighboring unrelated combat logic. Split into submodules: `combat/initiative.rs`, `combat/attack.rs`, `combat/morale.rs`, etc.

- **God file: `src/gmapi/interface.rs` (1,805 lines)**. Every GM command handler lives in one file. The `handle_request()` dispatcher (lines 15-107) delegates to ~40 handler functions all defined below it. Grouping handlers into submodules (combat_handlers, exploration_handlers, query_handlers) would make navigation tractable.

- **DRY violation: party member lookup repeated 15 times** in `interface.rs`. The pattern:
  ```rust
  let character = match state.party.find_member(name) {
      Some(c) => c,
      None => return GMResponse::err(id, format!("no party member named '{}'.", name), state.mode.clone()),
  };
  ```
  Should be extracted to a helper like `fn require_member<'a>(id: &str, state: &'a GameState, name: &str) -> Result<&'a Character, GMResponse>`.

- **DRY violation: combat state validation repeated 13 times** in `interface.rs`. The pattern:
  ```rust
  let combat = match state.combat.as_mut() {
      Some(c) => c,
      None => return GMResponse::err(id, "no active combat.", state.mode.clone()),
  };
  ```
  Same remedy: extract to `fn require_combat<'a>(id: &str, state: &'a mut GameState) -> Result<&'a mut CombatState, GMResponse>`.

- **Deep nesting in `move_through_door_with()`** (`src/engine/exploration.rs:384-490`, 106 lines). This function checks light, finds door, validates passability, moves rooms, auto-closes doors, checks traps, checks placed monsters (with nested spawned/unspawned filtering at 4 levels deep), and checks wandering monsters. Each concern is a candidate for extraction.

## Minor Issues
(P2 - Nice to fix)

- **God file: `src/engine/exploration.rs` (1,265 lines)** and **`src/engine/wilderness_engine.rs` (1,203 lines)**. Both are approaching pain thresholds. Less urgent than combat.rs/interface.rs but worth monitoring.

- **`src/command/gm_cmds.rs` (844 lines)**: Large match-based command dispatcher with inline parsing logic. Similar pattern to `handle_request()` but at the CLI layer.

- **Primitive obsession with `is_elf: bool`** in `search()` (`interface.rs:62`) and `listen_at_door_with()` (`exploration.rs:251`). The boolean parameter obscures intent. A `Race` enum or `SearchModifiers` struct would be clearer and extensible (e.g., when halflings get different bonuses).

- **33 `_with<R: Rng>` function pairs** across the engine. Every randomized function has a public wrapper that calls `_with(&mut rand::thread_rng())`. This is a disciplined testability pattern but doubles the public API surface. Consider a trait-based or context-passing approach to reduce the boilerplate.

- **5 `#[allow(dead_code)]` suppressions** on JSON metadata structs (`treasure.rs:112`, `magic_item.rs:66`, `spell_data.rs:50`, `equipment.rs:311`, `monster.rs:168`). These indicate deserialized fields that are never read in code. Either use the fields or remove them from the structs.

- **Data clump: encounter parameters**. The tuple `(name, count, hd, ac, hp, damage, morale, distance)` appears in both `combat_cmds.rs` (CLI parsing) and `protocol.rs` (`EncounterParams` struct). The struct exists but isn't used consistently at the CLI layer.

## Observations
(Non-blocking notes and suggestions)

- **Minimal TODO debt**: Only 2 TODOs found in the entire codebase (`src/rules/monster.rs:263-264` about loading module/user monster data). This is exceptionally clean for a codebase of this size (~23k lines).

- **Strong type discipline**: Enums for `Class`, `Alignment`, `GameMode`, `DoorState`, `Terrain` prevent many categories of bugs. The `GMCommand` enum in protocol.rs provides exhaustive command coverage.

- **Good error handling**: Consistent use of `Result<T, String>` in engine functions and `GMResponse::err()` in API handlers. The patterns are repetitive but correct.

- **Test coverage is solid**: Most engine files have `#[cfg(test)]` modules with thorough scenario coverage, including the `_with` RNG pattern enabling deterministic tests.

- **Feature envy in interface.rs**: Most handler functions reach deep into `state.party`, `state.combat`, `state.dungeon`, `state.time` rather than operating through higher-level GameState methods. This creates tight coupling between the API layer and internal state representation. Adding a method layer on GameState (e.g., `state.require_combat()`, `state.find_party_member()`) would reduce coupling and the 76 `GMResponse::err()` calls.

- **The next painful change**: Any refactor to `CombatState` will require updating combat.rs (2,256 lines), interface.rs (handlers that access combat), and combat_cmds.rs (CLI layer) — a classic shotgun surgery pattern across 3 files.
