# RFC 088: A Colour-Vision Setting with a Red–Green-Safe Palette

**Status.** Implemented (`0.191.0`, `e3a5a26`/`3f3b409`/`8f449f2`; reviews 162–163). **Open:** F199. Removed vs replaced lines were not measured, and in Light the red–green-safe palette brings them closer. Addressed by [RFC-090](../accepted/090-replaced-lines-in-the-red-green-safe-palette.md) (accepted 2026-10-15), which amends §2 and §3.
**Accepted.** 2026-10-15 by the project owner.
**Scheduling.** `0.191.0`. See `ROADMAP.md` §
"Remaining proposed RFCs".
**Tracks.** The owner's question, 2026-10-14: is there a plan for users with
colour vision deficiency (CVD)? There was none. The owner accepted the direction
on 2026-10-15.
**Amends.** RFC-009 (§6, theme semantics: a second, independent
axis beside the theme) and RFC-024 (the colour/background affordance per
semantic state). Each carries a one-line Status note pointing here.
**Touches.**
- `crates/forskscope-ui/assets/css/01-tokens.css` (palette tokens)
- `crates/forskscope-core/src/persist/schema/settings.rs` (a new setting)
- `crates/forskscope-ui/src/state/settings.rs`
- `crates/forskscope-ui/src/ui/view/settings/modal.rs`
- an `xtask` gate
- i18n
- `docs/src/users/settings.md`

## Summary

Add a **Colour vision** setting beside *Theme*, with two choices:

- **Standard**: today's colours.
- **Red–green safe**: removed is orange and added is blue.

It combines with every theme (Dark, Light, Night), so there are no new themes
to maintain. A build gate simulates colour vision deficiency on every palette, so
a later colour change cannot quietly undo it.

## Motivation

RFC-009, RFC-019 and RFC-024 require that no state is shown by colour **alone**,
and the app follows that:

- changed lines carry `+` / `−` marks;
- Explorer rows carry `=` `≠` `←` `→`;
- unsaved tabs say *unsaved*.

So the meaning survives without colour. **The at-a-glance cue does not.**

Measured on 2026-10-14 with the Machado et al. (2009) simulation, at full
severity. The numbers are CIE76 colour difference; below about 10, two colours
are hard to tell apart at a glance.

| Removed vs added, today | Normal | Deuteranopia | Protanopia |
|---|---|---|---|
| Line background, Dark | 34.5 | **2.0** | 9.5 |
| Line background, Light | 28.9 | **3.7** | 8.4 |
| Line background, Night | 36.5 | **3.7** | 10.7 |
| Character mark, Night | 118.5 | **8.2** | 31.9 |

Under deuteranopia, the most common red–green deficiency, removed and added
lines are nearly the same colour in every theme. The reader has to find the `+`
or `−` mark on each line instead of seeing the pattern of changes at once, and
seeing that pattern at once is what a diff view is for.

## Design

### 1. The setting

- `PersistedSettings.colour_vision: ColourVision`, with two values: `standard`
  (the default) and `red-green-safe`.
- **The schema follows RFC-076:**
  - add `#[serde(default)]` with `Standard` as the documented default;
  - a settings file without the field loads as `standard`;
  - pin both values in the golden fixtures, as F26 and F27 require, so a rename
    cannot pass silently;
  - a migration test loads a 0.190.x file.
- **In the Settings dialog,** a select labelled *Colour vision* / *色覚*, placed
  directly under *Theme*, **not** under *Advanced*. A user who needs it must find
  it without knowing it exists, so it belongs beside the setting it modifies. It
  does not conflict with the owner's rule that detailed settings stay hidden,
  because this is not a detail. Choices:
  - *Standard* / *標準*
  - *Red–green safe (blue and orange)* / *赤緑の区別に配慮（青とオレンジ）*
- It applies at once, like *Theme*.

### 2. The palette

The setting adds a class to the root, beside the theme class. The red–green-safe
values override only these tokens:

| Token | Dark | Light | Night |
|---|---|---|---|
| `--del-bg` (removed line) | `#4d2e14` | `#ffe2c7` | `#3d2410` |
| `--ins-bg` (added line) | `#14304d` | `#d4e6ff` | `#10284a` |
| `--del-mark` (removed mark, character highlight) | `#ff9b4a` | `#944800` | `#f0883e` |
| `--ins-mark` (added mark, character highlight; Explorer `=`) | `#5aa9ff` | `#0a5fd6` | `#4fa3ff` |

Every other token is unchanged. `--warn` (Explorer `≠`) and `--rep-bg`
(replaced lines) are already told apart from the new marks by hue and glyph.

**Measured, red–green safe** (CIE76, removed vs added):

| Pair | Normal | Deuteranopia | Protanopia | Tritanopia |
|---|---|---|---|---|
| Background, Dark | 44.7 | 45.9 | 40.6 | 39.3 |
| Background, Light | 31.7 | 32.7 | 28.6 | 24.6 |
| Background, Night | 42.1 | 44.3 | 39.3 | 34.0 |
| Mark, Dark | 110.5 | 112.5 | 98.4 | 88.9 |
| Mark, Night | 112.5 | 114.4 | 99.5 | 97.1 |
| Mark, Light (`#944800` / `#0a5fd6`) | 116.6 | 119.4 | 107.5 | 78.4 |

*Corrected 2026-10-15 (review 162):* the three protanopia mark figures were
98.5, 99.6 and 107.6. The values sit on a rounding boundary. The architect's
first measurement used the approximate CIE constants (`0.008856`, `7.787`);
the gate uses the exact ones (`216/24389`, `24389/27`) and gives the figures
above. No threshold or decision is affected.

**Readability (WCAG contrast ratio):**

- Text on the new backgrounds: 9.3 to 12.8, against 4.5 required.
- Each mark on its own background: 5.3 or more. The Light removed mark was
  darkened from `#b35900` (3.9) to `#944800` (5.3) for this.

### 3. The gate: `cargo xtask colour-vision`

It parses `01-tokens.css` and, for each theme and palette, measures the
removed/added pairs: the backgrounds, and the marks.

| Palette | Simulations | Each pair must reach |
|---|---|---|
| Standard | Normal vision only (its known limit is the reason this RFC exists) | ΔE ≥ 20 |
| Red–green safe | Normal, deuteranopia and protanopia | ΔE ≥ 20 |

It also checks the WCAG contrast of text and marks on their backgrounds: ≥ 4.5
for text, ≥ 3 for marks.

- It runs in CI with the other gates.
- Falsify it with one token changed to fail.
- The simulation matrices and the colour maths live in the gate with their
  source cited. No new runtime dependency.

## Non-goals

- A blue–yellow (tritan) palette. The red–green-safe palette already measures
  well for it (above), and tritanopia is rare. Revisit on request.
- A high-contrast theme. The button-contrast question is F172's, the owner's.
- Colours outside the diff and Explorer status: buttons, focus rings, the
  accent.

## Acceptance criteria

1. *Colour vision* appears under *Theme*, and switching it changes the diff
   colours at once in all three themes.
2. The choice persists, and an old settings file loads as *Standard*.
3. `cargo xtask colour-vision` passes, runs in CI, and fails when one token is
   made to fail.
4. Screenshots of one comparison in each theme, in both palettes, six in all.
   Each red–green-safe one is also shown passed through a deuteranopia
   simulation, so the reviewer sees what the user sees.
5. The user guide's Settings page describes the option.

## Open questions

None.
