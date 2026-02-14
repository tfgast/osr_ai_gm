# Style Review

**Reviewer:** polecat/fury (hq-leg-g3cac)
**Date:** 2026-02-05
**Scope:** Full codebase (~50 source files across command/, engine/, rules/, state/, session/, gmapi/, dice/, persist/, model/, telemetry + 2 integration test files)

## Summary

The codebase is well-structured with consistent naming conventions (snake_case/CamelCase), good test coverage with descriptive names (no `test_` prefix), and a well-applied `_with<R: Rng>()` pattern for testable randomness. The main style issues fall into a few systemic categories: (1) `///` used instead of `//!` for module-level docs across ~19 files, (2) `format!("{}", x)` used instead of `.to_string()` in ~30+ locations, (3) inconsistent import ordering (std vs external vs crate), and (4) a handful of project-convention violations (error message casing, import paths, struct doc comments). No critical issues were found.

## Critical Issues
(P0 - Must fix before merge)

None. No correctness bugs, security issues, or build-breaking style problems were found.

## Major Issues
(P1 - Should fix before merge)

### 1. Module-level doc comments use `///` instead of `//!` (~19 files)

Rust `///` documents the *next item*, not the enclosing module. When placed at the top of a file before `use` statements, they attach to the first import, producing misleading rustdoc and triggering the `empty_line_after_doc_comments` clippy warning. This accounts for 17 of the 29 known clippy warnings.

**Affected files (non-exhaustive):**
- `engine/chargen.rs:1`, `engine/combat.rs:1-8`, `engine/retainer.rs:1`, `engine/xp.rs:1`
- `rules/ability.rs:1`, `rules/class.rs:1`, `rules/encumbrance.rs:1-12`, `rules/equipment.rs:1`, `rules/magic_item.rs:1`, `rules/module.rs:1`, `rules/monster.rs:1`, `rules/save.rs:1`, `rules/spell.rs:1`, `rules/spell_data.rs:1`, `rules/thief.rs:1`, `rules/treasure.rs:1-3`, `rules/turn.rs:1-18`, `rules/xp.rs:1`
- `tests/integration.rs:1-3`, `tests/gmapi_protocol_qa.rs:1-8`

**Fix:** Replace leading `///` with `//!` in all these files.

### 2. Inconsistent `use super::` vs `use crate::command::` in command/ module

10 of 12 command files use `use super::{Command, CommandResult}`, but two deviate:
- `command/gm_cmds.rs:1` -- `use crate::command::{Command, CommandResult};`
- `command/lookup_cmds.rs:1` -- `use crate::command::{Command, CommandResult};`

**Fix:** Change both to `use super::{Command, CommandResult};` to match the dominant convention.

### 3. Error messages with uppercase first letter violate project convention

MEMORY.md documents: "Error messages: lowercase start (since `CommandResult::error()` prepends `Error: `)." Two violations:
- `command/exploration_cmds.rs:270` -- `"Door {} not found."` (should be `"door {} not found."`)
- `command/exploration_cmds.rs:275` -- `"Door {} is locked..."` (should be `"door {} is locked..."`)

### 4. Doc comments on command structs in gm_cmds.rs violate project convention

MEMORY.md states: "No doc comments on command structs (rely on `help()` method instead)." `gm_cmds.rs` is the only file that adds `///` doc comments to command structs (11 instances at lines 12, 81, 101, 125, 160, 175, 202, 229, 256, 278, 299). No other command file does this.

**Fix:** Remove or convert to `//` comments.

### 5. `format!("{}", x)` instead of `.to_string()` (~30+ sites)

Clippy flags `format!("{}", x)` where `x` implements `Display` as `clippy::to_string_in_format_args`. Affects:

