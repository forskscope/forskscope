# Review Request 088 — F92b: refreshing the documentation B5 made wrong

Handoff: `dev-record/handoffs/019-f92b-refresh-docs-after-b5.md`
Commit: `55789b4` (pushed to `main`)
Documentation only — no code changed (`docs/src/intermediate/file-types.md`,
`docs/src/intermediate/cli.md`).

## 1. The numeric-character-reference limitation (§3a)

**Before:**
> **Limitation:** if you add characters that the saved encoding cannot
> represent, they are currently written as numeric character references
> instead of being rejected or flagged — for example, saving `😀` into a
> `Shift_JIS` file writes the literal text `&#128512;`.

**After:**
> If you add characters that the saved encoding cannot represent — for
> example, typing an emoji into a `Shift_JIS` file — the save is
> **refused**, not written. The dialog names the characters that cannot be
> represented and the target encoding. Choosing **Save as UTF-8** writes
> the file in UTF-8 instead; the file's tracked encoding then becomes
> UTF-8, so later saves of it stay in UTF-8 too.

How I confirmed each clause is true, by reading the shipped code (not by
re-deriving it from having written it):

- **"refused, not written"** — `AppErrorKind::EncodeLossy`'s
  `default_recovery_actions` offer `SaveAsUtf8`/`Dismiss` only, and the save
  path constructs it from `CoreError::Encode`, which `encode_text` returns
  instead of writing bytes (`core/src/error/app.rs:171`,
  `core/src/encoding_tests.rs::encoding_an_emoji_into_shift_jis_is_reported_lossy`).
- **"names the characters ... and the target encoding"** —
  `describe_unmappable_characters` (`core/src/error/app.rs:408-421`) builds
  exactly `"'😀' cannot be represented in Shift_JIS. Save as UTF-8 to keep
  them, or go back and edit the file to remove them."` from the real
  `CoreError::Encode` payload, via `AppError::for_core_error`.
- **"Save as UTF-8 writes the file in UTF-8 instead; the tracked encoding
  then becomes UTF-8"** — read `retry_save_as_utf8`
  (`ui/src/ui/view/diff_actions.rs:429-435`, forces `encoding_label =
  "UTF-8"` then reuses the normal save path) together with `handle_result`'s
  success arm (`diff_actions.rs:331-341`, replaces `tab.save_target` with a
  `Writable` state carrying that same `encoding_label`, which
  `current_encoding_label` reads for the *next* save). Test:
  `ui::view::diff_actions::tests::save_as_utf8_writes_the_file_and_updates_the_save_targets_encoding`
  (line 819) asserts the written bytes and that `save_target`'s
  `encoding_label` reads back `"UTF-8"` afterward.

## 2. F88a's decode-substitution refusal (§3c)

**Added** (new paragraph, not a replacement — this case had no documentation
before):
> A separate, unrelated case: if a file could not be fully decoded when it
> was *opened* — some of its bytes were not valid in the detected encoding
> and were replaced with the substitution character (�) — saving that file
> is refused entirely, with no offer to save as UTF-8. The bytes those
> substitutions stood for were already lost when the file was read, before
> any edit happened, so no encoding choice at save time can restore them.
> **There is no in-app recovery for this file** — to save its original
> bytes, use a different tool.

How confirmed: `AppErrorKind::UnsavableAfterDecodeLoss`'s
`default_recovery_actions` is `&[RecoveryAction::Dismiss]` only —
`core/src/error/app.rs:144` — no `SaveAsUtf8` arm exists for it anywhere in
`handle_save_recovery_action`, which is the exhaustive handler for every
button the dialog can show. Its static message
(`core/src/error/app.rs:311-315`) is the direct source for the wording
above. Test:
`state::compare::tests::a_file_that_decoded_with_replacement_characters_cannot_be_saved_without_the_guard`
(review request 087) drives this exact refusal end-to-end and confirms the
file's bytes are untouched afterward — the same evidence "no in-app
recovery" rests on: nothing the dialog offers writes anything.

## 3. Missing side restorable by saving (§3b)

**Classification table** (`file-types.md:29`): `Merge / Save` for
`Missing` changed from `—` to `✓ (creates the file)`.

**`cli.md:101-105`:** added "— and saving can create it, restoring a
deleted file" to the sentence describing a missing-side startup.

How confirmed: `SaveCapability`'s `Missing` carve-out
(`core/src/compare_prep.rs`, review 087) and
`state::compare::tests::a_missing_right_side_can_be_created_by_saving`
(review 087), which deletes a file, applies a merge hunk, saves, and
asserts the file exists again with the merged content.

## Scope

Only `docs/src/intermediate/file-types.md` and `docs/src/intermediate/cli.md`
touched. No other F92 claim (README directory-CLI/patch claims,
`patch-export.md`'s compatibility claim, doc-vs-doc contradictions) was
swept in — those are RFC-083/RFC-084's. No code. `mdbook build docs`
passes; `git diff --check` clean.

## Gate

`mdbook build docs` — clean, no warnings. `git diff --check` — clean.
Pushed as `55789b4`; CI run 33565693398 confirmed green.

## This closes B5

All three of B5's stale/false statements are corrected. Per handoff 019/review 090: **B5 is closed.**
