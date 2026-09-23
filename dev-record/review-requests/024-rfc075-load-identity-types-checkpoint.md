# Review Request: RFC-075 Load Identity Types Checkpoint

**Date:** 2026-07-15
**Reviewer stance:** focused implementation/architecture review
**Repository state:** inspect the current working tree directly

## Summary

Review the first RFC-075 implementation checkpoint: a pure, UI-independent
runtime identity model for asynchronous compare loads.

The patch adds non-zero process-local compare-tab IDs, monotonic per-tab load
generations with fail-closed exhaustion, complete load tokens, and a pure
completion decision function. It deliberately does not yet change `Store`,
`CompareTab`, Dioxus components, or asynchronous load dispatch/completion.

## Scope Followed

- Added `CompareTabId` as a process-local identity distinct from vector index
  and all persisted workspace/session IDs.
- Added `LoadGeneration`, reserving zero and rejecting overflow rather than
  wrapping to a previously used generation.
- Added `LoadToken` as the immutable `(tab_id, generation)` pair captured by a
  load attempt.
- Added `LoadIdentitySnapshot` and `completion_decision` as a pure validation
  boundary for background completion.
- Required exact tab identity, exact generation, and live loading state before
  accepting a completion.
- Re-exported the model from `forskscope-ui-logic` for later Store/UI wiring.
- Added deterministic unit coverage for accepted and rejected completions,
  reserved values, monotonic advancement, and generation exhaustion.

## Files Changed

- `crates/forskscope-ui-logic/src/compare/load_identity.rs`
- `crates/forskscope-ui-logic/src/compare.rs`
- `crates/forskscope-ui-logic/src/lib.rs`

## Design Decisions And Assumptions

- Runtime IDs and generations are opaque newtypes. Numeric getters exist only
  for diagnostics and deterministic tests.
- Generation 1 is the initial generation; generation 0 is always invalid.
- A completion with the correct token is still rejected when the tab is no
  longer in a loading state.
- Tab identity is checked before generation and state, so an incorrect lookup
  cannot accidentally classify a different tab as a generation-only mismatch.
- Generation mismatch is checked before loading state because stale identity
  is the stronger rejection reason.
- Exhaustion is represented as an error. Later Store wiring must fail closed
  rather than reuse a token.
- The model contains no serialization traits and must never be persisted.

## Review Questions

1. Does the model faithfully encode RFC-075's immutable tab identity and
   per-load generation invariants without depending on vector position?
2. Are the completion decision ordering and rejection outcomes suitable for
   later Store/UI integration and diagnostics?
3. Is fail-closed `u64` generation exhaustion adequate for the runtime model?
4. Is the boundary sufficiently explicit that persisted legacy IDs cannot be
   confused with runtime load identities?
5. May implementation proceed to the Store/CompareTab wiring checkpoint?

## Tests And Gates Run

Observed in this implementation thread:

```text
cargo fmt --check
cargo test -p forskscope-ui-logic load_identity
  11 passed; 0 failed
cargo test --workspace
cargo clippy --workspace -- -D warnings
git diff --check
```

All commands above passed.

The stronger advisory command
`cargo clippy --workspace --all-targets -- -D warnings` did not pass. It reports
nine existing test-target lints outside this patch: two `manual_contains` cases
in `diff_corpus.rs`, one `type_complexity` case in `search_index.rs`, one
`manual_contains` case in `diff_tests.rs`, one `cmp_owned` case in
`dir_index_tests.rs`, and four `assertions_on_constants` cases in
`job_tests.rs`. No diagnostic points to `load_identity.rs`.

## Generated Artifacts

- This ignored review request is the only review artifact.
- No binary, package, generated documentation tree, or runtime evidence was
  produced.

## Known Limitations

- This checkpoint does not yet prevent B1 in production because current
  compare tabs and asynchronous completions are not wired to these identities.
- No UI interaction, tab close/reorder, rapid reopen, or overlapping-load
  integration test is included yet.
- Allocation and ownership of `CompareTabId` remain for the Store integration
  checkpoint.
- RFC-075 and the v1 stabilization milestone remain incomplete and No-Go.

## Recommended Next Step

Architect should issue Accept, Accept with notes, or Needs changes for this
pure identity boundary. After acceptance, implement Store-owned tab ID
allocation, generation advancement, and token-based completion lookup without
using vector indices across an asynchronous boundary.
