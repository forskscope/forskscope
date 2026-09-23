# Developer Handoff 013 — F92: delete two documented controls that do not exist

**From:** architect
**Date:** 2026-09-01
**Register:** F92 (its two control claims only). **Governing: RFC-082** (accepted).
**Gate:** Release-blocking. **Documentation only — no code.**

---

## 1. Task title

Remove two sentences that assert safety controls the product does not implement.

## 2. Purpose

The 2026-09-01 audit checked 34 documented claims and found twelve false. Two are
qualitatively worse than the other ten, because they assert a **control** rather
than a feature — a reader relies on them:

- **`docs/src/intermediate/file-types.md:44`** — *"If you add characters outside
  the charset, a save guard warns you before writing."* That guard is
  `EditabilityClass::requires_save_guard()`, which has **zero call sites** (F88).
  So the document describes the mitigation for F87 — silent corruption when
  saving into a legacy encoding — **as if it shipped.**
- **`docs/src/maintainers/threat-model.md:67`** — *"No crash or panic path exists
  from oversized input (fuzzing confirmed in test suite)."* **No fuzz or property
  testing exists anywhere in the repository** — no `proptest`, no `quickcheck`,
  no `cargo-fuzz`, no such string in `crates/`, `xtask/`, `tests/` or `.github/`.

RFC-082 sequences these **first and separately**, before any code: a document
that asserts a control which does not exist is worse than no document, and
neither correction depends on a fix.

## 3. Required implementation

**Delete both claims. Do not soften them.**

- `file-types.md` — remove the sentence. **Do not replace it with a promise about
  what will happen**; RFC-082 §D4 will add a real guard, and the accurate
  sentence gets written *then*, describing what the guard actually does. Until
  then the honest state is silence.
- `threat-model.md` — remove the parenthetical assurance. The surrounding claim
  about oversized input may stand **only if** something supports it; if the
  section's confidence rests on the deleted clause, weaken the section to what
  the test suite actually demonstrates.

**Add a limitation while you are in `file-types.md`:** saving into an encoding
that cannot represent the content **currently writes numeric character
references** (e.g. `😀` becomes the literal text `&#128512;`). That is F87 and it
is true today. A user reading the file-types page is exactly the person who needs
to know.

## 4. Explicit non-change scope

- **The other ten false claims.** They are real and registered (F92), but they
  are features, not controls, and several are corrected by RFC-083/084's own
  documentation work. Do not sweep them in.
- **No code.** Not one line. If you find yourself editing `crates/`, stop.
- `ROADMAP.md`, the RFCs — not yours.

## 5. Required tests

None, and that is not a shortcut: this deletes two false sentences. `mdbook build
docs` must pass, which is the only mechanical check that applies.

**What replaces a test here is a check that the claim is actually gone**, not
reworded into something equally unsupported. State in your review request the
exact before/after text of both edits.

## 6. Acceptance criteria

- Neither sentence survives, in any form.
- `file-types.md` states the current lossy-encoding behaviour.
- `grep -ri "fuzz" docs/` returns nothing that claims fuzzing is performed.
- `mdbook build docs` passes; `git diff --check` clean.

## 7. Prohibited shortcuts

- **Do not reword a false assurance into a vaguer one.** "Care is taken to…" is
  the same defect with a longer sentence.
- **Do not add the guard's description in advance of the guard.**

## 8. Required review-request format

Short. Quote the before and after for each of the two edits, and the new
limitation sentence.
