# RFC 082: Write-Path Integrity

**Status.** Proposed
**Scheduling.** Pre-v1 — **audit blocker B5**, the release-blocking cluster from
the 2026-09-01 independent audit. See `ROADMAP.md` § "Remaining proposed RFCs",
which must list every file in this folder and nothing else (F83).
**Tracks.** Audit finding B5. Register F85–F89.
**Touches.** `core/src/merge/session.rs`, `core/src/merge/three_way/session.rs`,
`core/src/encoding.rs`, `core/src/save.rs`, `core/src/file_kind.rs`,
`ui/src/state/tab.rs`, `ui/src/state/compare.rs`,
`ui/src/ui/view/diff_actions.rs`, and two user documents.
**Origin.** Independent audit, 2026-09-01, commit `099e2c0`. Every finding below
was reproduced by the auditor and **re-reproduced independently by the architect**
before this RFC was written.

## Summary

The 2026-09-01 audit found 63 issues. Its own summary of their distribution is
the reason this RFC exists and is scoped as it is:

> both Criticals and three of the eight Highs are in one subsystem — the
> merge-and-save write path — while the diff engine, the concurrency design, the
> persistence layer, and the process discipline are all strong.

This RFC covers that subsystem and nothing else. Encoding breadth (RFC-083) and
patch conformance (RFC-084) are separate and **not release-blocking**.

**The audit's verdict agrees with the project's — v1 is No-Go — but disagrees
with the reason.** `ROADMAP.md` recorded one open blocker, B4 (platform runtime
evidence), which implies the software is correct and only its evidence is
missing. That was not true. This RFC is **B5**, and it is the parallel of
B1–B4: one blocker, one RFC (B1→RFC-075, B2→RFC-076, B3→RFC-077, B4→RFC-078).

**This document is deliberately self-contained.** The audit report lives at
`.git-exclude/tmp/audit/report/AUDIT-2026-09-01.md`, which is **untracked**; if
it is lost, the analysis below survives.

## The five defects

Each was re-reproduced by the architect on 2026-09-01 at `099e2c0`. Where a
transcript is quoted it is from that reproduction, not from the audit.

### 1. Save after "Swap sides" writes into the other file — F85, Critical

`swap_sides` (`ui/src/state/tab.rs`) swaps `left_doc`/`right_doc` and
`left_path`/`right_path`, recomputes `can_save`, recomputes the diff, and
persists the session. **It never touches `tab.save_target`.**

For a normal save (`build_request` with `target: None`), the destination path,
the precondition **and** the encoding label come *solely* from
`tab.save_target` — verified by reading `diff_actions.rs`'s `None` arm.

So after swapping A↔B: the merge result reflects A's side, `save_target.path`
still names B, and B's `MustMatch` fingerprint still matches disk because B was
never touched — **so no conflict dialog fires**. B is backed up to `B.bak` and
overwritten with A's content while the user believes they are editing A.

This is precisely the failure RFC-077 was written to eliminate. Making
`save_target` authoritative fixed the *load* path; `swap_sides` was never updated
to maintain it. **No test covers `swap_sides` against `save_target`** — confirmed;
the only matches are comments.

### 2. Unsaved merge work reports clean — F86, Critical

`MergeSession::is_dirty()` is `undo_stack.len() != saved_baseline`, and
`mark_saved()` sets `saved_baseline = undo_stack.len()`. **Depth is not identity.**

Reproduced by the architect against the real `MergeSession`:

```
saved  = "a\nb\nc\nD\n"      // what mark_saved() recorded
buffer = "A\nb\nc\nd\n"      // what result_text() now returns
is_dirty() = false
```

Sequence: apply hunk A → save (depth 1) → undo (depth 0, correctly dirty) →
apply hunk B (depth back to 1) ⇒ **reported clean while the content differs.**

Every dirty guard reads this: the tab dirty dot, the close prompt, Ctrl+W, the
reload and swap confirmations — and `disabled: !snap.is_dirty` on the **Save
button itself**, so the user cannot save the change they can see, and closing
does not warn. `ThreeWayMergeSession` has the identical predicate.

### 3. Saving into a legacy encoding corrupts silently — F87, High

