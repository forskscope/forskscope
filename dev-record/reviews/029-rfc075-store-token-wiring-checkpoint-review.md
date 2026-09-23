# RFC-075 Store and token wiring checkpoint review

**Review date:** 2026-07-15  
**Repository baseline:** working tree over `ad2e98e931c8f6e49d88c295ed770996e0b19202` (`ui-logic: add async compare load identity model`)  
**Review scope:** `dev-record/review-requests/025-rfc075-store-token-wiring-checkpoint.md`, RFC-075 and its handoff, all tracked and untracked working-tree changes, affected compare/session/startup paths, deterministic tests, documentation, and observed gates.  
**Review mode:** Independent architecture checkpoint; no source implementation changes.

## 1. Verdict

**Accept with notes.**

The working-tree implementation satisfies RFC-075's runtime integrity boundary.
Compare open and reload tasks now capture immutable `(CompareTabId,
LoadGeneration)` tokens, completion resolves by ID rather than vector position,
and both prepared successes and failures are rejected unless the exact token is
still loading. The next patch may finalize RFC-075's status and developer
handoff; no additional runtime implementation is required first.

This acceptance closes the implementation substance of stabilization finding
B1, subject to the final status/handoff evidence patch. It does not alter the
project-level No-Go caused by the remaining stabilization findings and platform
acceptance work.

## 2. Blocking findings

None.

## 3. Non-blocking findings

### N1 — Final RFC wording should distinguish new-tab and reload exhaustion

RFC-075 currently states that “ID/generation allocation failure becomes
`TabState::Error`” (`rfcs/proposed/075-async-compare-identity-and-generation.md:148-154`).
The implementation reasonably distinguishes the two cases:

- generation exhaustion belongs to an existing tab, so reload changes that tab
  to `TabState::Error` and spawns no task
  (`crates/forskscope-ui/src/state/compare.rs:94-115`);
- tab-ID exhaustion occurs before a new tab exists, so open emits the existing
  error notice and appends/spawns nothing (`compare.rs:139-146`).

Both paths fail closed and neither can reuse an identity. The final RFC/handoff
patch should record this distinction so the durable design matches the accepted
behavior rather than implying that a nonexistent new tab receives an error
state.

### N2 — Failure-edge integration behavior is established mainly by composition

The allocator tests directly prove monotonic allocation and fail-closed
exhaustion (`crates/forskscope-ui-logic/src/compare/load_identity/tests.rs:125-143`),
and code inspection establishes that Store/open/startup compose those results
correctly. There is no direct automated test that seeds a Store at allocator
exhaustion and verifies both “no tab appended” and “mergetool target not
redirected.” Similarly, generation exhaustion is tested on the value type but
not through `reload_tab`.

These are effectively unreachable `u64` exhaustion edges in ordinary runtime,
and the production branches are short and fail closed, so this is not a blocker.
If a later refactor adds injectable Store construction or makes `open_compare`
return a typed outcome, add integration tests for these dispositions rather
than relying on tab-count observation.

## 4. Architecture assessment and review-question answers

1. **Allocator ownership and exhaustion:** Yes. `Store` privately owns a
   root-scoped `Signal<CompareTabIdAllocator>` (`crates/forskscope-ui/src/state.rs:106-144`).
   The allocator keeps a high-water mark, starts at one, has no release API,
   uses checked addition, and leaves its state unchanged on exhaustion
   (`crates/forskscope-ui-logic/src/compare/load_identity.rs:58-92`).
2. **Reload-start coherence:** Yes. The live tab is selected synchronously by
   caller index, then generation advancement, `Loading`, paths, options, and
   token are captured while one mutable tabs borrow is held, before spawning
   (`crates/forskscope-ui/src/state/compare.rs:94-128`). There is no async yield
   or second tab lookup within that transition.
3. **Centralized completion:** Yes. `commit_load_result` locates by
   `CompareTabId`, projects current generation/state, calls the pure decision,
   and mutates only after `Accept`; ready and error results share this boundary
   (`compare.rs:19-92`). Both `open_compare` and `reload_tab` call this helper,
   and neither task captures a vector index (`compare.rs:118-137,180-199`).
