# Handoff 078 — 0.191.0: a colour-vision setting, and one bar at the top

**From:** architect. **Date:** 2026-10-15. **Release:** `0.191.0`.
**Implements:**

- **RFC-088** (`rfcs/accepted/088-colour-vision-palette.md`)
- **RFC-089** (`rfcs/accepted/089-one-bar-at-the-top.md`)

Both were accepted by the owner on 2026-10-15.

**Riders:** F196, and review 161's `role="none"`. Both are small fixes that need
no RFC.

**The RFCs are the authority.** Where this document and an RFC differ, the RFC
wins. If implementation shows an RFC is wrong anywhere, stop and tell me; I take
it back to the owner. Do not change the design yourselves, and do not edit any
RFC. This handoff only adds how to build and verify it.

---

## 1. RFC-088: the colour-vision setting

Build exactly RFC-088 §1 to §3.

**The setting:**

- **Field:** `PersistedSettings.colour_vision`, with `#[serde(default)]`,
  `Standard`. Serialized as `standard` / `red-green-safe`.
- **Fixtures:** pin both values in the golden fixtures (the F26/F27 pattern).
- **Migration test:** a 0.190.x settings file with no field loads as `standard`.
- **UI:** the select sits directly under *Theme*, outside *Advanced*. It applies
  at once.

**The palette:**

- Use a root class beside the theme class, for example `.cv-rgsafe` on the same
  element. Combined selectors (`.theme-dark.cv-rgsafe`) override **only** the
  four tokens in RFC-088 §2's table, with exactly those values.
- `cargo xtask css` regenerates `main.css`.

**The gate:** `cargo xtask colour-vision`, as RFC-088 §3 specifies.

- Parse `01-tokens.css`. Do not duplicate the values in Rust; the gate must read
  what ships.
- Simulate with the Machado et al. (2009) severity-1.0 matrices in linear RGB,
  then measure CIE76 ΔE in CIELAB (D65). Cite the source in the module doc.
- Check the WCAG contrast of text and marks.
- Add it to `ci.yml` beside the other xtask gates.
- **Falsify:** set one red–green-safe token to its Standard value, and the gate
  fails, naming the pair and the simulation.
- **Cross-check:** the gate's own numbers for the red–green-safe pairs must
  match RFC-088's tables to one decimal. If they do not, stop and tell me; the
  RFC's numbers or the gate is wrong.

## 2. RFC-089: one bar at the top

Build exactly RFC-089 §1 and §2.

**The header and the tab bar:**

- Remove `Header` from `app.rs`, and delete `ui/layout/header.rs` and its CSS
  (`.header`, `.brand`).
- In `TabBar`, after the `role="tablist"` element: a spacer, then the *Settings*
  and *?* buttons. Same labels, titles and actions as today.
  - They are **siblings of the tab list**, never inside it, so RFC-087's
    arrow-key navigation cannot reach them.
- They stay at the end of the bar however many tabs are open. Check:
  - **10 tabs at 1280 px;**
  - **under 720 px**, where the bar already wraps (`90-responsive.css`).
- The keyboard tests that render `Header` render the tab bar instead.

**The window title:**

- One effect on the selected tab, the tab titles and the unsaved state calls the
  desktop window's `set_title`, following RFC-089 §2's table exactly.
- The tab title text comes from the same function the tab bar uses, so the two
  cannot disagree.
- `main.rs`'s initial `with_title("ForskScope")` stays: it is the Explorer's
  title.

## 3. Rider: F196, the collapsed-lines divider

`ui/view/hunk.rs:100`'s *"··· N unchanged lines — click to expand ···"* is a
`div` with `onclick` only.

- Make it a real `<button>` with the same text and look. Enter and Space then
  expand it natively.
- Its accessible name says what it does: *Show N unchanged lines*, and the
  Japanese to match.
- Test it, and show it live: Tab reaches it, and Enter expands it.

This restores RFC-019 §10's *keyboard path*. It is a defect fix.

## 4. Rider: `role="none"` on `.tab-slot` (review 161)

Add `role="none"` to the `.tab-slot` wrapper, so `tablist` → `tab` stays direct
on Chromium-based WebView2 as well as on WebKitGTK.

Re-run the AT-SPI check from request 152:

- tabs are still `page tab`;
- `do_action` selects;
- the close buttons are still outside the tabs.

## 5. Verification

Use F171's procedure, plus screenshots:
`niri msg action screenshot-window --id <id> --path <absolute path>`. Run the app
sandboxed, with invented files only. See the README screenshots, commit
`624cc54`.

1. **Unit tests through the functions the app calls,** each falsified:
   - the setting round-trips, and an old file loads as `standard`;
   - the root class follows the setting;
   - the window title for each row of RFC-089 §2, as a pure function
     `(selected tab, title, dirty) → string` that the effect calls;
   - the divider is a button.
2. **RFC-088's acceptance criteria 1 to 5, one by one.** For criterion 4:
   - take six screenshots: Dark, Light and Night, in each palette;
   - pass each red–green-safe one through a deuteranopia simulation with the
     same Machado matrix, for example
     `magick in.png -colorspace RGB -color-matrix "<3x3>" -colorspace sRGB out.png`;
   - include both the original and the simulated image.
3. **RFC-089's acceptance criteria 1 to 5, one by one.** Read the window title
   from `niri msg -j windows` at each step of criterion 3.
4. **Riders:** F196 live, and the AT-SPI tree after `role="none"`.

## Scope

- **In:** §1 to §4, `settings.md` and the other user pages RFC-089 makes stale,
  the keyboard reference if anything moves, and i18n (English and Japanese).
- **Out:**
  - Any palette token not in RFC-088 §2.
  - A tritan palette.
  - Button contrast (F172).
  - Any other control move.
  - The broad documentation corrections (F195), which are a separate handoff
    after this release.
  - `CHANGELOG.md`, `ROADMAP.md`, the version.
  - RFCs.

## Gates

The standard set, including the Windows-target clippy with `--all-targets` and
`cargo fmt --manifest-path xtask/Cargo.toml --check`, `cargo xtask` with
`css --check`, `i18n`, `rfc-sync`, `audit-deps`, `version-sync`,
`ui-logic-connectivity`, `ui-logic-docs`, `xlsx-fixtures --check`, the new
`colour-vision`, `pages-origin-check` and `pages-link-check` on an assembled
site, `mdbook build docs`, `actionlint`, and a green CI run with its ID.

Reply in `.git-exclude/review-request/153-colour-vision-and-one-bar.md`.
