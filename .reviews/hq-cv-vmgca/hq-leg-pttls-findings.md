# Security Review

## Summary

The OSR AI Game Master codebase demonstrates strong foundational security through Rust's memory safety guarantees, minimal `unsafe` usage (only 2 blocks, both in test code), no shell command execution with user input, and no database/SQL usage. However, critical path traversal vulnerabilities exist in file operations, and several denial-of-service vectors through unbounded inputs require remediation. The GM API lacks authentication, which matters if deployed as a network service.

## Critical Issues

### P0-1: Path Traversal in Module Loading
- **src/rules/module.rs:83-95** — `load_module()` accepts arbitrary user-provided paths with tilde expansion but no sanitization. Also reachable via `src/command/module_cmds.rs:26` and `src/gmapi/interface.rs:650-683`.
- **Impact:** Attacker can read arbitrary files (`load_module ../../../../etc/passwd`, `load_module ~/.ssh/id_rsa`). Error messages from JSON parse failures may leak partial file contents.
- **Fix:** Canonicalize paths and validate they resolve within an allowed modules directory. Consider a module registry with named lookups instead of raw paths.

### P0-2: Path Traversal in Save/Load
- **src/persist/mod.rs:76-101** — `save()` and `load()` accept arbitrary paths without validation. Reached via `src/command/system.rs:26-52` and `src/gmapi/interface.rs:1199-1217`.
- **Impact:** Arbitrary file read via `load /etc/shadow`; arbitrary file write/overwrite via `save /etc/cron.d/malicious`. The atomic write pattern (write-to-temp-then-rename) actually creates temp files in the target directory, extending the attack surface.
- **Fix:** Restrict save/load to a dedicated saves directory (e.g., `~/.osr_data/saves/`). Validate filenames are simple names, not paths.

### P0-3: Integer Overflow in Dice/Treasure Arithmetic
- **src/rules/npc_party.rs:222-227** — Unchecked `i32` addition in dice roll loop: `total += rng.gen_range(1..=sides) as i32` with no upper bound on `count`. Expression like `99999999d6+2147483647` causes overflow (panic in debug, wraparound in release).
- **src/rules/treasure.rs:323,330** — `.iter().sum()` on gem values without bounds on count. 100,000 gems at 1000 GP each overflows `u32`.
- **src/rules/encumbrance.rs:63** — `.iter().sum()` on item weights, same overflow risk.
- **Fix:** Use `checked_add()` / `saturating_add()`. Cap dice count (max 100), cap gem/item counts. Validate dice expressions before evaluation.

## Major Issues

### P1-1: Unbounded Monster Spawn Count (DoS)
- **src/gmapi/interface.rs:291** — `for i in 0..params.count` with no upper bound. Also at `src/command/combat_cmds.rs:24-27` (lower bound checked, no upper).
- **Impact:** `SpawnEncounter { count: 999999999, ... }` causes memory exhaustion / OOM kill.
- **Fix:** Add `MAX_MONSTER_COUNT` constant (e.g., 100). Reject requests exceeding it.

### P1-2: No Authentication on GM API
- **src/gmapi/interface.rs:15-107** — `handle_request()` processes all commands without any authentication or authorization. Any client can spawn/delete encounters, save/load files, award XP, create/delete characters.
- **Impact:** Unauthorized game state manipulation, file system access via path traversal issues above.
- **Fix:** Implement API key or session-based auth. Add role-based command gating (GM vs Player).

### P1-3: Panics in Production Code Paths
- **src/gmapi/protocol.rs:367,382,397,411,424,436,448,499,512,579** — Multiple `panic!("expected ...")` calls in accessor methods that can be reached via deserialized commands.
- **src/rules/module.rs:207,211** — `panic!("Expected coins")` in module validation.
- **src/rules/encounter.rs:67**, **src/rules/npc_party.rs:191** — `.expect()` on JSON parsing of embedded data.
- **Impact:** Denial of Service. Any reachable panic crashes the process.
- **Fix:** Replace `panic!()` with `Result` returns. Use `unreachable!()` only for provably impossible states. Replace `.expect()` with graceful error handling.

