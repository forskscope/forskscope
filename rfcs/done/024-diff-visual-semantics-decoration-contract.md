# RFC 024 — Diff Visual Semantics and Decoration Contract

**Status.** Implemented (v0.61.0) — core complete; renderer wiring in Dioxus diff component deferred to UI layer

## Status

Proposed.

## Summary

Define how diff information is translated into visual decorations in the Dioxus UI and editor adapter. This RFC ensures that line status, inline changes, selected hunks, merge state, and warnings are displayed consistently regardless of whether the rendering surface is plain Dioxus, CodeMirror, or a fallback viewer.

## Problem

A diff/merge app depends on visual trust. If colors, highlights, signs, and hunk indicators are inconsistent, users can make unsafe merge decisions. Rendering details must therefore be driven by a stable semantic decoration model.

## Goals

- Define semantic diff decorations independent of rendering technology.
- Support line-level and inline character-level changes.
- Support selected/current hunk state.
- Represent merge-applied, unresolved, and conflicted states.
- Allow accessible non-color indicators.
- Enable renderer fallback without changing core semantics.

## Non-goals

- Final CSS theme values.
- Syntax highlighting design.
- Language-aware semantic diff.
- Pixel-perfect editor implementation.

## Semantic decoration model

```rust
pub struct DiffDecorationSet {
    pub comparison_id: ComparisonId,
    pub left: Vec<LineDecoration>,
    pub right: Vec<LineDecoration>,
    pub inline: Vec<InlineDecoration>,
    pub hunks: Vec<HunkDecoration>,
    pub warnings: Vec<DecorationWarning>,
}

pub struct LineDecoration {
    pub side: DiffSide,
    pub line_index: usize,
    pub kind: LineDecorationKind,
    pub hunk_id: Option<HunkId>,
}

pub enum LineDecorationKind {
    Unchanged,
    Added,
    Deleted,
    Modified,
    EmptyCounterpart,
    MovedCandidate,
    Conflict,
    MergeApplied,
    Warning,
}

pub struct InlineDecoration {
    pub side: DiffSide,
    pub line_index: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub kind: InlineDecorationKind,
}

pub enum InlineDecorationKind {
    InsertedChars,
    DeletedChars,
    ReplacedChars,
    WhitespaceOnly,
}
```

## Visual contract

Every visual state must have:

1. a color/background affordance,
2. a non-color indicator,
3. a screen-reader label or accessible description where practical,
4. a stable CSS class or renderer token.

Example mapping:

```text
Semantic state       Visual indicator              Non-color indicator
Added                added line background          + sign in gutter
Deleted              deleted line background        - sign in gutter
Modified             changed line background        ~ sign in gutter
Conflict             warning/error background       ! sign in gutter
Merge applied        subtle applied marker          ✓ sign in gutter
Selected hunk        border or current marker        current hunk label
```

## Dioxus/CodeMirror class contract

The editor adapter should receive semantic tokens, not app-specific CSS names. The Dioxus layer maps tokens to CSS classes.

```text
fs-line-added
fs-line-deleted
fs-line-modified
fs-line-conflict
fs-line-merge-applied
fs-inline-inserted
fs-inline-deleted
fs-inline-replaced
fs-hunk-current
fs-hunk-hovered
fs-gutter-added
fs-gutter-deleted
fs-gutter-modified
```

## Wireframe: decorated diff view

```text
+--------------------------------------------------------------------------------+
| Left                                   | Right                                  |
+----------------------------------------+----------------------------------------+
|  10   unchanged line                   |  10   unchanged line                    |
| -11   old value = 1                    | +11   new value = 2                     |
|  12   unchanged line                   |  12   unchanged line                    |
|      [Copy →] [Mark Resolved]          |                                        |
+--------------------------------------------------------------------------------+
| Hunk 3 of 12 | Modified | whitespace significant | inline diff on             |
+--------------------------------------------------------------------------------+
```

## Accessibility requirements

- Do not rely only on red/green.
- Provide signs in gutters.
- Provide hunk summary text.
- Keyboard focus must identify current hunk.
- Screen-reader labels should summarize line changes where possible.
- Theme must support high-contrast mode.

## Large-file fallback

For large files, inline decorations may be disabled. The UI must disclose this explicitly:

```text
Inline character diff disabled for large-file safe mode.
Line-level differences are still available.
```

## Acceptance criteria

- Core emits renderer-independent decoration sets.
- Dioxus UI consumes decoration sets without recomputing diff semantics.
- Added/deleted/modified/conflict/merge-applied states are visually distinct.
- Non-color indicators exist for all major states.
- Large-file mode can disable inline decorations safely.
- Snapshot tests cover decoration generation.

## Test strategy

- Unit tests for line decoration generation.
- Unit tests for inline decoration spans.
- Snapshot tests for decoration JSON.
- Visual regression screenshots for sample files.
- Accessibility checks for color-independent indicators.

## Dependencies

- RFC 002 similar v3 diff engine.
- RFC 006 Diff/merge workspace.
- RFC 013 Large-file performance.
- RFC 025 Editor adapter prototype.
