# Developer Handoff 017 — F87: a save that cannot represent the content must not happen

**From:** architect
**Date:** 2026-09-01
**Register:** F87 (High). **Governing: RFC-082 §D4** (accepted).
**Gate:** Release-blocking (audit blocker B5). **The last blocking corruption defect.**
**Not in scope:** F88a/F88b — handoff 018. See §6.

---

## 1. Task title

Refuse a save whose content the target encoding cannot represent, name the
characters, and offer UTF-8.

## 2. Purpose

`encode_text` discards `encoding_rs`'s third return value — the flag reporting
that unmappable characters were replaced with numeric character references:

```rust
let (bytes, _, _) = enc.encode(content);
(bytes.into_owned(), false)
```

The `bool` it does return is `true` only for an **unknown label**, never for a
lossy encode. Reproduced by the architect:

```
encode_text("hi 😀\n", "shift_jis")
  bytes = "hi &#128512;\n"     ← literal ASCII written into the user's file
  flag  = false
```

The UI then reports "Saved." A combining accent behaves the same way:
`café` becomes `cafe&#769;`.

## 3. Where the refusal goes — and this is the load-bearing instruction

`save_text`'s order is: `check_precondition` → **`encode_text`** → **backup** →
write.

**Refuse immediately after `encode_text`, before the backup.** The backup step
copies over any existing `<name>.bak`, destroying the previous one — so refusing
one line later would already have cost the user their prior backup for a save
that never happens.

Nothing on disk may be touched when a save is refused for this reason. That is
the whole point of blocking rather than warning.

## 4. Required implementation

### 4a. Core reports what it could not write

`encode_text` must surface the flag. Beyond that, **the dialog has to name the
characters**, and `encoding_rs` reports only *that* replacement happened, not
which characters caused it.

**Find them on the failure path only.** The fast path — the overwhelming majority
of saves — must not pay for this: encode once, check the flag, and only if it is
set make a second pass to identify the offending characters. A per-character
encode on every save would be a real cost for a case that almost never occurs.

**Cap the list.** A file with a thousand unmappable characters must not produce a
thousand-item dialog. Report the first few distinct ones and a count; choose the
cap and say what you chose.

### 4b. A new error kind and a new recovery action

- **`AppErrorKind::EncodeLossy`** — symmetric with the existing `DecodeLossy`,
  which covers the read direction and has no encode counterpart.
- **`RecoveryAction::SaveAsUtf8`**, with `default_recovery_actions` mapping
  `EncodeLossy` to `{SaveAsUtf8, Dismiss}`.

Review 083 established that all twelve `RecoveryAction` variants are matched
exhaustively with **no catch-all**, so a thirteenth is a compile error at every
site. That is the property that makes this addition safe — the compiler will show
you every place that must decide.

**It also means review 083's subset test must be extended**, or it will fail: it
asserts each save-reachable `AppErrorKind` emits only handled actions. Update the
reachable set deliberately, not by loosening the assertion.

### 4c. The dialog already exists

F52 wired `SaveErrorView` and `Modal::SaveError`; `handle_result` routes
non-`Conflict` errors through it. A new `CoreError` variant reaching
`AppError::from_core` flows there with no new plumbing.

**The conflict arm stays untouched** — handoff 012 §3's constraint has not
lapsed.

### 4d. What `SaveAsUtf8` does

Re-run the save with `encoding_label = "UTF-8"`.

**And update the tab's save target to match**, or the next save reverts to the
original encoding and blocks again — the user would fix the same problem twice
and reasonably conclude the first fix did nothing. Changing a file's encoding is
a real decision; once taken, it must stick.

### 4e. Consume the flag that already exists

`SaveOutcome.encoding_fallback_to_utf8` is produced today and **read by nobody**,
though its own doc says the UI must warn. It reports an *unknown label*, which is
a different condition from this one. Surface it. It is one line and it has been
waiting since it was written.

## 5. What the dialog must say — §D4, and this is a requirement

"Cannot encode" is meaningless to someone who has never thought about charsets.
The dialog must:

- **name the characters** it cannot write — the user has to find them;
- **say what the file's encoding is** — that is the fact they are missing;
- **offer the escapes that preserve their data** — save as UTF-8, or go back and
  edit.

Substituting `&#128512;` is a fourth option nobody would choose, taken silently.
Do not reproduce it in a friendlier form: a dialog that says "some characters
will be replaced — continue?" is the same defect with consent attached.

## 6. Explicit non-change scope

- **F88a/F88b (`can_save`, `EditabilityClass`, `save_capability()`)** — handoff
  018. `can_save`'s pair-wide expression stays wrong for now; do not touch it.
- **The `.bak` clobbering** (`save.rs:70-77`) — a separate audit finding.
- **The conflict arm**, `precheck_save_as_target`, `persist_noclobber`.
- `ROADMAP.md`, the RFCs.

## 7. Required tests

1. **A save with unmappable content writes nothing.** Assert the target is
   **byte-identical** afterwards **and that no `.bak` was created or replaced** —
   §3's ordering requirement. **Falsify by moving the refusal after the backup
   step**; the `.bak` assertion must fail while the target assertion still
   passes. That is the test that proves the ordering, and only the `.bak`
   assertion catches it.
2. **The characters are named.** `encode_text`-level: `😀` into Shift_JIS reports
   that character, not merely "lossy".
3. **The fast path does not scan.** A save with no unmappable content must not
   run the identification pass. If you cannot express that as a test, say so
   rather than implying it.
4. **`SaveAsUtf8` writes the file and updates the save target's encoding**, so an
   immediately following save does not block again.
5. **Review 083's subset test still passes**, with `EncodeLossy` in its set.

## 8. Acceptance criteria

- Saving `😀` into a Shift_JIS file writes nothing and raises the dialog.
- No `.bak` is touched by a refused save.
- The dialog names the characters and the encoding.
- `SaveAsUtf8` succeeds and sticks.
- No catch-all added over `RecoveryAction`.
- Gates green, as handoff 016 §6.

## 9. Prohibited shortcuts

- **Do not warn-and-write.** §5.
- **Do not refuse after the backup.** §3.
- **Do not scan per-character on the success path.** §4a.
- **Do not loosen review 083's subset assertion** to accommodate the new variant.
- **Do not report a falsification you did not run.**

## 10. Required review-request format

Lead with test 1's falsification — specifically the `.bak` assertion failing when
the refusal is moved after the backup.

State plainly:
- the cap you chose for the character list, and why;
- what `SaveAsUtf8` updates besides the file;
- whether the fast path is provably unscanned, or only argued.
