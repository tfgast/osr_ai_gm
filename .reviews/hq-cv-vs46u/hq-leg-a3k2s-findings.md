# Elegance Review

## Summary
The changes are primarily data corrections, and the overall structure remains easy to scan. The main design concern is that `hit_dice` is a stringly‑typed field doing double duty for display and mechanics; introducing ranged formats and special‑ability markers in the same string further couples data entry to ad‑hoc parsing logic. This makes the model brittle and harder for new contributors to reason about.

## Critical Issues
(P0 - Must fix before merge)
- None.

## Major Issues
(P1 - Should fix before merge)
- `hit_dice` is overloaded as both a display string and a rules input; it now mixes ranges (`"7-9**"`) and special‑ability markers (`*`, `**`) that must be interpreted elsewhere. The parser only handles a narrow subset of formats and lives far from the data, so adding new HD notations requires silent coordination across files. `src/rules/monster.rs:246-280`, `src/rules/attack.rs:72-95`
  - Impact: The data model is fragile and easy to misuse; future stat corrections or new monsters can subtly break combat/XP logic without any compiler help.
  - Suggested fix: Introduce a structured `HitDice` type (e.g., base, bonus, special_bonus_count, display_range) and compute the display string separately. Centralize parsing/formatting in one module so monster definitions can’t silently drift from runtime expectations.

## Minor Issues
(P2 - Nice to fix)
- XP numbers are hand‑maintained alongside HD + special‑ability markers, which duplicates domain rules without an explicit source of truth. Consider deriving XP from structured HD + special‑ability flags or documenting the exact rule in code to reduce future mismatches. `src/rules/monster.rs:246-305`

## Observations
(Non-blocking notes and suggestions)
- Tests were updated alongside the data changes, which helps keep the table edits from silently regressing. `src/rules/monster.rs:450-468`, `tests/integration.rs:286-290`
