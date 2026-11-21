# RFC 098: Layout Checks in CI — Every Size, Language and Theme, on Every Push

**Status.** Implemented in `0.199.0` (handoff 101; reviews 216–225). The gate has run on every push since `5ba4ff3`.
**Proposed.** 2026-11-15 by the architect. The owner asked for the draft
the same day.
**Accepted.** 2026-11-16 by the project owner ("Yes. All approved."),
with both open questions decided as proposed (§Decisions).
**Amended.** 2026-11-17 by the architect (review 216): a seventh check,
label placement, so F217's shape is caught; and the matrix measured per
check, on the axes each depends on. Review 217 added an eighth, nothing
covers an open dialog, after F225. Review 218 added a ninth, overlap, and
named the toast as the eighth check's one allowance.
**Scheduling.** The first handoff of the `0.199.0` cycle (handoff 101). It is tooling, not a product feature, so it
needs no release of its own. The checks start gating `main` as soon as they
land.
**Tracks.** The owner's *Test Optimization Rules* (GUI rules, 2026-11-11):
*"Headless UI … Use it for interaction and layout properties: nothing
overflows, every label is fully visible, minimum hit area, contrast. Check
them across every text size, language, theme and window/viewport size, on
every CI run."*
**Touches.**
- `crates/forskscope-ui`: a test-only probe behind a Cargo feature, never in
  a release;
- `xtask`: the driver and the report;
- `.github/workflows/ci.yml`: one new job;
- `release.yml`: one more "not in the release" check;
- the GUI rules and the maintainers' testing page.

## Summary

**Add a CI job that starts the real app under a virtual display.** Inside
it, the app measures its own rendered screens, through the same web engine
it ships with, for every combination of:
- text size;
- language;
- theme;
- contrast;
- colour vision;
- window size;
- view.

**It fails on four kinds of defect:**
1. **overflow:** a box wider than its parent with nowhere to scroll;
2. **clipped labels:** a label cut off without an ellipsis that is allowed
   there;
3. **small targets:** an interactive element under 24 × 24 CSS px;
4. **contrast,** measured on the colours actually rendered, not only the
   declared tokens.

**It replaces most of the capture matrices** that RFC-097's handoffs needed.
Captures remain for what needs eyes: the worst case, plus a baseline when
something is compared.

## Motivation

**RFC-097 needed seven review rounds of captures** to settle layout
questions a machine could have answered:
- the stacked labels (F217);
- the collapsing views (F218);
- the gutter column (F219);
- the status bar's cascade, which the static `contrast` gate and my own
  review both missed (F220);
- the target sizes;
- the "? Shortcuts" wrap.

Each was found by a person looking at a picture of one combination. **The
same defects in the combinations nobody photographed are still unknown.**

**What exists today:**
- unit tests on state and logic;
- markup tests through Dioxus's virtual DOM (`rebuild_to_vec`), which see
  what is rendered but not where;
- **static gates** over the CSS source (`contrast`, `colour-vision`,
  `css --check`), which see declared pairs but not layout;
- **real-window captures,** costly and partial.

**Nothing in between measures layout.** But CI already runs the real app
headless: `render_check.py` (F34) starts the release build under
`xvfb-run` with a private session bus and measures the comparison view's
geometry through AT-SPI, in `release.yml` and `render-check.yml`. **This RFC
generalizes that path** and moves it into the gated Rust tooling (DEC-005,
F97).

## Design

### 1. The probe: a test-only feature in the app

A Cargo feature, **`layout-probe`**, off by default.
- **Never built for a release:** `release.yml` fails if the probe's marker
  string appears in a release binary, the same way it checks for the
  `show_to_person` test seam today.
- **With the feature on,** the app accepts a probe plan (a JSON file,
  through an environment variable that exists only in this build). For each
  combination, it:
  1. sets the state in-process (text size, language, theme, contrast,
     colour vision), through the same signals the settings use;
  2. opens each view from fixtures (the Explorer, a file comparison,
     Deep Compare, the mergetool layout, Settings, *Shortcuts*);
  3. waits for the view to settle, on a defined condition, not a sleep;
  4. runs a measurement script in the page through `document::eval`;
  5. writes one JSON result per view and combination, then exits.
- **The window size** is the one thing set at launch, by the driver. One
  app run per window size. Everything else changes in-process, so the
  matrix doesn't pay a launch for every combination.
- **No network.** It reads only fixtures under the project. It writes only
  its results file.

### 2. What the measurement checks

Inside the page, for every element of the view:

| Check | Rule | Allowed by |
|---|---|---|
| **Overflow** | content wider or taller than its box, with the box not a scroll container | the known scroll owners (diff panes, trees, lists), named in a list in the probe |
| **Clipped text** | `scrollWidth > clientWidth` on an element with visible text | elements whose CSS says `text-overflow: ellipsis`, plus the F220 status bar rules |
| **Off-screen controls** | an interactive element outside the viewport and not inside a scroll container that could bring it in | none |
| **Hit area** | an interactive element (`button`, `[role=button]`, `a`, `input`, `select`, `[tabindex]`) under 24 × 24 CSS px | WCAG 2.5.8's spacing exception, by named selector and stated reason |
| **Rendered contrast** | each text node's computed `color` against its effective background, walking up the tree and compositing `color-mix` and opacity as rendered: 4.5:1 (7:1 under *High*), 3:1 for control edges | the `contrast` gate's own exemptions (disabled controls; under *High*, their own 3:1 floor) |
| **Text size** | computed `font-size` below 12 px at text size 100 % | the `contrast` gate's non-text glyph list |
| **Label placement** (added 2026-11-17, review 216) | a labelled region's *visible* label isn't over that region: no horizontal overlap, or below the region's top, or another labelled region in between | none |
| **Nothing covers an open dialog** (added 2026-11-17, review 217) | with a dialog open, a point sampled inside the dialog's box hits an element that isn't the dialog or its descendant (`elementFromPoint` over a grid) | the toast: the app's only error notice, transient and dismissed by a click, so it sits above a dialog by design (review 218) |
| **Overlap** (added 2026-11-17, review 218) | two visible boxes that each hold text or are interactive, neither containing the other, both in normal flow (not sticky, fixed or absolute), intersect by more than 1 px in both directions; parts clipped out by their scroll container are skipped | none |

