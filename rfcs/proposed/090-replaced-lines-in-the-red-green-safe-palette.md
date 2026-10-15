# RFC 090: Replaced Lines in the Red–Green-Safe Palette

**Status.** Proposed — awaiting the owner's acceptance. Implementation does not
start before this moves to `accepted/` (RFC-000, 5-folder variant).
**Scheduling.** Planned for `0.192.0`. See `ROADMAP.md` § "Remaining proposed
RFCs".
**Tracks.** F199.
**Amends, on acceptance.** RFC-088 (§2, the palette's token list; §3, the gate's
pairs). RFC-088 gets a one-line Status note pointing here.
**Touches.** `crates/forskscope-ui/assets/css/01-tokens.css` (one token),
`xtask`'s `colour-vision` gate, and `docs/src/users/settings.md`.

## Summary

The red–green-safe palette changes removed and added lines to orange and blue,
but leaves **replaced** lines (`--rep-bg`) in their amber. Under red–green
colour blindness, amber sits close to the new orange, so a removed line and a
replaced line look alike. In the Light theme the palette made that pair
**worse** than Standard.

This RFC gives replaced lines a muted teal in the red–green-safe palette, and
extends the gate to measure all three pairs.

## Motivation

RFC-088 measured one pair, removed against added. Review 163 found the gap in
the deuteranopia-simulated screenshots of request 153b. Removed vs replaced
background, CIE76 ΔE, under deuteranopia:

| Theme | Standard | Red–green safe (0.191.0) |
|---|---|---|
| Dark | 5.0 | 9.9 |
| Light | 13.8 | **4.6** |
| Night | 1.3 | 8.2 |

Under normal vision, Light also fell from 21.7 to 9.4.

Other cues remain: a removed line has an empty line opposite it, and a replaced
line has text on both sides. But the palette exists so the pattern of changes
reads **at a glance**, and replaced lines are the most common kind of change.

## Design

### 1. One token, red–green safe only

| Token | Dark | Light | Night |
|---|---|---|---|
| `--rep-bg` (replaced line) | `#35504c` | `#abbab7` | `#2c4042` |

- **One muted teal family** across the themes, so a user who switches theme
  meets the same hue. Saturation is low, because replaced lines are the most
  common change and a strong colour would dominate the view.
- **The Standard palette is unchanged.**

### 2. Measured

The red–green-safe palette, CIE76 ΔE, exact CIE constants (the gate's).

**Replaced vs removed:**

| Theme | Normal | Deuteranopia | Protanopia | Tritanopia |
|---|---|---|---|---|
| Dark | 33.8 | 26.2 | 23.7 | 38.1 |
| Light | 26.9 | 26.2 | 21.6 | 25.7 |
| Night | 28.8 | 24.4 | 22.0 | 31.8 |

**Replaced vs added:**

| Theme | Normal | Deuteranopia | Protanopia | Tritanopia |
|---|---|---|---|---|
| Dark | 26.4 | 24.3 | 23.8 | 12.5 |
| Light | 22.0 | 21.4 | 21.5 | 17.3 |
| Night | 25.0 | 23.1 | 22.0 | 9.7 |

**Text on the new background** (WCAG contrast): 7.0 (Dark), 7.9 (Light), 7.1
(Night), against 4.5 required.

**Tritanopia** (blue–yellow) brings teal and blue closer, at 9.7 to 17.3. It
stays outside RFC-088's scope, as RFC-088's non-goals say.

**How the colours were chosen:** a search over hue, saturation and lightness for
each theme, under these constraints:

- text contrast of at least 7;
- a visible difference from the plain background, ΔE 8 to 30;
- saturation of at most 0.3.

The search maximised the smallest ΔE against both removed and added, under
normal vision, deuteranopia and protanopia.

Saturated violet reached ΔE 35 to 39 but was rejected as too loud for the most
common change. A neutral grey reached only 16.9 in Light.

### 3. The gate

`cargo xtask colour-vision` measures **three** pairs per theme:

- removed–added;
- replaced–removed;
- replaced–added.

| Palette | Pairs | Simulations | Each must reach |
|---|---|---|---|
| Red–green safe | all three | normal, deuteranopia, protanopia | ΔE ≥ 20 |
| Standard | removed–added | normal | ΔE ≥ 20, unchanged |
| Standard | the two replaced pairs | normal | ΔE ≥ 15 |

The Standard replaced pairs measure 16.2 to 21.7 today, so the 15 floor guards
them against regression without redesigning a palette that is not this RFC's
subject.

Falsify the gate by setting the red–green-safe `--rep-bg` back to the Standard
amber: it fails on Light.

## Non-goals

- Changing the Standard palette.
- A tritan palette.
- Any token other than `--rep-bg`.
- A non-colour mark for replaced lines. The `~` sign RFC-024 suggests is not
  needed once the colours separate, and it would change every user's view.

## Acceptance criteria

1. The red–green-safe `--rep-bg` carries the values in §1, in all three themes.
   Standard is unchanged.
2. `cargo xtask colour-vision` measures the three pairs as §3 says. Its figures
   match §2 to one decimal. It runs in CI, and it fails when `--rep-bg` is set
   back to amber.
3. Screenshots show one comparison with a removed, an added and a replaced line
   (with *Inline diff* on), in each theme with red–green safe, each beside its
   deuteranopia simulation. Each is described after being looked at.
4. `CHANGELOG.md`'s 0.191.0 known limitation is resolved in 0.192.0's entry.

## Open questions

None.
