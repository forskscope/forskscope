# RFC 059: Explorer and Compare UI/UX Audit Remediation

**Status.** Proposed — core-testable slice implemented (v0.41.0); UI/keyboard slice open

## Status
Partially implemented in v0.41.0. The core-testable items shipped:

- **H1** — CSS de-duplicated: three conflicting `.explorer` rules collapsed
  to one; dead 5-column `.row` rule removed. One rule of each remains.
- **M2** — Typed `DigestKey` enum replaces the stringly-typed `r:` prefix
  in `explorer.rs`.
- **M5** — `compute_aligned_rows` / `merge_level` extracted into a new
  `forskscope-explorer-align` crate (no GUI dependency); 9 unit tests added
  covering pairing, one-sided rows, ordering, and recursive expansion;
  `explorer.rs` reduced from 426 ELOC to 354.
- **L5** — Unjustified `unsafe impl Send/Sync` removed from
  `FilteringExecutor`; `IgnoreRules` is `Send + Sync` by auto-impl.

Remaining open items (all require UI testing): H2 + H3 (Explorer keyboard
completeness → RFC-019), M1 (digest cache lifetime → RFC-023), M3 + L3
(diff nav feedback → RFC-024), M4 (search traversal → RFC-014), L1
(i18n coverage → RFC-009), L2 (nav button aria-labels → RFC-019). This RFC consolidates the audit findings into a
single tracked remediation, cross-referencing the existing RFCs that own
each affected area so nothing is renumbered or duplicated.

## Summary

The v0.40.0 audit reviewed `explorer.rs`, `dir_pane.rs`, `diff.rs`,
`hunk.rs`, `diff_actions.rs`, and `main.css` against the GUI/UX external
design, the non-goals/Don't list, the "less is more" principle, and the
accessibility requirements. The two surfaces are broadly healthy — unified
single-grid rows make diff scroll-sync structural, colour-independence is
real (glyphs + `sr-only` labels), and merge truth lives in core. This RFC
fixes the gaps the audit found, in priority order, and records which
findings extend already-proposed RFCs versus which are new here.

## Motivation

A diff/merge tool's two primary surfaces must be keyboard-complete, cheap to
operate on real (large) trees, and free of latent layout bugs. The audit
found one spec-level accessibility failure (Explorer is mouse-only for the
core select-and-compare loop), one performance cliff (digest cache cleared
on every expand), and several correctness/cleanliness issues. Capturing them
as a single RFC keeps the remediation coherent and reviewable.

## Findings and ownership

Each finding is tagged with its audit ID, severity, and the RFC that owns
the design surface. Findings without an existing owner are **owned by this
RFC**.

| ID | Severity | Area | Owner |
|----|----------|------|-------|
| H1 | High | Dead/conflicting CSS (`.explorer` ×3, `.row` ×2) | **RFC-059** |
| H2 | High | Explorer keyboard selection across both panes | RFC-019 (extends) |
| H3 | High | Keyboard "compare selected" / "compare same-name" | RFC-019 (extends) |
| M1 | Med | Digest cache cleared on every toggle | RFC-023 (extends) |
| M2 | Med | Digest key namespacing (`r:` string prefix) | **RFC-059** |
| M3 | Med | Diff hunk navigation wrap with no edge feedback | RFC-024 (extends) |
| M4 | Med | Search is substring-only; no next/prev traversal | RFC-014 (extends) |
| M5 | Med | `explorer.rs` 426 ELOC; `compute_aligned_rows` untested | **RFC-059** |
| M6 | Med | Path bar / folder-pick affordance polish | RFC-055 (done; minor follow-up) |
| L1 | Low | Hardcoded English mode labels and tooltips | RFC-009 (extends) |
| L2 | Low | Nav arrow buttons lack `aria_label` | RFC-019 (extends) |
| L3 | Low | No "applied N/M" confirmation feedback | RFC-024 (extends) |
| L5 | Low | Unnecessary `unsafe impl Send/Sync` | **RFC-059** |
| Spec | Med | Missing size / last-modified columns in Explorer | RFC-020-listing¹ |
| Spec | Low | No minimap/overview (optional in D-003) | RFC-024 (defer) |