`encode_text` does `let (bytes, _, _) = enc.encode(content)`, discarding
`encoding_rs`'s third return value — the flag reporting that unmappable
characters were replaced with numeric character references. The `bool` it *does*
return is `true` only for an **unknown label**, never for lossy encoding.

Reproduced:

```
encode_text("hi 😀\n", "shift_jis")
  bytes  = "hi &#128512;\n"     ← literal ASCII written to the user's file
  flag   = false                 ← "no fallback occurred"
```

The UI then reports "Saved."

### 4. The guard that would have prevented (3) is unwired — F88, High

`EditabilityClass`, including `requires_save_guard()`, has **zero production call
sites** — only `lib.rs`'s re-export and its own tests. The UI instead derives
`can_save = ld.kind.is_mergeable_text() && rd.kind.is_mergeable_text()`,
consulting neither `had_decode_errors` nor the encoding label.

Two consequences, one of them the audit's finding #16:

- a file that decoded with replacement characters is `FileKind::Text`, so
  `can_save == true`, and saving writes U+FFFD over the original bytes;
- `can_save` is a property of the **pair**, so a missing right side
  (`FileKind::Missing`) disables the entire merge/save toolbar — **you cannot use
  ForskScope to restore a deleted file**, although `cli.md` advertises exactly
  that, and `inspect_save_target` already reports that path as
  `Writable { MustBeAbsent }`.

### 5. Every save and every settings write uses an insecure temp file — F89, High

`atomic_replace` builds a **fully predictable** temp path —
`.{filename}.fsk-tmp` in the target's own directory — and writes it with
`fs::write`, which follows symlinks and does not use `O_EXCL`.

Reproduced by the architect (CWE-59 / CWE-378):

```
victim.txt now = Ok("user's new content\n")   ← unrelated file overwritten
doc.txt is_symlink = true
doc.txt -> "/tmp/.../victim.txt"              ← the user's document became a link
```

Two concurrent saves of the same target also collide on that one path, which
defeats the module's own headline promise of never exposing a partial write.

**The correct primitive is 80 lines away in the same file.**
`persist_noclobber` already uses `tempfile::Builder…tempfile_in(dir)` with a
careful comment about requesting `0o666` and letting the umask apply. `tempfile`
is already a `forskscope-core` dependency.

## Design principles

Three, in priority order. The owner set the bar as *"finally clean, safe and
secure, and robust and sophisticated"*, and added that the design must be
**friendly and in harmony with the user's instinct**. Where those pull in
different directions this RFC says so rather than pretending they agree.

**P1 — Never write something the user did not ask for.** All five defects violate
this. It outranks convenience everywhere below.

**P2 — The user's instinct is the specification.** A diff tool's user believes:
the right pane is the file I am editing; a dot means I have unsaved work; Save
writes what I see. Every defect here is a place where the code and that belief
disagree — and in each case **the belief is correct and the code is wrong.** That
is a useful test: where a fix has options, prefer the one a user would predict.

**P3 — Prefer preventing the class over patching the instance.** F85 could exist
silently because nothing in the UI shows where Save will write; a wrong
destination is indistinguishable from a right one until the file is gone. D6
addresses that directly.

## Design

### D1 — Dirty state is content identity, not stack depth

Record what was saved, not how deep the stack was. **Decision: hash
`result_text()` at `mark_saved()` and compare on `is_dirty()`.**

A monotonic revision counter is the obvious alternative and is **rejected**, on
P2. It reports *dirty* after the user undoes back to exactly the state they
saved — the buffer on screen is byte-identical to the file on disk and the app
insists there is unsaved work. That is the safe direction, so it would be
*acceptable*; it is not what the user's eye says. **Content identity matches the
screen**, in both directions, which is the whole job of a dirty indicator.

Both `MergeSession` and `ThreeWayMergeSession` get the same treatment. The
project's own `TransactionLog` already compares revisions (`merge/log.rs:235`);
this RFC does not adopt that shape, for the reason above.

### D2 — `save_target` is a function of `save_destination`, re-derived when its inputs change

