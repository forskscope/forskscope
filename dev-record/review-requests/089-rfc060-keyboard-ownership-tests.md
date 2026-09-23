# Review Request 089 — RFC-060: making the keyboard-ownership rule enforceable

Handoff: `rfcs/handoffs/060-global-keyboard-scope-and-modal-input-safety/020-rfc060-keyboard-ownership-tests.md`
Commit: `59cd070` (pushed to `main`)

## The three falsifications (§6), run against the shipped functions

### Test 1 — remove `app.rs`'s `modal_open` early return

The check now lives in `keyboard::global_key_action`
(`crates/forskscope-ui/src/keyboard.rs`) as `if modal.is_open() { return
A::Ignore; }`. I replaced it with `if false { return A::Ignore; }`
(commented, noting it was temporary) and re-ran
`ctrl_s_is_ignored_while_a_modal_is_open`:

```
thread 'keyboard::tests::ctrl_s_is_ignored_while_a_modal_is_open' panicked at crates/forskscope-ui/src/keyboard.rs:248:9:
assertion `left == right` failed
  left: Save
 right: Ignore
test result: FAILED. 0 passed; 1 failed
```

With the guard gone, Ctrl+S decides `Save` instead of `Ignore` while a modal
is open — exactly the defect this guard exists to prevent. Restored, re-ran
green.

### Test 2 — remove the `SettingsRecovery`/`SessionRecovery` exclusion

The Escape branch is `modal.is_open() && modal != ModalState::Recovery`. I
dropped the second condition and re-ran
`escape_does_not_dismiss_a_recovery_modal`:

```
thread 'keyboard::tests::escape_does_not_dismiss_a_recovery_modal' panicked at crates/forskscope-ui/src/keyboard.rs:285:9:
assertion `left == right` failed
  left: CloseModal
 right: Ignore
test result: FAILED. 0 passed; 1 failed
```

Escape now decides `CloseModal` for a recovery dialog — the exact defect
RFC-076 patch 6 forbids (a recovery choice made by an accidental keypress).
Restored, re-ran green.

### Test 3 — remove `stop_propagation` from one converted input surface

I picked `explorer/filter.rs`'s filter input. `FilterBar`'s `onkeydown`
calls `filter_input_keydown(&e)` — not a closure with inline logic — and I
edited *that exact function*, commenting out its one line
(`crate::keyboard::swallow_when_typing(e);`), then re-ran
`filter_input_keydown_swallows_every_key`:

```
thread 'ui::view::explorer::filter::tests::filter_input_keydown_swallows_every_key' panicked at crates/forskscope-ui/src/ui/view/explorer/filter.rs:194:9:
typing in the filter input must not let Ctrl+S (or any other key) reach the global keyboard handler behind it
test result: FAILED. 0 passed; 1 failed
```

