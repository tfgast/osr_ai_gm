# Review Pass 5: GM API Protocol and Handlers

## Scope

- `src/gmapi/protocol.rs`
- `src/gmapi/interface.rs`
- `src/gmapi/combat_handlers.rs`
- `src/gmapi/exploration_handlers.rs`
- `src/gmapi/query_handlers.rs`
- `src/gmapi/mod.rs`

## What I Checked

- Protocol parsing/validation behavior (`parse_request`, `extract_request_id`, command field constraints)
- Error handling and response contract consistency (`GMResponse`)
- Handler thin-adapter behavior vs. direct engine delegation
- Structured response data consistency for query endpoints

## Findings Filed

1. `oag-hkn6a` (P2 bug)
- Title: GM API `parse_request` skips length validation for `AddGold` and `Unequip` parameters
- Impact: oversized strings for these commands bypass the normal 128-char limits applied elsewhere.

2. `oag-p23qa` (P2 bug)
- Title: GM API parser accepts empty request IDs
- Impact: request/response correlation can degrade because empty IDs are accepted and echoed.

3. `oag-trfz5` (P1 bug, pre-flight health check)
- Title: Pre-existing failure: clippy redundant closure in `src/gmapi/query_handlers.rs`
- Impact: `cargo clippy -- -D warnings` fails on current `origin/main` baseline.

## Notes

- Existing issue `oag-2gkmw` tracks the separate error-path ID propagation bug.
- No code changes were made to GM API logic in this pass; this was a review-and-file findings pass.
