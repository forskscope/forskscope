# Review Request: RFC-076 M2-B Patch 3 — Runtime Adapters

**Date:** 2026-08-03
**Reviewer stance:** design + implementation review
**Repository baseline:** `9abafab` (`persist: resolve N1 and add RFC-076
patch 3 runtime adapters`)
**Governing documents:** RFC-076 §"User-facing behavior", §"UI integration";
`rfcs/handoffs/076-versioned-runtime-persistence/implementation-handoff.md`
§4.4 patch 3 ("Runtime adapter tests before changing `App`"); audit finding
B2; review 037 (patch 2 approval, N1/§4.4 recommendations)

Patch 3 of 5. **No production `forskscope-ui` call site is changed.**
`app_json_settings::ConfigManager` remains the only thing the running
application actually calls; the switch is patch 4. This patch resolves
review 037's N1 finding, adds the runtime-decision layer that will drive
patch 4, and the pure view-models that will drive patch 5 — proven with
tests, not wired into `App`.

## Implementation Summary

### 1. Review 037 N1 — `commit_migration` now verifies before writing

`commit_migration(&self, value, original_bytes)` in both repositories now
calls a new `verify_unchanged(path, expected_bytes)` as its first step:
re-reads the target and compares against `original_bytes`, refusing to
proceed (new `PersistenceCommitError::Conflict`) if the file changed or
vanished since the caller's read. The return type changed from
`Result<PersistenceSaveOutcome, PersistenceIoError>` to
`Result<PersistenceSaveOutcome, PersistenceCommitError>` (which
`From`-converts an ordinary I/O failure, so the backup/write steps are
unchanged).

New `SettingsRepository::load_with_raw()` / `SessionRepository::load_with_raw()`
return `(PersistenceLoad<T>, Option<Vec<u8>>)` — the exact bytes a caller
needs to pair with `commit_migration` without a second, racy read.
`load()` is now `self.load_with_raw().0`.

### 2. Review 037 §4.4 — a real failure-window test

