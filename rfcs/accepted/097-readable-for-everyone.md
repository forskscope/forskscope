# RFC 097: Readable for Everyone — Text Size, Contrast and Visible Controls

**Status.** Accepted.
**Proposed.** 2026-11-04 by the architect, from the owner's question about
older users (layout, typography, colour).
**Accepted.** 2026-11-04 by the project owner ("Your recommendation is
accepted and authorized").
**Scheduling.** Stage 1 in `0.197.0`; stages 2 and 3 in `0.198.0`. The
defect fixes found by the same audit (F214–F216, handoff 094) come first, in
`0.196.4`, and are not part of this RFC.
**Touches.**
- `crates/forskscope-ui`: the app shell, the keyboard handler, the settings
  modal, the CSS;
- `crates/forskscope-ui-logic`: the settings view-model;
- `crates/forskscope-desktop`: the persisted settings schema;
- the i18n table (English and Japanese);
- the guide's settings, keyboard and accessibility pages.

## Summary

ForskScope already works well with screen readers and for colour-blind
users. It is still built for young eyes:
- the text outside the code panes is small and fixed;
- some controls are faded or appear only on hover;
- some buttons are only a symbol.

This RFC adds three things, in three stages:
1. **Text size:** one setting that enlarges the whole app, controls
   included.
2. **Theme and contrast follow the system:** a *System* theme, a high-contrast
   option and reduced motion.
3. **Visible controls:** labels where a symbol alone is unclear, a key to the
   Explorer's status symbols, and no controls hidden until hover.

## Motivation

The audit of 2026-11-04 read the stylesheets, the settings code and the
three listing screenshots:
- **Most interface text is 11–12 px,** some is 9–10 px, and the base size is
  14 px. Only the code panes have a size setting (*Diff font size*).
- **Sizes are fixed in CSS pixels,** so the system's large-text setting
  probably does not reach the app. Stage 1 measures this before relying on
  it.
- **The default theme is Dark,** and the app ignores the system's light/dark
  choice. Many older eyes, especially with astigmatism, see light text on
  dark as blurred.
- **There is no high-contrast option,** and the app ignores the system's
  contrast and reduced-motion preferences.
- **Some controls are hidden or faded until hover:**
  - the × on tabs that aren't selected;
  - the folder-comparison row buttons.
- **Several buttons are a symbol only,** explained by a tooltip on hover:
  ◀ ▶ ↺ 🔍 ?.
- **The Explorer's status symbols** (≠ = − →) are 11 px, with no visible key.
- **In the comparison toolbar, *Undo* is visible and *Redo* is under
  *More*.**

The contrast failures and the smallest text sizes are **defects** against
accessibility the project already claims: F214–F216, fixed first and
separately. This RFC covers what is **new**.

## Design principles

- **One setting enlarges everything,** not each part on its own. The person
  who needs larger text needs larger buttons too.
- **Follow the system where it can be read reliably;** otherwise say nothing
  and use the app's own default. Never guess.
- **Less is more,** by showing fewer controls at full strength, never by
  fading or hiding the ones that are there.
- **No existing user's saved choice changes.** New defaults apply only where
  nothing was saved.

## Stage 1 — Text size (`0.197.0`)

### 1.1 The setting

**Settings → *Text size*:** *System*, 100 %, 125 %, 150 %, 175 %, 200 %. It
scales the whole web view through the platform zoom (wry's
`WebView::zoom`, reached from Dioxus as the desktop context's `webview`). So
every part grows together: text, controls, click targets and the code panes.
- ***Diff font size* stays** and still sets the code panes' size relative to
  the rest. Its label and help text say so.
- ***System* uses the platform's text-scaling factor,** but only where stage
  1's measurement (§1.4) shows that it can be read reliably and isn't already
  applied. Elsewhere *System* isn't offered, and the default is 100 %.
- **The default is *System* where it is offered, else 100 %.** With no
  system factor, *System* is 100 %. So nobody's app changes size unless their
  system already asks for larger text.

### 1.2 Keyboard

**Ctrl+= (and Ctrl++), Ctrl+− and Ctrl+0** step through the sizes and back to
the default.
- They change the saved setting.
- They are listed in the *Keyboard shortcuts* dialog.
- **The web view's own zoom hotkeys are turned off** (WebView2 enables them by
  default on Windows), so there is exactly one zoom, and it is saved.

### 1.3 Persistence

- **The persisted settings gain one field, added with a default:**
  - an older settings file loads unchanged, as *System*/100 %;
  - a value outside the steps loads as the nearest step.
- **It goes through the same versioned schema** and isolation rules as every
  other setting.

### 1.4 Measure before relying on the system (gate for *System*)

On each platform, before *System* is offered there, show what happens:
- **Linux:** GNOME's text scaling (`text-scaling-factor`; GTK's
  `gtk-xft-dpi`) and KDE's font DPI.