- `command/combat_cmds.rs` -- 7 instances (lines 157, 175, 227, 257, 308, 337, 590)
- `command/exploration_cmds.rs` -- 5 instances (lines 82, 102, 124, 245, 314)
- `command/gm_cmds.rs:97`, `command/wilderness_cmds.rs:89`, `command/system.rs:15-16`, `command/mod.rs:118`
- `engine/combat.rs` -- 6 instances (lines 305, 401, 483, 564, 592, 686)
- `engine/encounter_engine.rs:266`
- `gmapi/interface.rs` -- 6 instances (lines 360, 387, 404, 578, 622, 638)

**Fix:** Replace with `x.to_string()` throughout.

### 6. Duplicate `Alignment` enum in npc_party.rs

`rules/npc_party.rs:121-126` defines its own `Alignment` enum (`Lawful`, `Neutral`, `Chaotic`) with its own `Display` impl, duplicating `rules::alignment::Alignment`. Changes to alignment semantics require updating two places.

**Fix:** Import and use `super::alignment::Alignment` instead.

### 7. Wildcard imports in main.rs obscure dependencies

`main.rs:11-22` has 9 wildcard imports (`use command::party::*;`, etc.) making it impossible to trace which struct comes from which module. Contrast with the explicit import of `gm_cmds::{...}` on line 20.

**Fix:** Replace wildcard imports with explicit imports listing the specific command structs used.

## Minor Issues
(P2 - Nice to fix)

### 8. `GameMode` is `Clone` but not `Copy`, causing ~100 unnecessary `.clone()` calls

`state/game.rs:4` derives `Clone` but not `Copy` on `GameMode`, a unit-variant-only enum. This forces `state.mode.clone()` at every `GMResponse` construction in `interface.rs`. Adding `Copy` would eliminate all of them.

### 9. Inconsistent RNG parameter position in `_with()` functions

`chargen.rs` and `combat.rs` consistently put `rng` **last**. `exploration.rs`, `encounter_engine.rs`, `wilderness_engine.rs` consistently put `rng` **first**. `xp.rs` puts `rng` first (inconsistent with the combat.rs family). This forces callers to remember which convention each function uses.

### 10. Missing doc comments on public `_with<R: Rng>()` functions (11 functions)

The convention (visible in `retainer.rs:66`, `exploration.rs:86`) is `/// Testable version with explicit RNG.` Many `_with` functions lack any doc comment:
- `chargen.rs:22,36,47`
- `combat.rs:148,350,424,501,585,611`
- `wilderness_engine.rs:270,325`

### 11. Import ordering: `std` after external crates

Rust convention orders: `std` -> external -> crate. Several files reverse this:
- `engine/combat.rs:10-11` -- `rand::Rng` before `std::fmt`
- `state/dungeon.rs:1-4` -- `std` imports interleaved with `serde`
- `persist/mod.rs:1-4` -- `serde` before `std`

### 12. Unsorted module declarations

Module declarations in several `mod.rs` files are in no discernible order:
- `command/mod.rs:1-12` -- 12 module declarations in arbitrary order
- `session/mod.rs:1-3` -- `player`, `state`, `io` (not alphabetical)
- `state/mod.rs:1-4` -- `time`, `dungeon`, `wilderness`, `game` (not alphabetical)

### 13. Unnecessary `mut` on test variables

- `command/gm_cmds.rs:825` -- `let mut c = make_leveled_fighter(...)` immediately moved, never mutated
- `command/gm_cmds.rs:836` -- same pattern

### 14. Duplicated test helper functions across integration test files

`tests/integration.rs:18-95` and `tests/gmapi_protocol_qa.rs:23-84` duplicate `unique_tmp_path()`, `req()`, `make_fighter()`, `make_thief()`, `make_cleric()` nearly identically. These should be extracted to a shared test helper module.

### 15. Inline `std::fmt` usage instead of import

`engine/exploration.rs:53-54` and `engine/wilderness_engine.rs:43-44` use `std::fmt::Display` and `std::fmt::Formatter<'_>` fully qualified, while all other files import `use std::fmt;`.

### 16. Inconsistent compact vs expanded formatting for name()/help()

