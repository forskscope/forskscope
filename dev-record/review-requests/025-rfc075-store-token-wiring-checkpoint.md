# Review Request: RFC-075 Store And Token Wiring Checkpoint

**Date:** 2026-07-15
**Reviewer stance:** focused implementation/architecture review
**Repository baseline:** `ad2e98e931c8f6e49d88c295ed770996e0b19202`
**Repository state:** inspect the current working tree directly

## Summary

Review the second RFC-075 implementation checkpoint. The patch wires stable
runtime identities through `Store`, `CompareTab`, `open_compare`, and
`reload_tab`; centralizes prepared-result installation behind exact token
validation; and adds deterministic collection-level race tests.

No asynchronous compare completion now uses a captured vector index as its
identity. The caller's index is used synchronously only to select a live tab
before reload starts. Each spawned file-I/O task carries an immutable
`(CompareTabId, LoadGeneration)` token and resolves the current vector by ID on
completion.

## Scope Followed

- Added a root-owned `Signal<CompareTabIdAllocator>` to `Store`.
- Added checked, never-reusing tab-ID allocation with a distinct
  `TabIdExhausted` error and deterministic exhaustion coverage.
- Added process-local `id` and `load_generation` fields to `CompareTab`.
- Assigned generation 1 to every newly opened or restored compare tab.
- Made reload generation advancement, `Loading` transition, path/options
  capture, and token capture one coherent mutable-tab operation.
- Centralized both successful and failed completion installation in
  `commit_load_result`.
- Resolved completions by `CompareTabId`, then applied the accepted pure
  generation/state decision before any mutation.
- Migrated both `open_compare` and `reload_tab` away from captured-index
  completion writes.
- Added deterministic close/reindex, overlapping-reload, replaced-slot,
  current-failure, and obsolete-failure regression tests.
- Updated architecture and threat-model wording to describe the token guard.
- Made startup mergetool path adjustment conditional on a tab actually being
  appended, so allocator exhaustion cannot redirect it to an existing tab.
- Moved identity tests into a separate test module per repository conventions.

## Files Changed

- `crates/forskscope-ui-logic/src/compare/load_identity.rs`
- `crates/forskscope-ui-logic/src/compare/load_identity/tests.rs`
- `crates/forskscope-ui-logic/src/lib.rs`
- `crates/forskscope-ui/src/state.rs`
- `crates/forskscope-ui/src/state/tab.rs`
- `crates/forskscope-ui/src/state/compare.rs`
- `crates/forskscope-ui/src/state/compare/tests.rs`
- `crates/forskscope-ui/src/app.rs`
- `docs/src/maintainers/architecture.md`
- `docs/src/maintainers/threat-model.md`

## Design Decisions And Assumptions

- The Store owns a typed allocator signal rather than a raw `Signal<u64>`;
  this preserves deterministic root ownership while encapsulating checked
  advancement and the no-reuse invariant.
- Allocation stores a high-water mark. Closing a tab has no release operation,
  so its ID cannot be allocated again during the process lifetime.
- A new-tab allocation failure shows the existing error notice and appends no
  tab. A reload-generation failure changes that live tab to `TabState::Error`
  and spawns no work. Neither path wraps or reuses an identity.
- `commit_load_result` accepts a boxed prepared success payload or an error.
  Both dispositions pass through the same identity/generation/state guard.
- Rejected lifecycle completions remain silent. The returned
  `CompletionDecision` supports deterministic tests and future path-free debug
  logging.
- Synchronous UI mutations such as swap and save continue to use the current
  rendered index. RFC-075 forbids index identity across asynchronous file-load
  boundaries, not synchronous collection access.
- Session restore continues to call `open_compare`, so every restored path pair
  receives a fresh allocator-issued runtime ID and initial generation; no
  runtime token is serialized.

## Review Questions

1. Does the root-owned typed allocator satisfy the Store ownership,
   never-reuse, and fail-closed exhaustion requirements?
2. Is reload start sufficiently atomic with respect to generation, state,
   paths/options, and token capture?
3. Does `commit_load_result` correctly centralize both success and failure
   installation without any captured-index identity remaining?
4. Do the deterministic tests cover RFC-075's collection-level race matrix?
5. Is the distinction between safe synchronous index selection and forbidden
   asynchronous index identity explicit and correctly implemented?
6. Does the startup mergetool guard correctly preserve fail-closed behavior if
   opening a new tab cannot allocate an ID?
7. May the next patch finalize the RFC-075 handoff/status evidence, or is
   additional runtime implementation required first?

## Tests And Gates Run

Final observed results:

```text
cargo fmt --check
  pass
cargo test -p forskscope-ui-logic load_identity
  13 passed; 0 failed
cargo test -p forskscope-ui state::compare::tests
  6 passed; 0 failed in each of the lib and bin test targets
cargo +1.91 test -p forskscope-ui-logic load_identity
  13 passed; 0 failed
cargo +1.91 test -p forskscope-ui state::compare::tests
  6 passed; 0 failed in each of the lib and bin test targets
cargo test --workspace
  pass
cargo clippy --workspace -- -D warnings
  pass
git diff --check
  pass
```

The final workspace test was run with access to an external temporary
directory because the restricted sandbox exposed `/tmp` as read-only. Two
intermediate sandbox-only reruns were invalid environment attempts: the first
could not create test files, and the second used a repo-local temp root that
violated two VCS tests' required “outside any repository” premise. The final
ordinary `cargo test --workspace` run with valid temp access passed.

The stronger advisory command
`cargo clippy --workspace --all-targets -- -D warnings` still reports exactly
the nine known test-target lints recorded in reviews 024/028. No diagnostic
points to this patch.

## Generated Artifacts

- This ignored review request is the only retained review artifact.
- Temporary test directories were removed.
- No binary, package, generated documentation tree, or release archive was
  produced.

## Known Limitations

- Obsolete blocking I/O is not cancelled; its prepared result is rejected.
- No timing-based GUI race test was added. The integrity transitions are tested
  deterministically with prepared results, as RFC-075 requires.
- Rejection decisions are not yet debug-logged; logging is optional and must
  not include paths or file content.
- The RFC status and developer handoff are intentionally not finalized before
  this implementation checkpoint is accepted.
- B1 and RFC-075 remain open pending architecture acceptance and final handoff
  evidence.

## Recommended Next Step

Architect should issue Accept, Accept with notes, or Needs changes for this
Store/token wiring checkpoint. If accepted, finalize RFC-075 status and its
developer handoff with the observed evidence; do not begin RFC-076 in the same
patch.
