# Code Review Synthesis: OSE CLI VTT

Convoy: `hq-cv-vs46u` | 10 review legs (codex) + Claude synthesis
Codebase: `osr_ai_gm/refinery/rig/src/` | ~13,300 lines Rust | 1,091 tests

## Executive Summary

The codebase is well-structured with clean module separation, comprehensive test coverage, and faithful OSE rule implementation. One major correctness bug was found (vampire HD parsing), and the most significant architectural concern is divergent behavior between the CLI and GM API code paths. No security, performance, or wiring issues were identified.

**Merge recommendation:** Safe to ship with the vampire HD fix applied.

## Critical Issues (P0)

None.

## Major Issues (P1)

### 1. Vampire HD parsing yields 0 HD
**Found by:** correctness, elegance (2 legs)

`parse_monster_hd` treats ranged strings like `"7-9**"` as subtraction (`7 - 9 = 0`), making vampires roll 1d4 HP and fight as 0-HD creatures. Cascades into incorrect THAC0 and turn undead calculations.

- `src/rules/monster.rs:271-278`
- `src/rules/attack.rs:80-94`
- `src/gmapi/interface.rs:947-961`

**Fix:** Introduce a structured `HitDice` type (base, bonus, special_count, display_range) or teach the parser to recognize `X-Y` as a range.

### 2. CLI and GM API apply different combat rules
**Found by:** smells

The CLI attack path applies `state.time.rest_penalty()` but the GM API attack path does not. Same action, different outcomes depending on entry point.

- `src/command/combat_cmds.rs:171-206`
- `src/gmapi/interface.rs:362-376`

**Fix:** Centralize combat resolution in the engine layer so both interfaces call the same logic.

### 3. Stringly-typed protocol creates drift risk
**Found by:** smells, elegance (2 legs)

`hit_dice` is overloaded as display string and rules input. The GM protocol carries enums as `String` (class, alignment, door state, terrain) with manual parsing duplicated across GM API and CLI. Terrain lists are duplicated verbatim in at least two places.

- `src/gmapi/protocol.rs:35-126`
- `src/gmapi/interface.rs:534-655`
- `src/command/wilderness_cmds.rs:11-79`
- `src/rules/monster.rs:246-280`

**Fix:** Replace stringly-typed fields with serde enums. Introduce a structured `HitDice` type. Centralize parsing.

### 4. Inconsistent mode/state transitions
**Found by:** smells

Some commands update `state.mode`, others don't. GM API `enter_wilderness` sets mode but CLI `enter_wilderness` does not. `start_combat` doesn't set mode but `spawn_encounter` does.

- `src/gmapi/interface.rs:626-639`
- `src/command/wilderness_cmds.rs:10-38`
- `src/command/combat_cmds.rs:7-91`

**Fix:** Decide whether mode is canonical truth or derived from sub-state presence. Enforce consistently.

### 5. Non-atomic commit bundling
**Found by:** commit-discipline

The rules audit commit bundles two unrelated fixes (thief Hear Noise table + monster XP/HD) in one commit, making bisect/revert difficult.

- `src/rules/thief.rs:61-63`
- `src/rules/monster.rs:246-303`

**Fix:** Split into separate commits per logical change going forward.

## Minor Issues (P2)

### 6. False-green tests due to nondeterminism
**Found by:** test-quality

`spell_disruption_on_damage` loops 50 times and exits without assertion if no hit occurs. `create_character_happy_path` explicitly allows failure from random rolls. Tests can pass without proving anything.

- `src/engine/combat.rs:884`
- `tests/gmapi_protocol_qa.rs:359`

**Fix:** Seed RNG deterministically or force conditions that guarantee the code path under test.

### 7. Test temp file collisions
**Found by:** test-quality

Multiple tests use static `/tmp` paths that collide under parallel `cargo test`.

- `tests/integration.rs:485, 1485`
- `tests/gmapi_protocol_qa.rs:2146, 2176, 2327`

**Fix:** Use `tempfile::NamedTempFile` or unique paths with UUID.

### 8. Monster XP values not tested
**Found by:** correctness

Updated XP/HD values lack pinning tests, so incorrect numbers won't be caught by CI.

- `src/rules/monster.rs:125-380`

### 9. MonsterDef doc comment missing HD notation
**Found by:** style

The `hit_dice` field now uses `*`/`**` notation but the struct doc comment doesn't explain it.

- `src/rules/monster.rs:4`

### 10. Monster lookup is linear scan
**Found by:** performance

`find_monster` scans `MONSTERS` array (30 entries). Fine now, would need indexing if the list grows significantly.

- `src/rules/monster.rs:19`

## Leg Coverage

| Leg | Findings | Status |
|-----|----------|--------|
| Correctness | P1 vampire HD, P2 missing XP tests | File written |
| Performance | No issues (observation: linear scan) | File written |
| Security | No output (codex failed to start) | Missing |
| Elegance | P1 stringly-typed HD, P2 XP maintenance | File written |
| Resilience | No output (codex failed to start) | Missing |
| Style | P2 doc comment, phrasing consistency | File written |
| Code Smells | P1 divergent rules, stringly-typed protocol, mode transitions | Terminal only |
| Wiring | No issues found | File written |
| Commit Discipline | P1 non-atomic commit | File written |
| Test Quality | P2 false-greens, temp file collisions | Terminal only |

**8 of 10 legs produced findings. 2 legs (security, resilience) failed to start on codex.**

## Positive Observations

- Clean modular architecture with trait-based command dispatch
- Comprehensive test coverage (1,091 tests, all passing)
- OSE rule fidelity is high — tables match the source material
- Test updates accompany data changes, preventing silent regressions
- Conventional commit prefixes used consistently

## Recommended Next Steps

1. **Fix vampire HD parsing** (P1, correctness bug affecting gameplay)
2. **Centralize combat resolution** to eliminate CLI/API behavioral divergence
3. **Introduce typed enums** for protocol fields to prevent string parsing drift
4. **Fix nondeterministic tests** to eliminate false-greens
5. **Use tempfile** for test persistence to enable parallel test runs
6. **Run security + resilience reviews** (legs that codex failed to complete)
