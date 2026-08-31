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

## Design

### D1 — Dirty state is content identity, not stack depth

Record what was saved, not how deep the stack was. **Decision: hash
`result_text()` at `mark_saved()` and compare on `is_dirty()`.**

A monotonic revision counter is the obvious alternative and is **rejected**: it
reports *dirty* after undoing back to the saved state. That is the safe
direction, so it would be acceptable — but it makes the dirty dot lie in the
other direction, and this project has spent a program's effort on markers that
claim more than they measure. Content identity is exact.

Both `MergeSession` and `ThreeWayMergeSession` get the same treatment. The
project's own `TransactionLog` already compares revisions (`merge/log.rs:235`);
this RFC does not adopt that shape, for the reason above.

### D2 — One source of truth for the save target

**`save_target` is authoritative and every mutation of `right_path` must
maintain it.** RFC-077 established the first half; this RFC closes the second.

- `swap_sides` re-derives `tab.save_target` from the new right side.
- **In `CompareLaunchMode::MergeTool`, swap is refused outright** — the save
  destination is a third path (`$MERGED`) that swapping cannot express, and
  silently keeping it while the panes swap is worse than declining.
- A regression test asserts the invariant after **every** mutation of
  `right_path`, not just after swap.

### D3 — One source of truth for save capability

Replace the pair-wide `can_save` expression with a single `save_capability()`
derived from **`EditabilityClass` of the editable side** and the target's
`SaveTargetState`. That is one change closing three defects: the unguarded lossy
save, the missing-side restriction, and one of the two drifts that produced D2's
defect.

### D4 — A lossy encode must not be silent

`encode_text` returns `encoding_rs`'s unmappable flag; `SaveOutcome` carries it;
the save path **blocks with a confirmation** before writing. `AppErrorKind` and
`RecoveryAction` already carry the taxonomy for that dialog, and F52 has just
wired the dialog itself.

Also required, and separate: **consume `SaveOutcome.encoding_fallback_to_utf8`**,
which is produced today and read by nobody, though its own doc says the UI must
warn.

### D5 — `atomic_replace` uses the primitive already in the file

`tempfile::Builder::new().permissions(0o666).tempfile_in(dir)` then `persist()`.
Random `O_EXCL` name; no symlink vector; no collision between concurrent saves.
**Do not write a second implementation** — reuse `persist_noclobber`'s shape,
including its permissions reasoning.

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

## Open questions for the owner

- **Q1 — mergetool swap.** Refusing the swap is this RFC's recommendation.
  The alternative is to allow it and re-derive `$MERGED` — but nothing in the
  mergetool contract says the merged path follows the panes, so refusing is the
  honest reading. Confirm.
- **Q2 — lossy-encode default.** Block with confirmation (recommended), or warn
  after writing? Confirmation is the only option that keeps the file intact.
- **Q3 — B5's standing.** This RFC assumes B5 joins B4 as a release blocker, and
  that the release-blocking outcomes list gains a write-path outcome. Confirm.