9 of 12 command files use compact single-line style for `name()` and `help()`. Three files (`lookup_cmds.rs`, `treasure_cmds.rs`, `module_cmds.rs`) use expanded multi-line formatting.

### 17. `roll_gem_value()` and `roll_jewellery_value()` bypass `_with` testability pattern

`rules/treasure.rs:263-267,288-294` call `rand::thread_rng()` directly without providing `_with<R: Rng>()` variants, unlike the rest of the codebase.

### 18. Boilerplate state access pattern in interface.rs

The pattern `match state.combat.as_mut() { Some(c) => c, None => return GMResponse::err(...) }` appears ~20 times. A helper method on `GameState` (e.g., `state.require_combat()`) would reduce 4 lines to 1.

### 19. Enum variant comments use `//` instead of `///` in retainer.rs

`engine/retainer.rs:42-46,97-99` uses inline `//` comments on enum variants while other enum types use proper `///` doc comments.

### 20. `module_to_dungeon` visibility may be overly broad

`command/module_cmds.rs:59` declares `pub fn module_to_dungeon(...)` but it appears to only be used within the same file and tests. Should be `pub(crate)`.

### 21. Unsafe env var manipulation in telemetry tests

`telemetry.rs:116-129` uses `unsafe { std::env::remove_var("HOME") }` and `set_var` which race with parallel test execution.

## Observations
(Non-blocking notes and suggestions)

- **Test naming** is fully compliant across all ~50 files -- descriptive names without `test_` prefix. Zero violations.
- **No wildcard imports** in production code (only `use super::*` in test modules, which is standard).
- **Naming conventions** are perfect -- all functions snake_case, all types CamelCase, all constants SCREAMING_SNAKE_CASE. Zero violations.
- **`_with<R: Rng>()` pattern** is applied universally for testable randomness (except P2-17 above). This is a strong project pattern.
- **`combat.rs`** at 2257 lines is 3-4x the size of any other file. Section banner comments help, but it could benefit from extraction of movement/retreat, turn undead, and status display logic into separate sub-modules.
- **`DoorState::Spiked`** renders as `"spiked open"` in `dungeon.rs:350` `status()` but as `"spiked"` in `Display`/`FromStr` -- minor inconsistency in user-facing output.
- **`capitalize()` utility** is buried as a private function in `state/time.rs:4-11`. If needed elsewhere, it would need duplication.
- **`session/state.rs:106`** has a test helper named `test_session()` -- the one place where the `test_` prefix convention is violated (should be `sample_session()` or `fixture_session()`).

## Prioritized Fix Recommendations

| # | Sev | Description | Files | Effort |
|---|-----|-------------|-------|--------|
| 1 | P1 | Convert `///` to `//!` for module docs | ~19 files | Mechanical, low risk |
| 2 | P1 | `format!("{}", x)` to `.to_string()` | ~12 files, ~30 sites | Mechanical, low risk |
| 3 | P1 | Fix `use super::` consistency in command/ | 2 files | Trivial |
| 4 | P1 | Fix uppercase error messages | 1 file, 2 lines | Trivial |
| 5 | P1 | Remove doc comments from command structs in gm_cmds.rs | 1 file, 11 lines | Trivial |
| 6 | P1 | Remove duplicate Alignment enum from npc_party.rs | 1 file | Low |
| 7 | P1 | Replace wildcard imports in main.rs | 1 file | Low |
| 8 | P2 | Add `Copy` to `GameMode` | 1 file + ~100 .clone() removals | Medium |
| 9 | P2 | Standardize RNG parameter position | ~15 functions | High (API change) |
| 10 | P2 | Add doc comments to `_with` functions | 11 functions | Low |
| 11 | P2 | Fix import ordering (std first) | ~5 files | Trivial |
| 12 | P2 | Sort module declarations | 3 files | Trivial |
| 13 | P2 | Extract shared test helpers | 2 files | Low |
| 14 | P2 | Add `_with` variants to treasure.rs roll functions | 1 file | Low |
