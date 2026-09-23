# Review Request: F32 — Compare-View Changed-Line Misalignment Fix

**Date:** 2026-08-04
**Reviewer stance:** verification of a release-blocking visual defect fix
**Repository baseline:** `cb6a852` (`fix: move sr-only diff label inside
.cell to fix WebKitGTK row shift (F32)`)
**Governing documents:** `rfcs/handoffs/074-v1-release-stabilization-program/f32-compare-view-alignment-handoff.md`;
RFC-024, RFC-061, RISK-002, RFC-078 P03

## Implementation Summary

Moved the `.sr-only` screen-reader-label `span` from being the first child
of each `.diff-row` (`display: table-row`) to being the first child of the
`.cell` div (a `display: table-cell`) instead, in both `RowLeft` and
`RowRight` (`crates/forskscope-ui/src/ui/view/hunk.rs`). Exactly the
mechanism the handoff named as the obvious fix — no CSS change was needed.

## Addressed Items

- **F32** — WebKitGTK no longer wraps the `.sr-only` span in an anonymous
  table cell, since it's no longer a direct child of the `table-row`. Each
  `.diff-row` now renders exactly three table cells (gutter, diff-mark,
  cell) regardless of hunk kind.
- The label is not deleted — moving it inside `.cell`, ahead of the line
  content, keeps it in the accessibility tree and keeps its announcement
  order (label, then content) intact, per RFC-024/G-007.

## Verification

### 1. Visual — both hunk shapes the handoff called out

Built and ran the real `forskscope` binary under an isolated `HOME`/
`XDG_CONFIG_HOME` (so the capture touches no real config or embeds no
personal path), screenshotted via
`niri msg action screenshot-window --show-pointer false`.

- **Replace/Insert hunks** (`.git-exclude/tmp/demo/{before,after}/src/config.rs`,
  the fixture the handoff pointed at) — `.git-exclude/tmp/shots/f32-fix-replace-insert.png`.
  Every changed row (`#[derive(...)]` replace, `ignore_hidden` insert,
  `max_depth` replace, the multi-line `validate` replace) begins at the
  same left offset as unchanged rows; `−`/`+` sit immediately after the
  line number; no content is clipped at the pane edge.
- **Pure Delete hunk** — the demo fixture only produces Replace/Insert
  hunks (diffing it confirms no isolated single-sided removal), so I built
  a second, minimal fixture (`.git-exclude/tmp/delete-test/{left,right}.txt`,
  a 4-line vs. 3-line file differing by one removed line) to exercise
  `HunkKind::Delete` specifically —
  `.git-exclude/tmp/shots/f32-fix-delete-kind.png`. Same result: `line two`
  (the deleted row) aligns with the unchanged rows around it.

Both screenshots are referenced here rather than committed to the repo
(they're demo/scratch content, not release artifacts); available at the
paths above if you want to view them directly.

### 2. Accessibility label survival — via the real AT-SPI bus, not just the DOM

The handoff asked for confirmation "not merely that it exists in the
source." I queried the running app's actual accessibility tree over the
AT-SPI session bus (`gi.repository.Atspi`, Python) — the same bus a screen
reader (Orca) reads from — rather than only inspecting the DOM. Confirmed
for all three label kinds:

```text
row 2:  '3 | Changed: #[derive(Debug, Clone)]'          (Replace)
row 14: '14 | Changed:             max_depth: 8,'        (Replace)
row 37: '8 | Inserted:     pub ignore_hidden: bool,'      (Insert)
row 46: '17 | Inserted:             ignore_hidden: true,' (Insert)
row 1:  '2 | Deleted: line two'                           (Delete, second fixture)
```

(Full output: `.git-exclude/tmp/shots/f32-atspi-verification.txt` — 14
matching rows out of 60 `table row` accessibles for the Replace/Insert
fixture — and `.git-exclude/tmp/shots/f32-atspi-verification-delete.txt` —
1 matching row out of 8 for the pure-Delete fixture.) Each row's accessible
text is `<gutter> | <label>: <content>` — the label is present, and
precedes the line content, exactly as required, for `Deleted`, `Changed`,
and `Inserted` alike.

### 3. `forskscope-core` untouched

`git diff --stat` for this commit shows exactly one file:
`crates/forskscope-ui/src/ui/view/hunk.rs`, +2/−2 lines.

## Files Changed

- `crates/forskscope-ui/src/ui/view/hunk.rs` — `RowLeft`/`RowRight`: moved
  the `.sr-only` span from being `.diff-row`'s first child to being
  `.cell`'s first child. No other line changed.

## Not Addressed Here (per the handoff's explicit scope)

- Other compare-view styling, the act column, or any other RISK-002
  surface not already confirmed as this specific defect.
- A screenshot step in release preflight (registered separately as F34).
- Anything RFC-076 patch 5 touched — confirmed disjoint by file, not by
  crate boundary, per the handoff's explicit warning that patch 5's rename
  reaches six `forskscope-ui` files. `hunk.rs` was not one of them.

## Tests And Gates Run

```text
cargo fmt --check                pass
cargo test --workspace            pass — 1007/1007, unchanged (no automated test catches this defect, per the handoff — reporting the count as evidence gates ran clean, not as evidence the fix works)
cargo clippy --workspace -D warnings   pass
cargo xtask css --check           pass
git diff --check                  pass
```

CI run `30864375956`: Test & Lint green, on `cb6a852`.

## Requested Review Focus

1. Confirm the fix mechanism itself — span inside `.cell`, ahead of
   content — is what you intended by "move the span inside the `.cell`
   div, ahead of the line content," versus the `aria-label`-on-row
   alternative the handoff also permitted.
2. Whether the demo/scratch fixtures and AT-SPI evidence files under
   `.git-exclude/tmp/` are sufficient as referenced evidence, or whether
   any of this warrants promotion into the repository (e.g. as a
   permanent visual-regression fixture) — the handoff registered that
   question separately as F34, so I left it there rather than deciding it
   here.
