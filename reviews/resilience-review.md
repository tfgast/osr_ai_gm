# Resilience Review

## Summary

The codebase demonstrates solid resilience practices overall. The CLI architecture inherently avoids many resilience pitfalls: there are no external service dependencies, no network calls, no async operations, and the command pattern returns `CommandResult` values rather than panicking. Error handling is consistent across all 12 command modules, with proper validation of user input and graceful `CommandResult::error()` returns. The save system uses atomic writes (temp file + rename) which is the gold standard for file corruption prevention. The most notable findings are minor: a potential arithmetic underflow in an error message format string, orphaned temp files on save failure, and a few unnecessary `unwrap()` calls that could be replaced with safer alternatives.

## Critical Issues
(P0 - Must fix before merge)

None found.

## Major Issues
(P1 - Should fix before merge)

### 1. Orphaned temp file on `save()` rename failure
**File:** `src/persist/mod.rs:90-91`

If `fs::write(&tmp_path, &json)` succeeds but `fs::rename(&tmp_path, path)` fails (e.g., cross-device rename, permissions), the temp file `.save.json.tmp` is left on disk and never cleaned up. While the atomic write pattern itself is correct, there's no cleanup of the temp file on the rename failure path.

**Impact:** Disk clutter over time; user confusion from orphaned `.*.tmp` files.

**Suggested fix:** Add cleanup on the error path:
```rust
fs::write(&tmp_path, &json)?;
fs::rename(&tmp_path, path).map_err(|e| {
    let _ = fs::remove_file(&tmp_path); // best-effort cleanup
    e
})?;
```

### 2. Arithmetic underflow in `MonsterAttackCommand` error message
**File:** `src/command/combat_cmds.rs:205`

```rust
"monster index {} out of range (0-{})",
    monster_idx, combat.monsters.len() - 1
```

`combat.monsters.len()` is `usize`. If `monsters` is empty, `len() - 1` underflows to `usize::MAX`, producing a confusing error message like `"monster index 0 out of range (0-18446744073709551615)"`.

**Impact:** While combat creation validates `count >= 1`, a corrupted save file loaded via `load` could deserialize a `CombatState` with zero monsters. The `AttackCommand` at line 152 has the same pattern (indexing `combat.monsters[monster_idx]`) but is implicitly guarded by the subsequent `is_alive()` check, which would still succeed on an empty vec without reaching the underflow.

**Suggested fix:** Use saturating subtraction or check for empty:
```rust
if combat.monsters.is_empty() {
    return CommandResult::error("no monsters in combat.");
}
```

## Minor Issues
(P2 - Nice to fix)

### 3. `enter_dungeon` silently ignores `add_room` error
**File:** `src/command/exploration_cmds.rs:23`

```rust
dungeon.add_room(Room::new(0, &room_name)).unwrap();
```

This `unwrap()` is technically safe (adding room 0 to a fresh dungeon can't fail), but if the `DungeonState::new()` implementation ever changes to pre-populate rooms, this would panic. A `.map_err()` + `CommandResult::error()` would be more defensive.

### 4. `enter_wilderness` silently ignores `add_hex` error
**File:** `src/command/wilderness_cmds.rs:21`

```rust
ws.add_hex(HexCell::new(0, 0, terrain)).unwrap();
```

Same pattern as above — safe today but fragile to future changes.

### 5. `roll_gold` panics on malformed class data
**File:** `src/engine/chargen.rs:50,54`

```rust
let expr = dice::parse(dice_part).expect("valid dice notation in starting_gold");
```

If a class definition has a malformed `starting_gold` string, the entire process panics. This is data-driven from `class_def()` which is compile-time-hardcoded, so it's currently safe, but `expect()` on data parsing is a code smell.

### 6. Telemetry file grows unboundedly
**File:** `src/telemetry.rs:25-29`

`log_failed_command` appends to `~/.osr_data/telemetry/commands.jsonl` with no rotation, truncation, or size limit. Over long play sessions with many typos, this file could grow indefinitely.

**Impact:** Low — the file only logs errors, not all commands, and typical play sessions are short.

### 7. No save-file version migration
**File:** `src/persist/mod.rs:14,20-21`

`SAVE_VERSION` is set to 1 and `default_version()` returns 0, but there's no migration logic. If a future version bumps `SAVE_VERSION`, old save files will load with `version: 0` but no code handles the upgrade path.

**Impact:** Future technical debt — not a current issue, but worth noting for when schema changes are needed.

### 8. `LootCommand` room treasure matching is case-insensitive but exact
**File:** `src/command/inventory_cmds.rs:187`

Room treasure lookup uses `eq_ignore_ascii_case` which requires the full treasure description to match exactly (minus case). If a module defines treasure as "500 gold pieces" but the player types "gold pieces", the loot fails. No fuzzy matching or suggestions like the `buy` command provides.

**Impact:** UX friction — players must know exact treasure names from the module.

## Observations
(Non-blocking notes and suggestions)

### Error handling is consistently excellent
Every command validates arguments, checks for required game state (combat active, dungeon entered, etc.), and returns `CommandResult::error()` with helpful messages. The pattern of `match state.combat.as_mut() { Some(c) => c, None => return CommandResult::error(...) }` is used consistently and correctly throughout.

### The `unwrap()` after `is_none()` guard pattern is safe but verbose
Files like `wilderness_cmds.rs:87,101,115` use:
```rust
if state.wilderness.is_none() { return CommandResult::error(...); }
let ws = state.wilderness.as_mut().unwrap();
```
This is functionally correct but could use `match` or `let-else` for clarity. Not a bug, just a style inconsistency — other commands use the cleaner `match` pattern.

### OpenCommand has multiple safe unwraps guarded by prior checks
`src/command/exploration_cmds.rs:293,298-299,311` — The unwraps on `state.dungeon.as_mut().unwrap()` and `doors.iter().find(...).unwrap()` are all guarded by earlier checks that ensure the values exist. The logic is sound but the code is harder to audit because the safety invariants are implicit.

### No external service dependencies = no circuit breaker needs
The codebase is a pure CLI tool with no network calls, no database connections, and no external APIs. The questions "What happens when external services fail?" and "Can the system recover from partial failures?" are not applicable. The only I/O is file reads/writes for save/load and module loading, which are properly handled with `Result` returns.

### Recovery from partial failures
The `save()` function's atomic write pattern means save corruption from crashes is prevented. The `load()` function properly validates JSON structure via serde. The only partial failure risk is in `EndCombatCommand` where combat state is consumed via `take()` before the output string is built — if the process crashes between `take()` and the user seeing the output, the combat state is lost but the rest of the game state is intact (which is the correct behavior for "combat ended").

### Panic behavior is appropriate for a CLI tool
The handful of `expect()` calls in non-test code are all on compile-time-embedded data (`include_str!` JSON) or hardcoded dice expressions (`"3d6"`), which is acceptable — these would only fail if the bundled data is corrupted, which is a build-time defect, not a runtime error.
