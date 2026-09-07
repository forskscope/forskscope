# RFC-015 — Undo/Redo Transaction Log and Merge Operation History

**Status.** Implemented (v0.47.0) — transaction log model; history panel UI and crash recovery open

## Status
Implemented (v0.47.0). The `TransactionLog` companion type and supporting
types ship in `forskscope-core::merge`:

- **`TransactionKind`** — typed enum covering all current merge operations:
  `ApplyHunkLeftToRight`, `RevertHunk`, `ApplyAllLeftToRight`, conflict
  resolution variants (Left/Right/Both/Manual/Ignore/Reopen), plus
  `ManualTextEdit` and `ApplyExternalPatch` for future paths. Each variant
  carries its `HunkId` or `ConflictId` for hunk-level navigation.
  `kind.label()` produces a human-readable English description.
- **`SessionRevision`** — a typed monotonic revision counter replacing the
  raw `usize` offset. `INITIAL` is revision 0; each `push()` call
  increments by one. Revisions are `Ord` so dirty-state is `current > saved`.
- **`TransactionEntry`** — one log record: revision, kind, label, timestamp
  (`UnixTimestamp`), and an `active` flag (false when the entry has been
  undone) so the history panel can show the full session history with
  greyed-out undone entries.
- **`TransactionLog`** — a companion struct (attach to either session type):
  `push(kind)` records an operation; `record_undo()` / `record_redo()` sync
  with the session stack; `mark_saved()` sets the clean baseline;
  `is_dirty()`, `can_undo()`, `can_redo()`, `active_entries()`,
  `undone_entries()`, `active_ops_since_save()`. New push after undo
  discards the redo branch correctly (RFC-015 §8 rule 1).
- **23 tests** covering all RFC-015 §13 requirements.

Remaining open: the history panel UI (RFC-015 §10), persistent crash-recovery
journal (deferred in §4), and editor-local vs core undo precedence (RFC-015
§9, depends on RFC-004 editor adapter).

The current application has preliminary merge history behavior around diff indices. The migration must replace this with a canonical transaction log owned by the core model.

## 2. Motivation

Users must be able to trust merge operations. If copying a hunk or editing a result cannot be undone predictably, the app is dangerous for real work.

The transaction log is also necessary for:

- deterministic merge state;
- dirty state calculation;
- redo after undo;
- replay tests;
- save preflight;
- future crash recovery;
- editor/core synchronization.

## 3. Goals

- Define transaction types for merge and text edit operations.
- Provide undo and redo semantics.
- Separate editor undo from core transaction undo.
- Allow transaction replay in tests.
- Support dirty state calculation from transaction history.

## 4. Non-Goals

- This RFC does not require collaborative operation transforms.
- This RFC does not define persistent crash recovery journal in v1.
- This RFC does not implement semantic language-aware refactoring undo.

## 5. Transaction Model

```rust
pub struct TransactionLog {
    pub base_revision: SessionRevision,
    pub current_revision: SessionRevision,
    pub undo_stack: Vec<MergeTransaction>,
    pub redo_stack: Vec<MergeTransaction>,
}

pub struct MergeTransaction {
    pub transaction_id: TransactionId,
    pub timestamp: Timestamp,
    pub command: CommandId,
    pub before: TransactionSnapshot,
    pub after: TransactionSnapshot,
    pub affected_hunks: Vec<HunkId>,
    pub user_visible_label: String,
}
```

## 6. Transaction Kinds

```rust
pub enum TransactionKind {
    CopyHunkLeftToRight,
    CopyHunkRightToLeft,
    CopyAllLeftToRight,
    CopyAllRightToLeft,
    ManualTextEdit,
    RecomputeDiffAfterEdit,
    MarkResolved,
    RevertHunk,
    ApplyExternalPatch,
}
```

## 7. Snapshot Granularity

The first implementation should not snapshot the entire file for every small edit if that would be too expensive. It should support patch-based snapshots.

```rust
pub enum TransactionSnapshot {
    FullText { text: String, revision: TextRevision },
    TextPatch { inverse: TextPatch, forward: TextPatch },
    HunkState { before: HunkStateMap, after: HunkStateMap },
}
```

The implementation may begin with full snapshots for small files and move to patches when large-file mode is implemented.

## 8. Undo/Redo Semantics

Rules:

1. Applying a transaction pushes it to `undo_stack` and clears `redo_stack`.
2. Undo applies the inverse operation and moves the transaction to `redo_stack`.
3. Redo reapplies the operation and moves it back to `undo_stack`.
4. Recomputing the diff must not erase undo history **when the recomputed
   hunks are identical to the ones the history references**. When they are
   not, the history is discarded and the user is told before it happens.
   **Met** as of RFC-086 (`77666ec`, 2026-09-07). *Amended by RFC-086 §5 — see
   the amendment note below; the original wording and why it could not be kept
   are preserved there rather than edited away.*
5. Save marks a clean baseline revision but does not erase history automatically.

