# Handoff 097 — RFC-097 stage 3: visible controls

**From:** architect. **Date:** 2026-11-08. **Release:** `0.198.0`.
**RFC:** `rfcs/accepted/097-readable-for-everyone.md`, §Stage 3, as amended
2026-11-08: **revealed by state, never by pointer.** The owner accepted it
the same day, with the release scope. Read the amendment first. It
reconciles stage 3 with the owner's F162 rule, and every item below follows
from it. **Start now.** Handoff 098 (stage 2) comes after this one is
approved.

## Do

### 1. Words beside the unclear symbols

- **In the comparison toolbar** (`ui/view/diff/toolbar.rs`):
  - ◀ and ▶ read *Previous change* and *Next change*, with the symbol kept;
  - 🔍 reads *Search*;
  - ↺ (*Reload*) stays a symbol and keeps its accessible name.
- **In the tab bar** (`ui/layout/tabs.rs`), "?" reads ***Shortcuts***. Its
  existing name "Keyboard shortcuts (Ctrl+/)" stays as the accessible name.
- **Japanese** goes through the real i18n table. Reuse existing entries
  where the meaning matches. Otherwise use "前の変更", "次の変更", "検索"
  and "ショートカット", unless the table's established style differs, in
  which case say so.
- **At 200 % in 1000×700,** the toolbar may wrap, as it already does. A word
  may fold into its symbol only at a size where it can't fit, and never at
  100 %. Propose the layout with captures at 100 % and 200 %.

### 2. A key to the Explorer's symbols, naming only those on screen

The nine symbols are in `forskscope-ui-logic/src/explore/status.rs`
(`StatusGlyph::glyph`/`aria_label`):

| Symbol | Meaning |
|--------|---------|
| = | identical |
| ≠ | different |
| ← | left only |
| → | right only |
| … | comparing |
| ⊘ | unreadable |
| – | not compared |
| ↗ | symbolic link |
| ≈ | same name and size, contents not read |

- **The key lists the symbols present in the two panes' visible rows,** and
  updates as they change. Use the same wording as each symbol's existing
  screen-reader label, so the words never drift apart.
- **Its place:** propose one with captures. The Explorer's bottom bar beside
  *Compare* is the obvious candidate. Wherever it goes:
  - it is visible without hovering;
  - it is one line at 100 %;
  - it never pushes *Compare* out of reach at 200 %.
- **Accessibility:** it's a plain, readable line, and the rows' own
  `role="img"` labels stay. **Don't make it a live region:** announcing it
  on every change would be noise.

### 3. Revealed by state, never by pointer

- **The tab ×:** on every tab, at full strength. Remove `.tab-close`'s
  opacity rules and its three `OPACITY_ALLOWLIST` entries in `contrast.rs`.
  `contrast` must then pass without them.
- **Deep Compare row selection** (`ui/view/deep_compare.rs`):
  - **A click on a row selects it.** A double-click, or Enter on the
    selected row, opens the comparison where *Compare* is available.
    Otherwise it does nothing.
  - **The first row is selected** when the list appears, and again whenever
    a filter change removes the selected row.
  - **The selected row shows its actions** (*Compare*, *Copy to right*,
    *Copy to left*, whichever apply) **at full strength.** Other rows show
    none.
  - **The selected row carries the Explorer's accent bar** (`inset 3px 0 0 0
    var(--accent)`), with the bar's pair added to `contrast`'s `PAIRS`.
  - **Remove `.deep-compare-btn`'s opacity rules** and their allowlist
    entries.
  - **The keyboard:**
    - Tab reaches the list;
    - Up and Down move the selection;
    - Enter opens;
    - Tab from the selected row reaches its buttons.
  - **Choose an ARIA pattern that allows buttons inside a row,** and say
    which. For example, a `grid` with one row per entry. **Not
    `listbox`/`option`,** which may not contain interactive children. The
    selected row is announced with its path and status.
  - `.deep-row`'s `cursor: pointer` now means something. Keep it.

### 4. *Redo* beside *Undo*

Move *Redo* out of *More* and put it next to *Undo*, with the same enabled
and disabled logic. Ctrl+Y is unchanged.

### 5. Click targets

**Every interactive element is at least 24×24 CSS px at 100 %,** or meets
WCAG 2.5.8's spacing exception, stated per element. The toolbar's main
controls are at least 32 px tall; most already are, through
`--control-h`. **Known small ones to check:**
- the tab ×;
- `.search-nav`;
- `.filter-toggle` and `.filter-clear` (24 px now);
- `.tree-verify-btn`;
- breadcrumb segments (`.bc-seg`, about 18 px tall);
- `.path-btn`;
- `.profile-delete`;
- path-bar menu items;
- the *Use* button.

List every interactive element you checked, with its measured size: from
captures (its `--control-border` edge makes it measurable) or from layout
tests.

### 6. Settings help text visible

The help sentences that are `title` tooltips in
`ui/view/settings/modal.rs`:
- *Text size*;
- *Enable binary comparison*;
- *Remember Explorer directories*;
- the hidden-files sentence.

**Show each as a short `--muted` line under its field,** linked with
`aria-describedby`. Then **drop the `title`,** so it isn't said twice.
Titles that name a control (*About ForskScope*, *Delete profile*) stay.

### 7. No italic hint

The Explorer's "Choose a file or folder on each side to compare" is
upright, in `--muted`.

## Verification

**AT-SPI is unavailable** (the sandbox; review 202), so:
- **Markup tests** for each new accessible name, the key's contents for a
  given set of statuses, the Deep Compare row pattern and
  `aria-describedby`. Use the real i18n table, never a stand-in.
- **Captures,** isolated `--data-dir`, views opened by command-line paths,
  the window size set at launch. Use 100 % and 200 % in 1000×700, Light and
  Dark:
  - the comparison toolbar;
  - the tab bar with three tabs;
  - the Explorer with the key and at least four kinds of symbol on screen;
  - Deep Compare with its first row selected;
  - Settings with the help lines.
- **Deep Compare's keyboard:** `wtype` on the Wayland backend, under the GUI
  rules, with the niri focus check before each key. **This tests the
  keyboard, so keys are allowed.**
  - Down once: a capture shows the selection moved.
  - Enter: a capture shows the comparison tab opened.

  Reaching the list may need a real `xdotool` click on X11 first. Use reply
  178c's warm-up method and the 25 px geometry offset from reply 182.
- **The contrast gate:**
  - passes without the five removed opacity entries (three for the tab ×,
    two for Deep Compare's buttons);
  - fails if one is restored, as a falsification;
  - includes the Deep Compare bar's pair.
- **Regression:** captures of views this doesn't touch (the diff lines
  themselves), before and after, by exact pixel count.
- **Test names diffed before and after.**

## Scope

- **In:** §1–§7 and the guide pages they change (settings, keyboard,
  Explorer, Deep Compare).
- **Out:**
  - stage 2;
  - the path-bar reduction (F162's open question);
  - CHANGELOG and the version.

## Gates

**The standard set:** Windows GNU clippy, both audits, `colour-vision`,
`contrast`, `i18n`, `keyboard-docs` and `css --check`, and a green CI run.
Reply in `.git-exclude/review-request/183-visible-controls.md`.
