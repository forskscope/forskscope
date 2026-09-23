# Developer Handoff 019 — the documentation B5 made wrong

**From:** architect
**Date:** 2026-09-01
**Register:** F92 (its remaining scope), F87/F88 follow-up.
**Gate:** Blocking B5's closure. **Documentation only — no code.**

---

## 1. Task title

Update the two user-facing statements that B5's fixes made false.

## 2. Purpose — and this one is mine

Handoff 013 added a **true** limitation to `file-types.md`:

> **Limitation:** if you add characters that the saved encoding cannot
> represent, they are currently written as numeric character references
> instead of being rejected or flagged — for example, saving `😀` into a
> `Shift_JIS` file writes the literal text `&#128512;`.

It was true when written. **F87 made it false four handoffs later**, and nothing
required updating it — handoff 017 said nothing about the sentence handoff 013
had just added. That is my chaining failure, not yours.

So the product now refuses that save and offers UTF-8, while its own
documentation tells the user it silently corrupts the file. **A document that
denies a control the product has is the mirror of F92's original defect**, and it
is worse than it sounds: a user who reads that page will avoid a feature that
works, or distrust a tool that is now correct.

## 3. Required implementation

### 3a. `file-types.md` — replace the limitation with the behaviour

Describe what now happens, from the user's side:

- a save whose content the target encoding cannot represent is **refused**, not
  written;
- the dialog **names the characters** and the encoding;
- **Save as UTF-8** is offered and works, and the file's encoding then stays
  UTF-8 for later saves.

**Do not describe the internals.** No `CoreError::Encode`, no `encode_text`. The
page is for users.

### 3b. `file-types.md` — the classification table's save column

`file-types.md:23-29` maps kinds to "Merge / Save ✓" and treats `Missing` as
one-sided. F88 changed that: **a missing side is empty text and can now be
created by saving**, which is how a deleted file gets restored.

`cli.md:102-103` already says a deleted file "opens normally with the missing
side shown as empty" — true then, incomplete now. It should say the file can be
**restored** by saving.

### 3c. The guard for a file read with substitutions

F88a added a refusal the docs have never described: a file whose bytes could not
be decoded is read with replacement characters, and **saving it will not
reproduce the original**, so the save is refused.

State it plainly, and state the consequence honestly: **there is no in-app
recovery** for such a file. That is true and the user needs it — it is the one
case where the answer is "use a different tool for this file".

## 4. Explicit non-change scope

- **F92's other claims** — the README's directory-CLI and directory-patch
  claims, `patch-export.md`'s compatibility claim, and the doc-vs-doc
  contradictions belong to RFC-083 and RFC-084's documentation work.
  **Do not sweep them in.**
- **No code.** Not one line.
- `ROADMAP.md`, the RFCs.

## 5. Required tests

None — this is prose. `mdbook build docs` must pass.

**What replaces a test is checking the claims against the product**, which is
what this handoff exists to fix. For each of the three statements you write,
say in the review request **how you confirmed it is true** — a test name, a
manual run, or a code path you read. A documentation change verified only by
having been written is how the original defect happened.

## 6. Acceptance criteria

- No document says characters are written as numeric character references.
- The refusal, the named characters, and Save-as-UTF-8 are described.
- A missing side is documented as restorable by saving.
- The decode-substitution refusal is documented, including that there is no
  in-app recovery.
- `mdbook build docs` passes; `git diff --check` clean.

## 7. Prohibited shortcuts

- **Do not describe a control more capable than the one that shipped.** F87
  refuses and offers UTF-8; it does not, for example, highlight the offending
  characters in the editor.
- **Do not delete the limitation without replacing it.** Silence would leave a
  user unable to predict a refusal they will meet.
- **Do not touch the other F92 claims.**

## 8. Required review-request format

Quote each before/after. For each new statement, say how you confirmed it.