**Corrected 2026-09-01, before acceptance. An earlier draft of this RFC said
"refuse the swap in mergetool mode." That was wrong**, and reading
`load_and_diff` shows why: `save_target` does not derive from `right_path`. It
derives from **`save_destination`**, which is fixed at launch:

- `SaveDestination::RightInput` → `save_target_from_loaded(&right, &rd)`
- `SaveDestination::Explicit(merged)` → `inspect_save_target(merged, …)` — a
  third path, **independent of both panes**

So there is one invariant and it needs no special case:

> **`save_target` is re-derived from `save_destination` exactly when the inputs
> it derives from change.**

Under it, `swap_sides` re-derives in `RightInput` mode (closing F85) and does
nothing in mergetool mode, because `$MERGED` did not change. **No refusal, no
mode check at the call site.**

Refusing the swap would also have been unfriendly on P2: swapping in mergetool
mode is a legitimate thing to want — *resolve using LOCAL as the base* — and
nothing about the merged output makes it unsafe.

**One trap the blunt version would have walked into, recorded so nobody
reintroduces it.** Re-deriving *unconditionally* would call
`inspect_save_target($MERGED)` on a tab whose merged file has **not been
re-read**, refreshing its `MustMatch` fingerprint against whatever is on disk
now — silently destroying external-modification detection for the one file the
mergetool contract cares about. Deriving only when the input changed avoids it;
patching the call site would not have.

A regression test asserts the invariant after **every** mutation of the compared
inputs, not only after swap.

### D3 — One source of truth for save capability, and F88 splits in two

Replace the pair-wide `can_save` expression with a single `save_capability()`
derived from the **`EditabilityClass` of the editable side** and the target's
`SaveTargetState`.

**F88 splits, and only half of it blocks** (owner decision, 2026-09-01):

- **F88a — the unwired guard blocks.** `requires_save_guard()` is D4's
  mitigation; shipping D4 without consulting it would leave the same hole in a
  different shape.
- **F88b — the missing-side restriction does not block.** Being unable to restore
  a deleted file is a **feature gap**, not a data-loss path. It is fixed in the
  same change because it is the same expression, but it must not hold the
  release. Blocking things that need not block delays v1 without making it safer.

F88b is nonetheless the clearest P2 case in this RFC. The user opens a deleted
file against its old version, sees both panes, and expects to save the left side
back — `cli.md` even advertises it — and instead the entire merge and save
toolbar disappears with no explanation. `inspect_save_target` already reports
that path as `Writable` with `MustBeAbsent`; the machinery is ready and only the
gate is wrong.

### D4 — A lossy encode blocks, names what it cannot write, and offers a way out

`encode_text` returns `encoding_rs`'s unmappable flag; `SaveOutcome` carries it;
the save path **refuses to write** and raises a dialog.

Blocking is P1. What the dialog *says* is P2, and it is the part worth
specifying, because "cannot encode" is meaningless to someone who has never
thought about charsets. The dialog must:

- **name the characters it cannot write**, not merely report that some exist —
  the user needs to find them;
- **say what the file's encoding is**, since that is the fact they are missing;
- **offer the two escapes that preserve their data**: save as UTF-8, which is an
  encoding change they are choosing deliberately, or go back and edit.

Substituting `&#128512;` is none of those things: it is a fourth option nobody
would pick, taken silently.

`RecoveryAction` has no "save as UTF-8" variant. Adding one is in scope; review
083 established that the twelve variants are matched exhaustively with no
catch-all, so a thirteenth is a compile error at every site — the addition is
safe by construction.

**Separately, and already produced:** `SaveOutcome.encoding_fallback_to_utf8` is
set today and read by nobody, though its own doc says the UI must warn. Consume
it.

**Out of scope, noted as the better long-term answer:** marking unmappable
characters in the editor as they are introduced, so the problem is visible before
Save. That is prevention (P3) and it is a larger change; it should not delay this
one.

### D5 — `atomic_replace` uses the primitive already in the file

`tempfile::Builder::new().permissions(0o666).tempfile_in(dir)` then `persist()`.
Random `O_EXCL` name; no symlink vector; no collision between concurrent saves.
**Do not write a second implementation** — reuse `persist_noclobber`'s shape,
including its permissions reasoning.

