# Review Request: RFC-076 M2-B Patch 2 — Repositories and Safe-Write Tests

**Date:** 2026-08-02
**Reviewer stance:** design + implementation review
**Repository baseline:** `b710e7f` (`persist: add explicit-path repositories
and safe migration writes (RFC-076 patch 2)`)
**Governing documents:** RFC-076; `rfcs/handoffs/076-versioned-runtime-persistence/implementation-handoff.md`
§4.4 patch 2 ("Repository and safe-write tests"); audit finding B2

Patch 2 of 5. **No production `forskscope-ui` call site is changed.**
`app_json_settings::ConfigManager` remains the only thing the running
application actually calls; the switch is patch 4. This patch adds the
repositories and proves them with tests, nothing more.

## Implementation Summary

`SettingsRepository` / `SessionRepository`
(`crates/forskscope-core/src/persist/v2/{settings,session}/repository.rs`),
each taking an explicit `PathBuf` — never resolving a platform config
directory itself, per RFC-076's ownership boundary — with three methods:

- `load() -> PersistenceLoad<T>` — real file I/O wired to patch 1's routing.
- `save(&T) -> Result<(), PersistenceIoError>` — ordinary atomic envelope
  write.
- `commit_migration(&T, original_bytes: &[u8]) -> Result<PersistenceSaveOutcome, PersistenceIoError>`
  — RFC-076's "first durable rewrite": preserves the pre-migration original
  as a non-overwriting `<name>.pre-v2.bak`, then atomically replaces it with
  the v2 envelope.

Shared mechanics live in `persist/v2/repository.rs` (envelope construction,
atomic write, the backup dance) as plain functions operating on strings/
bytes, not generic over `T` — with only two concrete repositories, generics
would trade a little duplication for indirection neither call site needs.

## Addressed Items

- RFC-076 §"Repository API" and the "On first durable rewrite" six-step
  sequence — implemented for both settings and session.
- Handoff §"Files changed" repository/safe-write scope — done.
- B2 — **still not closed.** This patch makes the write path exist and
  proves it safe in isolation; it closes only when patch 4 makes the
  running application call it.

## Files Changed

New:

- `crates/forskscope-core/src/persist/v2/repository.rs` — shared
  `PersistenceSaveOutcome`, `PersistenceIoError`, envelope/atomic-write/
  backup helpers (77 ELOC)
- `crates/forskscope-core/src/persist/v2/settings/repository.rs` —
  `SettingsRepository` (48 ELOC)
- `crates/forskscope-core/src/persist/v2/session/repository.rs` —
  `SessionRepository` (46 ELOC)
- `crates/forskscope-core/src/tests/persist_v2_repository_tests.rs` — 9
  safe-write tests

Modified:

- `crates/forskscope-core/src/save.rs` — extracted `pub(crate) fn
  atomic_replace(target, bytes)` (the temp-write-then-rename step) out of
  `save_text`; `save_text` now calls it. Behavior-preserving: all 11
  pre-existing `save_tests` pass unmodified.
- `crates/forskscope-core/src/persist/v2.rs` — new `PersistenceError::Io`
  variant; `mod repository;`
- `crates/forskscope-core/src/persist/v2/settings.rs` — `impl Default for
  PersistedSettingsV2` (matches shipping `AppSettings::default()` for
  UI-owned fields, core defaults for core-owned fields); `mod repository;`
  + re-export
- `crates/forskscope-core/src/persist/v2/session.rs` — `#[derive(Default)]`
  for `PersistedSessionV2` (empty tabs/no active tab/no explorer roots is
  exactly the "no session yet" state); `mod repository;` + re-export
- `crates/forskscope-core/src/tests.rs` — declared the new test module
- `crates/forskscope-core/src/tests/persist_v2_settings_tests.rs` — 3
  `bool_assert_comparison` fixes (see "Drive-by cleanup" below)

Not staged: `xtask/Cargo.lock` (pre-existing untracked convention).

## Important Implementation Decisions

### `atomic_replace` extraction touches a security-critical path (RFC-007/S-005) — disclosing explicitly

RFC-076 requires reusing or generalizing "core safe-file primitives rather
than creating an unrelated unsafe writer." The only existing primitive that
matches is inside `save_text`, which is the tested, production, S-005-critical
document-save path. I extracted *only* the temp-write-then-rename step (not
the fingerprint check, not the `.bak` backup logic, not `SaveRequest`/
`SaveOutcome`) into a new `pub(crate) fn atomic_replace`, and verified
behavior preservation by running all 11 existing `save_tests` unmodified
before and after — they pass identically. I chose to actually share the
code (not just duplicate the pattern) because the alternative — hand-rolling
a second temp-then-rename implementation for repositories — is exactly the
"unrelated unsafe writer" RFC-076 says not to create, and a real shared
function is more honestly "reused" than two copies of the same seven lines.
Flagging this explicitly because it is the one change in this patch that
touches code outside `persist/v2/`.

### Two write methods, not one, because migration and ongoing saves have different backup semantics

