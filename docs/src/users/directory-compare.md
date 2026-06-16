# Comparing Directories

ForskScope compares two directory trees at two levels of depth:

- **Browse mode** (default) — shows the current directory on each side with equality indicators for same-name files.
- **Deep compare mode** — scans both trees recursively and produces a flat report of every differing file.

---

## Browse mode

When you launch ForskScope without arguments, the Explorer workspace opens with two side-by-side directory panes.

**Navigating:**

- Click a folder to open it.
- Click `↑` or press `Alt+↑` to go up one level.
- Use the `◀` / `▶` history buttons to go back and forward.
- Type a path in the path bar and press Enter.

**Status icons per file:**

| Icon | Meaning |
|------|---------|
| `✓`  | Same name on both sides; content is identical |
| `⚠`  | Same name on both sides; content differs |
| `←`  | File exists only on the left side |
| `→`  | File exists only on the right side |
| `⊙`  | Digest comparison is running |

**Opening a comparison:**

- **Double-click** a file that appears on both sides (✓ or ⚠) to open a file comparison.
- **Click** a file on one side, **click** a file on the other side, then press **Compare** to compare two differently-named files.

---

## Modes

Two mode buttons in the toolbar switch between views:

| Button | Purpose |
|--------|---------|
| **Browse** | Side-by-side directory panes with digest status icons |
| **Directory Report** | Recursive scan — flat report of every file across both trees |

---

## Copying files between sides

Every row in Directory Report that is not equal shows a copy button (`Copy →` or `← Copy`). Clicking opens a confirmation dialog showing the exact source and destination paths. If the destination already exists, a `.bak` backup is created before overwriting.

---

## Deep compare mode

Click **Directory Report** in the mode toolbar to switch to the recursive scan view.

ForskScope walks both directory trees in the background and builds a flat report. The scan runs in two phases:

1. **Fast listing** — identifies all files and their one-sided/common status immediately.
2. **Digest comparison** — computes file equality for same-name pairs one by one. The `checking N/total…` counter in the status line tracks progress.

**Result table columns:**

| Column | Content |
|--------|---------|
| Status icon | ⚠ changed, ← left-only, → right-only, ✓ equal, ⊙ computing |
| Path | Relative path from the root |
| Size | File size (or `old → new` if sizes differ) |
| Compare button | Opens a file comparison for differing files |

**Filtering results:** use the **Different / All / Equal** buttons (same as browse mode).

**Batch copy:** when differing files exist, **Copy N →** and **← Copy N** buttons appear in the toolbar. Clicking shows a confirmation with the count; all copies use the same `.bak` backup policy as single-file copy.

---

## Tips

- **Compare profiles** affect the file comparison that opens when you click Compare in the explorer. Set "Ignore whitespace" as the active profile if you're comparing generated files with inconsistent formatting.
- The last-used left and right directories are remembered across launches.
- The deep compare report resets when you click ⟳ Deep compare again with different directory paths.
