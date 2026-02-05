# Security Review

## Summary

The osr_ai_gm codebase (~24K lines of Rust) has a generally strong security posture thanks to Rust's memory safety guarantees and the absence of shell execution or network-facing attack surfaces. The application is a local CLI tool and JSON-pipe server for tabletop RPG game mastering. However, several issues were identified across path traversal, denial-of-service via unbounded allocation, panic-inducing unwraps, and missing authorization boundaries. No hardcoded secrets, command injection vectors, or XSS/SQL injection vulnerabilities were found.

## Critical Issues

(P0 - Must fix before merge)

- **No critical issues identified.** The application runs locally and has no network listeners, authentication systems, or remote attack surface. All findings below are P1/P2 given the local-only threat model.

## Major Issues

(P1 - Should fix before merge)

### 1. Path Traversal in Save, Load, and LoadModule commands

**Files:**
- `src/rules/module.rs:69-72` - `load_module()` passes user path directly to `fs::read_to_string()`
- `src/gmapi/interface.rs:1199-1200` - `save_game()` passes user path directly to `persist::save()`
- `src/gmapi/interface.rs:1206-1207` - `load_game()` passes user path directly to `persist::load()`
- `src/command/system.rs:27` - CLI `SaveCommand` passes user arg to `persist::save()`
- `src/command/system.rs:40` - CLI `LoadCommand` passes user arg to `persist::load()`
- `src/command/module_cmds.rs:26-27` - CLI `LoadModuleCommand` passes user arg to `load_module()`

**Impact:** A user (or AI GM via JSON protocol) can read/write arbitrary files accessible to the process. For the JSON protocol interface (where an AI agent sends commands), a compromised or misbehaving AI could:
- Read arbitrary files: `{"type":"Load","params":{"path":"/etc/passwd"}}` (would fail JSON parse, but error message leaks file existence)
- Write to arbitrary locations: `{"type":"Save","params":{"path":"/tmp/exploit.json"}}` (writes valid game state JSON)
- Read arbitrary JSON: `{"type":"LoadModule","params":{"path":"../../../sensitive.json"}}` (if it happens to parse as ModuleDef)

**Suggested fix:** Validate and constrain paths to a known safe directory. For Save/Load, restrict to the current working directory or a designated saves folder. For LoadModule, restrict to `data/modules/` subtree. Use `std::fs::canonicalize()` + `starts_with()` to prevent traversal.

### 2. Denial of Service via Unbounded Monster Spawning

**Files:**
- `src/gmapi/interface.rs:291-307` - `spawn_encounter()` allocates `count` monsters in a loop with no upper bound
- `src/gmapi/interface.rs:1011-1038` - `spawn_monster()` allocates `count` monsters with no upper bound

**Impact:** A JSON API caller can send `"count": 4294967295` (u32::MAX) causing the process to attempt allocating billions of Monster structs, leading to OOM crash or system resource exhaustion.

**Suggested fix:** Add a reasonable upper bound (e.g., `const MAX_MONSTERS: u32 = 100`). The CLI version in `src/command/gm_cmds.rs:28` has minimal validation (`n >= 1`) but also lacks an upper bound.

### 3. No Size Limits on Deserialized Files

**Files:**
- `src/persist/mod.rs:97` - `load()` reads entire file into memory via `fs::read_to_string()` with no size check
- `src/rules/module.rs:71` - `load_module()` reads entire file with no size check

**Impact:** A crafted multi-gigabyte save file or module file could exhaust memory. Combined with the path traversal issue, this could be used to read large files into memory.

**Suggested fix:** Check `fs::metadata(path)?.len()` against a maximum (e.g., 10 MB) before reading.

## Minor Issues

(P2 - Nice to fix)

### 4. Unwrap Panics on Pre-validated Optional State

**Files:**
- `src/gmapi/interface.rs:385` - `state.combat.as_mut().unwrap()` in `monster_attack()` (after early-return None check at line 367)
- `src/gmapi/interface.rs:459` - `state.combat.as_mut().unwrap()` in `retreat()` (after `is_none()` check at line 449)
- `src/gmapi/interface.rs:501` - `state.combat.take().unwrap()` in `end_combat()`
- `src/gmapi/interface.rs:721` - `state.wilderness.as_mut().unwrap()` in `travel()`
- `src/gmapi/interface.rs:542` - `dungeon.add_room(Room::new(0, room_name)).unwrap()` in `enter_dungeon()`

**Impact:** While these are logically safe (preceded by None checks), they represent fragile patterns. If control flow changes, these become panic points. In a server context, panics crash the process.

**Suggested fix:** Replace with `match` or propagate errors via `GMResponse::err()`.

