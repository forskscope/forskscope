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
navigation within each pane. **Alt + ←** and **Alt + →** do the same for the
**focused pane**.

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
| **Alt + ←** / **Alt + →** | Back / forward in the focused pane's directory history |

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

A **file** pair of the same size, 64 MiB or larger, is not read this way — the
row settles straight at `≈`, the same "sizes match, contents not compared"
state a rested-on folder pair can reach, without a `…` step. A pair whose
sizes differ still becomes `≠` immediately either way; only a same-size, large
pair is affected, and only because reading both files in full, automatically,
for every such pair a folder happens to contain, is not something browsing
should cost you. Compare the pair directly (or use a Directory Report) for a
byte-for-byte answer.

A same-named **folder** pair starts as `–`. Select the row and rest on it for a
moment (arrowing past it does nothing) and ForskScope compares the names and
sizes inside the two folders, without reading any file: if something is on one
side only, or a file's size differs, the row becomes `≠`. If nothing differs at
that level it becomes `≈` — the same names and sizes, contents not compared.
`≈` is deliberately not `=`: a change that keeps a file's size cannot be seen
this way. Moving to another row, or navigating, stops the check.

**Verify.** A row at `≈` — a folder pair, or a large file pair at or over the
64 MiB automatic-comparison size — shows a small **Verify** button next to its
icon. Clicking it reads every byte: a folder's Verify re-walks the pair's full
contents, and a file's reads the pair directly, bypassing the size limit for
that one explicit request. The row shows `…` while this runs and settles at
`=` or `≠` once it finishes — this is the only way either row kind reaches `=`
from a `≈` state, and `≈` never becomes `=` on its own. Several rows can be
verified at once; they complete in turn rather than all at once. Navigating
away, or selecting elsewhere while a folder's rest-triggered `≈` check is still
pending, cancels whatever is in flight, including a running Verify.

<!-- status-glyphs:begin -->
| Icon | Meaning |
|------|---------|
| `=` | **Equal** — same name on both sides, with identical content |
| `≠` | **Different** — the content differs (in the Explorer, also when one side is a file and the other a folder, or a same-named folder pair has a subdirectory only one side has) |
| `…` | **Computing** — the comparison is still running |
| `⊘` | **Unreadable** — something could not be read, so nothing was compared. This is not a verdict: the two sides may or may not match |
| `←` | **Left only** — present only on the left |
| `→` | **Right only** — present only on the right |
| `–` | **Not compared** — a same-named folder pair that has not been examined. Select the row and rest on it to compare names and sizes; use a Directory Report to compare contents (Explorer only) |
| `≈` | **Metadata matches; contents not compared** — the two folders have the same names and sizes throughout (a file pair: the same size), and no file was read. This is not "identical": an edit that keeps a file's size goes unnoticed (Explorer only) |
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

## Sync panes

Click the **⇄** toggle (next to **⊞**, always visible) to make both panes
follow each other. Turning it on anchors each pane to its directory at that
moment. While sync is on, navigating one pane — double-clicking a folder,
the path bar's **↑**/breadcrumbs, typing a path, **⌂** — moves the other
pane to the same path relative to *its own* anchor: entering `a/b` below
where the first pane started opens `a/b` below where the second pane
started, and going up moves the other pane to the matching point on its
own side, however far either pane has wandered since.

If the other pane has no folder at that relative path, it stays exactly
where it is and sync stays on — this is expected when the two trees
genuinely differ at that point, not a malfunction. Because each move is
measured from the anchors, not from wherever the panes currently sit, a
later navigation to a path that exists relative to both anchors brings them
back in step — there is no lingering gap to carry forward.

Navigating a pane above its own anchor is treated the same way: the other
pane is left alone. Turn sync off and back on to re-anchor both panes to
wherever they are then.

Sync is not persisted — it resets to off when you restart the app, the same
as the filter bar's open state.

---

## Layouts

The Explorer has two layouts. They differ in one thing only: which entries share
a row.

- **Compact** is the default for new installs. Each pane packs its own entries,
  and row *n* shows each pane's *n*-th entry. The two entries in a row are not the
  same entry, and nothing claims they are. When one pane has more entries, the
  other's column ends early.
- **Aligned** puts same-name entries on the same row, with spacer rows where one
  side is missing.

Both layouts scroll together, in one list, and both take the same keys: arrows and
Enter act on the focused pane, and F6 moves focus between panes. Double-clicking a
file compares it with the file picked on the other side, or, if nothing is picked
there, with the same-named file on the other side.

Existing installs keep the layout they have. Only new installs, and settings files
that do not record a layout, start in Compact. To change it, use
**Settings → Advanced → Explorer layout**.

On a narrow window (under 720 px), each row's two halves stack and the spacer rows
are hidden, so the list reads as one column of entries.

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
