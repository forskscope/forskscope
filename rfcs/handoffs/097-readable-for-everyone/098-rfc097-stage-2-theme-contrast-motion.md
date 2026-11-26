# Handoff 098 — RFC-097 stage 2: theme, contrast and motion

**From:** architect. **Date:** 2026-11-08. **Release:** `0.198.0`, or
`0.199.0` for any part the measurement defers.
**RFC:** `rfcs/accepted/097-readable-for-everyone.md`, §Stage 2. The owner
accepted the 0.198.0 scope on 2026-11-08.
**Start after handoff 097 is approved.**
**Like stage 1, this begins with a measurement,** which comes to me before
anything is built.

## 1. Measure first

**The question, for each preference and each platform:** does the web view
already report it through the CSS media query, with no code of ours?

| Preference | Media query | Linux, WebKitGTK 2.54 | Windows, WebView2 | macOS, WKWebView |
|---|---|---|---|---|
| Light / dark | `prefers-color-scheme` | GNOME `color-scheme` (`prefer-dark` / `default`) | *Choose your app mode* (`AppsUseLightTheme`) | Appearance (`AppleInterfaceStyle`) |
| Higher contrast | `prefers-contrast: more` | GNOME *High Contrast* (`org.gnome.desktop.a11y.interface high-contrast`) | a contrast theme (and see §1.2) | *Increase contrast* |
| Less motion | `prefers-reduced-motion: reduce` | GNOME *Animations* off (`enable-animations false`) | *Animation effects* off | *Reduce motion* |

### 1.1 How to measure

**The observable is the media query's answer inside the app's own web view.**
- **Read it with a measurement-only commit that never reaches `main`:** a
  few lines that write each `matchMedia(...).matches` into the window title
  or a visible corner, on a scratch branch or local build.
- **Measure each setting off, then on, with a fresh launch for each.**
- **Linux:** set the GNOME keys through the `keyfile` GSettings backend under
  a scratch `XDG_CONFIG_HOME`, as reply 177 did. **Never the real desktop's
  settings.** Read the answer from a capture.