### P1-4: Unbounded Deserialization (DoS)
- **src/rules/module.rs:86**, **src/persist/mod.rs:98** — `serde_json::from_str()` on file contents with no size limit. Multi-GB JSON files cause OOM. Deeply nested structures cause stack overflow.
- **Fix:** Check file size via `fs::metadata()` before reading (e.g., `MAX_MODULE_SIZE = 10MB`). Set recursion depth limits.

### P1-5: Unbounded String Inputs (DoS)
- **src/command/gm_cmds.rs:169** — `ruling` command stores `args.join(" ")` with no length limit. Also **src/command/system.rs:76** for `note`.
- **src/gmapi/protocol.rs:16-241** — JSON deserializer accepts unbounded string lengths for all text fields.
- **Impact:** Memory exhaustion via repeated multi-MB notes; save file bloat.
- **Fix:** Limit text input length (e.g., 10KB max). Limit total stored notes count.

### P1-6: Excessive `unwrap()` in Interface Code
- **src/gmapi/interface.rs** — 53 `.unwrap()` calls in the API handler. Many operate on lookup results that could fail with adversarial input.
- **Impact:** Panics on unexpected state, causing DoS.
- **Fix:** Audit all `.unwrap()` calls. Replace with `?` operator or `.unwrap_or_default()` where appropriate.

## Minor Issues

### P2-1: Information Disclosure in Error Messages
- **src/rules/module.rs:87** — `format!("Failed to read module file {}: {}", path.display(), e)` leaks filesystem paths. Also at **src/gmapi/interface.rs:656-657** and **src/gmapi/protocol.rs:300-311**.
- **Fix:** Use generic messages for user-facing errors. Log detailed errors server-side only.

### P2-2: Telemetry Without User Consent
- **src/telemetry.rs:5-48** — Logs raw player input (including failed commands) to `~/.osr_data/telemetry/commands.jsonl`. No opt-in/opt-out mechanism. Logs grow unbounded.
- **Fix:** Add opt-in/opt-out. Implement log rotation. Redact potentially sensitive patterns.

### P2-3: Unsafe Environment Variable Manipulation in Tests
- **src/telemetry.rs:116,128** — `unsafe { std::env::remove_var("HOME") }` and `unsafe { std::env::set_var("HOME", home) }`. Thread-unsafe global state mutation.
- **Fix:** Use `#[serial]` test attribute. Consider `temp-env` crate.

### P2-4: Unbounded Dice Expression Complexity
- **src/command/system.rs:13** — `roll` command accepts arbitrary dice expressions (e.g., `999999999d999999999`). No complexity limits.
- **Fix:** Limit dice count, sides, and modifier ranges in parser.

### P2-5: Float-to-Integer Overflow in XP Calculation
- **src/rules/xp.rs:56-57** — `adjusted.round().max(0.0) as u64` can overflow if `base_xp` is near `u64::MAX`.
- **Fix:** Add saturation: `.min(u64::MAX as f64) as u64`.

### P2-6: Silent Data Loss on Missing Files
- **src/rules/equipment.rs:464-466**, **src/rules/monster.rs:257-259**, **src/rules/treasure.rs:211-212** — When JSON data files fail to load, registries silently run with empty data. Only `eprintln!` warning.
- **Fix:** Return `Result<>` from init functions. Fail startup if core data is missing.

### P2-7: ASCII-Only Case Comparison
- Multiple locations use `eq_ignore_ascii_case` — only handles ASCII, not Unicode. Minor for a game context but inconsistent with international characters.

## Observations

- **No unsafe blocks in production code** — only in test utilities. Excellent.
- **No shell/command injection** — no `std::process::Command` with user input.
- **No SQL/XSS/SSRF** — no database, HTML rendering, or external HTTP calls.
- **Atomic file writes** — save system uses write-temp-then-rename pattern, preventing corruption.
- **No hardcoded secrets** — no API keys or passwords in source.
- **Strong type system** — enums and structs prevent many invalid states.
- **Good use of `saturating_sub`** for underflow prevention (e.g., `src/rules/attack.rs:51`).
- **RNG is `thread_rng()`** — not cryptographically secure, but appropriate for gameplay randomness.
- **Limited dependencies** — only serde, serde_json, rand. Small attack surface.
