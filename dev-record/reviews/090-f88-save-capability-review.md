# Review 090 — Request 087: F88, save capability

**Reviewer:** architect
**Date:** 2026-09-01
**Reviewed:** `a24d593`, against baseline `cbee017`
**Verdict:** **Approved, and your deviation was right — my handoff was wrong.**
F88a and F88b are closed. **B5 is not yet closed**: it made two user documents
false, which is handoff 019.

## 1. The deviation — you caught a regression my handoff would have caused

Handoff 018 §4 said *"`ReadWriteWithGuard` must not silently permit the save"*,
i.e. trigger the guard on `requires_save_guard()`. **That is wrong**, and I
verified why rather than taking your word:

```
clean Shift_JIS        -> ReadWriteWithGuard   requires_save_guard = true
clean windows-1252     -> ReadWriteWithGuard   requires_save_guard = true
clean UTF-8            -> ReadWrite            requires_save_guard = false
UTF-8 w/ decode errors -> ReadWriteWithGuard   requires_save_guard = true
```

`requires_save_guard()` is true for **every non-UTF-8 file**, not only for
decode-substituted ones. Implemented as written, my handoff would have made
every legacy-encoded file permanently unsaveable — and, worse, would have made
**F87's `SaveAsUtf8` escape unreachable**, because the block fires in
`build_request` before `save_text` ever runs. I would have broken the feature
shipped two handoffs earlier, inside the work meant to make saving safe.

**I confirmed your test catches exactly that.** I implemented my own instruction
literally and ran the suite:

```
a_cleanly_decoded_non_utf8_file_is_not_swept_into_the_new_guard ... FAILED
```

So the test exists to prevent my regression specifically, and it works.

Your §4 also caught that the message I dictated — *"there is no save as UTF-8
that helps, because UTF-8 is already the problem"* — is **literally false** for a
cleanly-decoded non-UTF-8 file, where that escape is exactly what helps. The
sentence was right for the case I had in mind and wrong for the case my
instruction covered.

**Disclosing this instead of implementing it silently is the whole point.** You
argued it from RFC-012 §9.2's own table, gave the concrete regression, and
offered to narrow back if I disagreed. That is what a handoff being wrong should
look like from the outside.

## 2. The design keeps `EditabilityClass` honest

Taking `had_decode_errors` directly could have reduced `EditabilityClass` to
decoration. The `debug_assert!` that `had_decode_errors` implies
`requires_save_guard()` prevents that: the class is now an **invariant check**
rather than an unused parameter, and a future `from_kind` change that broke the
subset relationship would fail in every debug build and test run.

The composition order is also right — target state, then mergeability, then the
guard — so widening never reaches the binary case, which you tested with genuinely
binary fixtures rather than trusting `classify`.

`Missing` carved out before the guard check, `from_kind` untouched: §3 as
specified.

## 3. Verified

Falsification reproduced; the clean-Shift_JIS guard test catches my handoff's
version; workspace green (core 717, ui-logic 200, ui 91). The byte evidence in
your §2 is the clearest statement of why F87 cannot cover this: `ff` → `ef bf bd`
at **decode** time, after which every save-time check is looking at valid UTF-8.

Driving it through the real app — Ctrl+S, screenshot, `od -An -tx1` confirming
the bytes were untouched — went beyond what was asked, and it is the only way to
know the dialog a user meets is the one the code builds.

## 4. Why B5 is not closed yet, and it is my fault

Handoff 013 added a **true** limitation to `file-types.md`: that unmappable
characters are written as numeric character references, with `😀` → `&#128512;`.

**F87 made that false**, and handoff 017 said nothing about the sentence handoff
013 had added four handoffs earlier. So the product now refuses that save and
offers UTF-8 while its own documentation says it silently corrupts the file.

That is the mirror of F92's original defect — a document **denying** a control the
product has — and a user reading it will avoid a feature that works. B5's
outcome *"no document asserts a safety control the product does not implement"*
cannot honestly be marked met while its inverse is on the page.

**Handoff 019** covers it, plus two more B5 made stale: the classification
table's treatment of a missing side (now restorable by saving), and F88a's
refusal, which no document describes at all — including that it has **no in-app
recovery**, which is true and which a user needs.

## 5. Status

F88a and F88b closed. B5 closes when 019 lands.
