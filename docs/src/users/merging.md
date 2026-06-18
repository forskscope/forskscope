# Merging Files

Merging means taking changes from the **left (source)** side and applying them to the **right (result)** side. This is useful when you want to accept specific changes from an older version into your current file, or when resolving a two-way difference.

> For three-way conflict resolution (base/ours/theirs), see [Git Integration](../intermediate/git-integration.md).

---

## The merge model

ForskScope keeps a **result buffer** separate from the rendered diff. What you see in the right pane is always the current state of that buffer. Applying a hunk writes to the buffer; undoing reverts it. The file on disk is only changed when you explicitly **Save**.

This means it is safe to experiment: apply hunks, undo them, reapply, and only commit to disk when satisfied.

---

## Applying a change

Every changed hunk (Replace, Delete, Insert) shows a **▶ Use** button in the centre gutter.

Clicking it copies the left-side content into the right-side result buffer for that hunk. The hunk turns grey with a ✓ to indicate it has been applied.

**Keyboard:** Press **Enter** to apply the focused (highlighted) hunk and automatically advance to the next one. Combined with **F8** to navigate, a full merge can be done without touching the mouse:

1. Press **F8** to jump to the first change.
2. Press **Enter** to apply it.
3. Repeat until done.
4. Press **Ctrl+S** to save.

---

## Undoing and redoing

| Action | Shortcut |
|--------|----------|
| Undo last apply | **Ctrl+Z** or the **Undo** button |
| Redo (re-apply undone) | **Ctrl+Y** or the **Redo** button (in More ▼) |

Undo restores the result buffer exactly to its state before the last apply. Redo re-applies it. The undo/redo stack persists for the entire session until you close the tab or reload.

---

## Dirty state

When the result buffer differs from the original right-side file, the tab is marked **dirty**: a `●` appears before the tab title and the status bar shows *unsaved*. The **Save** button becomes active.

Closing a dirty tab shows a confirmation dialog. Reloading asks before discarding changes.

---

## Saving the result

Click **Save** (or **Ctrl+S**) to write the result buffer to the right-side file path.

If the file on disk was modified by another process since it was loaded, ForskScope detects the conflict and shows an **overwrite confirmation** rather than silently replacing external changes.

**Save As** writes the result to a different path. The tab then points to the new path for future saves.

Before overwriting, ForskScope creates a `.bak` sibling backup if the destination already exists.

---

## Merge and the compare profile

When you open a comparison, the **active compare profile** determines the initial diff options. If "Ignore whitespace" is the active profile, whitespace-only changes are hidden from the start and will not appear as hunks to apply. Choose "Exact (default)" when you need to see and act on every character.

---

## Full workflow example

```
1.  forskscope feature-v1.rs feature-v2.rs
    # Opens: left = v1 (old), right = v2 (new/result)

2.  F8 → see first change
3.  Enter → accept it (copies v1 version into result)
4.  F8 → see second change
5.  (skip — keep the v2 version here, don't press Enter)
6.  F8 → see third change
7.  Enter → accept it

8.  Ctrl+S → saved to feature-v2.rs
```

The result file now has the first and third changes reverted to v1, with the second change left as v2.