¹ Directory listing/metadata model. If no proposed RFC owns the
listing-columns decision, this RFC records the product question (below) and
defers the implementation to the Explorer listing owner.

## Scope

### Owned and specified here (RFC-059)

**H1 — CSS de-duplication.** `main.css` defines `.explorer` three times
(flex-column, two-column grid, flex-column) and `.row` twice (5-column then
7-column). Last-wins silently resolves these; the superseded rules are dead
code and the grid `.explorer`/`.deep-compare { grid-column: 1/-1 }` pairing
never applies. Resolution:
- Decide the single intended Explorer layout (the flex-column shell with an
  inner aligned grid is what the component actually renders; the standalone
  two-column `.explorer` grid at line ~267 is the orphan). Keep one.
- Delete the dead 5-column `.row` rule; keep the 7-column rule that matches
  the rendered grid (`4ch 1.2ch 1fr 5ch 4ch 1.2ch 1fr`).
- Add a CSS lint/check step (RFC-020 CI gates) that flags duplicate
  top-level selectors to prevent regression.

**M2 — Typed digest keys.** Replace the stringly-typed digest map
(`rel_path` for left, `PathBuf::from("r:").join(rel)` for right-only) with a
typed key:
```rust
enum DigestKey { Common(PathBuf), RightOnly(PathBuf) }
```
This removes the aliasing footgun (a real file named `r:`), makes the
left/right lookup unambiguous, and is a precondition for the M1 cache work.

**M5 — Module split and algorithm tests.** Extract the aligned-row merge
(`compute_aligned_rows` / `merge_level` and the `RowData` / `AlignedRow`
types) from `explorer.rs` into a sibling module (e.g.
`explorer/align.rs`), bringing `explorer.rs` under the 300-ELOC guideline.
The extracted alignment functions take plain data (flat row tuples + roots)
and return plain data, so they are **unit-testable without the GTK
runtime** — add a test module covering: same-name pairing, one-sided rows
(spacer on the other side), directories-first ordering, nested expansion
recursion, and case/locale ordering. The two near-identical pane-half render
blocks should also collapse into one parameterized half-renderer.

**L5 — Remove unnecessary `unsafe`.** `FilteringExecutor` asserts
`unsafe impl Send/Sync`. If `IgnoreRules` is already `Send + Sync` (it should
be — it is plain data), the unsafe impls are unnecessary and must be removed;
if a genuinely non-`Send` field exists, document why. No raw `unsafe` should
remain without justification (RFC-020 review gate).

### Extends existing RFCs (specified there, cross-referenced here)

**H2 + H3 — Explorer keyboard completeness (RFC-019).** The aligned-tree
keydown handler delegates only to the left tree; the right pane and the
select-and-compare loop are mouse-only, failing the spec's "no mouse-only
path" criterion (§8.6: Tab between panes, Ctrl+Enter compare selected, Enter
to compare same-name). RFC-019 (command registry + shortcuts +
accessibility) is the owner; this RFC adds the concrete requirements:
- A focused side (left/right) with `Tab` / `Shift+Tab` to switch panes.
- `Space` selects the focused file as that side's pick.
- `Enter` on a same-name file opens the comparison; `Ctrl+Enter` compares
  the current left/right picks.
- `Alt+Up` already moves both panes up; keep it.
- Every Explorer command flows through the RFC-019 command registry, not an
  ad-hoc keydown match.

**M1 — Digest cache lifetime (RFC-023 / RFC-023-adjacent digest policy).**
`on_toggle` currently calls `digest_map.write().clear()`, discarding every
computed digest on each expand/collapse and forcing full re-computation
(O(files) byte-compares per toggle). The digest-cache policy owner must
define stable keying (see M2) and **subtree-scoped invalidation** so a
toggle only invalidates entries under the toggled directory. Until then, the
clear-on-toggle is a known performance cliff on large trees.

