# Review Request 087 — F88: one source of truth for whether a save is possible

Handoff: `dev-record/handoffs/018-f88-save-capability.md`
Commit: `a24d593` (pushed to `main`, CI green — run 33564427209)

## Falsification (test 1)

`state::compare::tests::a_file_that_decoded_with_replacement_characters_cannot_be_saved_without_the_guard`
loads §2a's exact fixture (a UTF-8 BOM followed by an invalid byte,
`[0xEF, 0xBB, 0xBF, 0xFF, b'a', b'\n']`) as the right side, and asserts the
guard fires on save.

I falsified it by reverting `load_and_diff`'s `save_capability` computation
to the pre-F88 pair-wide expression (`is_mergeable_text() && is_mergeable_text()`,
ignoring `had_decode_errors` entirely) and disabling only the test's own setup
assertion so it could reach the downstream consequence. Re-running it then:

```
FALSIFICATION OBSERVED MODAL: Discriminant(0)

thread '...a_file_that_decoded_with_replacement_characters_cannot_be_saved_without_the_guard' panicked at crates/forskscope-ui/src/state/compare/tests.rs:754:28:
expected Modal::SaveError, got a different modal: Discriminant(0)
test result: FAILED. 0 passed; 1 failed
```

`Discriminant(0)` is `Modal::None` — with the guard gone, `save_tab` ran the
save straight through instead of blocking it. I reverted both changes
afterward and confirmed the working tree matched the committed state
byte-for-byte (`git status`/`git diff` clean except for an unrelated
untracked `xtask/Cargo.lock`).

## The byte difference (test 2)

`tests::encoding_tests::a_decode_substituted_reencode_would_differ_from_the_original_bytes`
asserts this directly, not just that the guard fires. Re-running the same
decode/re-encode with a temporary print statement (removed before commit):

```
original:   [ef, bb, bf, ff, 61, 0a]
re-encoded: [ef, bf, bd, 61, 0a]
```

The `0xFF` (invalid UTF-8 lead byte) becomes `ef bf bd` — the UTF-8
encoding of U+FFFD, the replacement character. The re-encode is one byte
longer than the byte it replaced and is not the original byte at all. This
is also, separately, why F87's own lossy check stays silent here: `ef bf bd`
*is* valid UTF-8 and re-encodes losslessly as UTF-8 — the loss already
happened at decode time, invisibly to any save-time check.

## Where the capability lives, and its inputs

`SaveCapability` and the composing function `save_capability()` live in
`core/src/compare_prep.rs`. It replaces the old pair-wide `can_save: bool`
as the source of truth; `CompareTab.can_save` and `PreparedCompare`'s old
field are now *derived* from it (`save_capability.is_saveable()`) at the two
places that install it (`load_and_diff` → `commit_load_result`, and
`swap_sides` → the new `refresh_save_capability`).

The handoff names three inputs — both sides' `FileKind`, both sides'
`EditabilityClass`, and the target's `SaveTargetState`. I'm disclosing a
deviation here rather than letting the signature imply something coarser
than what it does: `save_capability()`'s actual parameter list is

```rust
fn save_capability(
    left_kind: &FileKind, right_kind: &FileKind,
    left_editability: EditabilityClass, right_editability: EditabilityClass,
    left_had_decode_errors: bool, right_had_decode_errors: bool,
    target_state: &SaveTargetState,
) -> SaveCapability
```

— i.e. it also takes `had_decode_errors` for each side, alongside
`EditabilityClass`, rather than deriving the guard purely from
`EditabilityClass::requires_save_guard()`. This is a deliberate, load-bearing
narrowing, not scope creep, and I want it reviewed as its own decision:

