# RFC-075 Developer Handoff: Async Compare Identity

## 1. Summary

Implement stable process-local tab IDs and per-tab load generations so async
compare/reload completions cannot write into a different or newer load. RFC-075
is authoritative; this handoff provides the recommended patch sequence and
review boundaries.

## 2. Scope followed

In scope:

- framework-independent identity/token decision types;
- `CompareTab` identity and generation;
- deterministic ID allocation in `Store`;
- centralized completion commit logic;
- migration of open/reload paths;
- deterministic close/reindex and stale-reload tests;
- threat-model and architecture wording.

Out of scope:

- persistent IDs, task scheduler redesign, UI restyling, or unrelated tab work;
- cancellation as a substitute for token validation;
- RFC-077 mergetool target refactoring.

## 3. Files changed

Expected files (confirm during implementation):

- `crates/forskscope-ui-logic/src/compare.rs`
- new `crates/forskscope-ui-logic/src/compare/load_identity.rs`
- `crates/forskscope-ui-logic/src/lib.rs`
- `crates/forskscope-ui/src/state.rs`
- `crates/forskscope-ui/src/state/tab.rs`
- `crates/forskscope-ui/src/state/compare.rs`
- state test files or a new GTK-free state-transition module
- `docs/src/maintainers/threat-model.md`
- `docs/src/maintainers/architecture.md`

Do not add files merely to match this list; keep the smallest cohesive patch.

## 4. Design decisions and assumptions

- `(CompareTabId, LoadGeneration)` is the sole async completion identity.
- `CompareTabId` is runtime-only and distinct from legacy persisted core
  `session::TabId`; restored tabs always receive a fresh runtime ID.
- IDs are never reused during one process; generations never wrap silently.
- Completion locates the tab by ID and then validates generation/state.
- Obsolete successes and failures are discarded without user notification.
- Tests control completion order directly; no sleeps or scheduler luck.
- Ordered `Vec<CompareTab>` remains the rendering collection.

Suggested reviewable patches:

1. Identity types and pure decision tests.
2. Tab/store fields and centralized commit helper with state tests.
3. Open/reload migration and docs.

Stop for owner/architect review if implementing the model requires persistent
identity, a map-based store, or changes to save-target semantics. RFC-076's v2
migration may parse legacy persisted IDs but must never install them as
`CompareTabId` values.

## 5. Tests and gates run

No implementation gates have been run for this handoff because it is a design
package. The developer must observe and record:

```sh
cargo fmt --check
cargo test -p forskscope-ui-logic load_identity
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Also run the stronger advisory command and ensure no new warnings are added:

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

Existing warnings may remain only under the program's documented policy.

## 6. Generated artifacts

None expected. Do not generate a release archive for this workstream alone.
Attach concise command output to the implementation review request.

## 7. Known limitations

- Obsolete `spawn_blocking` work may continue until completion; results are
  rejected. Cancellation is optional follow-up optimization.
- Runtime GTK smoke testing is deferred to RFC-078, but deterministic state
  tests are release-blocking here.
- RFC-077 will build its atomic prepared-result model on these tokens.

## 8. Recommended next step

Review and accept RFC-075. Then implement patch 1 (identity types/tests) and
request a design checkpoint before changing `CompareTab` and `Store`.
