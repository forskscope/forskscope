# Features

## Core workflow

- **Side-by-side two-pane comparison** — Left / Old and Right / New panes with
  a fixed divider. Long lines scroll horizontally within the shared view;
  a single scroll bar scrolls both panes together.
- **Line-level diff** — insertions (green), deletions (red), and replacements
  (amber) with colour-independent `+`/`−`/`~` gutter marks.
- **Hunk navigation** — ◀ ▶ buttons (F7/F8) jump between changes; a position
  indicator shows `current / total`.
- **Inline character diff** — for replaced lines, toggle character-level
  highlighting to see exactly what changed within the line.
- **Context collapse** — long runs of equal lines collapse automatically;
  click the `···` divider to expand.
- **Selective merge** — apply individual hunks left → right; skip the ones
  you do not want.
- **Undo / redo** — every merge action is reversible; Ctrl+Z / Ctrl+Y.
- **Safe save** — external-modification detection before writing; automatic
  `.bak` backup; atomic write.

---

## Explorer

- Two directory panes with lazy-loading tree views.
- Path bar, directory picker dialog, history ◀▶, and Alt+↑ to go up.
- Digest equality icons: **✓** identical, **⚠** different, *(none)* one-sided.
- Full keyboard navigation: arrows to move, Space to select, Enter to
  open/compare, **F6** to switch focused pane.
- **Alt+↑** navigates the focused pane up one level.
- Same-name file shortcut: double-click to compare immediately.
- **Empty state** shown when no entries are visible, with a first-run orientation hint.
- Multiple comparisons open as independent tabs; switch freely.

---

## Directory compare (Deep compare)

- **Directory Report** mode recursively scans both directory trees.
- Two-phase scan: fast listing first, then background digest comparison.
- Status per file: ⚠ changed, ← left-only, → right-only, ✓ equal, ⊙ scanning.
- Filter results: **Different** / **All** / **Equal**.
- **Per-file copy** — explicit `Copy to right` / `Copy to left` buttons; confirmation shows full source and destination paths; `.bak` backup created when destination exists.
- **Batch copy** — explicit direction buttons (`Copy to right N` / `Copy to left N`); confirmation and result summary; restore manifest written to `$XDG_DATA_DIR/forskscope/manifests/`.

---

## Diff options (per-tab)

Accessed via **More ▼** in the toolbar:

| Option | What it does |
|--------|-------------|
| Inline diff | Character-level highlighting in replaced lines |
| Wrap | Wrap long lines instead of scrolling horizontally |
| Ignore WS | Treat whitespace-only differences as equal |
| Ignore case | Treat case differences as equal |
| Algorithm | Myers, Patience, or Histogram |

---

## Compare profiles

Store a named combination of diff options as a profile. Built-in profiles:
**Exact (default)** (Myers), **Ignore whitespace**, **Ignore case**,
**Histogram**. Add custom profiles in Settings.

---

## File types

| Type | Diff | Merge / Save |
|------|------|--------------|
| Text (any encoding) | Line + inline | ✓ |
| Excel `.xlsx` | Derived text (read-only) | — |
| Binary | Hex preview | — |
| Missing (one side) | One-sided diff | — |

Encoding is detected automatically (`chardetng`/`encoding_rs`). Save preserves
the original encoding. UTF-8 BOM is round-tripped.

---

## Patch export

Export a unified-diff patch from a file or directory comparison. The patch is
compatible with `patch -p1` and `git apply`.
See [Patch export](../intermediate/patch-export.md).

---

## Three-way merge (git mergetool)

Use ForskScope as a `git mergetool` to resolve conflicts:

```sh
git config merge.tool forskscope
git config mergetool.forskscope.cmd 'forskscope "$LOCAL" "$REMOTE" --merged "$MERGED"'
```

See [Git integration](../intermediate/git-integration.md) for full setup.

---

## Appearance and localisation

- Three built-in themes: **Dark** (default), **Light**, **Night**.
- Configurable diff font size (8–32 pt).
- **English** and **日本語** (Japanese) interfaces — all labels, dialogs,
  and status messages are translated.

---

## Privacy

No accounts, no telemetry, no cloud upload, no network access. Everything stays
on your machine.