### 5. No Authorization Model Between GM and Player Roles

**Files:**
- `src/command/mod.rs:89` - `CommandRegistry::dispatch()` has no role checking
- `src/gmapi/interface.rs:15` - `handle_request()` has no caller authentication

**Impact:** All commands (including GM-only commands like `damage`, `set_hp`, `award_xp`, `kill`) are accessible to any caller. The protocol comments mark commands as "GM-only" but this is not enforced. In a multi-player session, a player could issue GM commands.

**Suggested fix:** For the current single-user CLI tool, this is acceptable. If multi-player or AI-player sessions are planned, add role tagging to commands and validate caller permissions in the dispatch layer.

### 6. Telemetry Logs Raw User Input Without Sanitization

**File:** `src/telemetry.rs:25-28`

**Impact:** Failed commands are logged to `~/.osr_data/telemetry/commands.jsonl` including `raw_input`. If a user types sensitive information as a command (accidentally), it persists in the telemetry log. The log file has no rotation or size limits, so it grows unboundedly.

**Suggested fix:** Consider adding log rotation (max file size or max entries). Document what is logged. Ensure the telemetry directory has appropriate permissions (currently relies on HOME directory defaults).

### 7. Error Messages Leak Filesystem Paths

**Files:**
- `src/rules/module.rs:72` - `format!("Failed to read module file {}: {}", path.display(), e)`
- `src/persist/mod.rs:98-99` - IO error includes system error details

**Impact:** Error messages returned to callers include full filesystem paths and OS-level error messages, which could reveal system structure to an untrusted caller.

**Suggested fix:** For JSON API responses, consider sanitizing error messages to remove absolute paths.

### 8. Dice Roller DoS via Large Count

**File:** `src/dice/mod.rs:95-100`

**Impact:** While `count` is validated as `> 0`, there's no upper bound. A dice expression like `999999999d6` would allocate a huge Vec and iterate billions of times. Through the JSON API: `{"type":"Roll","params":{"notation":"999999999d6"}}`.

**Suggested fix:** Cap dice count at a reasonable maximum (e.g., 1000).

### 9. XP Addition Without Overflow Protection

**Files:**
- `src/gmapi/interface.rs:808` - XP award adds to `character.xp` (u64) without `checked_add()` or `saturating_add()`

**Impact:** In release builds, Rust u64 arithmetic wraps silently on overflow. While practically unlikely (u64::MAX is ~18 quintillion), a malicious JSON API caller could award `u64::MAX` XP to wrap a character's XP back to near zero.

**Suggested fix:** Use `character.xp = character.xp.saturating_add(xp_amount)`.

### 10. HP Subtraction Without Saturation

**Files:**
- `src/engine/combat.rs:384,466,546` - HP damage uses raw `i32` subtraction

**Impact:** HP is `i32`. While negative HP is semantically valid (dead), extremely large damage values could theoretically wrap. In practice, damage values are small, so this is low risk. The `is_alive()` check (`hp > 0`) handles negative HP correctly.

**Suggested fix:** Consider `saturating_sub()` for defensive hardening.

### 11. No Limit on Party Size or String Lengths

**Files:**
- `src/gmapi/interface.rs:231` - `CreateCharacter` adds members without a party size cap
- Multiple locations accept character/room/spell names without length limits

**Impact:** Unbounded party creation or very long string inputs could cause gradual memory growth. A caller could create millions of characters or submit megabyte-length names.

**Suggested fix:** Cap party size (e.g., 100) and string inputs (e.g., 200 chars).

## Observations

(Non-blocking notes and suggestions)

- **No `unsafe` in production code.** The only `unsafe` blocks are in test code (`src/telemetry.rs:116,128`) for manipulating environment variables. This is excellent.
- **No shell execution.** No use of `std::process::Command` anywhere in the source. No command injection surface exists.
- **No network listeners.** The application reads from stdin only. No HTTP server, no sockets.
- **Static data files use `include_str!` and hardcoded paths.** Equipment, monsters, spells, etc. are loaded from compile-time or known-relative paths (e.g., `data/core/monsters.json`). These are not user-controllable.
- **Atomic save writes.** The `persist::save()` function uses write-to-temp-then-rename, preventing save file corruption on crash. Good practice.
- **Proper input validation in most handlers.** Monster index bounds checks, character existence checks, and numeric parsing with error returns are consistently applied.
- **Serde deserialization is well-structured.** The tagged enum pattern with proper error propagation handles malformed JSON gracefully without panics.
- **XP and gold use `u64`.** Integer overflow at u64 scale (18 quintillion) is practically impossible in normal gameplay.