4. **Race matrix:** Yes. The six deterministic tests cover lower-tab close and
   reindex, stale versus current reload generations, a closed ID with a reused
   vector slot, current-token failure isolation, obsolete failure after a newer
   ready result, and accepted ready-state installation
   (`crates/forskscope-ui/src/state/compare/tests.rs:46-149`). They use prepared
   values and no timing/sleeps.
5. **Synchronous indices:** Yes. The remaining compare-tab indices select a
   tab during the current UI call, set the active rendered tab, or perform the
   immediate startup adjustment. No index crosses the `spawn_blocking`/async
   boundary. This matches RFC-075's distinction.
6. **Mergetool allocation guard:** Yes by inspection. Startup records the prior
   count and adjusts only when open synchronously appended a tab; on allocator
   failure the count does not increase, so no existing tab is redirected
   (`crates/forskscope-ui/src/app.rs:30-46`). The index is computed only after
   the strict count increase and does not cross an async boundary. N2 records
   the absent direct edge test.
7. **Next step:** Yes. Finalize RFC-075 status and handoff evidence in a narrow
   patch. Do not combine RFC-076 implementation with that closure patch.

Session persistence serializes paths only, and restore routes every pair through
`open_compare`, so restored tabs receive allocator-issued identities and initial
generation without persisting tokens (`crates/forskscope-ui/src/state/session.rs:21-45`).
The repository-wide search found no other compare `load_path` entry point and no
other production assignment to `TabState::Loading`.

The architecture and threat-model updates accurately describe the implemented
token guard (`docs/src/maintainers/architecture.md:48-78` and
`docs/src/maintainers/threat-model.md:37-49`). Obsolete blocking work continuing
without cancellation remains within the RFC's explicit non-goals.

## 5. Missing evidence

- The RFC status and developer handoff have not yet been finalized; this is the
  intended next patch rather than an omission from this checkpoint.
- Direct Store-level exhaustion, reload-exhaustion, and startup-mergetool failure
  tests are absent as described in N2.
- There is no timing-based GUI race test. RFC-075 explicitly prefers the
  deterministic prepared-result tests now present, so GUI timing evidence is
  not required for this workstream.
- Rejection decisions are not debug-logged. Logging is optional under RFC-075
  and its absence does not weaken the mutation guard.
- Runtime/platform acceptance remains deferred to RFC-078 and is outside this
  checkpoint.

## 6. Observed gates in this review

| Command | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test -p forskscope-ui-logic load_identity` | Pass; 13 passed, 0 failed |
| `cargo test -p forskscope-ui state::compare::tests` | Pass; 6 passed in each of the lib and bin targets |
| `cargo +1.91 test -p forskscope-ui-logic load_identity` | Pass; 13 passed, 0 failed |
| `cargo +1.91 test -p forskscope-ui state::compare::tests` | Pass; 6 passed in each of the lib and bin targets |
| `cargo test --workspace` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `git diff --check ad2e98e931c8f6e49d88c295ed770996e0b19202` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Fail on exactly the nine previously recorded test-target lints; no diagnostic points to this patch |

The all-target Clippy command remains a stronger advisory check, not the
documented mandatory gate. Its known debt should stay visible in the final
handoff but does not block RFC-075.

## 7. Recommended next action

Create a documentation-only RFC-075 closure patch that:

1. incorporates N1's precise allocation-failure policy;
2. changes RFC-075 from Proposed to the repository's completed status;
3. completes the developer handoff with the final changed-file list, decisions,
   observed commands/results, limitations, and links to both checkpoint reviews;
4. records B1 as resolved by stable tab ID plus generation validation and the
   deterministic race suite; and
5. leaves RFC-076 implementation for a subsequent, separately reviewed patch.

Do not claim the overall v1 stabilization program or public release is Go when
closing RFC-075; the other accepted release blockers and RFC-078 platform
evidence remain outstanding.