`settings_commit_migration_survives_failure_between_backup_and_replace` (and
its session mirror) obstructs the sibling temp path `atomic_replace` writes
to (`.{name}.fsk-tmp`, matching `save.rs`'s naming) by pre-creating it as a
directory, so the backup succeeds and the subsequent write fails. Asserts
the backup holds the original bytes and the target is untouched — the
actual failure window, using only ordinary filesystem behavior.

This also required re-verifying (not restructuring)
`settings_commit_migration_does_not_overwrite_existing_backup`: under the
new `verify_unchanged` gate, the old test's "call `commit_migration` twice
with different bytes" no longer models a legal caller, since a real second
call would now be rejected as a conflict. Rewrote it to pre-seed a stale
backup on disk and prove one legitimate commit does not touch it — the
non-overwrite semantic it was checking is unchanged, only how it's
demonstrated.

### 3. Core runtime resolution — `persist::v2::{settings,session}::runtime`

`resolve_and_commit(&repo) -> {Settings,Session}RuntimeResolution` is the
orchestration layer RFC-076 calls the "runtime adapter": it calls
`load_with_raw()`, and for `MigratedLegacy`/`MigratedVersion` immediately
calls `commit_migration` to durably write the migration (this is the first
real caller `commit_migration` has ever had). Produces:

```rust
pub struct SettingsRuntimeResolution {
    pub value: PersistedSettingsV2,
    pub write_disabled: bool,
    pub outcome: SettingsRuntimeOutcome,
}

pub enum SettingsRuntimeOutcome {
    Fresh,
    Current,
    Migrated { backup_path: Option<PathBuf>, committed: bool },
    Incompatible { schema: String, version: u32 },
    CorruptPreserved { detail: PersistenceError },
}
```

`write_disabled` is `true` only for `Incompatible`/`CorruptPreserved`,
matching RFC-076's `persistence_write_disabled`. `Migrated.committed` is
`false` only if the commit lost the N1 race or hit an I/O failure — the
migrated value is still used for this run, nothing was written, and the
file is simply re-migrated next run. Session's module is a line-for-line
mirror. Pure orchestration, no dialog text.

### 4. `forskscope-ui-logic` recovery view-models

`settings::persistence_recovery::SettingsRecoveryView` (+ session mirror)
maps a `*RuntimeResolution` to what a recovery dialog needs:

```rust
pub struct SettingsRecoveryView {
    pub migration_notice: Option<MigrationNotice>,       // Migrated{committed:true}
    pub dialog: Option<RecoveryDialogView>,               // Incompatible / CorruptPreserved
}
```

`Incompatible` offers `Exit` + `ContinueWithTemporaryDefaults`, never
`ResetAndBackupOriginal` (the file may be valid to a newer build).
`CorruptPreserved` offers `ContinueWithTemporaryDefaults` +
`ResetAndBackupOriginal`, never `Exit`. `RecoveryDialogAction` has no
"choose another location" variant — RFC-076 lists that only "if that
capability is later approved," and it isn't. Same "core decides, ui-logic
renders" split as the existing `compare::save_error::SaveErrorView`; no
Dioxus/GTK dependency, fully unit-tested.

## Addressed Items

- Handoff §"Implementation sequence" step 4: "Add runtime adapter tests
  before changing `App`" — done, nothing calls this from `App` yet.
- Review 037 N1 — resolved as recommended, in patch 3, "when the runtime
  adapter that calls this first exists."
- Review 037 §4.4 — the more direct crash-safety test is added.
- RFC-076 §"User-facing behavior" — every outcome
  (Missing/Current/MigratedLegacy/MigratedVersion/FutureVersion/Corrupt) has
  a runtime decision and, where the RFC calls for one, a view-model.
- B2 — **still not closed.** Nothing here is called by the running
  application; that's patch 4.

## Files Changed

New:

- `crates/forskscope-core/src/persist/v2/settings/runtime.rs` (58 ELOC) —
  `resolve_and_commit`, `SettingsRuntimeResolution`, `SettingsRuntimeOutcome`
- `crates/forskscope-core/src/persist/v2/session/runtime.rs` (58 ELOC) —
  session mirror
- `crates/forskscope-core/src/tests/persist_v2_runtime_tests.rs` — 15
  runtime-resolution tests (fresh/current/migrate-v0/migrate-v1/
  future/corrupt/uncommitted-migration, settings + session)
- `crates/forskscope-ui-logic/src/settings/persistence_recovery.rs` (91
  ELOC) — `SettingsRecoveryView`, `RecoveryDialogView`,
  `RecoveryDialogAction`, `MigrationNotice`, 6 tests
- `crates/forskscope-ui-logic/src/session.rs`,
  `crates/forskscope-ui-logic/src/session/persistence_recovery.rs` (89
  ELOC) — session mirror, 6 tests. New top-level `session` module
  (previously only `compare`/`explore`/`settings` existed).

Modified:

- `crates/forskscope-core/src/persist/v2/repository.rs` — new
  `PersistenceCommitError` (`Conflict`/`Io`), `From<PersistenceIoError>`,
  `verify_unchanged`
- `crates/forskscope-core/src/persist/v2.rs` — re-exports
  `PersistenceCommitError` (it appears in `commit_migration`'s public
  signature, so it must be nameable outside `persist::v2`)
- `crates/forskscope-core/src/persist/v2/{settings,session}/repository.rs`
  — `load_with_raw`, `commit_migration` signature/verification change,
  `mod runtime` + re-export at the parent
- `crates/forskscope-core/src/tests/persist_v2_repository_tests.rs` — N1
  conflict test, failure-window test, `load_with_raw` tests, rewritten
  non-overwrite test (both settings and session)
- `crates/forskscope-core/src/tests.rs` — declares
  `persist_v2_runtime_tests`
- `crates/forskscope-ui-logic/src/lib.rs`, `.../settings.rs` — module
  declarations and re-exports for the new view-models

Not staged: `xtask/Cargo.lock` (pre-existing untracked convention).

## Important Implementation Decisions

### N1's fix changed the legal shape of a `commit_migration` retry

Before this patch, a caller could call `commit_migration` twice with
different "original" bytes and it would silently proceed both times
(patch 2's own test exploited this to prove the backup doesn't get
overwritten). Once `verify_unchanged` gates the call, a second call with
bytes that no longer match what's on disk is now correctly rejected as a
conflict — because after the first `commit_migration` succeeds, the file
holds the v2 envelope, not the original bytes, so a second call passing
the *original* bytes again is exactly the stale-read scenario N1 exists to
catch. I rewrote
`settings_commit_migration_does_not_overwrite_existing_backup` to pre-seed
a stale backup file directly and issue one legitimate `commit_migration`
call, which still proves the `ensure_pre_v2_backup` non-overwrite semantic
without relying on a caller pattern the new contract now correctly forbids.
Flagging this because it's a test change driven by a fix, not new coverage.

### `commit_migrated`'s failure path never surfaces the error, only `committed: false`

When the commit fails (conflict or I/O), `resolve_and_commit` uses the
migrated value for this run and reports `committed: false` — it does not
propagate `PersistenceCommitError` up to the caller. RFC-076 doesn't
specify recovery behavior for this case (it's a race the RFC didn't
enumerate, only review 037 raised it), and the two outcomes I considered —
surfacing it as another `RecoveryDialogView`, or treating it as a
temporary-defaults case like `Corrupt` — both felt like inventing UI
requirements the RFC never asked for on a window review 037 itself
described as "microseconds during startup." Silently retrying next run,
with the migrated value still usable this run, seemed the least invasive
choice consistent with "never lose data, never overwrite a file you didn't
verify." Flagging this as a judgment call since it's the one runtime
outcome with no explicit RFC-076 user-facing behavior to match against.

### `RecoveryDialogAction` omits "choose another config location"

RFC-076's `FutureVersion` behavior lists "Exit, Continue with temporary
defaults, or choose a different config file location if that capability is
later approved." Since that capability doesn't exist, I didn't add an enum
variant with no implementation behind it — a dialog offering a button that
does nothing would be a real UI bug, not a placeholder. Adding the variant
when the capability actually arrives is a small, localized change to two
`match` arms in each `persistence_recovery.rs`.

### New top-level `session` module in `forskscope-ui-logic`

Previously the crate had `compare`, `explore`, `settings` as its three
feature-area modules; nothing session-specific existed yet since RFC-075's
session logic lives in `compare::tab_state`/`load_identity`, which are
about runtime tab identity, not persisted-session recovery. Persisted
session recovery is a distinct concern from those, so it gets its own
module rather than being folded into `compare` (wrong feature area) or
`settings` (wrong file entirely — it's a different persisted document).

## Not Implemented in This Patch, on Purpose

- Nothing calls `resolve_and_commit` or the view-models from `App`. No
  config-directory resolution, no Dioxus dialog component, no wiring into
  startup effects. That's patches 4 and 5 per the handoff's sequence.
- The `Migrated` outcome doesn't distinguish `MigratedLegacy` from
  `MigratedVersion` in its view-model — RFC-076's user-facing behavior
  treats both identically ("show a single informational notice"), so the
  view-model doesn't need the distinction the core outcome could still
  carry if a future patch wants source-specific wording.
- `RecoveryDialogAction::ResetAndBackupOriginal` has no implementation
  behind it yet (no `App`-level "explicit reset" action exists) — only the
  view-model advertises the action; patch 5 wires actual behavior to it.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test -p forskscope-core -p forskscope-ui-logic            pass — 1010 (was 978: core lib 678→698, ui-logic lib 241→253, +32 total, matching exactly)
cargo test -p forskscope-core persist_v2                        pass — 55/55 (was 35 at end of patch 2; +9 repository tests — conflict, failure-window, load_with_raw ×2 sides, plus a new session non-overwrite mirror patch 2 didn't have — and +11 new runtime-resolution tests)
cargo clippy -p forskscope-core -p forskscope-ui-logic -D warnings   pass
cargo clippy -p forskscope-core --tests -D warnings              8 pre-existing findings (F6) untouched; 0 new
cargo clippy -p forskscope-ui-logic --tests -D warnings          1 pre-existing finding (search_index.rs type_complexity) untouched; 0 new
cargo test --workspace                                           pass — unchanged forskscope-ui counts
cargo clippy --workspace -D warnings                              pass
cargo xtask version-sync                                          pass — v0.165.1
git diff --check                                                  pass
```

CI run `30770696155`: Test & Lint green in 4m53s, all 18 steps pass, on
`9abafab`.

## Known Limitations

- B2 remains open; still no production code path uses any of this.
- The uncommitted-migration race (N1's original concern) is now guarded
  against corrupting data, but its recovery UX (what, if anything, tells
  the user a migration is pending retry) is unspecified — see the judgment
  call above. If review wants this surfaced, it's a small addition to the
  view-model.
- F26 (schema-enum scalar variants unpinned by golden fixtures) is
  unaffected by this patch and remains scheduled before patch 4.
- B3, B4 remain open. v1/public release stays **No-Go**.

## Requested Review Focus

1. Is the N1 fix's scope right — verify-then-write inside
   `commit_migration` itself, rather than pushing the verification
   responsibility onto `resolve_and_commit` (the only current caller)?
2. Does silently reporting `committed: false` (rather than surfacing the
   conflict/IO error) match what RFC-076 would want for a race review 037
   itself called unlikely, or does this need a more visible failure path
   before patch 4 makes it reachable from real startup?
3. Is the core/ui-logic split — `*RuntimeResolution` in core (does I/O),
   `*RecoveryView` in ui-logic (pure) — the right boundary, matching
   `SaveErrorView`'s existing pattern, or should the recovery-view mapping
   live in core alongside the resolution it's derived from?
4. Anything about the rewritten
   `settings_commit_migration_does_not_overwrite_existing_backup` test that
   under-proves the non-overwrite semantic now that it no longer performs
   two real `commit_migration` calls?
