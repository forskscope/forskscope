# Review Request 086: F87 — a save that cannot represent its content must not happen

**Governing task.** `dev-record/handoffs/017-f87-lossy-encode-guard.md`
**Register.** F87 (High, audit blocker B5 — the last blocking corruption defect). Governing: RFC-082 §D4.
**Baseline.** `main` at `55d965d` (docs: F87 handed off, handoff 017, split from F88)
**Commit.** `c8aaeca`

## 1. Test 1's falsification — `.bak` catches the ordering, target-alone doesn't

Temporarily moved the refusal check to *after* the backup step in `save_text`, ran the ordering test:

```
test tests::save_tests::a_lossy_save_writes_nothing_and_never_touches_the_backup ... FAILED
...
thread '...' panicked at .../save_tests.rs:414:5:
assertion `left == right` failed: the backup step must never run for a refused save — a later refusal would already have destroyed the user's prior backup for a save that never happens
  left: [111, 114, ...]   ("original content\n")
 right: [97, 32, 112, ...] ("a prior backup that must survive untouched\n")
```

Exactly as the handoff predicted: the **target-untouched** assertion in the same test still passed (nothing ever reached `atomic_replace` either way), and **only the `.bak` assertion** caught the reordering. Restored the real ordering (refusal before backup); both assertions pass again.

## 2. The cap: 5 named characters

`MAX_REPORTED_UNMAPPABLE_CHARS = 5` (`encoding.rs`). Chose a small, fixed constant over anything content-dependent: a dialog is meant to be read, and five named characters plus a count ("`'😀'`, `'😁'`, `'😂'`, `'😃'`, `'😄'`, and 1 more") is already at the edge of what's skimmable — a thousand-character file caps at the same five names plus `"995 more"`, never a thousand-item list. `unmappable_characters_caps_the_list_and_counts_the_rest` proves the cap and the count separately (6 distinct emoji, cap 5 → 5 named + `additional == 1`), and `unmappable_characters_deduplicates_repeated_occurrences` proves repeated occurrences of the same character don't each count toward the cap or the overflow count.

## 3. What `SaveAsUtf8` updates besides the file

Nothing beyond what already existed — deliberately. `retry_save_as_utf8` reuses `build_request`'s normal (non-explicit-target) path to re-derive the target and precondition fresh from the tab, then overrides only `request.encoding_label = "UTF-8"` before dispatching through the same `save_text`/`handle_result` pipeline every other save uses. `handle_result`'s success arm already builds the tab's new `save_target` from `request.encoding_label` — since that field now reads `"UTF-8"`, the tab's save target picks it up automatically, with no special-cased "and also update the encoding" step. `save_as_utf8_writes_the_file_and_updates_the_save_targets_encoding` proves both halves: the file on disk actually contains the emoji afterward, and `tabs[0].save_target`'s `encoding_label` reads `"UTF-8"` — the second assertion is what makes "does not block again" true, not just "the file happened to save this once."

## 4. The fast path — proven, not argued

Handoff §7 test 3 explicitly allows "if you cannot express that as a test, say so" — I could express it, so I did. `unmappable_characters` increments a `#[cfg(test)]` **thread-local** counter (`UNMAPPABLE_SCAN_CALLS`) on every call — thread-local rather than a process-global `AtomicUsize` specifically so a reset-then-check test can never be made flaky by another test running concurrently on a different thread. `encode_text_success_path_never_calls_the_unmappable_scan` resets the counter, runs a clean `encode_text` call, and asserts it's still `0`. This is definitive, not inferential: the counter would catch a future change that moved the scan onto the success path even if every other assertion in the suite kept passing.

## 5. What the dialog actually says, and how it's proven

`AppError::from_core`'s existing message construction was purely `UserMessage::for_kind(kind)` — a static per-kind template with no path for per-instance data, which meant nothing in this codebase (including the pre-existing, never-constructed `DecodeLossy`) had ever put a real character or encoding name in a dialog body. Added `UserMessage::for_core_error(kind, err)`: falls through to the static template for every other case, but for `CoreError::Encode` builds the detail from the error's own `encoding_label`/`sample_characters`/`additional_count` fields — e.g. `"'😀', '🎉', and 3 more cannot be represented in shift_jis. Save as UTF-8 to keep them, or go back and edit the file to remove them."` — naming the characters, naming the encoding, and offering both escapes in one sentence, per §5's three requirements. `app_error_from_core_encode_names_characters_and_encoding_in_the_detail` asserts all three pieces land in `message.detail`, not just that the message is non-empty.

## 6. Review 083's subset test — extended, not loosened

`every_save_reachable_kind_only_emits_handled_recovery_actions` (`diff_actions.rs`) now iterates `{FileReadFailed, FileWriteFailed, BackupFailed, EncodeLossy}` against `HANDLED = {ChooseAnotherFile, Dismiss, SaveAs, SaveAsUtf8}` — one kind and one action added, the existing three of each left exactly as review 083 verified them. `handle_save_recovery_action` names all twelve `RecoveryAction` variants explicitly (no `_ =>`), so `SaveAsUtf8` forced a real match arm, not a silent fallthrough — confirmed by the compiler rejecting the build until I added it.

## 7. `encoding_fallback_to_utf8` — the other flag, consumed

Distinct from `EncodeLossy` (that one blocks; this one already wrote successfully with a substituted label). `handle_result`'s success arm now checks `outcome.encoding_fallback_to_utf8` and shows a warning toast instead of the plain "Saved." when it's set — the one line the field's own doc comment has been asking for since before this handoff existed.

## 8. Scope discipline

- F88a/F88b (`can_save`, `EditabilityClass`, `save_capability()`) — untouched.
- The `.bak` clobbering behavior itself (`save.rs:70-77`, a separate audit finding) — untouched; only the *ordering* relative to it changed.
- The conflict arm in `handle_result` — untouched (confirmed via `git diff`: only doc comments moved around it).
- `precheck_save_as_target`, `persist_noclobber` — untouched.
- `ROADMAP.md`, RFCs — not touched by this commit.

## 9. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (clean), `cargo test --workspace` (716 core / 87 ui / 200 ui-logic tests, all green — a `/tmp`-full condition from an unrelated project's session data made the default doctest run fail with a linker "no space left on device" error unrelated to this change; re-ran with `TMPDIR` pointed at a filesystem with space and all doctests passed), `cargo xtask css --check`, `cargo xtask version-sync`, `cargo xtask i18n` (238 keys), `cargo xtask rfc-sync`, `git diff --check`. Pushed as `c8aaeca`; CI dispatched on push (run 33513415302).
