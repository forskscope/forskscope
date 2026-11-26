# Handoff 095 — RFC-097 stage 1: Text size

**From:** architect. **Date:** 2026-11-04. **Release:** `0.197.0`.
**RFC:** `rfcs/accepted/097-readable-for-everyone.md`, §Stage 1 (accepted by
the owner 2026-11-04). **Start after handoff 094 is approved,** so that this
work builds on the readability floor and its `contrast` gate.

## Do

### 1. Measure first (RFC §1.4)

Before writing the setting, answer these on Linux (the X11 and Wayland
backends) and Windows:

1. **Does today's app grow with the system's text factor?**
   - **Linux:** GNOME's `text-scaling-factor` at 1.25, applied in a nested
     session or with an isolated `XDG_CONFIG_HOME`/dconf profile. **Never
     change the maintainer's real desktop settings.** Also try KDE's font
     DPI if you can set it up the same isolated way. If you can't, say so.
   - **Windows:** *Make text bigger* at 125 %, on the CI or test machine
     only.

   **Measure, don't judge by eye:** the `getBoundingClientRect` height of a
   known element, read through a test-only path, or the pixel height of a
   fixed glyph in a capture. Compare at the factor and at 1.0.
2. **Can the factor be read with what is already linked?**
   - Linux: GTK's `gtk-xft-dpi`.
   - Windows: a registry read through a crate already in the graph.
   - Name the API and the crate.

**Decision rule:**
- **The app already grows on a platform:** *System* there must not apply the
  factor again. Report how the two would combine.
- **Reading the factor needs a new dependency:** stop and report. I decide.
- **Otherwise:** *System* is offered on that platform.
- **macOS:** *System* is not offered (RFC §1.4).

**Send the measurement as an interim reply before building §2–§4.** It is
`.git-exclude/review-request/177-text-size-measurement.md`. I answer it
before you go on.

### 2. The setting

- **Settings → *Text size*:**
  - *System* (where §1 allows it), then 100 %, 125 %, 150 %, 175 % and 200 %;
  - it is the first item of the appearance group;
  - help text: it enlarges the whole app; *Diff font size* sets the code
    panes relative to it.
- **Applied with `webview.zoom(f)`** from the Dioxus desktop context:
  - at start-up, before the first paint if possible;
  - on every change.
- **The web view's own zoom hotkeys are off** (wry `with_hotkeys_zoom(false)`,
  or the Dioxus equivalent). Show that on Windows, Ctrl+wheel and Ctrl+= no
  longer zoom on their own.
- **English and Japanese strings,** through the real i18n table, checked by
  `cargo xtask i18n`.

### 3. Keyboard

- **Ctrl+= and Ctrl++ go up one step,** Ctrl+− goes down one, and Ctrl+0
  returns to the default. Each change is saved.
- **The steps stop at the ends:** no wrap-around, and no step outside the
  list.
- **Add them to the *Keyboard shortcuts* dialog and the guide's keyboard
  page.**
- **Check the Japanese keyboard layout:** whether `=` and `+` need Shift
  there, and what key event arrives. Read the code, or test with `wtype`
  under the GUI rules.

### 4. Persistence

- **One new field in the persisted settings, added with a serde default.**
- **Tests:**
  - an older file without the field loads as the default;
  - a value outside the steps loads as the nearest step;
  - a round trip keeps each step.
- **Follow the schema's versioning rules** as the last settings addition
  did. Say which rule applied.

### 5. Layout at every size (RFC §1.5)

The test plan, at a 1000×700 window, set at launch (the GUI rules). Each row
is checked at 100 %, 150 % and 200 %:

| View | Expected |
|------|----------|
| Explorer, two folders open | Breadcrumbs, filter bar and the *Compare* bar visible; the tree scrolls |
| File comparison | Toolbar fully reachable (overflow into *More* is acceptable); both panes and the gutter visible; the *Use* button usable |
| Deep Compare | Filters, actions and rows reachable; row buttons not cut off |
| Settings | Every field reachable by scrolling; nothing overlaps |
| Keyboard shortcuts dialog | Fits or scrolls; nothing cut off |
| Mergetool layout | Result path and *Save* reachable |

**How each row is checked:**
- **"Reachable"** means AT-SPI finds the control with a non-empty extent
  inside the window. Prefer that over a capture.
- **Captures** only for 100 % and 200 % of the Explorer and the file
  comparison. Those are deliverables for the review.
- **Isolated config and state,** one instance, AT-SPI only. Keystrokes only
  in §3's keyboard test.

**If a view fails at 200 %,** fix it in CSS within the existing design.
Moving controls is RFC-097 stage 3: report it instead of doing it.

## Verification

- The §1 measurements, with how each was taken.
- Unit tests for the step logic and persistence. The i18n test runs against
  the real table, not a stand-in.
- §5's table filled in with results.
- **A live check that the setting survives a restart,** from the isolated
  config file's contents.
- **Test names diffed before and after.**

## Scope

- **In:** RFC-097 §Stage 1 only.
- **Out:**
  - stage 2 (themes, contrast, motion);
  - stage 3 (labels, hover controls, *Redo*);
  - CHANGELOG and the version.

## Gates

**The standard set,** including Windows GNU clippy, both audits,
`colour-vision`, `contrast`, `i18n`, and a green CI run. Final reply in
`.git-exclude/review-request/178-text-size.md`.