**M3 + L3 — Diff navigation and apply feedback (RFC-024).** `move_focus`
wraps with `rem_euclid` and gives no edge cue; `apply_focused_hunk`
auto-advances but offers no aggregate confirmation. RFC-024 (diff visual
semantics) should specify: an edge indication on wrap (or a brief toast),
and an "applied N / M changes" surface (status bar or toolbar).

**M4 — Search traversal (RFC-014).** Current search is case-insensitive
`contains` with a match count but no next/prev jump and no scroll-to-first.
RFC-014 (search/filter/navigation) owns the upgrade: F3 / Shift+F3 (or
Enter / Shift+Enter) match stepping, scroll-into-view, and a considered
decision on whole-word / regex (regex behind disclosure to honour D-012).

**L1 — Localization of Explorer labels (RFC-009).** Mode buttons
("Browse" / "Directory Report") and several `title=` tooltips bypass
`t(lang, …)`. RFC-009 (localization) owns the requirement that all
user-visible strings route through i18n; this RFC records the specific
offenders found.

**L2 — Glyph-button labels (RFC-019).** Toolbar nav arrows (`◀ ▶`) have
`title` but no `aria_label`, unlike Save/Undo/search. RFC-019 a11y pass.

### Deferred / product decisions

- **Size and last-modified columns (Spec/Med).** The v0.22 baseline and the
  GUI wireframe (§9) show size + mtime per entry; the current aligned tree
  shows name + status glyph only. This is either an intentional "less is
  more" simplification or a regression. **Product decision required**
  (Open Questions). Recommendation: show file size as a secondary/aligned
  column or on-hover, since a size mismatch is a fast pre-digest signal; keep
  mtime out of the default to avoid noise.
- **Minimap/overview.** Listed as *optional* in D-003. Defer; not required
  for parity.

## Acceptance Criteria

- `main.css` contains exactly one `.explorer` and one `.row` top-level rule;
  a CI check flags duplicate top-level selectors.
- The Explorer digest map uses a typed key; no string-namespaced paths.
- `compute_aligned_rows` (and `merge_level`) live in their own module with a
  unit-test suite covering pairing, one-sided rows, ordering, and nested
  expansion; `explorer.rs` is under 300 ELOC.
- No unjustified `unsafe` remains in `dir_pane.rs`.
- The full Explorer select-and-compare loop is completable with the keyboard
  alone (verified per RFC-019 acceptance).
- Expanding/collapsing a directory does not recompute unrelated digests
  (verified per the digest-cache policy).
- Diff search supports next/previous match traversal with scroll-into-view.
- All Explorer user-visible strings route through i18n.

## Dependencies

- RFC 005 — Explorer Workspace (**shipped**) — the surface under audit.
- RFC 006 — Diff/Merge Workspace (**shipped**) — the surface under audit.
- RFC 009 — Settings, Theme, Localization, Accessibility — L1.
- RFC 014 — Search, Filter, and Navigation — M4.
- RFC 019 — Command Registry, Shortcuts, Palette, Accessibility — H2, H3, L2.
- RFC 020 — Developer Architecture, CI, and Test Gates — H1 lint, L5 review.
- RFC 023 — Atomic File Operations / digest policy — M1, M2.
- RFC 024 — Diff Visual Semantics and Decoration Contract — M3, L3.
- RFC 054 — Explorer Tree-View and Interaction Model (**shipped**) — the
  interaction baseline these findings refine.

## Open Questions

- **Size/mtime columns:** restore per the GUI wireframe, or keep the
  minimal name+status view? (Product decision; recommendation above.)
- **Layout authority for H1:** confirm the flex-column shell + inner aligned
  grid is the intended Explorer layout before deleting the orphan grid rule.
- **Search regex:** ship whole-word only for v1, or include regex behind
  disclosure? (Coordinate with RFC-014.)
- **Should the alignment module move to `forskscope-core`?** It is pure data
  logic and core-testable, but it is also UI-presentation-specific
  (visible-row merging). Recommendation: keep it in the UI crate as a
  testable submodule, not core.
