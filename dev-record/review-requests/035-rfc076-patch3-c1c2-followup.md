# Review Request: RFC-076 Patch 3 — C1/C2 Follow-up

**Date:** 2026-08-03
**Reviewer stance:** verification of two required corrections
**Repository baseline:** `6956b79` (`persist: distinguish deferred vs
failed migration commits (review 038 C1/C2)`)
**Governing documents:** review 038 (`dev-record/reviews/038-rfc076-patch3-runtime-adapters-review.md`)

Addresses both required corrections from review 038. No new scope beyond
what the review specified.

## C1 — the failure cause is now carried, and the persistent case is surfaced

`MigrationCommitOutcome::Committed { .. } | committed: bool` is replaced by
a three-way outcome that distinguishes *why* a commit didn't land:

```rust
pub enum MigrationCommitOutcome {
    Committed { backup_path: Option<PathBuf> },
    DeferredByConflict,      // review 037 N1's race — benign, self-healing
    Failed { detail: String }, // permission denied, read-only dir, full disk — persistent
}
```

`commit_migrated` now matches on `PersistenceCommitError` explicitly:
`Conflict` → `DeferredByConflict`, `Io(detail)` → `Failed { detail }`. The
defensive `None` (bytes absent, which `load_with_raw` should never produce
alongside a `Migrated*` load) also maps to `Failed`, not silence — an
unexpected case should surface, not vanish.

`SettingsRecoveryView`/`SessionRecoveryView` now have three arms instead of
one `committed.then(...)`: `Committed` still produces the one-time
migration notice; `DeferredByConflict` stays silent (matches review 038
§4.2 — the race itself is fine to stay quiet about); `Failed` produces a
blocking dialog ("Settings/Session could not be upgraded ... Changes will
not be saved until this is resolved.") with `Exit` +
`ContinueWithTemporaryDefaults` actions, so a persistent failure is
visible rather than shaped identically to success.

Doc comments on `SettingsRuntimeOutcome`/`MigrationCommitOutcome` are
rewritten to state the real distinction instead of "only if... a race".

## C2 — a refused/failed commit now disables writes

`commit_migrated`'s `write_disabled` was hardcoded `false`; it's now
`!matches!(commit, MigrationCommitOutcome::Committed { .. })` — `true` for
both `DeferredByConflict` and `Failed`. This is the "smallest change"
option review 038 named as acceptable: we could not establish that
overwriting the file is safe this run, so we don't write to it this run.
Closes the compound path the review traced: N1's guard refusing a commit
no longer leaves the file writable for the next settings change, which
would have overwritten whatever caused the conflict with no verification
and no backup.

## Files Changed

- `crates/forskscope-core/src/persist/v2/settings/runtime.rs`,
  `.../session/runtime.rs` — `MigrationCommitOutcome`, cause-preserving
  `commit_migrated`, `write_disabled` now derived from the commit outcome
- `crates/forskscope-core/src/tests/persist_v2_runtime_tests.rs` —
  `settings_resolve_surfaces_a_failed_commit_and_disables_writes` (renamed
  from the old "uncommitted" test, which was actually already exercising
  the `Io`/`Failed` path via the directory-obstruction technique, not the
  `Conflict` path — the name and assertions now say so) and its new
  session mirror; both assert `write_disabled` per C2
- `crates/forskscope-ui-logic/src/settings/persistence_recovery.rs`,
  `.../session/persistence_recovery.rs` — `Failed` dialog arm, tests for
  `DeferredByConflict` silence and `Failed` visibility

## A note on what wasn't testable

Review 038 didn't ask for a test of `resolve_and_commit` hitting the
`Conflict` arm specifically (as opposed to `Failed`), and I didn't add one:
`resolve_and_commit` calls `load_with_raw()` then `commit_migration()`
back-to-back within one synchronous function call, so nothing can modify
the file in between from a single-threaded test — that window is only
reachable via real concurrency, which the repository-level test
(`settings_commit_migration_rejects_stale_bytes_after_external_change`,
patch 3) already covers directly against `commit_migration` by writing a
stale value between two calls. The `resolve_and_commit`-level tests instead
cover `Committed` and `Failed`, plus the `DeferredByConflict` mapping is
covered at the view-model level by constructing that variant directly. Flag
if a different level of coverage was expected here.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test -p forskscope-core -p forskscope-ui-logic            pass — 1013 (was 1010; +3: 1 settings + 1 session runtime test net-new, +2 ui-logic Failed-dialog tests, -0 net from the DeferredByConflict rename)
cargo clippy -p forskscope-core -p forskscope-ui-logic -D warnings   pass
cargo clippy -p forskscope-core --tests -D warnings              8 pre-existing findings (F6) untouched; 0 new
cargo clippy -p forskscope-ui-logic --tests -D warnings          1 pre-existing finding untouched; 0 new
cargo test --workspace                                           pass
cargo clippy --workspace -D warnings                              pass
cargo xtask version-sync                                          pass — v0.165.1
git diff --check                                                  pass
```

CI run `30771769968`: Test & Lint green, on `6956b79`.

## Requested Review Focus

1. Does the `Failed` dialog's action set (`Exit` +
   `ContinueWithTemporaryDefaults`, no `ResetAndBackupOriginal`) make sense
   for this case — there's nothing corrupt to reset, only an unwritable
   target — or should `Failed` offer something reset-shaped too?
2. Is setting `write_disabled: true` for `DeferredByConflict` (the benign
   race) an overcorrection, since the review's C2 language centered on
   "refused... because the file did not match" generally rather than
   distinguishing conflict from I/O failure for this specific flag? I
   applied it to both, reasoning that "we could not establish overwriting
   is safe" holds equally for both causes even though only `Failed` needs
   to be *visible*.