**F40 (2026-08-08).** Rule 4 is unmet in the shipped implementation.
`recompute_diff` (`forskscope-ui::state::tab`) always rebuilds
`MergeSession::from_diff` from the two documents, which discards every
applied merge and the entire undo/redo stack — it does not reapply history
against the new hunk set. This is a real gap, not a documentation-only
inaccuracy: hunk identity (`HunkId`) is derived in part from `DiffId`, a
process-global counter incremented on every `compute_diff` call
(`forskscope-core::diff::engine`), so a hunk's ID is never stable across a
recompute — not even between two recomputes with identical content and
options. Reapplying transactions against the new hunk set (§12's original
intent: "history remains valid but hunk navigation may show it as stale")
would require a rebasing rule for transactions whose hunk no longer exists
under the new, unrelated IDs.

Given that, the two call sites that trigger a recompute while merge state
may be dirty (`swap_sides`, and — as of F40's fix — `change_diff_options` for
the ignore-whitespace/ignore-case/algorithm toolbar controls) both **ask
first** rather than silently discarding: a dirty tab defers to a confirm
dialog (`Modal::ConfirmSwap` / `Modal::ConfirmDiffOptionChange`) naming what
will be lost, and only recomputes if the user confirms. `is_dirty()` never
silently becomes `false` while work is discarded out from under the user —
but the work itself is still discarded, once confirmed. Preserve-and-reapply
(rule 4 as originally written) remains a follow-up, not implemented here.

**Amendment (RFC-086 §5, 2026-09-07). Rule 4 originally read: *"Recomputing
diff after an edit must not erase undo history."* It stood recorded **Not met**
for thirteen months — from F40 (2026-08-08) until this amendment — and it was
never going to be met, because it cannot be met safely.**

The cause was one input. `hunk_id_for` hashed a **process-global counter**
alongside the hunk's own content and position, so every recompute changed every
id and `MergeSession::swap_in` could no longer find the hunk a transaction
referenced. RFC-086 removed the counter — it had no other consumer — and a
recompute over unchanged content now preserves every id, so `change_diff_options`
keeps the undo stack instead of discarding it after a prompt.

**What remains impossible, and why the original wording is withdrawn rather than
scheduled:** after a genuine *edit*, ranges shift from the applied hunk onward,
so those hunks are legitimately different and the history refers to hunks that no
longer exist. The only way to "preserve" it is to rebase stored rows onto
whichever new hunk looks closest — applying a user's merge to a place they did
not choose, silently. That is the F73/F85 defect class, and this project fails
closed rather than guesses.

So the rule now promises what the design can actually deliver. **A rule kept on
the books as *Not met* against an outcome nobody intends to pursue is worse than
no rule**: it reads as scheduled work, and it hid a one-line defect for over a
year.

## 9. Editor Undo vs Core Undo

This is a critical boundary.

If CodeMirror is used, it has its own editor history. ForskScope must decide which history owns user-visible undo.

Recommended policy:

```text
Core command undo owns merge operations.
Editor local undo owns active text typing inside the editor.
On editor transaction commit, core receives structured text edit events.
Global Undo dispatches to editor if editor focus is inside editable text.
Global Undo dispatches to core if focus is outside editor or command was merge-level.
```

The command registry must make this visible and testable.

## 10. Merge Operation History UI

```text
+--------------------------------------------------------------+
| History                                                      |
+--------------------------------------------------------------+
| 10:42  Copy hunk #4 left → right                             |
| 10:43  Manual edit in right pane                             |
| 10:44  Mark hunk #4 resolved                                 |
|                                                              |
| [Undo] [Redo]                                                |
+--------------------------------------------------------------+
```

This panel may be hidden by default but should exist as a diagnostics/developer feature early.

## 11. Dirty State

```rust
pub struct DirtyState {
    pub clean_revision: SessionRevision,
    pub current_revision: SessionRevision,
    pub has_unsaved_changes: bool,
    pub changed_sides: ChangedSides,
}
```

Dirty state must be derived from revisions and transactions, not from UI assumptions.

## 12. Conflict With Recomputed Hunks

Manual text edits can invalidate hunk IDs. The core must preserve history even when current hunks are recomputed.

Rules:

- Transactions refer to stable text ranges and prior hunk IDs where available.
- If a hunk no longer exists after edit, history remains valid but hunk navigation may show it as stale.
- Undo of a stale hunk operation must apply the stored inverse patch, not search for the old hunk by index.

## 13. Testing Requirements

- Copy one hunk and undo.
- Copy one hunk, undo, redo.
- Copy hunk, manual edit, undo edit, undo hunk.
- Manual edit invalidates hunk; undo still works.
- Save after transaction marks clean baseline.
- Close dirty tab detection uses dirty state.
- Large file snapshot policy does not exceed configured limits.

## 14. Acceptance Criteria

- Merge commands are undoable.
- Manual edits can participate in dirty-state calculation.
- Undo/redo behavior is deterministic and tested.
- Hunk identity changes do not break undo safety.
- Editor-local undo and app-level undo have explicit precedence.

## 15. Risks

| Risk | Severity | Mitigation |
|---|---:|---|
| Two undo systems conflict | High | Focus-based precedence and command registry |
| Full snapshots consume memory | Medium | Patch snapshots and large-file thresholds |
| Hunk index undo applies wrong range | Critical | Stable IDs and inverse patches |
| Save resets too much history | Medium | Clean baseline separate from history |

## 16. Open Questions

- Should v1 expose a visible history panel or keep it diagnostic-only?
- Should undo history persist across app restart?
- Should merge operations and manual edits share a single visible history list?
