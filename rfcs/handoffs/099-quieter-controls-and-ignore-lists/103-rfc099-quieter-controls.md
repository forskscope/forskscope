# Handoff 103 — RFC-099: quieter close buttons, the title bar, and badges

**From:** architect. **Date:** 2026-11-16. **Release:** `0.199.0`.
**RFC:** `rfcs/accepted/099-quieter-controls-and-ignore-lists.md`, accepted
by the owner on 2026-11-16, with the preview at
https://claude.ai/artifact/MhkfJrM1pbDDZBB7tr3Qwj.
**Start after handoff 101 is approved,** so that RFC-098's checks verify
this work across the whole matrix.

## Evidence plan first (the GUI rules)

The checks below run at the cheapest level:
- **unit and markup tests** for logic, names and serialisation;
- **RFC-098's layout job** for layout, targets and contrast at every size,
  language and theme;
- **captures** only for what needs eyes.

**Send the plan as the first section of your reply.**

## 1. One icon-button style for every ×

RFC §1. Apply `.icon-btn` to:
- the tab × (`ui/layout/tabs.rs`, both tab kinds);
- `.search-close` (`ui/view/search.rs`);
- `.filter-clear` (`ui/view/explorer/filter.rs`);
- `.profile-delete` (`ui/view/settings/modal.rs`);
- §3's badge remove buttons.

**Specifically:**
- **No border or fill at rest.** The glyph is `--muted` on inactive tabs and
  plain surfaces, and `--text` on the selected tab.
- **The click area is at least 24 × 24.**
- **The hover tint** is `color-mix(in srgb, var(--text) 14%, transparent)`.
  **Not `--danger-bg`.**
- **The usual focus ring.**
- **One glyph, ×.** The filter's ✕ changes.
- ***High*** uses the raised tokens.
- **Forced colours:** `ButtonText` for the glyph, and a `Highlight` outline
  when pointed at.
- **Accessible names:**
  - `aria-label` on each, so the filter's clear gains "Clear filter",
    through the real i18n table;
  - **the tab × stays `aria-hidden` and `tabindex="-1"`** (RFC-087 §1).
- **`contrast`:** the glyph pairs at 4.5:1 on each surface the buttons sit
  on. Falsify one.

## 2. The title bar follows the resolved theme

RFC §2. Use tao's `set_theme` from the **resolved** theme, at start-up and
whenever it changes:
- *Dark* and *Night* → `Dark`;
- *Light* → `Light`;
- *System* → whatever it currently resolves to. **Never `None`,** which tao
  treats as light on Linux.

**Unit-test the mapping,** including *System* resolving each way.

**Captures:**
- **Linux:** a GTK client-side title bar in a nested session, *Dark* on a
  light GTK setting, before and after.
- **Windows:** one push-triggered scratch run (authorized, deleted
  afterwards). Measure the title bar's own region before and after,
  **with a number**, not by description.

**The guide's settings page** says what follows on which platform, and that
a title bar the desktop draws itself (KDE's default) doesn't.

## 3. The ignore lists as badges

RFC §3. For *Ignore file extensions* and *Ignore directory names*:
- **Badges, each with a remove button** (§1), named "Remove `<entry>`".
- **The input after them:**
  - Enter or a comma adds;
  - **leaving the field adds what was typed;**
  - pasting `a, b, c` adds three.
- **Normalised display:**
  - extensions use the exact normalisation of
    `IgnoreRules::from_settings` (trim, drop the leading dot, lower case).
    **Share the function,** don't copy it;
  - directory names are trimmed.
- **Duplicates aren't added,** and the field says "Already in the list".
- **No Backspace deletion.**
- **Badges wrap, and are at least 24 px tall.**
- **Screen readers:**
  - the badges are a list, and the input is labelled by the field;
  - one polite announcement per add or remove.
- **Saved as the same comma-separated text, on each add or remove.** There
  is no schema change, and older files load as they are.
- **Tests:**
  - the normalisation **equals** the engine's on the same inputs;
  - serialisation round-trips;
  - duplicates;
  - paste;
  - leaving the field;
  - every accessible name, through the real table.

## 4. F224: a dialog never exceeds the window (added 2026-11-17, review 216)

**The defect:**
- `.modal` has no `max-height` and no scrolling, and `.scrim` centres it.
- So at large text sizes a dialog taller than the window, such as Settings
  at 200 %, loses its title and its *Close* button off both edges, with
  nothing to scroll.

RFC-098's probe found it (reply 188, `div.scrim`).

**Fix, for every dialog:**
- **The dialog's height is capped to the window, minus a margin,** and its
  content scrolls inside it.
- **The title and the action buttons stay visible,** if the dialog's
  structure allows a scrolling body between a fixed header and footer.
  **Otherwise, the whole dialog scrolls.** Say which you chose, and why.
- **The scroll container is reachable by keyboard,** as the dialog's own
  focusable content already is.

**RFC-098's checks** then show no `div.scrim` overflow at any size, and the
probe's *Shortcuts* and Settings views are the evidence.

## Verification

- **RFC-098's job green,** with the badge lists at 200 % holding many
  entries in its fixtures.
- **The `div.tab-slot` expected failure is gone** (review 216 §5): the
  icon button's 24 px removes it.
- **The captures in §2,** plus the worst case and one baseline for §1 and
  §3, at 200 %, Light, red-green-safe.
- **Test names diffed before and after.**

## Gates

**The standard set,** and RFC-098's job. Reply in
`.git-exclude/review-request/191-quieter-controls.md`.

## Round 3 (added 2026-11-17, review 218)

**§2 is done** on Windows and Linux. Two changes remain, in one commit on
top of `b723d41`:
- **F225's toast regression:** give `.toast` an explicit `z-index` above
  the scrim (110), and add it to the list with its reason. Prove it with
  a probe view, falsified on `b723d41`.
- **The badge status line:**
  - one permanent `.sr-only` live region per list;
  - a visible line for "Already in the list" only, in normal flow, with
    no negative margin;
  - no visible "Added" or "Removed".

Review 218 §2–§3 has the detail. Reply in
`.git-exclude/review-request/191c-quieter-controls.md`.

## Closed (2026-11-17, review 219)

**Approved.** F224 and F225 are done. One line remains: clear the live
region on input, so that a repeated duplicate is announced (review 219
§2). It goes with stage B and is named in reply 189.

**Any further AT-SPI run** uses a private accessibility bus (review 219
§3).
