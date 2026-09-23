# RFC-075 load identity types checkpoint review

**Review date:** 2026-07-15  
**Repository baseline:** `ad2e98e931c8f6e49d88c295ed770996e0b19202` (`ui-logic: add async compare load identity model`)  
**Review scope:** `dev-record/review-requests/024-rfc075-load-identity-types-checkpoint.md`, RFC-075, its implementation handoff, the three-file patch, current affected runtime code, and observed local gates.  
**Review mode:** Independent architecture checkpoint; no source implementation changes.

## 1. Verdict

**Accept with notes.**

The checkpoint faithfully implements RFC-075's framework-independent identity
types and pure completion rule. It is suitable as the foundation for the next
Store/`CompareTab` wiring checkpoint. This is not acceptance of RFC-075 as a
whole and does not resolve release-blocking finding B1: the shipping completion
paths still capture vector indices and guard only on `TabState::Loading`.

## 2. Blocking findings

None for this checkpoint.

## 3. Non-blocking findings

### N1 — Tab-ID exhaustion remains an explicit obligation for Store wiring

`LoadGeneration::next()` uses checked addition and returns
`GenerationExhausted` (`crates/forskscope-ui-logic/src/compare/load_identity.rs:72-77`),
so generation reuse fails closed as required. `CompareTabId`, appropriately for
this first checkpoint, validates only that an already allocated value is
non-zero (`load_identity.rs:39-47`); allocation is not yet modeled.

The next checkpoint must advance the Store's ID source with checked arithmetic,
must never reuse an ID after close, and must represent exhaustion without
wrapping through zero. It should include a deterministic allocator-exhaustion
test and a distinct internal error/disposition (whether by extending
`LoadIdentityError` or by an equally explicit Store error). RFC-075 requires
both ID and generation allocation failures to fail closed
(`rfcs/proposed/075-async-compare-identity-and-generation.md:148-154`).

### N2 — The process-local/persistence boundary is architectural, not enforced by construction alone

The boundary is sufficiently explicit for this checkpoint: `CompareTabId` is a
separate opaque numeric newtype, legacy `forskscope_core::session::TabId` is a
distinct string newtype, and the new types implement no serialization traits.
Module documentation also states that the identities are never restored from
disk (`load_identity.rs:1-8`).

Because `CompareTabId::new(u64)` is public so the UI Store can allocate IDs, the
type system cannot by itself prevent later persistence code from converting a
stored number into a runtime ID. RFC-076 integration must therefore retain the
documented adapter rule: restored tabs receive freshly allocated IDs and legacy
IDs are never installed as `CompareTabId` values. This is a future integration
review point, not a defect in the present API.

## 4. Architecture assessment and review-question answers

1. **Identity invariants:** Yes. `CompareTabId` and `LoadGeneration` are opaque,
   non-zero newtypes; `LoadToken` binds them; vector position is absent from the
   API (`load_identity.rs:35-110`).
2. **Decision ordering:** Yes. `completion_decision` rejects absence or an
   incorrect candidate ID first, then generation mismatch, then non-loading
   state, and accepts only the exact live loading token
   (`load_identity.rs:121-144`). Returning `RejectTabMissing` for an incorrect
   candidate ID is consistent with RFC-075's specified outcome set and avoids
   misdiagnosing an invalid lookup as a generation race.
3. **Generation exhaustion:** Yes. Checked `u64` advancement is adequate for a
   process-local generation and never reuses a token. The error is recoverable
   and tested at `u64::MAX` (`load_identity.rs:257-262`).
4. **Persistence boundary:** Yes for this layer, subject to N2's later adapter
   obligation. There is no `serde` implementation or conversion from the
   persisted core `TabId`.
5. **Proceed to Store/`CompareTab` wiring:** Yes. Proceed narrowly under the
   conditions in the recommended next action below.

The test set covers the RFC's pure-test matrix, including both directions of
generation mismatch and the intended precedence when state is also stale
(`load_identity.rs:162-270`). The crate-root re-export is coherent with the
existing view-model API layout (`crates/forskscope-ui-logic/src/lib.rs:36-43`).

## 5. Missing evidence

Expected and still missing because this is implementation sequence step 1:

- Store-owned, never-reused tab ID allocation and its exhaustion behavior.
- `CompareTab` identity and generation fields.
- A centralized commit helper that resolves by ID and installs both successes
  and failures only for the accepted token.
- Migration of `open_compare` and `reload_tab` away from captured indices. They
  currently write through `tabs.get_mut(index)`/`tabs.get_mut(idx)` plus a
  loading-state check (`crates/forskscope-ui/src/state/compare.rs:16-70,73-139`).
- Deterministic close/reindex, overlapping reload, replaced-slot, current-token
  failure, and obsolete-failure state tests from RFC-075.
- Audit of every file-I/O transition to `TabState::Loading`.
- Threat-model and architecture wording updated to the token guard.
- Completed RFC-075 handoff with final observed workstream evidence.

Accordingly, the RFC-075 acceptance criteria and the v1 stabilization release
gate remain incomplete and No-Go. No runtime/UI claim follows from the pure
unit tests in this checkpoint.

## 6. Observed gates in this review

| Command | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test -p forskscope-ui-logic load_identity` | Pass; 11 passed, 0 failed |
| `cargo +1.91 test -p forskscope-ui-logic load_identity` | Pass on declared MSRV; 11 passed, 0 failed |
| `cargo test --workspace` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `git diff HEAD^ HEAD --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Fail on the nine pre-existing test-target lints enumerated in the request; no diagnostic points to this patch |
| `cargo doc -p forskscope-ui-logic --no-deps` | Completes; one pre-existing broken intra-doc-link warning in `scroll_sync.rs`, none in this patch |

The stronger all-target Clippy command remains advisory under the documented
policy. Its current failure does not block this checkpoint, but the nine-lint
debt should remain visible rather than being described as a clean gate.

## 7. Recommended next action

Proceed to the Store/`CompareTab` wiring checkpoint. In that patch:

1. add a root-owned deterministic ID allocator with checked, never-reused
   advancement and exhaustion coverage;
2. add `id` and `load_generation` to every construction/restore path;
3. capture the generation increment, `Loading` transition, paths/options, and
   token from one coherent mutable-tab operation;
4. centralize completion lookup by `CompareTabId`, call
   `completion_decision`, and mutate state only on `Accept`; and
5. add the RFC's deterministic collection-level race tests before migrating or
   alongside both async completion paths.

Do not mark B1 or RFC-075 complete until index-based async identity is absent,
all load-entry points are audited, the threat-model wording is updated, and the
full RFC acceptance evidence is observed.
