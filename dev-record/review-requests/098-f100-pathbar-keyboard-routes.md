# Review Request 098 — Handoff 028: F100 PathBar keyboard routes

Handoff: `dev-record/handoffs/028-f100-pathbar-keyboard-routes.md`
Commit: `88ec5f1` (pushed to `main`). CI run `34169371491`: green.

## §5's falsifications, run for real

### 1 — removing a binding fails a test asserting the action, not the key

Falsified by disabling the Alt+Home branch entirely (`if false && ...`):

```
thread 'ui::view::explorer::tree::tests::alt_home_navigates_the_focused_pane_to_the_home_directory'
panicked: Alt+Home must be recognized as this view's binding

thread 'ui::view::explorer::tree::tests::alt_home_targets_the_right_pane_when_the_right_pane_is_focused'
panicked at .../tree.rs:423:13:
assertion `left == right` failed
  left: "/somewhere/deep"
 right: "/home/<user>"
```

Both assert the actual navigation (`left_dir`/`right_dir`'s new value),
not merely that the key was consumed.

### 2 — each binding acts on the focused pane, not a hardcoded side

Falsified by hardcoding `is_left = true` in the Alt+Home branch:

```
test alt_home_navigates_the_focused_pane_to_the_home_directory ... ok
test alt_home_targets_the_right_pane_when_the_right_pane_is_focused ... FAILED

thread '...alt_home_targets_the_right_pane_when_the_right_pane_is_focused'
panicked at .../tree.rs:423:13:
assertion `left == right` failed
  left: "/somewhere/deep"
 right: "/home/<user>"
```

Exactly the asymmetry the handoff predicted: the "left focused" test
passes by coincidence (the hardcoded value happens to match), and the
"right focused" test — "the one that matters" — catches it. Same
falsification repeated against `apply_picked_folder`'s own branch
(shared by Home and the completed Open-folder pick):

```
thread 'ui::view::explorer::tree::tests::apply_picked_folder_targets_the_focused_pane'
panicked at .../tree.rs:488:13:
assertion `left == right` failed
  left: "/right/unmoved"
 right: "/picked"
```

### 3 — removing a help-modal entry fails a test

Falsified by deleting the `Ctrl + O` `KbRow`:

```
thread 'ui::overlay::keybindings::tests::help_modal_documents_the_new_pathbar_shortcuts'
panicked at .../keybindings.rs:112:13:
expected "Ctrl + O" among the rendered help-modal text [... 36 other
strings, no "Ctrl + O" ...]
```

All four restorations verified green afterward; full suite re-run after
each, not just the one test that caught it.

## Design decisions disclosed

**Took the collision map as given, verified nothing else.** `Ctrl+O` and
`Alt+Home` are both free per the handoff's own read of `explorer/tree.rs`
and `keyboard.rs`; I didn't re-derive the map, only confirmed bare `Home`
(`TreeKey::Home`, jump to first row) still dispatches correctly and isn't
shadowed by the new `Alt+Home` check (which is gated on the `Alt`
modifier and runs strictly before the `TreeKey` match). Both bindings
live in the tree's own `onkeydown`, not the global handler, matching
`Alt+↑`'s existing precedent exactly — same reasons the handoff gave
(pane-focus state only this handler has, and keeping the `Alt`-family
together).

**Extracted `dispatch_pathbar_shortcut`/`apply_picked_folder`**, the same
shape `dir_pane.rs`'s `path_input_keydown` already established for
testing keyboard handling without a full render. The Open-folder
binding's native `rfd` picker cannot run headlessly — the identical
limit `export_patch`'s save dialog has (RFC-084) — so only the
dispatch-and-capture-focused-pane half is exercised by
`ctrl_o_is_recognized_as_this_views_binding`; what a *completed* pick
does is `apply_picked_folder`, tested directly and identically to the
Home binding. `spawn()` inside the Ctrl+O branch needed
`Runtime::current().in_scope(ScopeId::ROOT, ...)` around the test call —
the same reason `path_input_keydown`'s own tests need it for
`EventHandler::new`.

**The help-modal test had no prior shape — said so, then built one.**
`overlay/keybindings.rs` had zero existing tests. Rather than invent
something silently, I used the same rendered-output-inspection
technique F99's PathBar `aria-label` test established (no `dioxus-ssr`
in this workspace): render `KeyboardRefModal` through a bare
`VirtualDom`, call `rebuild_to_vec()`, and scan the mutations — this
time for `Mutation::CreateTextNode` (text content) rather than
`Mutation::SetAttribute` (an attribute), since `KbRow`'s `keys`/`desc`
are dynamic text children, not attributes.

**The live compositor check the handoff asked for: attempted, and
inconclusive — stated honestly rather than skipped or overclaimed.**
Built the desktop binary, launched it under niri, and confirmed directly
that `~/.config/niri/cfg/keybinds.kdl` does **not** bind `Alt+Home` at
the compositor level in this environment — only `Mod+Home` and
`Mod+Ctrl+Home` are bound — so there is no compositor interception
configured here. I could not go further than that: firing the binding
for real requires DOM focus on the `#aligned-tree` div, which needs a
mouse click (no mouse-automation tool exists in this environment —
`ydotool`/`wlrctl` are both absent, the same gap disclosed for RFC-083's
encoding `<select>`), and blind `Tab`-key cycling only ever visibly
landed on `PathBar`'s buttons (confirmed via screenshots — a focus ring
moved button to button) and never confirmably on the tree div itself
(plain `<div>`s have no `:focus-visible` style in this app's CSS, so
even reaching it would be invisible). As a control, I sent the
**already-shipped** `Alt+↑` binding under the exact same conditions and
it *also* produced no visible effect — meaning this is a tooling
limitation on reaching the right element, not evidence against either
binding. Per the handoff's own "if it does not [reach the app], propose
an alternative rather than shipping a binding that silently does
nothing": I have no evidence it *doesn't* reach the app (no compositor
binding exists to intercept it), only that I couldn't drive a
conclusive positive test in this headless-ish environment. Flagging this
as an open verification gap rather than a pass.

## Scope

Touched exactly what §6 named: `explorer/tree.rs`'s keydown handler,
`overlay/keybindings.rs`, i18n strings, tests. Did not touch the global
`keyboard.rs` handler, did not re-audit RFC-061 further, and found no
other unbound controls while in these files to report.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (736 core / 133 ui-lib / 133 ui-bin /
204 ui-logic, all green — +5 ui-lib tests, +5 ui-bin tests over the
pre-handoff baseline: 4 `dispatch_pathbar_shortcut`/`apply_picked_folder`
tests, 1 help-modal test), `cargo xtask css --check`, `version-sync`,
`i18n` (**246 keys, +2** — exactly as the handoff predicted, since new
help-modal strings needed Japanese translations this time, unlike
handoff 027's unchanged count), `rfc-sync`, `audit-deps`, `git diff
--check`, `mdbook build docs` — all green. `cargo audit`: exit 0, the
same 14 pre-existing warnings as recent reviews (no dependency added).
