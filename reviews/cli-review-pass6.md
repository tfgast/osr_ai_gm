# CLI Review Pass 6

- Reviewer: `polecat/fury`
- Date: 2026-02-14
- Scope: `src/command/` (all files), `src/main.rs`
- Focus: argument parsing correctness, thin-adapter compliance, error message quality, registry completeness, alias coverage

## Findings

### P2: `parse_args` drops adjacent suffix text after quoted span with spaces

- Area: `src/main.rs:45`, test at `src/main.rs:306`
- Repro:
  1. `printf "note pre\"fix suf\"fix\nnotes\nquit\n" | cargo run -q`
  2. Observed note text: `prefix suffix`
  3. Expected shell-like tokenization: `prefix suffixfix`
- Impact: lossy parsing for mixed quoted/unquoted tokens; user input can be silently altered.
- Tracking bead: `oag-z395a`

### P2: Aliases are mode-blind (`status`, `go`) and dispatch to exploration commands outside exploration mode

- Area: `src/command/mod.rs:65`
- Repro A (`status`):
  1. `printf 'enter_wilderness forest\nstatus\nwilderness_status\nquit\n' | cargo run -q`
  2. Observed: `status` -> `Error: not in exploration mode.`
  3. Expected: mode-appropriate status or no alias.
- Repro B (`go`):
  1. `printf 'enter_wilderness forest\nadd_hex 1 0 forest\ngo 1 0\ntravel 1 0\nquit\n' | cargo run -q`
  2. Observed: `go` -> `Error: not in exploration mode.`
  3. Expected: wilderness movement alias or no alias.
- Impact: common shorthand fails in non-exploration modes despite valid mode-specific commands existing.
- Tracking bead: `oag-ucjln`

## Checks Performed

- Verified command registration coverage by comparing all `impl Command` implementations to `build_registry()` registrations; no missing production command registrations found.
- Reviewed command adapters for argument-length guards and obvious out-of-bounds indexing hazards; no crash-level parsing faults found in command modules.
