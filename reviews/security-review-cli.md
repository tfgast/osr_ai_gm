# Security Review

## Summary

The osr_ai_gm codebase is a Rust CLI application implementing an Old-School Renaissance tabletop RPG game engine. The security posture is strong overall: safe Rust throughout production code, minimal dependencies (serde, serde_json, rand), no network surface, no shell execution, and no database access. The primary attack surface is file I/O via user-provided paths in save/load/module commands, and JSON deserialization of untrusted data. The application runs as a local CLI tool (stdin/stdout) with an optional JSON pipe protocol for AI participants.

No critical vulnerabilities were found. The main concerns are path traversal in file operations and unbounded deserialization, both mitigated by the local-only execution model.

## Critical Issues

(P0 - Must fix before merge)

None found.

## Major Issues

(P1 - Should fix before merge)

### 1. Path Traversal in Save/Load Commands

**Files:**
- `src/command/system.rs:27` (SaveCommand)
- `src/command/system.rs:40` (LoadCommand)
- `src/gmapi/interface.rs:1199-1217` (save_game/load_game)

The `save` and `load` commands pass user-provided paths directly to filesystem operations with no sanitization:

```rust
// system.rs:27
let path = args.first().copied().unwrap_or("save.json");
match persist::save(state, std::path::Path::new(path)) { ... }
```

A user can write game state JSON to any writable path (`save ../../.bashrc`) or probe filesystem paths via load errors. Via the GM API (`GMCommand::Save`/`GMCommand::Load`), an AI agent could be manipulated into using traversal paths.

**Impact:** Arbitrary file write (game state JSON to any writable path). Arbitrary file read reconnaissance via error messages.

**Suggested fix:** Restrict save/load to a specific directory (e.g., working directory or `~/.osr_data/saves/`). Strip `..` components. Reject absolute paths.

### 2. Path Traversal in LoadModule

**Files:**
- `src/rules/module.rs:83-94` (load_module)
- `src/command/module_cmds.rs:26`
- `src/gmapi/interface.rs:650-683`

The `load_module` function reads any file the process can access. While `expand_tilde` resolves `~/` paths, there is no restriction on absolute paths or `../` traversal:

```rust
// module.rs:83-94
pub fn load_module(path: &str) -> Result<ModuleDef, String> {
    let expanded = expand_tilde(path);
    let path = Path::new(&expanded);
    let content = fs::read_to_string(path) ...
```

**Impact:** Can read contents of arbitrary files (must parse as JSON, but errors reveal file existence). Via the GM API `LoadModule` command, an AI agent could be directed to probe the filesystem.

**Suggested fix:** Restrict module loading to specific directories (e.g., `data/modules/`, `~/.osr_data/modules/`). Validate paths don't escape allowed roots.

## Minor Issues

(P2 - Nice to fix)

### 3. Unbounded JSON Deserialization

**Files:**
- `src/persist/mod.rs:96-101` (load)
- `src/rules/module.rs:86-89` (load_module)

No size limits on loaded JSON files. `serde_json::from_str` will attempt to parse arbitrarily large or deeply nested JSON, which could cause OOM or stack overflow.

**Impact:** Denial of service via crafted save/module file. Requires local filesystem access.

**Suggested fix:** Check file size before reading (e.g., reject files >10MB). Consider `serde_json::from_reader` with a size-limited reader.

### 4. Unbounded Telemetry Log Growth

**Files:**
- `src/telemetry.rs:25-48`
- `src/command/mod.rs:110-119`

Failed commands are logged to `~/.osr_data/telemetry/commands.jsonl` with no size limit and no rotation. Repeated invalid commands grow this file without bound.

**Impact:** Disk exhaustion over extended use. Low severity.

**Suggested fix:** Implement log rotation or a maximum file size check.

### 5. `unsafe` Environment Variable Mutation in Tests

**File:** `src/telemetry.rs:116,128`

Uses `unsafe { std::env::remove_var("HOME") }` and `unsafe { std::env::set_var("HOME", home) }` in tests. This is UB if other tests read environment variables concurrently (tests run multi-threaded by default).

**Impact:** Potential UB in test suite. Not a production issue.

**Suggested fix:** Use `#[serial]` from `serial_test` crate, or use `temp_env` for thread-safe env manipulation in tests.

### 6. GM API Bypasses Session Permission Layer

**Files:**
- `src/gmapi/interface.rs:15-107` (handle_request)
- `src/session/state.rs:76-98` (dispatch_with_session)

The `handle_request` function processes all GM commands without checking caller permissions. The session layer (`dispatch_with_session`) includes permission checking via `Session::check_permission`, but the GM API `handle_request` doesn't use it. GM-only commands like `SpawnEncounter`, `AwardXp`, `Save`, `Load` can be issued by any connected participant.

**Impact:** No access control on GM API commands. Currently mitigated because the API is local-only (pipes), but relevant if the transport layer ever changes.

**Suggested fix:** Integrate permission checking into `handle_request`, or document that the GM API trusts all callers.

### 7. Production `unwrap()` Calls on Precondition-Guarded Values

**Files:**
- `src/gmapi/interface.rs:385,459,501,542,691,721`

Several `unwrap()` calls in production code rely on prior guards (early returns if None). While logically sound, if the guard logic changes, these become crash vectors.

**Impact:** Process crash if preconditions are violated. Very low risk.

**Suggested fix:** Consider using `expect("reason")` to document the invariant, or propagate errors.

## Observations

(Non-blocking notes)

- **No `unsafe` in production code.** All `unsafe` blocks are test-only. The entire production codebase is safe Rust.
- **No network surface.** CLI-only (stdin/stdout) and JSON pipes. No HTTP, TCP, or UDP listeners.
- **No shell execution.** No use of `std::process::Command` anywhere in the codebase.
- **No SQL/LDAP/SSRF surface.** No database connections or outbound network requests.
- **Minimal dependencies.** Only `serde 1`, `serde_json 1`, `rand 0.8` - mature, widely audited crates.
- **Good error handling patterns.** Commands consistently return `CommandResult::error()` rather than panicking. Input parsing uses `match` with explicit error returns.
- **Atomic save writes.** `persist::save` uses write-to-temp-then-rename, preventing corruption on crash (src/persist/mod.rs:80-92).
- **Case-insensitive character lookup.** `Party::find_member` uses `eq_ignore_ascii_case`, preventing case-based confusion.
- **Module validation.** Loaded modules are validated for structural consistency (entry room exists, exits reference valid rooms, bidirectional door states match).
- **Integer overflow.** Some arithmetic on game values (gold, XP, rations) could theoretically overflow u32/u64, but game values are far too small to trigger this in practice. Levels cap at ~14, XP at ~1M.
