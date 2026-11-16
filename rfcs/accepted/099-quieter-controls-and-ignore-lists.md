# RFC 099: Quieter Close Buttons, a Title Bar That Follows the Theme, and Ignore Lists as Badges

**Status.** Accepted.
**Proposed.** 2026-11-16 by the architect, from the `0.199.0` scope:
- the owner noted that "border of close button in tabs can be beautified";
- and asked for the two ignore lists to be "shown as badges with removing
  buttons after input".

**Accepted.** 2026-11-16 by the project owner ("Yes. All approved."):
- the scope;
- the close-button preview
  (https://claude.ai/artifact/MhkfJrM1pbDDZBB7tr3Qwj);
- the title bar following the app's theme;
- and the badges, which were requested in the same message.

**Scheduling.** `0.199.0`, after RFC-098's checks land (handoff 101), so
they verify this work across every size, language and theme.
**Touches.**
- `crates/forskscope-ui`: the tab bar, the search bar, the Explorer
  filter, the Settings dialog, the app shell and the CSS;
- `crates/forskscope-ui-logic`: the ignore-list parsing for display;
- the i18n table;
- the `contrast` gate;
- the guide's settings page.

## Summary

Three small changes that make the app quieter and easier to use. **None of
them changes what the app does.**
1. **Close and clear buttons become icon buttons.** They are borderless at
   rest, and show a tint when pointed at and a ring when reached by
   keyboard. One style is used for every ×.
2. **The window's title bar follows ForskScope's theme,** not only the
   desktop's.
3. **The two ignore lists are shown as badges,** each with its own remove
   button, instead of one comma-separated text field.

## 1. Icon buttons for every ×

**Today, the tab ×** inherits the general button style, a full-height
bordered box inside the tab. The search bar's close button and the filter's
clear button are drawn differently from it and from each other. The filter's
uses a different glyph (✕), and its accessible name is that glyph, because it
has only a tooltip.

**One shared style** (`.icon-btn`) for:
- the tab × (both tab kinds);
- the search bar's close button;
- the filter's clear button;
- the Settings profile delete button;
- the new badge remove buttons (§3).

**The style:**
- **at rest:** no border and no fill. The glyph is `--muted` on an inactive
  tab or plain surface, and `--text` on the selected tab;
- **click area:** at least **24 × 24 CSS px** (WCAG 2.5.8);
- **pointed at:** a neutral tint (`color-mix(--text 14 %, transparent)`),
  and the glyph turns `--text`. **Not the danger red used today:** closing a
  tab is ordinary, and unsaved changes already ask first;
- **keyboard focus:** the app's focus ring;
- **one glyph, ×,** everywhere;
- ***High* contrast:** the glyph uses the raised tokens;
- **forced colours (Windows contrast themes):** system colours only; the
  glyph is `ButtonText`, and the pointed-at state is a `Highlight` outline.

**Accessibility:**
- **every icon button has an accessible name** (`aria-label`), not only a
  tooltip. The filter's clear button gains "Clear filter";
- **the tab × stays mouse-only,** as RFC-087 §1 decided. The keyboard closes
  a focused tab with Delete, and the × stays `aria-hidden` and out of the Tab
  order.

**The `contrast` gate** gains the glyph pairs: `--muted` and `--text` on the
surfaces each icon button sits on, at 4.5:1. That is above the 3:1 a
non-text glyph needs, because the glyph is the control's only visible cue.

## 2. The title bar follows the app's theme

**Today,** the title bar, and on Linux any other GTK-drawn part, follows the
desktop. With *Dark* chosen on a light desktop, the window has a light title
bar over a dark app.

**Change:** set the window's theme from **the resolved theme**, through
tao's `set_theme`, at start-up and whenever it changes:
- *Dark* and *Night* → dark;
- *Light* → light;
- ***System* → the light or dark it currently resolves to,** not "none".
  On Linux, tao treats "none" as light, so passing it would undo the
  desktop's own dark setting.

**Platform notes, stated as they are:**
- **Windows:** the title bar follows, through the system's dark title bar.
- **macOS:** the window's appearance follows.
- **Linux:**
  - **GTK's own title bar** (client-side, as under GNOME) follows.
  - **A title bar drawn by the desktop itself** (server-side, as under KDE
    by default) can't be changed by the app, and stays as the desktop draws
    it.
  - **GTK-drawn dialogs** follow too.

  The guide says exactly this.

## 3. The ignore lists as badges

Settings → *Ignore file extensions* and *Ignore directory names* are single
text fields today, comma-separated.

**Change:**
- **Each entry is a badge,** with a remove button (§1's icon button,
  accessible name "Remove `<entry>`").
- **An input follows the badges.** Enter, or typing a comma, adds what was
  typed. **Leaving the field adds it too,** so nothing typed is lost.
  - **Pasting** `a, b, c` adds three.
- **A badge shows the entry as it will apply.** Extensions without a leading
  dot and in lower case, exactly as the engine reads them
  (`IgnoreRules::from_settings`); directory names and patterns as typed,
  trimmed.
- **An entry already in the list isn't added twice.** The field says
  "Already in the list", and nothing changes.
- **Backspace in an empty input doesn't delete a badge.** Removal is always
  the explicit button, so nothing is removed by accident.
- **Badges wrap onto further lines.** Each is at least 24 px tall.
- **Screen readers:**
  - the badges are a list, and the input is labelled by the field's own
    label;
  - adding and removing are announced once, politely ("Added tmp",
    "Removed tmp").
- **What is saved is unchanged:** the same comma-separated text, written
  when a badge is added or removed. **Older settings files load as they
  are,** and there is no schema change.
- **English and Japanese,** through the real i18n table.

## Non-goals

- **No change to how the ignore rules match.**
- **No other control's look changes.** The general button style stays.
- **No change to the tab ×'s keyboard model.**

## Verification

- **RFC-098's layout checks** (handoff 101) cover all three across the full
  matrix: overflow, clipped text, the 24 px targets, and rendered contrast,
  including the badge lists at 200 % with many entries.
- **Unit and markup tests:**
  - adding, pasting, duplicates, leaving the field;
  - the badge normalisation, matching `IgnoreRules::from_settings` on the
    same input;
  - serialising back to the same text;
  - every accessible name;
  - the resolved-theme mapping, including *System* on Linux.
- **Real-UI captures** only for what needs eyes: the worst case and one
  baseline per item. The title bar is captured per platform where it can be:
  Linux on GNOME-style decoration, and Windows in a scratch run.
