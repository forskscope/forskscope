# F32 Developer Handoff: Compare-View Changed-Line Misalignment

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md) — release-blocking defect
**Related.** RFC-024 (diff decoration contract), RFC-061 (accessibility), RISK-002, RFC-078 P03
**Register item.** F32
**Blocks.** The next release cut, and the deferred README/docs update (F33)

This handoff directs a small fix with a large verification requirement. The code
change is a few lines; proving it is the work.

## 1. The defect

On WebKitGTK — the actual Linux runtime — every **changed** line in the compare
view renders shifted one full column to the right, with its content clipped off
the pane edge. Unchanged lines render correctly. The product's central view is
unreadable for precisely the lines it exists to show.

Present in published **0.165.0** and introduced in **0.164.0**.

## 2. Cause, confirmed by mutation

`crates/forskscope-ui/src/ui/view/hunk.rs` emits an `.sr-only` span as the
**first child of a `display: table-row`**, at lines 212 (`RowLeft`) and 275
(`RowRight`):

```rust
div { class: "{row_class}", role: "row",
    if let Some(ref lbl) = sr_label { span { class: "sr-only", "{lbl}: " } }
    div { class: "{gutter_class}", ... }      // table-cell
    span { class: "diff-mark", ... }          // table-cell
    div { class: "cell", ... }                // table-cell
```

WebKitGTK wraps that span in an anonymous table cell, so the row gains a fourth
column and the three real cells shift right by one. `sr_label` is `Some` only for
`HunkKind::Delete` and `HunkKind::Replace`, which is exactly why only changed
rows misalign, and both panes shift symmetrically because `RowLeft` and
`RowRight` emit it identically.

`.sr-only` is `position: absolute !important`, so per spec it should not
generate an anonymous cell. WebKitGTK evidently does. This is RISK-002
materialising verbatim: the 0.164.0 table-layout CSS was verified in Chromium and
never on WebKitGTK.

**Evidence.** Removing both spans, rebuilding, and re-capturing the identical
view produced correct alignment on every row. Before and after captures are at
`.git-exclude/tmp/shots/01-diff.png` and `02-mutation.png`.

**Sweep result.** These two are the only `sr-only` usages in `forskscope-ui`, and
the act-column table rows are unaffected. There is no wider instance of this
pattern to fix.

## 3. Required change — stated as a property

The mutation used for diagnosis simply deleted the spans. **Do not ship that.**
The "Deleted"/"Changed" label is required screen-reader output under G-007 and
RFC-024; deleting it would trade a visual defect for an accessibility
regression.

The property to satisfy:

> Each `.diff-row` contains exactly three table cells, and the
> Deleted/Changed label remains in the accessibility tree, announced with its
> row and before the line content.

The obvious mechanism is to move the span **inside** the `.cell` div, ahead of
the line content — it stays announced, and it no longer sits directly under a
`table-row`. An `aria-label` on the row element is an acceptable alternative if
you prefer it; verify announcement either way rather than assuming.

## 4. Verification — the part that matters

**No automated test in this repository catches this defect, and none will after
the fix.** The `css_coverage` integration test checks the class-to-model
contract, not layout. Unit tests, clippy, and the full workspace suite all pass
against the broken rendering — they passed for two releases while it shipped.

So the acceptance evidence is visual and must be captured on WebKitGTK:

1. Build and run the real binary against two files with deleted, replaced, and
   inserted lines. `.git-exclude/tmp/demo/before/src/config.rs` and
   `.../after/src/config.rs` already provide all three.
2. Capture the compare view:
   `niri msg action screenshot-window --id <ID> --show-pointer false --path <repo-relative path>`
3. Confirm in the image that changed rows begin at the same left offset as
   unchanged rows, that `−`/`+` markers sit immediately after the line number,
   and that no line content is clipped at the pane edge.
4. Run the app under an isolated `HOME` so the capture cannot touch your real
   config or embed a personal path:
   `HOME="$PWD/.git-exclude/tmp/demo-home" XDG_CONFIG_HOME="$PWD/.git-exclude/tmp/demo-home/.config" ./target/debug/forskscope <left> <right>`

Also confirm the label survives: inspect the rendered DOM through the WebView
inspector, or assert the span's presence and position in a component test if one
is practical.

## 5. Scope

In scope: the two `sr-only` spans in `hunk.rs`, any CSS needed to keep the label
hidden and announced, and the visual evidence above.

Out of scope:

- **Anything patch 5 is touching.** Convergence cleanup is in flight in the same
  working tree and it is *not* confined to `forskscope-core` — its rename reaches
  six `forskscope-ui` files (`state.rs`, `state/settings.rs`, `state/session.rs`,
  `ui/view/settings.rs` and their tests). The disjointness that matters here is
  at file level, not crate level: **`ui/view/hunk.rs` is the only file this fix
  needs, and patch 5 does not touch it.** Keep it that way rather than relying on
  a crate boundary that does not hold.
- other compare-view styling, the act column, or anything RISK-002 might also
  have affected but that is not this defect;
- adding a screenshot step to release preflight — worth doing, and recorded
  separately, but not this change.

## 6. Gates

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo xtask css --check
git diff --check
```

These will pass whether or not the fix works, which is the point of §4. Report
them, but do not present them as evidence that the defect is fixed.

## 7. Acceptance criteria

- Changed rows and unchanged rows share the same left offset, evidenced by a
  WebKitGTK screenshot committed to the review request or referenced from it.
- No line content is clipped at the pane edge.
- Each `.diff-row` contains exactly three table cells.
- The Deleted/Changed label is still present and announced.
- `forskscope-core` is untouched.
- All gates in §6 pass.

## 8. Required review-request content

Standard format, plus:

1. the before/after screenshots, or paths to them;
2. how you confirmed the accessibility label survives, not merely that it exists
   in the source;
3. confirmation that no `forskscope-core` file was touched.

## 9. Why this one is worth a moment's reflection

A defect this visible — the core view of a visual diff tool, unreadable for
changed lines — shipped in two releases and was found by the first person to
look at a screenshot. Every gate was green throughout.

The lesson is not that the gates are wrong; it is that they cannot see rendering,
and nothing else was looking. That argument belongs in the release preflight, and
is recorded separately.