- **Windows:** *Make text bigger*.
- **macOS:** there is no system-wide text scale, so *System* is not offered.

Answer two questions:
1. Does today's app already grow with the system's factor?
2. Can the factor be read without a new dependency?

**If the app already grows, *System* must not apply the factor a second
time.** If reading the factor needs a new dependency, stop and report: I
decide.

### 1.5 Layout at every size

At 1000×700, which is the listing's size, and at every step:
- nothing is cut off without a way to reach it;
- every control can be reached;
- the existing narrow-width rules (`90-responsive.css`) take over as the
  effective width shrinks.

The scenario table in the handoff is the test plan.

## Stage 2 — Theme and contrast follow the system (`0.198.0`)

### 2.1 A *System* theme

Theme gains *System*, which follows the platform's light/dark preference.
- **Light maps to Light, and dark maps to Dark.** Night stays a choice of its
  own.
- **It is the default only for a settings file that never saved a theme.** A
  saved Dark, Light or Night is kept.
- **Inside the Flatpak, the preference comes through the settings portal.**
  The stage-2 handoff measures how WebKitGTK 4.1 reports it there before the
  design relies on `prefers-color-scheme`.

### 2.2 Contrast: *Standard* or *High*

A third axis beside the theme and colour vision, combined with every theme
the same way `.cv-rgsafe` is (RFC-088):
- **Choices:** *System* (follows `prefers-contrast: more` where the platform
  reports it), *Standard*, *High*.
- ***High*:**
  - secondary text at the main text's colour or close to it (at least 7:1);
  - control edges at least 4.5:1;
  - a thicker focus ring;
  - no reduced opacity anywhere except disabled controls, which stay at
    least 3:1.
- **The contrast gate measures every *High* pair**, as `colour-vision` does
  for its axis.

### 2.3 Reduced motion

**Under `prefers-reduced-motion: reduce`:**
- the spinners stop rotating, and progress is shown by text;
- transitions are removed.

There is no setting for this; the system's preference is enough.

## Stage 3 — Visible controls (`0.198.0`)

- **Labels.** In the comparison toolbar, *Previous change*/*Next change* and
  *Search* show their words beside the symbol. *Reload* keeps its symbol,
  with an accessible name. The top-right "?" reads *Shortcuts*.

  The dev team proposes the layout, with captures at 100 % and 200 %. If
  the labels don't fit at 200 % in 1000×700, a label may fold into its
  symbol only at sizes where it can't fit, and never at 100 %.
- **A key to the Explorer's status symbols,** visible without hovering, in
  the Explorer itself (for example, in its bottom bar while nothing is
  selected). It is one short line, translated. Its place is proposed with
  captures.
- **No control hidden until hover:**
  - the × shows on every tab;
  - the folder-comparison row buttons show at full strength.

  If a row's three buttons then read as noise, reduce their number and keep
  them visible. Any reduction is a design proposal for me first.
- ***Redo* sits beside *Undo*,** not under *More*.
- **Click targets** are at least 24×24 px at 100 % (WCAG 2.5.8). The
  toolbar's main controls are at least 32 px tall. The *Use* button in the
  merge gutter is included.

## Non-goals

- **No re-design of the views,** and no move of *Settings* or *Compare*. The
  audit found the flow sound.
- **No new themes** beyond the *System* choice and the contrast axis.
- **No change to the code panes' font family or diff colours** beyond what
  the contrast gate requires.

## Verification

Each stage's handoff carries:
- a scenario table: every view × every size or mode, the expected result, and
  how it is checked;
- machine comparison before pictures;
- AT-SPI actions only, and keystrokes only to test the keyboard
  (`.git-exclude/rules/project-instructions-gui-common.md`);
- isolated configuration and state;
- captures only where the picture is the deliverable.

**The gates from F214–F216 stay green throughout.**

## Risks

- **Double scaling on Linux,** if WebKitGTK already follows the font DPI.
  §1.4 measures before *System* is offered.
- **Toolbar overflow at 200 %.** Covered by §1.5's scenarios and stage 3's
  rule for folding labels.
- **A changed default surprises an existing user.** It doesn't happen: new
  defaults apply only where nothing was saved, and the CHANGELOG says so.