`requires_save_guard()` is `true` for two situations `EditabilityClass::from_kind`
does not distinguish: decode substitution (`had_decode_errors` — unrecoverable,
the original bytes are already gone from memory) and a merely non-UTF-8
encoding that decoded *cleanly* (recoverable per-character, already checked
precisely at save time by F87's `save::save_text`/`encode_text`). If the guard
here triggered on `requires_save_guard()` alone, every legacy-encoded file
that decoded without error would be unconditionally blocked from saving,
with no escape — including F87's own shipped Shift_JIS `SaveAsUtf8` scenario,
which would become permanently unreachable, since the new guard would block
it before `save_text` ever ran. RFC-012 §9.2's original design table also
treats these as two different situations with two different UI treatments
(three recovery options for the clean-but-non-UTF-8 case vs. a guarded
warning for the substituted case) — I read that as confirming, not
contradicting, this split. The handoff's own guard message ("there is no
save as UTF-8 that helps, because UTF-8 is already the problem") would also
be literally false for a clean non-UTF-8 file — there the save-as-UTF-8
escape does help, which is exactly what F87 offers there.

So I check `had_decode_errors` directly, and treat `EditabilityClass` as
still genuinely exercised by a `debug_assert!` inside `save_capability` that
checks (in every debug build and test run) that `had_decode_errors` is
always a subset of `requires_save_guard()` — never an unrelated condition
smuggled in instead of it. `a_cleanly_decoded_non_utf8_file_is_not_swept_into_the_new_guard`
pins this with a verified-clean Shift_JIS fixture (`had_decode_errors: false`)
and asserts `SaveCapability::Saveable`, not `SaveableWithGuard`.

If this reading is wrong and the coarser `requires_save_guard()` trigger was
intended regardless of the F87 regression, say so and I'll narrow it back —
but I did not want to silently apply a judgment call this size.

`Missing` is carved out explicitly (checked before the `had_decode_errors`
check, inside the same `needs_guard` closure): it's empty text, contributes
no content, and needs no guard. `EditabilityClass::from_kind`'s
`Missing → ReadOnly` mapping is untouched, per §3/§8.

## The guard's message, and why it isn't `EncodeLossy`

New `AppErrorKind::UnsavableAfterDecodeLoss` (`core/src/error/app.rs`),
`default_recovery_actions = &[RecoveryAction::Dismiss]` only — no
`SaveAsUtf8`, with a comment on the match arm explaining why: the in-memory
content is already valid UTF-8 and re-encodes losslessly, so no encoding
choice at save time restores bytes that were lost at load time. Message:

> **Cannot save: file was read with substitutions**
> This file was read with replacement characters for bytes that could not
> be decoded. Saving it now will not reproduce the original file exactly.

This is static text via `UserMessage::for_kind`, not built through
`AppError::from_core`/`CoreError::Encode` the way `EncodeLossy` is —
`RequestOutcome::RequiresGuard` (`ui/view/diff_actions.rs`) is raised
straight out of already-loaded tab state (`tab.save_capability.requires_guard()`),
checked immediately after the existing `!tab.can_save` refusal in
`build_request`, before `save_text` is ever called. `EncodeLossy` names
specific unmappable characters from a save-time encode attempt; this kind
never reaches that code path at all, so reusing it would produce a message
naming characters that were never the problem. Reviewed 083's subset test
(`every_save_reachable_kind_only_emits_handled_recovery_actions`) is
extended to include the new kind.

I also drove this end-to-end through the real app (niri/wtype, not just
the test suite): a genuine BOM+invalid-byte fixture, Ctrl+S, screenshot
confirmed the exact title/body/path/single-Dismiss-button above, and
`od -An -tx1` confirmed the file's bytes were untouched afterward.

## Binary/spreadsheet sides

`a_binary_or_spreadsheet_side_is_still_not_saveable` uses two genuinely
binary files (leading `0x00` byte, so `classify()` doesn't misclassify them
as text) and asserts `SaveCapability::Blocked(SaveCapabilityBlockReason::NotMergeableText)`.
The composition checks `mergeable(left_kind) && mergeable(right_kind)`
(mergeable = `is_mergeable_text()` or `Missing`) before it ever looks at
`EditabilityClass` or `had_decode_errors`, so widening never had a chance to
reach this case.

## F88b, as a bonus

Not release-blocking, but implemented and tested alongside F88a since the
same `Missing` carve-out closes it for free:
`a_missing_right_side_can_be_created_by_saving` deletes the right file,
confirms `save_capability == Saveable` (no guard, no block), applies the
pending merge hunk, saves, and confirms the file is recreated with the
merged content.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` (717 core / 200 ui-logic / 91 ui-lib / 91 ui-bin, all
passing), `cargo xtask css --check`, `cargo xtask version-sync`,
`cargo xtask i18n`, `cargo xtask rfc-sync`, `git diff --check` — all green
locally and in CI (run 33564427209, all green). 9 files changed, none of
them F87's runtime guard, the conflict arm, `persist_noclobber`, `.bak`
clobbering, `ROADMAP.md`, or any RFC.

## This closes B5

F88a (release-blocking) is implemented, tested with a real falsification,
and verified end-to-end in the running app. F88b (not blocking) is done as
well. **This is B5's last item — it's closed.**
