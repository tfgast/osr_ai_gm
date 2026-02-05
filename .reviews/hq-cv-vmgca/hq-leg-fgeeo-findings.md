# Commit Discipline Review

## Summary

The repository (87 commits on main) uses conventional commit prefixes consistently and most commits are well-scoped, single-purpose changes with descriptive messages. The project improved markedly after the initial bootstrap phase: early commits are large omnibus bundles, but the more recent history shows disciplined, atomic commits with ticket references. The main issues are (1) several non-atomic omnibus commits in the early-to-mid history that bundle unrelated fixes, and (2) a cluster of data extraction commits that are enormous by line count but acceptable given their nature (generated JSON data files).

## Critical Issues

(None)

## Major Issues

### 1. Non-atomic omnibus fix commits bundle unrelated changes
**P1 - Should fix going forward**

Several commits bundle many unrelated fixes into a single commit, making `git bisect` and selective `git revert` impractical:

- `f464dcc` — "fix: 16 code review bugs — saves, combat, exploration, wilderness, architecture" (20 files, 1536+/1396-). Bundles level-0 saves, duplicate turn counter, move_to teleportation, monster THAC0, HD parsing, morale, door forcing, trap re-fire, lost chances, mountain movement, spiked doors, wandering monsters, combat phase tracking, drow spells, wilderness travel range, AND an architecture refactor extracting main.rs into 6 submodules. The refactor alone should have been a separate commit.

- `1bcd626` — "fix: 13 architecture & Rust quality fixes" (20 files, 232+/201-). Mixes type migration (String→Class enum), runtime help queries, equipment table refactor, racial modifier consolidation, error handling improvements, save versioning, and serialization derives.

- `aa82a6e` — "fix: 10 OSE rules accuracy bugs" (9 files, 306+/100-). Bundles equipment, exploration, wilderness, combat, and evasion fixes that are logically independent.

- `87b0dc4` — "fix: 9 state & data integrity issues" (10 files, 302+/84-). Bundles duplicate ID validation, dead monster targeting, combat distance updates, turn undead state, morale tracking, and log growth capping.

- `a7091e0` — "fix: three code quality fixes" (7 files, 87+/85-). Bundles an args-to-struct refactor, a nested format! fix, and a module rename — three unrelated changes.

- `65c0551` — "fix: thief Hear Noise table + 11 monster XP/HD accuracy bugs". Two completely unrelated rule domains in one commit.

**Impact:** If any individual fix introduced a regression, reverting the commit would undo all the other fixes too. `git bisect` would point to a 20-file commit and not help narrow the cause.

**Recommendation:** Each logical fix should be its own commit. Architecture refactors should never be bundled with bug fixes. The numbered-list format in commit bodies is good documentation, but each numbered item should ideally be its own commit.

### 2. Missing ticket references on 29 of 87 commits
**P1 - Should fix going forward**

58 of 87 commits (67%) include an `oag-*` or `hq-*` ticket reference. The remaining 29 have no traceability to a task or issue. Notable examples:

- `01e8edc` — "feat: add training-gated level advancement system" (524 insertions, 8 files) — a significant feature with no ticket
- `59b1eda` — "feat: add failed command telemetry for playtesting" (174 insertions) — no ticket
- `4aa807c` — "feat: add Winter's Daughter module parser" (700 insertions) — no ticket
- All 6 early bootstrap commits (`8da4529` through `1118a06`) — no tickets
- All data extraction commits (`8df2989`, `d9cf24f`, `e8ec9a4`, `36990e1`, `2e8967f`, `c2c2b98`) — no tickets

**Impact:** Without ticket references, there's no way to trace back to the original requirements or review discussion.

## Minor Issues

### 3. Early bootstrap commits are very large
**P2 - Acceptable for project inception**

The first 6 commits are massive feature slabs:
- `8da4529` — foundation (1007+)
- `5d4d08e` — character creation (2214+)
- `e21f9e2` — combat engine (2172+)
- `7ec2177` — exploration engine (3266+)
- `3e3ce21` — GM API (2287+)
- `1118a06` — game data/XP (3159+)

These are acceptable as initial project scaffolding — there's no meaningful way to bisect a project that doesn't exist yet. Noted for completeness but not actionable.

### 4. Data extraction commits are enormous but justified
**P2 - No action needed**

Several commits are 1000-9000+ lines but consist mostly of extracted JSON data:
- `8df2989` — 9449+ (monster JSON data)
- `e8ec9a4` — 7267+ (magic item JSON data)
- `a272b73` — 2927+ (encounter tables)
- `d9cf24f` — 2849+ (spell data)

These are inherently bulky and cannot be meaningfully split. The commit messages explain what was extracted and why, which is good.

### 5. One commit uses `feat(combat):` scoped prefix
**P2 - Inconsistent but minor**

`cbea1e0` — "feat(combat): add close command to change engagement distance" uses a scoped conventional commit prefix, while all other commits use unscoped prefixes. Not wrong, but inconsistent with the project's established convention.

### 6. `security review:` prefix not conventional
**P2 - Minor inconsistency**

`4957d05` — "security review: findings for hq-leg-pttls" uses `security review:` which isn't a standard conventional commit type. Should be `docs:` or `review:` to match the pattern used by other review commits (`a4b67ad`, `6217aef`).

## Observations

- **Conventional commits are well-adopted.** 84 of 87 commits use recognized prefixes (`feat:`, `fix:`, `refactor:`, `test:`, `chore:`, `docs:`, `review:`). Only `initial commit`, one `security review:` commit, and one `feat(combat):` deviate.

- **Commit messages are descriptive and explain the "why."** The project avoids vague messages like "fix", "update", or "stuff". Even the omnibus commits include detailed body text explaining each change.

- **Recent commits show improving discipline.** The most recent ~30 commits are well-scoped single-purpose changes (e.g., `fix: typo in starvation message`, `fix: correct burial mound room connections from actual map`, `refactor: return TrapResult struct`). The omnibus pattern is concentrated in the early-to-mid history.

- **Could this history be bisected effectively?** Mostly yes for recent commits. The 5-6 omnibus commits in the early/mid history would be bisect dead zones — they'd identify the commit but not the specific fix within it.

- **Would a reviewer understand the progression?** Yes. The commit messages tell a clear story of a project building up engine layers, then adding features, then fixing bugs from review. The progression is logical.

- **Are commits atomic?** Recent commits: yes. Early/mid commits: no — the omnibus fix bundles are the main gap.
