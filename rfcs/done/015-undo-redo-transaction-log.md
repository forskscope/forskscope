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
4. Recomputing diff after an edit must not erase undo history.
5. Save marks a clean baseline revision but does not erase history automatically.

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