**Every allowance is a named entry with its reason.** A new one is a review
decision, as with the existing gates' allowlists.

### 3. The matrix

| Dimension | Values |
|---|---|
| Text size | 100, 125, 150, 175, 200 % |
| Language | English, Japanese |
| Theme | Dark, Light, Night (*System* resolves to one of them) |
| Contrast | *Standard*, *High* |
| Colour vision | *Standard*, red-green-safe |
| Window | 1000×700 (the listing's size), 800×560, 1366×768 |
| View | the Explorer, a file comparison with a collapsed region, Deep Compare with a selected row, the mergetool layout, Settings with *Advanced* open, *Shortcuts* |

That is 5 × 2 × 3 × 2 × 2 × 3 × 6 = 2,160 measurements if every check ran on every axis. **They don't need to** (amended 2026-11-17, review 216): the layout checks need text size × language × window × view at one colour setting (180), and rendered contrast needs theme × colour vision × contrast × view at two layouts, one side by side and one stacked (144). That is about a minute in all, so everything runs on every push. Colour vision and
theme change only colours, so the layout checks can run with both at a
single value, and only the rendered-contrast check needs the full colour
axes. The plan states which checks run on which axes, so the cost stays
proportionate.

**Budget:** under five minutes of CI time per push. **The first handoff
measures the real cost before the matrix is fixed.** If it is over budget,
the matrix is cut by the rule above, never by dropping a check.

### 4. The driver and the report: `cargo xtask layout-check`

- **Builds** the app with `layout-probe`.
- **For each window size,** runs it under `xvfb-run --auto-servernum
  dbus-run-session`, as `render_check.py` does, with an isolated data
  directory and fixtures under the project.
- **Reads the results** and prints every failure with its view, its
  combination, the element (selector path and text) and the measured
  value.
- **Writes a capture of each failing view** in its failing combination,
  as an artifact. A capture appears only where the machine already found
  something.
- **Falsifiable:** an `--inject <defect>` flag, local and CI-dispatch only,
  plants one known defect per check (an overflowing label, an undersized
  button, a low-contrast pair, 11 px text), and the check must catch it.

### 5. Where it runs, and what it doesn't cover

- **Linux CI, on every push to `main` and on every pull request.** It
  measures WebKitGTK, which is the Linux and Flatpak engine.
- **Windows (WebView2) and macOS (WKWebView) are not covered.** Their layout
  differs only in fonts and minor engine behaviour. They stay with RFC-078's
  per-release evidence runs and, where a change needs it, one worst-case
  capture.
- **Fonts:** CI installs one fixed font set, named in the job. Results hold
  for that set. The checks keep margins, so a different system font at
  similar metrics passes too, but that isn't a promise for every font a user
  might have.

### 6. What it replaces

- **RFC-097-style capture matrices** become one worst-case capture per
  question, plus the job's output.
- **`render_check.py`** (F34) can move to the same probe later. That is not
  part of this RFC.
- **The static gates stay:** `contrast` checks declared pairs in milliseconds
  and guards tokens that aren't on screen in any fixture. The probe checks
  what is rendered. Each catches what the other can't.

## Non-goals

- **Interaction checks** (focus order, keyboard paths), which can follow on
  the same probe in a later RFC.
- **Pixel-comparison baselines.**
- **Windows or macOS layout in CI.**
- **Measuring user-supplied fonts.**

## Security and privacy

- **The probe is never in a release.** It is a Cargo feature, and
  `release.yml` checks the binary for the marker string.
- **It reads fixtures only,** writes one results file, and uses no network.
- **Captures of failures show fixtures only,** and are kept as CI artifacts
  with the default retention.

## Verification of this RFC's implementation

- **Each check falsified** with `--inject`, quoted.
- **The probe's absence from a release build,** shown with the check
  falsified on a build made with the feature on.
- **The real cost of the full matrix,** measured on CI.
- **Retroactive proof:** check out the commits before the fixes for F217,
  F218, F219 and F220, run the job, and confirm it reports each defect. A
  checker that wouldn't have caught the defects we found by hand isn't
  doing its job.

## Decisions (owner, 2026-11-16)

Both were accepted as proposed:

_The questions, as they were put:_

1. **Every push, or every push plus a nightly full matrix?** The rules say
   every CI run. If the measured cost is over budget, I'd propose the full
   colour axes nightly and on release commits, with every push running the
   layout checks at one colour value. That cuts no check, only how often the
   colour-only axes repeat.
2. **Fixed fonts in CI:** Noto Sans and Noto Sans CJK JP (for the Japanese
   labels), as the job's named set. Is that acceptable as "the" reference?
