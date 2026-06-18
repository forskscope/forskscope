# RFC 070: Font Family Selector in Settings

**Status.** Implemented (v0.152.0)
**Tracks.** A user setting to choose the font family used in the diff panes (and
optionally the UI), beyond the current diff font *size* control.
**Touches.** `crates/forskscope-ui/src/state/settings.rs` (new setting),
`ui/settings.rs` (selector), `assets/main.css` (font-family variable),
i18n. Self-contained.

## Summary

Settings currently exposes diff font *size* but not *family*. Users comparing
code generally want a monospace font, and individuals have strong preferences
(ligatures, glyph shapes, width). This RFC adds a **font family selector** so the
diff text can be set to the user's preferred family, with sensible cross-platform
defaults.

## Motivation

Font choice materially affects readability for the diff workspace's core
audience (developers reading code side by side). It is a common, expected
setting in diff/editor tools and is low-risk and self-contained.

## Design

### Setting

A **Diff font family** setting. Two viable models:

- **A — Preset list.** A small curated list of families that map to safe
  cross-platform CSS font stacks: e.g. "System monospace", "Sans-serif",
  "Serif", plus a few common monospace families with fallbacks. Simplest,
  no free-text validation, predictable rendering.
- **B — Free-text family name.** The user types a family name; the app applies it
  with a monospace fallback. More flexible, but a typo yields a silent fallback
  and there is no discovery of what is installed.

Recommendation: **A (preset list)** for the first version, because it is
predictable and needs no font enumeration (which WebKitGTK does not expose
portably). A free-text "custom" entry can be added later if requested.

### Scope: diff font only, or UI font too?

The current design has a single diff-font-size control. This RFC adds diff font
*family*. Whether to also expose a **UI font family** is an open question; the
reverse-engineered design separated diff font and UI font families, so the data
model can support both. Recommend shipping **diff font family** first (highest
value), and adding UI font family only if asked, to avoid settings bloat (D-001).

### Application

The selected family sets a CSS variable (e.g. `--diff-font-family`) on the diff
panes, parallel to the existing font-size variable. Default remains the current
monospace stack so existing users see no change unless they opt in.

### Persistence

Persisted with the other settings; applied immediately on change (consistent
with theme/size, which apply live).

## Non-goals

- No font *file* loading / embedding — only families available on the system (or
  generic CSS families) are used.
- No per-tab font override — this is a global preference.
- No font enumeration UI (WebKitGTK does not expose installed fonts portably);
  the preset list avoids needing it.

## Acceptance criteria

- A **Diff font family** setting changes the diff panes' font live.
- The default is unchanged from today (existing users see no difference until
  they choose).
- The setting persists across launches.
- Unknown/unavailable families fall back gracefully to monospace.

## Cross-references

- Existing diff font *size* setting — this parallels it.
- Reverse-engineered design — separate diff-font and UI-font families (data
  model precedent).
- RFC-017 — styling, theme tokens (font-family variable lives among the tokens).
- D-001 — avoid settings bloat (ship diff font family first; UI font only if
  asked).

## Open questions

- Preset list contents — which specific families/stacks to offer. Keep short and
  cross-platform-safe.
- Whether to also add UI font family now or defer. Recommend defer.
