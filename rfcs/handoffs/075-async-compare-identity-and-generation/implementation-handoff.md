# RFC-075 Developer Handoff: Async Compare Identity

## 1. Summary

RFC-075 is implemented. Stable process-local tab IDs and per-tab load
generations now prevent async compare/reload completions from writing into a
different or newer load. This handoff records the completed scope, accepted
decisions, observed evidence, and remaining program dependencies.

## 2. Scope followed

Completed:

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

Implemented files:

- `crates/forskscope-ui-logic/src/compare.rs`
- `crates/forskscope-ui-logic/src/compare/load_identity.rs`
- `crates/forskscope-ui-logic/src/compare/load_identity/tests.rs`
- `crates/forskscope-ui-logic/src/lib.rs`
- `crates/forskscope-ui/src/state.rs`
- `crates/forskscope-ui/src/state/tab.rs`
- `crates/forskscope-ui/src/state/compare.rs`
- `crates/forskscope-ui/src/state/compare/tests.rs`
- `crates/forskscope-ui/src/app.rs`
- `docs/src/maintainers/threat-model.md`
- `docs/src/maintainers/architecture.md`
- `rfcs/done/075-async-compare-identity-and-generation.md`
- `rfcs/handoffs/075-async-compare-identity-and-generation/implementation-handoff.md`
- `rfcs/proposed/074-v1-release-stabilization-program.md`
- `rfcs/README.md`
- `ROADMAP.md`

## 4. Design decisions and assumptions

- `(CompareTabId, LoadGeneration)` is the sole async completion identity.
- `CompareTabId` is runtime-only and distinct from legacy persisted core
  `session::TabId`; restored tabs always receive a fresh runtime ID.
- IDs are never reused during one process; generations never wrap silently.
- Completion locates the tab by ID and then validates generation/state.
- Obsolete successes and failures are discarded without user notification.
- Tests control completion order directly; no sleeps or scheduler luck.
- Ordered `Vec<CompareTab>` remains the rendering collection.
- Store owns a root-scoped `Signal<CompareTabIdAllocator>`; the allocator has a
  checked high-water mark and no release/reuse API.
- Generation exhaustion changes the existing tab to `TabState::Error` and
  spawns no reload. Tab-ID exhaustion occurs before a new tab exists, so open
  emits an error notice and appends/spawns nothing.
- Startup mergetool adjustment occurs only when open appended a new tab, so an
  allocation failure cannot redirect an existing tab.
- RFC-076 may parse legacy persisted IDs but must never install them as
  `CompareTabId` values; restored path pairs receive freshly allocated IDs.

## 5. Tests and gates run

Observed across the accepted implementation checkpoints and independent
reviews:

```text
cargo fmt --check
  pass
cargo test -p forskscope-ui-logic load_identity
  13 passed; 0 failed
cargo test -p forskscope-ui state::compare::tests
  6 passed; 0 failed in each of the lib and bin targets
cargo +1.91 test -p forskscope-ui-logic load_identity
  13 passed; 0 failed
cargo +1.91 test -p forskscope-ui state::compare::tests
  6 passed; 0 failed in each of the lib and bin targets
cargo test --workspace
  pass
cargo clippy --workspace -- -D warnings
  pass
git diff --check
  pass
```

The stronger advisory
`cargo clippy --workspace --all-targets -- -D warnings` still reports the nine
pre-existing test-target lints recorded by the stabilization program. No
diagnostic points to RFC-075 implementation files.

Ignored workspace-local review evidence (not committed/public links):

- `.git-exclude/reviewed/028-rfc075-load-identity-types-checkpoint-review.md`
  — Accept with notes; no blocking findings.
- `.git-exclude/reviewed/029-rfc075-store-token-wiring-checkpoint-review.md`
  — Accept with notes; no blocking findings; runtime integrity boundary
  accepted.

Implementation commits:

- `ad2e98e` — pure identity model and tests.
- `be5d28e` — Store/tab wiring, async migration, race tests, and docs.

## 6. Generated artifacts

No release archive or binary artifact was generated for this workstream.
Deterministic tests create only in-memory prepared results. Temporary workspace
test directories used during validation were removed.

## 7. Known limitations

- Obsolete `spawn_blocking` work may continue until completion; results are
  rejected. Cancellation is optional follow-up optimization.
- Direct Store-level `u64` exhaustion and startup-mergetool failure integration
  tests are absent; allocator/value tests cover exhaustion, and the accepted
  production branches fail closed by inspection.
- Runtime GTK smoke testing remains deferred to RFC-078; RFC-075's required
  integrity evidence is the deterministic prepared-result suite.
- The all-target Clippy advisory retains nine pre-existing test lints.
- RFC-077 will build its atomic prepared-result model on these tokens.

## 8. Recommended next step

Treat RFC-075 and audit finding B1 as complete. Begin RFC-076 as the next
single-developer stabilization workstream; keep its persistence adapters from
installing legacy persisted IDs as runtime identities. Do not claim overall v1
Go until RFC-076–078, integrated gates, platform evidence, and the final
architecture decision are complete.
