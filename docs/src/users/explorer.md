# Explorer Panes

The Explorer workspace is the starting point when no files have been opened as
arguments. It shows two directory panes side by side: **Left / Old** (source)
and **Right / New** (target).

---

## Opening directories

**From the path bar:** type or paste a path and press **Enter**, or click the
folder icon to open a directory picker dialog.

**From the command line:** pass two directory paths as arguments:

```sh
forskscope /path/to/old-version/ /path/to/new-version/
```

---

## Navigating

Click a folder row to expand it and show its contents. Click again to collapse.

Press **Alt + ↑** or click the **↑** button in the path bar to go up one level in the **focused pane**.

The **◀** and **▶** history buttons step through your recent directory
navigation within each pane.

---

## Focused pane

The Explorer has a **focused pane** — the pane that receives keyboard input. The
focused pane is indicated by a highlighted root header cell with a `◀` marker.

- Press **F6** to switch keyboard focus between the left and right panes.
- Click a pane's root header cell to focus it with the mouse.

When the Explorer first opens the left pane is focused. Use F6 after navigating
the left pane to switch to the right pane and select a file there.

---

## Empty state

When both panes are at the same directory and no entries are visible, the
Explorer shows a **Compare files or folders** card with a brief orientation
hint and a `🔒 Nothing leaves this computer.` trust notice.

---

## Selecting files for comparison

Click a file in the left pane — it becomes the left candidate (highlighted
with a selection marker). Then click a file in the right pane to select it as
the right candidate. Click **Compare** to open a diff tab.

**Keyboard:**

| Key | Action |
|-----|--------|
| **F6** | Switch keyboard focus between left and right pane |
| ↑ / ↓ | Move focus within the active pane |
| **Space** | Select the focused file as a comparison candidate |
| **Enter** | Open a folder, or compare a same-name file if a matching file exists in the opposite pane |
| **Alt + ↑** | Go up one directory level (focused pane only) |

---

## Same-name file shortcut

If a file with the same name exists in both panes, double-click it (or press
**Enter** with it focused) to compare it immediately without needing to select
it on both sides.

---

## Digest equality indicators

When both panes are showing directories, ForskScope computes a digest for each
file that appears on both sides:

| Icon | Meaning |
|------|---------|
| ✓ | File is byte-for-byte identical on both sides |
| ⚠ | File exists on both sides but differs |
| *(no icon)* | File exists only on this side, or digest not yet computed |

Digest computation runs in the background. Large directories may take a moment
before all icons appear.

---

## Multiple tabs

Each comparison you open creates a new tab. Click any tab to switch to it.
Close a tab with the **✕** button; if the comparison has unsaved merge changes,
you will be asked to confirm.

The Explorer tab is always available and cannot be closed.
