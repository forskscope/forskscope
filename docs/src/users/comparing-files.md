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
| Next change | **F8** or the **▶ Next change** button |
| Previous change | **F7** or the **◀ Previous change** button |
| Jump to hunk on screen | Click the hunk or use ▲/▼ in the centre gutter |

The status bar shows the total count of insertions and deletions: `+12 / -5`.

---

## Search within the diff

Press **Ctrl+F** or click the **🔍 Search** button to open the search bar. Type any substring; every matching row highlights in both panes simultaneously. The bar shows a live match count. **F3** / **Shift+F3** step through matches from elsewhere in the tab, but while focus is in the search box itself, use **Enter** / **Shift+Enter** instead — the box takes every key typed into it, F3 included. **Esc** closes the bar from either place.

---

## Compare options

**Ignore whitespace** — trailing spaces, tabs, and indentation differences are treated as equal. Toggle in the advanced toolbar (More ▼ → "Ignore whitespace: off/on") or via a compare profile.

**Ignore case** — uppercase/lowercase differences are treated as equal. Toggle with "Ignore case: off/on" in the same section.

**Diff algorithm** — Myers (default), Patience, or Histogram. Histogram often produces more readable diffs for structured code. Select from the dropdown in the advanced toolbar.

**Context lines** — sets how many equal lines are shown around each change before the rest collapses. Adjust in Settings → Advanced → "Context lines".

---

## Character-level diff

For replace hunks (a line changed rather than added or deleted), you can see exactly which characters changed. Click **Inline diff: off** in the advanced toolbar to turn inline highlighting on. A second click turns it off again.

---

## Bidirectional control characters

Unicode has a handful of invisible characters that change the *direction* text is read in — left-to-right or right-to-left — without showing anything themselves. Normal right-to-left text (Hebrew, Arabic and others) doesn't need them: it reads in the correct order within its own line automatically. These characters show up on their own, usually when something — deliberately or not — has placed one in a file that otherwise has no reason to contain one. The "Trojan Source" technique (CVE-2021-42574) uses them to make source code *read* one way to a person and *run* another way to a compiler: a comment that looks like it ends early, or a string that looks like it ends sooner than it does.

ForskScope never lets one of these reach the screen as itself. Instead, each one shows as a small bordered label in place of the character — `RLO`, `LRI`, and so on — in the warning colour, with a full name (for example "right-to-left override character") as its accessible name and tooltip. The label is what gets copied if you select and copy that text, never the invisible character. A file containing one of these also gets a line in the status bar: "Contains *N* direction-control characters".

This changes nothing about what's *in* the file — saving, applying a change with **Use**, merging, and exporting a patch all work on the real, unmodified text. It only changes what you *see*, so a line can never display differently from what it actually contains.
