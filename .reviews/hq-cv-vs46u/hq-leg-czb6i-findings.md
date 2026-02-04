# Commit Discipline Review

## Summary
The review target contains a single commit with a clear conventional prefix and descriptive message. However, the commit bundles two distinct fixes (thief Hear Noise table and monster XP/HD corrections) plus related tests, which makes the change set less atomic and harder to bisect or revert selectively.

## Critical Issues
(P0 - Must fix before merge)
- None.

## Major Issues
(P1 - Should fix before merge)
- Multiple unrelated fixes are combined in one commit (thief skill table vs. monster XP/HD data). This makes bisecting or reverting a single fix impossible without touching the other. `src/rules/thief.rs:61-63`, `src/rules/monster.rs:246-303`, `tests/integration.rs:286-287`
  - Impact: If only one of these changes is later found to be incorrect, the whole commit must be reverted or manually split, reducing the usefulness of `git bisect` and complicating rollback.
  - Suggested fix: Split into two commits, e.g., `fix(thief): correct Hear Noise table` and `fix(monster): correct XP/HD values`, with the test adjustments aligned to the change they validate.

## Minor Issues
(P2 - Nice to fix)
- None.

## Observations
(Non-blocking notes and suggestions)
- The commit message uses a conventional prefix (`fix:`) and describes both changes succinctly, which is good for scanability despite the mixed scope.