- **Windows and macOS: one push-triggered scratch branch,
  `scratch/098-preferences`, is authorized,** with a `windows-latest` and a
  `macos-latest` job. It is the reply 177/181 pattern, so no change to
  `main`'s workflows.
  - Set each preference by its registry key or `defaults write`.
  - Read the answer: from the window title through UI Automation on
    Windows, and through `osascript` on macOS (System Events reading the
    window's name), or from a capture.
  - Delete the branch afterwards, and quote the runs.
  - If a preference can't be set on a hosted runner, report it as
    unmeasured. Don't work around it.
- **The Flatpak is not measured here.** The guide claims nothing about it
  that wasn't measured.

### 1.2 Windows contrast themes: is the diff's meaning lost?

Under a Windows contrast theme, Chromium sets `forced-colors: active` and
replaces authors' colours with the system palette. That can erase the
diff's red, green and amber row backgrounds. **This may already be a
defect for Windows users today.**

In the same scratch run, with a contrast theme applied (for example
*Night sky*, through `SPI_SETHIGHCONTRAST` or its registry keys), capture a
file comparison with a delete, an insert and a replace. Report:
- whether `forced-colors: active` is reported;
- whether changed rows are still distinguishable from unchanged ones, and
  by what: the −/+ marks alone, or a background too.

**If only the marks remain, I'll register it as a finding.** It gets fixed
in this handoff (§2.4).

### 1.3 The evidence plan (the GUI rules, *Test Optimization Rules*, owner, 2026-11-11)

**Reply 184 also carries the evidence plan for §2,** before anything is
built:
- **the questions;**
- **the level each is checked at,** cheapest first:
  - **unit tests:** theme, contrast and persistence logic, and which choices
    are offered;
  - **the static gates:** `contrast` for every *High* pair;
  - **real-UI captures:** only for what needs eyes.
- **the captures that remain.** Per the rules, that is the worst case plus
  one baseline. For example: *High* contrast, Light, red-green-safe, at
  200 % in 1000×700, against *Standard*. **Not the full theme × contrast
  matrix:** the `contrast` gate covers that.

**The project has no headless layout harness yet** (the architect,
2026-11-11), so layout questions stay at the capture level for this
handoff. Keep them to the worst case.

**Use the shared session script and fixtures** once handoff 099 has made
them. Until then, reuse the 196/095 drivers rather than writing new ones.

### 1.4 Send the measurement first

Reply in `.git-exclude/review-request/184-preferences-measurement.md`, with
§1's table filled in: measured yes or no, with how, per cell. I decide from
it. **Nothing in §2 is built before my answer.**

## 2. Then build (after my answer to 184)

**This is the RFC's design.** My answer to 184 may narrow it per platform,
as review 199 did for stage 1.

### 2.1 A *System* theme

- **Theme gains *System*:** light maps to Light and dark maps to Dark, by
  `prefers-color-scheme`. Night stays its own choice.
- ***System* is offered only on platforms where §1 measured that the query
  follows the OS.** Elsewhere it isn't shown.
- **It is the default only when no settings file exists**, a new install. An
  existing file always has a theme saved, and keeps it. Dark stays the
  default where *System* isn't offered.
- **It switches live** when the OS setting changes, if the query reports the
  change live. Measure that as well.

### 2.2 *Contrast*: *Standard* or *High*

- **A third axis beside the theme and colour vision,** as a root class
  (`.contrast-high`) combined with every theme and both colour-vision modes,
  the way `.cv-rgsafe` is (RFC-088).
- ***System*** (follows `prefers-contrast: more`) **is offered only where
  §1 measured it.**
- ***High* raises:**
  - secondary text (`--muted`) to at least **7:1** on every surface;
  - text on the diff's rows to at least **7:1**;
  - control edges and the diff's marks to at least **4.5:1**;
  - the focus ring to **3 px**;
  - disabled controls to at least **3:1**: no longer exempt, since dimmed to
    unreadable helps no one who chose *High*.
- **The `contrast` gate gains the axis:** every pair at its *High* minimum,
  in theme × colour vision × contrast (3 × 2 × 2). Falsify one *High* pair.
- **The persisted settings gain one field, added with a default:** the
  `text_size` rules (review 199). An older file loads as *Standard*.

### 2.3 Reduced motion

**Under `prefers-reduced-motion: reduce`, wherever §1 measured it:**
- spinners stop rotating. Each keeps its static glyph, and its accessible
  name already says "loading";
- transitions are removed;
- any smooth scrolling becomes instant.

There is no setting for this.

### 2.4 Windows contrast themes (only if §1.2 finds the defect)

**Under `forced-colors: active`, changed rows must stay distinguishable at a
glance,** not only by their marks. Use system colours, for example a
leading bar in `Highlight` per changed row, or a border. **Don't use
`forced-color-adjust: none`:** it would override the user's own choice of
colours.

## Verification for §2

- **Markup and unit tests:**
  - theme, contrast and persistence: an older file, the default for a new
    install, round trips;
  - each choice shown only where measured.
- **Captures, per the evidence plan approved in 184:** the worst case plus
  one baseline, isolated `--data-dir`, by command-line paths. The `contrast`
  gate covers the full theme × contrast matrix, so captures don't repeat it.
- **The `contrast` gate's output for all twelve combinations.**
- **Live switching,** where §1 found it reported live: one capture before
  and one after changing the OS setting through the keyfile backend.
- **§2.4, if built:** one more scratch Windows run under a contrast theme.
  That's authorized as part of the same scratch branch's lifetime, or a
  second short-lived one, deleted afterwards.
- **Test names diffed before and after.**

## Scope

- **In:** RFC-097 stage 2, plus §1.2's finding if it is one.
- **Out:**
  - new themes;
  - changing diff colours beyond what *High* requires;
  - the Flatpak's behaviour;
  - CHANGELOG and the version.
- **Stop and report** if any part needs a new dependency.

## Gates

**The standard set:** Windows GNU clippy, both audits, `colour-vision`,
`contrast`, `i18n`, `keyboard-docs` and `css --check`, and a green CI run.
The final reply goes in `.git-exclude/review-request/185-theme-contrast-motion.md`.
