# Review Request 085: F89 — atomic_replace uses an unpredictable temp file

**Governing task.** `dev-record/handoffs/016-f89-secure-atomic-write.md`
**Register.** F89 (High, security, audit blocker B5). Governing: RFC-082 §D5.
**Baseline.** `main` at `b3a0b50` (docs: F89 handed off, handoff 016)
**Commit.** `03f5cfb`

## 1. Test 1's falsification — the attack succeeding, then failing

Temporarily restored the old body of `atomic_replace` (`fs::write(temp, bytes)` at the predictable `.{filename}.fsk-tmp` sibling path, then `fs::rename`), ran the new symlink test against it:

```
test tests::save_tests::atomic_replace_does_not_follow_a_pre_created_symlink_at_the_old_predictable_temp_path ... FAILED
...
thread '...' panicked at .../save_tests.rs:320:5:
assertion `left == right` failed: the unrelated file must be untouched — atomic_replace must never write through a pre-existing symlink at any predictable path
  left: "user's new content\n"
 right: "victim's own content\n"
```

`victim.txt` genuinely received `"user's new content\n"` against the old code — the same overwrite-through-a-symlink the architect reproduced. Restored the real implementation; the test passes. No root-skip, per the handoff — this test creates its own symlink and root changes nothing about link-following.

## 2. Permissions — checked, not assumed

`atomic_replace_output_is_not_left_with_tempfiles_narrow_default_permissions` (`save_tests.rs`) follows `persist_noclobber`'s own precedent exactly: mode compared against a same-directory reference file created by plain `fs::write` in the same process, not a hardcoded `0o644` (F38, review 051 §3.3 — correct under any umask). Ran it, and the whole `permissions`-filtered set, under `umask 077` locally (the same command CI's F41 step runs):

```
$ (umask 077; cargo test -p forskscope-core permissions)
test result: ok. 3 passed; 0 failed
```

All three permission tests pass (`persist_noclobber`'s existing one, `atomic_replace`'s new one, plus whatever else the filter catches). Confirmed the new test's name is picked up by CI's exact-match canary: `cargo test -p forskscope-core permissions -- --list` includes `tests::save_tests::atomic_replace_output_is_not_left_with_tempfiles_narrow_default_permissions`, so F41's umask-077 step covers it automatically — no CI file change needed.

Saved files get **umask-derived permissions** (0o666 requested, kernel applies the process umask, mirroring `persist_noclobber_with_hook`'s own reasoning) — not `NamedTempFile`'s `0600` default.

## 3. `atomic_replace` still does not create parent directories

Untouched by design (§3a). Proven at the caller level, not just by reading the code: new test `settings_save_into_a_missing_parent_directory_still_succeeds` (`persist_v2_repository_tests.rs`) points a fresh `SettingsRepository` at a path whose directory has never been created — not even by the usual `temp_path` helper's `create_dir_all` — and asserts the save still succeeds, via `atomic_write_envelope`'s own `create_dir_all` call, the one this handoff says must stay where it is.

## 4. `temp_path_for` — deleted

Checked before deleting: its only call site was `atomic_replace` itself (`grep -rn "temp_path_for"` across `crates/`). Removed along with the function.

## 5. Collateral: three pre-existing tests broke, and why

Not asked for by the handoff, but `cargo test --workspace` after the production change surfaced it, so fixed rather than left red:

- **Two tests relied on obstructing the old predictable temp path** to force a write failure between backup and replace (`settings_commit_migration_survives_failure_between_backup_and_replace`, `session_...` — plus the runtime-layer pair, `settings_resolve_surfaces_a_failed_commit_and_disables_writes`, `session_...`). Pre-creating a directory at `.{filename}.fsk-tmp` no longer blocks anything once `atomic_replace` tries a different random name every time. Rewritten to obstruct at the **directory** level instead: pre-create the backup file so `ensure_pre_v2_backup` finds it already there and never needs to write, then strip write permission from the directory so `tempfile_in` itself cannot create its new file, regardless of name. `#[cfg(unix)]`, with the same verify-before-assert root-skip pattern `dir_unreadable_tests.rs` established for handoff 006 — checked to have an effect before asserting, so a root CI runner doesn't pass these for the wrong reason. (These four *do* need the skip, unlike test 1 above — they depend on a permission bit actually restricting a write, which root ignores.)
- **One test's assertion went silently vacuous, not silently broken**: `settings_save_leaves_no_stray_temp_file` filtered directory entries for the substring `"fsk-tmp"`. Verified empirically (temporary debug print, since removed) that a real save's temp file now contains no such substring anywhere in its name — so the filter always finds nothing, regardless of whether a stray file is genuinely left behind. It would have kept reporting green through an actual regression. Rewritten to flag any entry that isn't the target file by name, a check that doesn't depend on knowing the implementation's temp-naming scheme. Falsified locally (temporarily wrote a decoy file into the directory before the assertion, confirmed the test fails, removed the decoy) to confirm the new assertion is not itself vacuous.

All four fixes are additive to test *mechanism*, not to what's being asserted — none weaken or remove a check; each restores the same original guarantee the test already claimed to provide, using a technique compatible with the new random-name behavior.

## 6. Scope discipline

- `persist_noclobber`/`persist_noclobber_with_hook` — untouched (confirmed via `git diff`: only mentioned in doc comments).
- `.bak` copy handling (`save.rs:70-77`) — untouched, separate audit finding per the handoff.
- F87/F88, `ROADMAP.md`, RFCs — not touched by this commit.
- Diff: `save.rs` (production) plus three test files — no other production code changed.

## 7. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (clean), `cargo test --workspace` (704 core tests, up from 701 — 3 net-new tests; all other crates unchanged), `cargo xtask css --check`, `cargo xtask version-sync`, `cargo xtask i18n` (237 keys, unchanged), `cargo xtask rfc-sync`, `git diff --check`. Pushed as `03f5cfb`; CI dispatched on push (run 33509980640).
