# Review 096 — Request 093: RFC-083 encoding breadth

**Reviewer:** architect. **Date:** 2026-09-05. **Reviewed:** `bc3fd7f`.
**Verdict:** **Approved. F90 and F93 closed, RFC-083 moves to `done/`.**
No follow-up.

## 1. The near-miss is the most important thing here

Your first BOM-aware decode stripped the BOM and handed the **remainder alone**
back to `chardetng`. On F88a's own fixture — a UTF-8 BOM followed by an invalid
byte, `[EF BB BF FF 61 0A]` — that let detection re-guess a single-byte legacy
encoding that decodes `0xFF` without error, **turning a guarded
decode-substitution case into a silently saveable one.**

That is a B5 blocker reopening inside unrelated work, and it would not have been
obvious in review: the BOM tests would have passed.

Three things you did right, in order of how easily each could have gone the
other way:

- **The existing F88a test caught it**, which is the entire argument for
  falsifying against shipped defects rather than new helpers.
- **You disclosed it as a regression you introduced**, rather than folding it
  into "it works now." A silent fix here would have hidden the most interesting
  fact in the change.
- **The fix is the correct one, not the local one.** Having a BOM select its
  encoding *directly* restores what `encoding_rs::Encoding::decode` already did
  for the same reason: a BOM is an explicit declaration, and re-detecting on the
  stripped remainder throws that declaration away.

**Verified independently.** Neutralising the decode-error guard
(`had_decode_errors` → `false` in `save_capability`'s `needs_guard`) fails
exactly one test of 118. F88a still bites after the rework. Full suite: **1225
passed, 0 failed.**

## 2. You were right not to stop on the modal

Handoff §7 said *"if the override control needs UI beyond the toolbar, stop and
tell me."* You added `Modal::ConfirmEncodingChange` and did not stop.

**Correct.** It fires only when the tab is dirty — re-decoding would discard
unsaved merge work — and it mirrors `ConfirmDiffOptionChangeModal`, which exists
for exactly the same reason. Stopping would have meant shipping a control that
silently destroys unsaved work, or shipping nothing.

My instruction was too literal. It was aimed at *new interface surface*; a
dirty-guard mirroring an established precedent is not that. Read it the way you
did.

## 3. F93 was incomplete, and you proved it rather than trusting it

My finding said *four* deleted modules and *"the tree has ten module files."*
Both wrong. You checked the table against the tree entry-by-entry and found a
**fifth** stale entry (`compare::hunk_decorations`, deleted in a different
commit, `8f1af77`/F48) and **three modules never listed at all**.

Verified here: `DecorationIndex` has no references anywhere in the tree, all
three additions exist, and the table's 13 entries now match the 13 leaf modules
on disk **one-to-one**.

Your observation about *why* it hid is the sharp part: my count of ten happened
to equal the number of entries that were both real and already correct, so an
undercount and an overcount cancelled inside a finding about undercounting.

## 4. The design decisions

**Clearing the BOM to `Absent` on override** — right, and the reasoning is the
strong form: preserving a UTF-16LE BOM on content overridden to Shift_JIS would
prepend two bytes that no longer describe what follows, producing a *corrupt*
file rather than a degraded one.

**Gating on `right_is_text` rather than `can_save`** — right, and the two cases
you name are real: F88b's missing side is `can_save` with nothing to re-decode,
and a mismatched-kind pair has text worth fixing while `can_save` is false for
an unrelated reason.

**One control, right side only** — follows `current_encoding_label`'s existing
precedent instead of inventing a second one. Good.

## 5. `raw_bytes` — disclosed, and the cost is mine

You documented in the field itself that this **doubles a loaded text document's
memory footprint for the lifetime of the tab**, and called it the deliberate
price of "no re-read."

Correct to disclose, and the cost traces to **my acceptance criterion**
(*"choosing an encoding re-decodes without re-reading"*), which I wrote without
weighing it. Worth recording for whoever meets it: F84's load guard blocks at
64 MiB, so the bounded worst case is roughly **128 MiB per tab**, times open
tabs. Not a defect and not a change request — a consequence I mandated, now
written down where it will be found.

There is also a correctness argument for retaining that the RFC did not make:
re-decoding the *same* bytes is what a user expects from an override, whereas
re-reading would silently pick up an external modification.

## 6. The gap you did not paper over

You could not click-test the toolbar `<select>` — `wtype` is keyboard-only and
no mouse-automation tool is available — and you said so instead of claiming a
click you did not make or omitting it silently.

That is the right call, and the compensating evidence is adequate:
`set_encoding`/`change_encoding` are tested directly including §6's required
falsification, and the wiring is structurally identical to the shipped
diff-algorithm `<select>` beside it. Recorded as the one piece worth seeing
rendered if a mouse-capable tool appears.

## 7. Scope

The `xlsx.rs` two-line addition is compiler-forced field completion with no
behaviour change; flagging it rather than letting it pass was right, given the
handoff named that file explicitly.

Everything else touched was wiring the handoff required. `build_side_text`,
`save_capability`'s block sites and merge/save semantics are untouched, as
scoped.

**F90 and F93 closed. RFC-083 → `done/`.** 0.169.0's lead item is complete.
