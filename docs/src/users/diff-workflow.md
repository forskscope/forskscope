# Basic Diff Workflow

The diff workspace shows your two files side by side. **Left / Old** is the
source or earlier version; **Right / New** is the target or later version.

Changed lines are highlighted:

| Colour | Meaning |
|--------|---------|
| Red background | Deleted — exists only on the left |
| Green background | Inserted — exists only on the right |
| Orange/amber background | Replaced — present on both sides but different |

Non-colour indicators (for accessibility): a `−` mark in the left gutter for
deleted/replaced lines, a `+` mark in the right gutter for inserted/replaced
lines.

---

## Navigating changes

The position indicator in the toolbar (e.g. **2 / 5**) shows which change you
are focused on out of how many. Use the **◀** and **▶** buttons, or press **F7**
and **F8**, to step through changes in order without scrolling manually.

When a change is focused it gets a highlight outline. Both panes scroll together
to keep the focused change visible.

---

## Scrolling

Each pane has its own horizontal scroll bar, so a long line in the left file
does not force the right pane to scroll unnecessarily. Vertical scrolling keeps
both panes row-aligned — use the window scroll or your mouse wheel.

---

## Inline character diff

To see exactly which characters within a line changed:

1. Click **More ▼** in the toolbar.
2. Toggle **Inline diff: on**.

Changed characters inside replaced lines become highlighted at the character
level. Toggle **Inline diff: off** to return to the plain line view.

Inline diff is only available for **Replace** hunks (lines that exist on both
sides but differ). Pure insertions and deletions are shown as whole lines.

---

## Collapsing equal context

Long stretches of identical lines collapse automatically. A divider like:

```
··· 42 unchanged lines — click to expand ···
```

marks a collapsed region. Click the divider to expand it.

The number of context lines shown above and below each change is set in
[Settings → Context lines](settings.md#context-lines).

---

## Reading the gutter marks

| Mark | Location | Meaning |
|------|----------|---------|
| **▶ Use** | Centre action column | This hunk can be applied (merged left → right) |
| **✓** | Centre action column | This hunk has been applied in this session |
| **−** | Left gutter | Line deleted or replaced on this side |
| **+** | Right gutter | Line inserted or replaced on this side |

---

## Diff options

The **More ▼** section of the toolbar exposes per-tab diff options:

| Option | What it does |
|--------|-------------|
| **Inline diff** | Toggle character-level highlighting within replaced lines |
| **Wrap** | Wrap long lines in each pane instead of scrolling horizontally |
| **Ignore WS** | Treat whitespace-only differences as equal |
| **Ignore case** | Treat uppercase/lowercase differences as equal |
| **Algorithm** | Myers (default), Patience, or Histogram — affects how changes are grouped |

These options apply to the current tab only. To make them the default for new
comparisons, create a [Compare Profile](settings.md#compare-profiles) in
Settings.
