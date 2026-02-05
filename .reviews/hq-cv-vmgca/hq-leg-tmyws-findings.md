# Test Quality Review

## Summary

The test suite comprises ~260+ test functions across two integration test files (`tests/gmapi_protocol_qa.rs` at 2622 lines and `tests/integration.rs` at 1616+ lines) plus unit tests in ~15 source files. Overall coverage is strong with good happy-path and error-path testing for most API commands. However, several tests contain **conditional assertions guarded by random dice rolls** that may silently pass without verifying critical behavior. There are also ~74 instances of weak `.is_some()` assertions that verify field presence without checking actual values, and one test that can silently pass without ever exercising its core verification.

The unit tests in `src/` are generally solid with good boundary testing (encumbrance thresholds, morale ranges, terrain types) but share a common pattern of `.is_err()` assertions without verifying error content.

## Critical Issues
(P0 - Must fix before merge)

No critical issues found.

## Major Issues
(P1 - Should fix before merge)

- **Conditional backstab multiplier assertions are bypassed on miss**
  - `tests/integration.rs:188-190`, `tests/integration.rs:748-750`, `tests/integration.rs:773-775`, `tests/integration.rs:797-799`, `tests/integration.rs:1412-1414`, `tests/gmapi_protocol_qa.rs:1790-1793`
  - All backstab tests guard the multiplier assertion with `if data["hit"].as_bool().unwrap_or(false)`. With THAC0 19 vs AC 6, a miss is likely (~60% chance). On a miss, the assertion `assert_eq!(data["multiplier"], 2)` is **never executed**. A bug in backstab multiplier calculation (e.g., always returning x1) would go undetected whenever the attack misses.
  - Suggested fix: Set monster AC to 9 and use a deterministic attack roll seed, or manually set the monster HP low enough that the test can verify damage * multiplier post-attack. Alternatively, directly test the backstab multiplier calculation at the rules layer rather than through the randomized API.

- **`monster_attack_api_damages_character` can pass without verifying damage**
  - `tests/integration.rs:957-987`
  - The test loops 10 times hoping for a hit. If all 10 attack rolls miss (possible with random dice: Orc THAC0 19 vs Fighter AC 3 needs 16+, ~25% hit rate, so P(10 misses) = 0.75^10 = 5.6%), the test silently passes without ever verifying that damage reduces HP. This is a test that **can't fail** under certain random conditions.
  - Suggested fix: Set the target's AC to 9 to guarantee hits, or test the damage application at a lower level.

- **Integration tests assert success without verifying state mutations**
  - `tests/integration.rs:590-618` (complete_ose_session combat section), `tests/integration.rs:900-948` (multi_round_combat_api)
  - Long integration tests issue sequences of Attack, MonsterAttack, and CheckMorale commands but only check `assert!(resp.success)` without examining the response data payload. A bug that returns `success: true` with incorrect data (wrong damage, wrong roll, wrong morale result) would not be caught.
  - Suggested fix: At minimum, verify key response data fields (hit/miss, damage dealt, remaining HP) for at least one attack per combat round.

- **No test for `count: 0` in SpawnEncounter/SpawnMonster**
  - Tests cover count=1 through count=6 but never test `count: 0`. If the API silently creates 0 monsters and enters combat mode with an empty monster list, downstream commands (CheckMorale, MonsterAttack) could panic or produce nonsensical results.
  - Suggested fix: Add a test verifying that `SpawnEncounter { count: 0, ... }` returns an error.

## Minor Issues
(P2 - Nice to fix)

- **74 weak `.is_some()` assertions across test files**
  - Examples: `tests/gmapi_protocol_qa.rs:301` (`data["round"].as_u64().is_some()`), `tests/gmapi_protocol_qa.rs:634-636` (initiative fields), `tests/gmapi_protocol_qa.rs:1544-1547` (reaction roll fields), `tests/gmapi_protocol_qa.rs:1909-1912` (encumbrance fields)
  - These verify that a field exists and has the right JSON type, but don't verify the actual value. For fields with known expected values (e.g., `round` should be 1 after first `RollInitiative`), these should use `assert_eq!`.

- **Error tests missing error message assertions**
  - `tests/gmapi_protocol_qa.rs:543-544` (`spawn_encounter_invalid_morale_high`): asserts `!resp.success` but doesn't verify error mentions "morale"
  - `tests/gmapi_protocol_qa.rs:1104-1105` (`advance_turn_not_exploring`): asserts `!resp.success` only
  - `tests/gmapi_protocol_qa.rs:1214-1215` (`add_door_no_dungeon`): asserts `!resp.success` only
  - `tests/gmapi_protocol_qa.rs:316-321` (`query_exploration_not_exploring`): asserts `!resp.success` only
  - Without error message assertions, the test passes even if the command fails for the wrong reason.

- **Unit tests use `.is_err()` without checking error content**
  - `src/state/dungeon.rs:393,400,407,448,455,465`: All use `assert!(x.is_err())` without verifying the error message
  - `src/state/wilderness.rs:325,334,341`: Same pattern
  - A function returning the wrong error for the wrong reason would still pass.

- **Duplicated test helpers between files**
  - `make_fighter`, `make_thief`, `make_cleric`, `unique_tmp_path`, and `req` are copy-pasted between `tests/integration.rs` and `tests/gmapi_protocol_qa.rs`. Changes to one are not propagated to the other.

- **Search tests don't verify search results**
  - `tests/gmapi_protocol_qa.rs:1252-1276` (`search_happy_path`, `search_as_elf`, `search_not_exploring`): Happy path tests only check `resp.success` without examining what the search found. A search that always returns empty results would pass.

- **Missing edge case: empty string character name**
  - No test verifies behavior of `CreateCharacter { name: "", ... }`. An empty name could cause display bugs or lookup failures.

- **Missing edge case: dead character attempting level up**
  - No test verifies that `LevelUp` rejects a dead character (hp <= 0).

- **Missing edge case: negative coordinates in Travel/AddHex**
  - While `session_b_wilderness_travel` uses `(1, -1)`, there's no explicit test for large negative coordinates or overflow.

## Observations
(Non-blocking notes and suggestions)

- The `assert_response_format` helper in `gmapi_protocol_qa.rs:87-98` is a well-designed contract verification that checks the success/error field invariant. This is a good pattern.
- Morale boundary testing (`spawn_encounter_morale_boundary_valid` at gmapi_protocol_qa.rs:547-578) is excellent, testing both min (2) and max (12) valid values plus both invalid sides.
- The mode transition tests (`mode_transitions`, `mode_transition_idle_exploration_combat_exploration`, `mode_transition_idle_wilderness_combat_wilderness`) are thorough and verify state preservation across transitions.
- Save/load roundtrip testing in `save_load_complex_state` (integration.rs:1841-2019) is exemplary, with field-by-field verification including light source remaining turns.
- The encumbrance unit tests in `src/rules/encumbrance.rs` are a model for the codebase: they test exact boundary values at every threshold.
- No flaky test indicators (sleep/delay, time-dependent assertions) were found.
- The thief skill alias testing (`thief_skill_check_aliases` at gmapi_protocol_qa.rs:1732-1758) is thorough.