Restored, re-ran green. This is the point of §3's distinction: the test
calls the identical function `FilterBar`'s `onkeydown: move |e|
filter_input_keydown(&e)` calls, not a copy of its logic — deleting the
call from the shipped function is what the test catches, not a
re-implementation deleted from the test itself.

## Part A — the decision, extracted

`keyboard::global_key_action(key: &Key, mods: Modifiers, modal: ModalState,
has_active_tab: bool) -> GlobalKeyAction` is the pure function; `app.rs`'s
`onkeydown` reads `store.modal`/`store.active`, calls it, and matches on the
11-variant `GlobalKeyAction` result to perform the actual store
mutation/spawn/eval — a thin dispatcher, no decision left in the closure.

I did not use `modal: &Modal` as the handoff's sketch showed. `Modal`'s
recovery variants carry a full `SettingsRuntimeResolution` /
`SessionRuntimeResolution` — real structs with several fields — and
building one just to exercise a keyboard-guard branch buries the test's
intent in fixture noise unrelated to what it's checking. `ModalState` is a
3-value summary (`None` / `Recovery` / `Other`) with a one-line
`from_modal(&Modal) -> ModalState` conversion that `app.rs` calls; the
tests construct `ModalState` directly (I did also write
`from_modal_classifies_settings_recovery_as_recovery` /
`..._session_recovery_...`, which do build both resolutions once, to pin
that the conversion itself is correct — cheap enough there since it's only
two tests, not the whole suite's fixture shape).

Behaviour is unchanged: Escape still closes any non-recovery modal and does
nothing otherwise; every other shortcut is still swallowed while any modal
is open; F7/F8/F3/Shift+F3/Enter/Ctrl+S/Z/Y/W///F map exactly as before.
Wired the same change: `app.rs` and `main.rs` (the binary target compiles
its own module tree, separate from `lib.rs` — `main.rs` needed its own
`mod keyboard;` too, caught immediately by `clippy --all-targets`).

## Part B — the input obligation

`keyboard::swallow_when_typing(e: &Event<KeyboardData>)` is one line
(`e.stop_propagation()`) — its value isn't complexity reduction, it's a
named, discoverable, testable obligation instead of a per-surface
`e.stop_propagation()` someone has to remember to write correctly.

`explorer/filter.rs`, `dir_pane.rs`'s path input, and `search.rs`'s search
input now all call it **first, unconditionally**, before any per-key
branching. For `filter.rs` this is a no-op (it was already blanket). For
`dir_pane.rs` and `search.rs` **this closes a real gap**, not just a
refactor: both previously called `stop_propagation()` only inside their
`Enter`/`Escape` arms, so e.g. typing Ctrl+S while renaming a directory
path, or while searching, would have bubbled to `app.rs` and saved the
active tab behind it — the exact class of defect RFC-060's purpose
statement names. I'm disclosing this as a deliberate behavior change, not
a pure refactor: after this change, every other key typed into those two
inputs is also swallowed, which is what "the global `onkeydown` yields ...
to text inputs" actually requires.

## The judgment call: `settings/modal.rs` isn't a text-input case

`settings/modal.rs:27`'s guard sits on the modal's outer `div.scrim`, not
on any text input — it closes the settings modal on Escape. I converted it
to call `swallow_when_typing` too, for construction-based uniformity with
the other three, but I want to flag plainly that **it was already fully
redundant before this change**, and stays so after it: `app.rs`'s own
`modal.is_open()` check swallows every non-Escape key for *any* open modal
regardless of what that modal's own handler does, and its Escape branch
already closes any non-recovery modal — including this one — by itself.
Tracing it: with `store.modal` set to `Modal::Settings`, Escape pressed
anywhere inside the modal bubbles to `app.rs` (whether or not
`settings/modal.rs`'s own handler ran first) and `global_key_action`
decides `CloseModal` from `ModalState::Other` alone, with no dependency on
`settings/modal.rs` having done anything. So this site's own
Escape-closes-the-modal logic has had no observable effect since RFC-060's
generic guard landed at the app root; it predates that guard and was never
removed. I left it converted rather than removed it — deleting live code
that isn't broken felt outside a "make the existing rule testable" handoff
even though it's provably inert — but wanted this stated plainly rather
than silently converted as if it were the same kind of case as the other
three.

## Verified for real, not only by the test suite

Built the desktop binary, opened a real two-file comparison
(niri/wtype): Ctrl+`/` opened the keyboard-reference modal (screenshot
confirms exact shortcut list rendered); Ctrl+S pressed while that modal was
open left the save-target file's bytes **byte-for-byte unchanged**
(`cat`'d before/after); Escape then closed the modal (screenshot confirms);
Ctrl+S with no modal open completed without error.

## Scope

`app.rs`'s keydown closure, the four surfaces named in §2, the new
`keyboard` module, and their tests — plus `lib.rs`/`main.rs`'s one-line
module wiring, required for either target to compile. Untouched:
`keybindings.rs`, every `onclick` guard, all key bindings, `ROADMAP.md`,
every RFC file — `rfcs/done/060-...` was not moved.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (717 core / 109 ui-lib / 109 ui-bin,
+18 new keyboard tests over the pre-handoff baseline / 200 ui-logic / other
suites unchanged, all passing), `cargo xtask css --check`,
`version-sync`, `i18n`, `rfc-sync`, `git diff --check` — all green
locally; CI run 33595298851 for `59cd070` confirmed green.
