# Explorer Panes

The Explorer workspace is the starting point when no files have been opened as
arguments. It shows two directory panes side by side: **Left / Old** (source)
and **Right / New** (target).

---

## Opening directories

**From the path bar:** type or paste a path and press **Enter**, or click the
folder icon to open a directory picker dialog.

**From the command line:** you cannot. Two arguments always mean two *files*:
`forskscope <dirA> <dirB>` does not open the Explorer, and opens a tab that
fails with "Could not open … not a regular file". Start ForskScope with no
arguments and choose the folders here.

---

## Navigating

Click a folder row to expand it and show its contents. Click again to collapse.

Press **Alt + ↑** or click the **↑** button in the path bar to go up one level in the **focused pane**.

The **◀** and **▶** history buttons step through your recent directory
navigation within each pane.

---

## Focused pane

The Explorer has a **focused pane** — the pane that receives keyboard input. The
focused pane is indicated by a highlighted root header cell (accent outline and
tinted background).

- Press **F6** to switch keyboard focus between the left and right panes.
- Click a pane's root header cell to focus it with the mouse.

When the Explorer first opens, the left pane is focused. Use F6 after navigating
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

## Status icons

Every row that has a counterpart on the other side, or none, carries a
one-character status icon. When both panes show directories, ForskScope
computes a digest for each same-named file in the background, so an icon may
first read `…` and then change.

<!-- status-glyphs:begin -->
| Icon | Meaning |
|------|---------|
| `=` | **Equal** — same name on both sides, with identical content |
| `≠` | **Different** — the content differs (in the Explorer, also when one side is a file and the other a folder) |
| `…` | **Computing** — the comparison is still running |
| `⊘` | **Unreadable** — something could not be read, so nothing was compared. This is not a verdict: the two sides may or may not match |
| `←` | **Left only** — present only on the left |
| `→` | **Right only** — present only on the right |
| `–` | **Not compared** — a same-named folder pair. The Explorer never looks inside folders; use a Directory Report to compare their contents (Explorer only) |
| `↗` | **Symlink**, not followed — a symbolic link is listed, and its target is not compared (Directory Report only) |
<!-- status-glyphs:end -->

Hover an icon for its name. Files detected as binary show a `bin` badge instead
while binary comparison is off (see below). Large directories may take a moment
before every icon has settled.

---

## Binary files

Files detected as binary show a `bin` badge and cannot be compared by default.
To enable binary comparison, go to **Settings → Advanced → Enable binary
comparison**.

When binary comparison is enabled, binary files are actionable and open a
hex-dump comparison. The comparison runs in the background — the tab opens
immediately and shows a spinner while loading.

---

## Filter bar

Click the **⊞** toggle (between the path bars and the pane headers) to reveal
the filter bar.

| Control | Effect |
|---|---|
| **Name input** | Narrows visible rows live — case-insensitive substring match. A pair is shown if either side's filename matches. |
| **Hide binary** checkbox | Hides rows where all present file sides are binary (only effective when binary comparison is off). |
| **Hide identical** checkbox | Hides rows whose status is `=` (equal). |
| **✕ Clear** | Resets all filters in one click (appears when any filter is active). |

Filter state is not persisted — it resets when you restart the app.

---

## Compact layout

By default, same-name entries are aligned across panes with spacer rows for
entries that exist on only one side. Switch to **Compact** layout in
**Settings → Advanced → Explorer layout** to remove the spacer rows and let each
pane pack its entries independently.

In compact mode, vertical scrolling is independent per pane. Same-name equality
badges still appear but cross-pane row alignment is not guaranteed.

---

## Targets label and Compare button

The footer always shows a **targets label** describing what the Compare action
will open:

| State | Label |
|---|---|
| No picks | *Choose a file or folder on each side to compare* |
| Left pick only | `filename ↔ Choose a file or folder on the right` |
| Right pick only | `Choose a file or folder on the left ↔ filename` |
| Both picks ready | `left-name ↔ right-name` |

The **Compare ▶** button is enabled only when both picks are set.

---

## Async tab opening

When you open a comparison, the tab appears immediately with a ⟳ spinner in
the tab title and a loading message in the workspace. File loading and diff
computation run in the background. You can switch to other tabs while a
comparison loads, or close the loading tab to cancel.

---

## Multiple tabs

Each comparison you open creates a new tab. Click any tab to switch to it.
Close a tab with the **✕** button; if the comparison has unsaved merge changes,
you will be asked to confirm.

The Explorer tab is always available and cannot be closed.