`save()` is a plain atomic write with no backup — appropriate for "the user
changed a setting," which happens often and has no legacy source to
protect. `commit_migration()` additionally preserves the original bytes as
`<name>.pre-v2.bak`, **created once and never overwritten by a retry** — this
is the semantic RFC-076 states explicitly ("without overwriting an existing
backup") and the one place I added a test specifically to prove a negative:
`settings_commit_migration_does_not_overwrite_existing_backup` calls
`commit_migration` twice with *different* "original" bytes the second time,
and asserts the backup on disk is still the *first* call's bytes. Conflating
the two methods (e.g., always backing up) would make routine settings saves
silently accumulate a stale `.pre-v2.bak` that no longer describes anything
real; conflating them the other way (never backing up on migration) would
lose the downgrade path RFC-076 requires.

### `created_unix` is preserved best-effort across ordinary saves

Not explicitly required by RFC-076, but a plain "always stamp now" `save()`
would silently reset a file's original creation time on every settings
change, which seemed like an avoidable regression relative to the existing
hand-written envelope's behavior. `build_envelope_json` reads whatever is
currently at the target path and reuses its `created_unix` if present and
parseable; any failure (missing file, malformed JSON) falls back to now,
silently — this is a cosmetic timestamp, not a correctness concern, and the
real validation path is `load()`'s routing, not this best-effort peek.

### `PersistenceError::Io`: a new variant, not folded into `Corrupt`'s existing meaning

An unreadable-but-present file (permission denied, etc.) is not `Missing`
(the file exists) and calling it plain `Corrupt` would blur "the content is
bad" with "the content could not be examined at all." Added
`PersistenceError::Io(String)` as a fourth-ish case under the `Corrupt`
*load* outcome (RFC-076's `PersistenceLoad<T>` taxonomy has no separate
top-level variant for this — an I/O failure still routes through `Corrupt`,
since the caller's obligation is the same: preserve, report, do not treat as
absent), distinguished at the `PersistenceError` level so the message a user
sees names the real problem.

### Not implemented in this patch, on purpose

- No caller decides *when* to invoke `commit_migration` versus just calling
  `load()` and leaving a legacy file un-migrated on disk. That decision (and
  the recovery-state/write-disable machinery around `FutureVersion`/
  `Corrupt`) is patch 3's "runtime adapters," per the handoff's sequence.
- No platform config-directory resolution. Both repositories take a bare
  `PathBuf`; `dirs_next::config_dir()` stays a UI/infrastructure concern.

## Drive-by Cleanup (disclosed, not hidden)

Running `cargo clippy -p forskscope-core --tests -- -D warnings` — beyond
the project's mandatory gate, which does not pass `--tests` — surfaced 14
lint findings: 8 pre-existing (in `diff_corpus.rs`, `diff_tests.rs`,
`dir_index_tests.rs`, `job_tests.rs` — none touched by this or any prior
patch) and 6 in code this patch or patch 1 introduced (3
`field_reassign_with_default` in the new repository tests,
3 `bool_assert_comparison` in patch 1's `persist_v2_settings_tests.rs`,
which I had not run `--tests` clippy against before). Fixed only the 6 I
introduced; left the 8 pre-existing ones untouched — they are F6's concern
(filed against M4), not this patch's.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test -p forskscope-core save_tests                       pass — 11/11, unmodified by the atomic_replace extraction
cargo test -p forskscope-core persist_v2                       pass — 35/35 (was 26 at end of patch 1's C1/C2 follow-up; +9)
cargo clippy -p forskscope-core -p forskscope-ui-logic -D warnings   pass
cargo clippy -p forskscope-core --tests -D warnings             6 new findings fixed; 8 pre-existing (F6) untouched
cargo test -p forskscope-core -p forskscope-ui-logic            pass — 978/978 (was 969; +9)
cargo test --workspace                                          pass — unchanged forskscope-ui counts
cargo clippy --workspace -D warnings                             pass
cargo xtask version-sync                                        pass — v0.165.1
git diff --check                                                pass
```

CI run `30752989766`: Test & Lint green in 5m20s, all 17 steps pass.

## Known Limitations

- Still no production code path uses any of this. B2 remains open.
- No runtime adapter yet decides when to call `commit_migration` versus
  leaving a file un-migrated — that is patch 3.
- F26 (schema-enum scalar variants unpinned by the golden fixtures) is
  unaffected by this patch and remains scheduled before patch 4.
- `commit_migration`'s crash-safety was reasoned about (see the design
  decisions above) but not tested by actually killing the process
  mid-write; the "no stray temp file after an ordinary save" test is the
  closest proxy available without fault injection.
- B3, B4 remain open. v1/public release stays **No-Go**.

## Requested Review Focus

1. Is extracting and sharing `atomic_replace` from `save.rs` the right call,
   or would a from-scratch, repository-local implementation have been the
   safer boundary despite the duplication?
2. Does the two-method split (`save` vs `commit_migration`) match RFC-076's
   intent, or should ordinary saves also protect against something I'm not
   accounting for?
3. Is routing I/O-read failures through `Corrupt` (via the new
   `PersistenceError::Io` variant) the right fit for RFC-076's taxonomy, or
   does it deserve to be a first-class `PersistenceLoad` variant of its own?
4. Anything about the crash-safety reasoning for `commit_migration`'s step
   order that warrants a more direct test than what's here?
