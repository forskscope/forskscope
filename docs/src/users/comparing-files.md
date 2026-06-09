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

**By dragging files onto the window:**

Drop two files onto the app window. The first file becomes the left (old) side; the second becomes the right (new) side.

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
| `−`    | Line deleted from the left side |
| `+`    | Line inserted on the right side |
| `~`    | Line replaced (shown on both sides) |
| (blank) | Equal line |

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

Press **Ctrl+F** or click the 🔍 button to open the search bar. Type any substring; every matching row highlights in both panes simultaneously. The bar shows a live match count. Press **Esc** to close.

---

## Compare options

**Ignore whitespace** — trailing spaces, tabs, and indentation differences are treated as equal. Toggle in the advanced toolbar (More ▼ → "Ignore WS: off/on") or via a compare profile.

**Ignore case** — uppercase/lowercase differences are treated as equal. Toggle with "Ignore case: off/on" in the same section.

**Diff algorithm** — Myers (default), Patience, or Histogram. Histogram often produces more readable diffs for structured code. Select from the dropdown in the advanced toolbar.

**Context lines** — sets how many equal lines are shown around each change before the rest collapses. Adjust in Settings → "Context lines".

---

## Character-level diff

For replace hunks (a line changed rather than added or deleted), you can see exactly which characters changed. Click **Inline: off** in the advanced toolbar to turn inline highlighting on. A second click turns it off again.
