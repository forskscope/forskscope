# Comparing Files

## Opening a comparison

**From the command line:**

```sh
forskscope old/version.rs new/version.rs
```

The app opens with the two files loaded side by side. The left pane shows the old/source file; the right pane shows the new/target file.

**From the Explorer:**

1. Navigate the left pane to the directory containing the old file.
2. Navigate the right pane to the directory containing the new file.
3. Double-click any file that appears on both sides to open a comparison instantly.  
   Or: click a file on the left, click a file on the right, then press **Compare**.

---

## Understanding the diff view

```
LEFT / OLD                    ↔    RIGHT / NEW
────────────────────────────────────────────────
  1  fn main() {                   fn main() {
- 2  │ old_code();              +   new_code();
  3  }                              }
```

| Symbol | Meaning |
|--------|---------|
| `−`    | Line deleted, or the left/old side of a replaced line |
| `+`    | Line inserted, or the right/new side of a replaced line |
| (blank) | Equal line |

There is no separate symbol for a replacement — a replaced line shows `−` on
the left and `+` on the right, the same marks a pure deletion or insertion
uses, with the pale amber background as the third cue that it is one change,
not two unrelated ones.

Colour-coding reinforces the symbols but is never the sole indicator — every change has a glyph, making the view readable in high-contrast mode.

---

## Navigating changes

| Action | Shortcut |
|--------|----------|
| Next change | **F8** or **▶** button |
| Previous change | **F7** or **◀** button |
| Jump to hunk on screen | Click the hunk or use ▲/▼ in the centre gutter |

The status bar shows the total count of insertions and deletions: `+12 / -5`.

---

## Search within the diff

Press **Ctrl+F** or click the 🔍 button to open the search bar. Type any substring; every matching row highlights in both panes simultaneously. The bar shows a live match count. **F3** / **Shift+F3** step through matches from elsewhere in the tab, but while focus is in the search box itself, use **Enter** / **Shift+Enter** instead — the box takes every key typed into it, F3 included. **Esc** closes the bar from either place.

---

## Compare options

**Ignore whitespace** — trailing spaces, tabs, and indentation differences are treated as equal. Toggle in the advanced toolbar (More ▼ → "Ignore whitespace: off/on") or via a compare profile.

**Ignore case** — uppercase/lowercase differences are treated as equal. Toggle with "Ignore case: off/on" in the same section.

**Diff algorithm** — Myers (default), Patience, or Histogram. Histogram often produces more readable diffs for structured code. Select from the dropdown in the advanced toolbar.

**Context lines** — sets how many equal lines are shown around each change before the rest collapses. Adjust in Settings → Advanced → "Context lines".

---

## Character-level diff

For replace hunks (a line changed rather than added or deleted), you can see exactly which characters changed. Click **Inline diff: off** in the advanced toolbar to turn inline highlighting on. A second click turns it off again.
