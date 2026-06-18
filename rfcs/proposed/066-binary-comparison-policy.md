# RFC 066: Binary Comparison Policy — Off by Default with Explicit Opt-In

**Status.** Proposed
**Tracks.** Treating binary comparison as an explicit, opt-in capability rather
than a default; showing binary files in the Explorer as visibly non-actionable
(not hidden) with a clear reason; a settings toggle to enable binary comparison.
**Touches.** `crates/forskscope-ui/src/state/settings.rs` (new setting),
`ui/explorer.rs` (binary row state), `ui/settings.rs` (toggle), `state/mod.rs`
(open-compare guard), and the Explorer digest/row model. Core binary
classification (`FileKind::Binary`) is unchanged.

## Summary

ForskScope renders binary files as a hex dump and diffs the hex text. Comparing
two binaries this way is rarely meaningful — a one-byte insertion shifts every
subsequent row, producing a wall of "differences" that tells the user nothing.
This is precisely the "weak comparison that looks credible but produces poor or
misleading results" that the product's own non-goals warn against (NG-005, D-009).

This RFC makes **binary comparison off by default**. Binary files remain
**visible** in the Explorer (not hidden — D-015), shown as non-actionable with a
clear reason and a pointer to the setting that enables them. A single setting,
**Enable binary comparison**, turns the capability on; when on, binary files
become actionable and open the hex comparison (run asynchronously per RFC-065).

## Motivation

From the product constitution:

- **NG-005:** "Weak document comparison is worse than no document comparison. It
  creates false confidence."
- **D-009:** don't ship comparison modes that "look credible but produce poor or
  misleading results."
- **D-015:** "Don't hide unsupported cases… Do not show empty panes or partial
  results without warning."

The synthesis of these three: don't *hide* binaries (D-015), but don't *invite*
a misleading comparison by default either (NG-005, D-009). The resolution is an
explicit, reasoned, off-by-default state with a discoverable opt-in.

## Design

### Setting

A new boolean setting **Enable binary comparison** (default: **off**), persisted
with the other settings. Placed under the **Advanced** disclosure in Settings
(RFC-063 C6), since it is a power-user capability with caveats.

When off (default): binary files cannot be opened for comparison.
When on: binary files open the hex comparison, loaded asynchronously (RFC-065).

### Explorer row state for binary files

Binary files are shown in the Explorer with an explicit, non-actionable
treatment — **not** hidden, per D-015:

- A `binary` badge on the row.
- The row is visually de-emphasised (muted) and its compare affordance is
  disabled while the setting is off.
- Tooltip / accessible label: "Binary file. Binary comparison is off — enable it
  in Settings → Advanced." (final wording in i18n).
- When the setting is on, the row becomes actionable like any other file and the
  badge remains (so the user still knows it is binary) but the disabled state is
  removed.

This keeps the file visible and the reason explicit (D-015) while preventing the
misleading default comparison (NG-005).

### Open-compare guard

`open_compare` (and the same-name / pick-pair paths) check the setting before
opening a binary pair:

- If either side is binary and the setting is off → do not open a diff tab;
  surface a brief, friendly notice ("Binary comparison is off. Enable it in
  Settings → Advanced.") rather than silently doing nothing (D-015).
- If on → open as today, but via the async lifecycle (RFC-065), because large
  binaries are the freeze risk this composes with.

### Relationship to the "hide binary" filter (RFC-067)

RFC-066 makes binaries *non-actionable by default but visible*. RFC-067 adds an
optional **filter** to *hide* binaries from the list for users who want a
cleaner view in binary-heavy directories. These are different controls:
RFC-066 is a capability gate (can I compare this?), RFC-067 is a view filter (do
I want to see this row at all?). They compose; neither replaces the other.

## Non-goals

- No semantic binary diff, no hex *editor*, no image diff (all remain non-goals
  per NG-005 / NG-010).
- This RFC does not change how a binary file is *rendered* once comparison is
  enabled (still the existing hex preview).
- Excel `.xlsx` (derived-text) comparison is unaffected — it is not classified as
  Binary.

## Acceptance criteria

- With the setting off (default), binary files appear in the Explorer with a
  `binary` badge, muted, non-actionable, with a clear reason on hover/focus.
- Attempting to compare a binary pair with the setting off produces a friendly
  notice, not silence and not an empty tab.
- With the setting on, binary pairs open the hex comparison via the async
  lifecycle and do not freeze the app.
- The setting persists across launches and lives under Settings → Advanced.

## Cross-references

- non-goals NG-005, NG-010; Don't-list D-009, D-015.
- RFC-065 — async comparison (how enabled binary compares run without freezing).
- RFC-067 — Explorer filters (optional hide-binary view filter).
- RFC-063 C6 — Advanced settings disclosure (where the toggle lives).

## Open questions

- Should enabling the setting apply to already-open Explorer views immediately
  (reactive) or only to subsequently opened comparisons? Reactive is friendlier
  and matches how other settings behave; confirm no performance cost.
- Exact badge glyph/word for "binary" in the row — align with the existing
  status-icon vocabulary.