### D6 — Show the user where Save will write

`save_target` reaches the UI in exactly one place: prefilling the Save As dialog
(`diff/toolbar.rs`). **It is never displayed.** So a wrong destination looks
identical to a right one until the file is overwritten — which is why F85 could
exist, and stay silent, in a codebase that had already built RFC-077 to prevent
exactly that.

**Surface the resolved save target in the workspace** — the toolbar or the status
line — so the answer to *"where does Save go?"* is on screen rather than inferred.

This is P3, and it is the only item here that is not a defect fix. It earns its
place because it is the one change that makes the *class* visible: in mergetool
mode the destination is a third file the user never sees in either pane, and
today nothing tells them what it is.

Scope discipline: display only. No new control, no editing of the target from the
status line — Save As already exists for that.

## Acceptance criteria

1. `apply → save → undo → apply` reports **dirty**, and Save is enabled.
   Two-way and three-way.
2. After `swap_sides`, `save_target.path == right_path`, and a save writes to the
   file shown on the right.
3. Swap is refused in mergetool mode.
4. Saving a character unmappable in the target encoding **does not write** without
   confirmation, and never writes a numeric character reference silently.
5. A file that decoded with replacement characters cannot be saved without the
   guard.
6. A missing right side can be **created** by saving.
7. Pre-creating `.{name}.fsk-tmp` as a symlink cannot redirect a save.
8. Two concurrent saves of one target cannot corrupt it.
9. `file-types.md`'s save-guard sentence is true, or gone.

## Testing

Each check demonstrated **failing** against the defect it exists to catch, per
this project's standing requirement — and against the *shipped* defect, not a
helper the fix introduces (reviews 072 and 074 both turned on that distinction).

Criteria 1–2 and 4–6 are unit-testable against the real sessions and the real
`Store` (`with_test_store`). Criterion 7 is a filesystem test and is
`#[cfg(unix)]`; it must **skip loudly** rather than pass when run as root, the
hazard handoff 006 §8 recorded. Criterion 8 needs two writers and is the one
place a timing test is justified — if it cannot be made deterministic, say so
rather than implying coverage.

**The audit's own recommendation is adopted:** add `proptest` invariants for
encode∘decode byte-identity. Two of these defects are single-property
falsifiable, and the project has no property testing at all today.

## Sequencing

**Immediately, and in parallel with the M5/F44 wait.** These are correctness and
security defects in shipped code; they do not wait on Gate D, and Gate D's matrix
must be re-run against a candidate carrying them anyway.

Documentation-only corrections — `file-types.md`'s false save-guard sentence and
`threat-model.md`'s false fuzzing claim — land **first and separately**, because
a document asserting a control that does not exist is worse than no document, and
neither correction depends on any code change.

## Open questions — all closed 2026-09-01

- **Q1 — mergetool swap. CLOSED.** Not refused. **The RFC's own first answer was
  wrong** and is replaced by D2's invariant: `save_target` derives from
  `save_destination`, so re-deriving when its inputs change is correct in every
  mode and needs no special case. Refusing would have added a restriction to
  cover a missing invariant, and would have masked external-modification
  detection on `$MERGED` if applied as an unconditional re-derive.
- **Q2 — lossy encode. CLOSED: block, name, offer.** See D4. Blocking alone is
  safe; naming the characters and offering UTF-8 is what makes it usable by
  someone who has never thought about charsets.
- **Q3 — B5's standing. CLOSED: five blocking outcomes, not six.** F85, F86, F87,
  F88a and F89 block, each being data loss, corruption or a security defect, as
  do F92's two false **control** claims — documentation-only edits that land
  first. **F88b does not block.**

## What this RFC deliberately does not do

- **It does not mark unmappable characters as they are typed.** That is the
  prevention D4 gestures at and it is a larger change; it must not delay this one.
- **It does not add an encoding picker.** RFC-083. D4's dialog must work against
  whatever label the file already has.
- **It does not touch the diff engine, the concurrency design or the persistence
  layer.** The audit examined all three and found them sound; the concentration
  of defects in one subsystem is the finding, and widening scope would blur it.
