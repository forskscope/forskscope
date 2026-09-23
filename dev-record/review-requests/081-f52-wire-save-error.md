# Review Request 081: F52 — give save failures a real dialog

**Governing task.** `dev-record/handoffs/012-f52-wire-save-error.md`
**Register.** F52. Wires one of F75(b)'s four KEEP modules.
**Baseline.** `main` at `8242a61` (feat: F84 — wire load_guard into both compare load call sites)
**Commit.** `6c75763`

## 1. Falsification 1 — a non-conflict save failure opens the dialog, not a toast

Drove the real `handle_result` with a **real `CoreError`**, not a hand-built one — the review 077 precedent (`classify_digest_outcome` fed a real `Err` from `file_digest_equal_with_cancel`). Test setup: a `SaveRequest` whose target's parent path is itself a plain file (not a directory), so `save_text`'s `atomic_replace` genuinely fails writing the sibling temp file — a real `CoreError::Io { operation: Write, .. }`.

Temporarily reverted `handle_result`'s `Err(e)` arm to the old `store.notify(e.to_string())`, ran `cargo test -p forskscope-ui ui::view::diff_actions::tests --lib`:

```
test ui::view::diff_actions::tests::a_real_non_conflict_save_failure_opens_the_save_error_dialog_not_a_toast ... FAILED
...
thread '...' panicked at .../diff_actions.rs:559:32:
expected Modal::SaveError — the old notify(...) toast path must be gone
test result: FAILED. 6 passed; 1 failed
```
Exactly the targeted test failed; everything else stayed green. Restored the real code; all 7 pass again.

## 2. Falsification 2 — the conflict path is untouched (matters more than test 1, per the handoff)

Temporarily deleted the `Err(CoreError::Conflict { .. })` arm entirely, so every `Err` (Conflict included) fell through to the `SaveError` arm. Test setup for this one is also a **real** conflict, not hand-built: `SaveRequest` with `TargetPrecondition::MustBeAbsent` against a target that already exists on disk — `save_text`'s `check_precondition` genuinely returns `CoreError::Conflict`.

```
test ui::view::diff_actions::tests::a_real_conflict_still_opens_confirm_overwrite_not_the_save_error_dialog ... FAILED
...
thread '...' panicked at ...:597:22:
a save conflict must still raise Modal::ConfirmOverwrite, not the F52 dialog
test result: FAILED. 6 passed; 1 failed
```
Restored the real `Err(CoreError::Conflict { .. })` arm (byte-identical to baseline — `git diff` on this commit's `handle_result` shows it untouched except for the trailing `Err(e)` arm below it); all 7 pass again.

## 3. Reachable `RecoveryAction` set, and how it was determined

**`{ChooseAnotherFile, Dismiss, SaveAs}`** — three actions, not the "plausibly just `Retry`, `SaveAs`, `Dismiss`" the handoff floated. Determined by tracing every `CoreError` `save_text` can produce, not guessing from the type signature:

- `check_precondition`: `Conflict` (excluded — §2) or `Io { Metadata }` (a `symlink_metadata` failure that isn't "not found").
- Backup (`BackupPolicy::SiblingBak`): `fs::copy` failure → `Io { CreateBackup }`.
- `persist_noclobber` (the `MustBeAbsent` commit path): `Io { Write }` (tempfile create/write) or `Io { Rename }` (the no-clobber commit itself) — `AlreadyExists` on that commit becomes `Conflict`, not `Io`.
- `atomic_replace` (the `MustMatch`/`Force` commit path): `Io { Write }` or `Io { Rename }`.
- Post-write `FileFingerprint::capture`: only `Io { Metadata }`.
- `encode_text` is infallible (`(Vec<u8>, bool)`, no `Result`) — no error path there at all.

So the only `CoreError` variants reachable, `Conflict` aside, are `Io { Metadata | Write | Rename | CreateBackup }`. `AppErrorKind::from_core` maps those to exactly `{FileReadFailed (from Metadata — the read/metadata half of `Io`, still what the mapping calls it even though the metadata call here is a post-write fingerprint capture), FileWriteFailed (from Write/Rename), BackupFailed (from CreateBackup)}`. `default_recovery_actions()` maps those three kinds to exactly `{ChooseAnotherFile, Dismiss}`, `{SaveAs, Dismiss}`, `{SaveAs, Dismiss}` — union: `{ChooseAnotherFile, Dismiss, SaveAs}`.

## 4. No button renders without a working handler

`handle_save_recovery_action` matches all 12 `RecoveryAction` variants explicitly — `Dismiss`, `ChooseAnotherFile`, `SaveAs` each have a real arm; the other 9 (`Reload`, `OverwriteAnyway`, `OpenLimitedDiff`, `OpenAsBinary`, `Retry`, `RetryWithoutInline`, `Cancel`, `StartFresh`, `ReportBug`) are named explicitly and grouped into one `unreachable!()` arm — no `_ =>` anywhere, so a future 13th `RecoveryAction` variant is a compile error here, not a silently swallowed button (the `file_digest_equal`/F77/review 074 §5 precedent). `every_reachable_recovery_action_has_a_working_non_panicking_handler` calls all three reachable actions directly and asserts the resulting modal for each.

**One judgment call, disclosed rather than presented as self-evident:** `ChooseAnotherFile` and `SaveAs` both open the existing `Modal::SaveAs(index, target)` dialog. `ChooseAnotherFile` is naturally a *load*-time action — it reads oddly in a save context — but it's reachable here only through the rare case where the save itself already succeeded and the *post-write* fingerprint capture then fails (e.g. the file is deleted by another process microseconds after the rename). There is no existing "pick a different save target" flow distinct from Save As, so I mapped it there rather than inventing a fourth dialog for a case this narrow.

## 5. What did not change

- The `Err(CoreError::Conflict { .. })` arm — untouched, shown by falsification 2.
- `precheck_save_as_target`/`SaveAsPrecheck` — untouched.
- Session-save failures (`state/session.rs`) — out of scope, not folded in.
- `palette_view`, `conflict_nav_view`, `load_guard` — untouched (`load_guard` is handoff 011/review 080).
- No no-allowlist gate added; `ROADMAP.md` not touched.
- Buttons render in `view.buttons` order (`SaveErrorView`'s own ordering, not re-sorted), with `autofocus` following `button.is_primary` directly.

## 6. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (clean), `cargo test --workspace` (all green, including the 3 new F52 tests), `cargo xtask css --check`, `cargo xtask version-sync`, `cargo xtask i18n` (236 keys, unchanged), `cargo xtask rfc-sync`, `git diff --check`. Pushed as `6c75763`; CI dispatched on push (run 33077493993).
