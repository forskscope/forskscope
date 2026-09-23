# Review Request: RFC-076 Patch 4 — C1 Follow-up (CLI-mode session protection)

**Date:** 2026-08-03
**Reviewer stance:** verification of one required correction
**Repository baseline:** `15f3227` (`persist: resolve session file
unconditionally at startup (review 041 C1)`)
**Responds to:** review 041 (`dev-record/reviews/041-rfc076-patch4-production-switch-review.md`),
correction C1

## C1 — closed: session resolution now runs unconditionally

Root cause, as review 041 identified it: `restore_session` conflated two
jobs — resolving the session file (which must always happen) and
restoring tabs from it (which must not happen when a CLI startup pair is
given). Only the conditional branch ran, so `session_write_disabled` was
never set in CLI mode, and a future-version or corrupt `session.json`
would be silently overwritten the first time `open_compare`'s tab
triggered the `tabs` `use_effect`.

Fix: split into two functions.

- `resolve_session(store: &mut Store) -> (SessionRuntimeResolution, Option<Notice>)`
  — loads via the repository, durably commits any legacy migration, sets
  `store.session_write_disabled`, and produces any notice. Runs
  unconditionally in `app.rs`'s startup hook, **before** the
  `STARTUP_PAIR` branch.
- `restore_tabs(store: &mut Store, resolution: &SessionRuntimeResolution)`
  — opens each tab whose paths still exist. Called only in the
  no-startup-pair branch, exactly where `restore_session`'s tab-opening
  logic used to live.

`save_session`'s write-disable gate is now a separately testable
`save_session_if_allowed(write_disabled: bool, payload: &PersistedSessionV2, repo: &SessionRepository)`
— the same split pattern used throughout patch 4 (`build_save_payload`/
`persist_settings`/`load_settings` and their session mirrors), so the
protection itself, not just its plumbing, is directly testable without a
Dioxus runtime.

## Test added — the exact CLI-mode regression

`future_version_session_stays_byte_identical_through_a_disabled_save`:
writes a future-version session fixture, calls `load_session` (confirms
`write_disabled == true`), then calls `save_session_if_allowed` with a
tab-pair payload — as `open_compare`'s effect would in CLI mode — and
asserts the file is byte-identical to what was written before. This is
the property review 041 asked for; I did not attempt a full
Dioxus-runtime CLI-mode test, since `Store::new` panics outside a running
runtime (`Signal::new_in_scope` requires one) — confirmed by trying it
directly. The `Store`-independent core this patch already splits out for
testability is exactly what makes the property provable without one.

## Manual verification against the real binary

Reproduced review 041's exact scenario:

1. Backed up the real (already-migrated-to-v2) `~/.config/forskscope/session.json`.
2. Replaced it with a hand-written `schema_version: 99` envelope.
3. Ran `./target/debug/forskscope <left> <right>` (CLI mode, the same
   invocation the README's first code block and the git difftool/mergetool
   configuration both use).
4. The diff view opened normally, with a toast: *"This session file uses
   'session' schema version 99, which this version of ForskScope does not
   understand. The file has not been modified."*
5. After the process exited, `session.json` was verified byte-identical
   to the hand-written future-version file — confirmed via direct file
   comparison, not just the toast text.
6. Restored the real config from backup afterward (verified via its
   `.pre-v2.bak`, which still held the original pre-migration bytes) and
   re-ran once normally to regenerate the correct migrated v2 state,
   leaving `~/.config/forskscope/` exactly as it was before this
   exercise, functionally.

## Files Changed

- `crates/forskscope-ui/src/state/session.rs` — `restore_session` split
  into `resolve_session`/`restore_tabs`; `save_session_if_allowed` added
- `crates/forskscope-ui/src/state/session/tests.rs` — new CLI-mode
  regression test
- `crates/forskscope-ui/src/app.rs` — `resolve_session` called
  unconditionally before the `STARTUP_PAIR` branch; `restore_tabs` called
  only in the no-pair branch; toast priority (settings notice wins if
  both fire) unchanged
- `crates/forskscope-ui/src/state.rs` — re-export list updated
  (`restore_session` → `resolve_session`, `restore_tabs`)
- `ROADMAP.md`, `rfcs/README.md`,
  `rfcs/handoffs/076-versioned-runtime-persistence/implementation-handoff.md`,
  `rfcs/proposed/076-versioned-runtime-persistence.md` — already-staged
  updates from RFC-076's 2026-08-03 amendment, committed alongside since
  they were present in the working tree; not authored by me
- New: `rfcs/handoffs/076-versioned-runtime-persistence/convergence-cleanup-handoff.md`
  (patch 5) — also already staged, not authored by me

## Not Addressed Here (per review 041's own sequencing)

- Review 041 §4.3: `Store::new`'s three-parameter signature. The review's
  own guidance was "fix C1 first... do not restructure the parameters on
  their own" — left as-is.
- Patch 5 (convergence cleanup, per the new handoff) — separate next
  step, not started.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test -p forskscope-ui                                     pass — 21/21 in each of the lib and bin unittest suites (was 20/20; +1 CLI-mode regression test, counted in both since forskscope-ui builds both a lib and a bin target)
cargo test --workspace                                          pass — 1065 total (was 1063; +2, one per unittest binary)
cargo clippy --workspace -D warnings                             pass
cargo xtask version-sync                                         pass — v0.165.1
git diff --check                                                 pass
Manual: real binary, CLI mode, future-version session fixture     pass — file byte-identical, toast shown, restored afterward
```

CI run `30797216154`: Test & Lint green, on `15f3227`.

## Requested Review Focus

1. Is `resolve_session`/`restore_tabs`'s split the right shape, or would
   you want `resolve_session` itself to take a flag for whether to
   restore tabs (keeping one function) rather than two functions called
   from two places in `app.rs`?
2. Confirm the toast-priority behavior (settings notice wins over session
   notice when both fire on the same launch) still reads correctly now
   that `resolve_session` runs earlier in the hook than before.
