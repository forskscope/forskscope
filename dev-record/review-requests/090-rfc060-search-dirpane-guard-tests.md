# Review Request 090 — RFC-060 follow-up: testing the two surfaces that actually needed it

Handoff: `rfcs/handoffs/060-global-keyboard-scope-and-modal-input-safety/020-rfc060-keyboard-ownership-tests.md`
Follow-up to: review 092 (`dev-record/reviews/092-rfc060-keyboard-ownership-review.md`), required §2
Commit: `0b089bd` (pushed to `main`)

## What review 092 found

Its §2 table, exactly as reasoned: `filter.rs` had a test but nothing to
fix (already blanket); `search.rs` and `dir_pane.rs` each carried the real,
newly-closed defect (Ctrl+S typed into either bubbled to `app.rs` and saved
the active tab) but were left as inline `onkeydown` closures — untestable,
so nothing would have caught either guard being deleted again.

## The fix — same pattern as `filter_input_keydown`, applied where it mattered

`search.rs`'s `SearchBar` now calls `search_input_keydown(&e, ctx)`, and
`dir_pane.rs`'s `PathBar` now calls `path_input_keydown(&e, edit_mode,
input_val, input_err, on_navigate, &path_str_reset)` — both named functions
their component's `onkeydown` attribute calls directly, not closures with
inline logic. Both signatures take exactly the signals/handler the
original closures captured; behavior is unchanged (same Enter/Escape
business logic, now reached after an unconditional `swallow_when_typing`
call, same as before this handoff for both).

## Falsifications, run against the shipped functions

### `search.rs`

Commented out `search_input_keydown`'s `swallow_when_typing(e)` call and
re-ran `search_input_keydown_swallows_every_key`:

```
thread 'ui::view::search::tests::search_input_keydown_swallows_every_key' panicked at crates/forskscope-ui/src/ui/view/search.rs:209:13:
typing in the search input must not let Ctrl+S (or any other key) reach the global keyboard handler behind it
test result: FAILED. 0 passed; 1 failed
```

Restored, re-ran green.

### `dir_pane.rs`

Commented out `path_input_keydown`'s `swallow_when_typing(e)` call and
re-ran `path_input_keydown_swallows_every_key`:

```
thread 'ui::view::dir_pane::tests::path_input_keydown_swallows_every_key' panicked at crates/forskscope-ui/src/ui/view/dir_pane.rs:600:13:
typing in the path input must not let Ctrl+S (or any other key) reach the global keyboard handler behind it
test result: FAILED. 0 passed; 1 failed
```

Restored, re-ran green.

This is what §2 of review 092 asked for directly: reproducing review 092's
own manual falsification (*"I removed `swallow_when_typing` from
`search.rs:90` ... nothing failed"*) now fails, because there is a named
function the test and the component both call — the same relationship
`filter_input_keydown` already had.

## Coverage beyond the swallow itself

Also added `escape_closes_the_search_bar_and_clears_the_query` (`search.rs`)
and `escape_resets_the_path_input_to_the_value_it_had_before_editing`
(`dir_pane.rs`) — not required by the handoff, but since both closures'
business logic moved into a named function alongside the swallow, a bare
swallow test alone wouldn't have caught a mis-extraction that broke Escape's
existing reset behavior while moving the code. Both assert the pre-existing
Escape behavior is intact post-extraction.

One mechanical note: constructing an `EventHandler<PathBuf>` for
`path_input_keydown`'s test needed `Runtime::current().in_scope(ScopeId::ROOT,
...)` around `EventHandler::new` — `with_test_store`'s `in_runtime` alone
pushes a `Runtime` but not a scope onto the stack, and `Callback::new` calls
`current_scope_id()` unconditionally (panics on an empty stack), unlike
`Signal::new_in_scope`, which takes the scope explicitly and doesn't need
one already active.

## §5's mechanism correction

Fixed `settings/modal.rs`'s comment per review 092 §5: it previously implied
`app.rs`'s generic Escape handling was "already" seeing the keypress
regardless of the local handler. It wasn't — the old handler's own
`stop_propagation()` meant the event never reached `app.rs` while that
handler existed. The redundancy is real (either mechanism alone produces
the same outcome — a non-recovery modal closes on Escape), just not for the
reason the comment gave. Reworded to state it as a counterfactual, not a
concurrent fact.

## §6 (naming) — left as-is

Review 092 flagged `swallow_when_typing`'s name describing three of four
call sites (not `settings/modal.rs`'s scrim wrapper, which involves no
typing) as worth a rename "if a better one is obvious," explicitly not
blocking. I didn't find one clearly better than the cost of touching five
call sites for a naming-only change, so left it — happy to take a specific
suggestion if one exists.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (717 core / 113 ui-lib / 113 ui-bin —
+4 over review 092's baseline / 200 ui-logic / other suites unchanged, all
green across three repeated full-workspace runs — one earlier single run
hit `state::compare::tests::opening_a_tab_persists_the_session_without_any_further_render`
failing, reproduced as a `--workspace`-only flake unrelated to this change:
passes in isolation every time, and the file it exercises is untouched by
either commit in this handoff), `cargo xtask css --check`, `version-sync`,
`i18n`, `rfc-sync`, `git diff --check` — all green locally; CI run 33600800500
for `0b089bd` confirmed green.

## Scope

Only the three files review 092 named: `dir_pane.rs`, `search.rs`,
`settings/modal.rs`. No change to `keyboard.rs`, `app.rs`, or `filter.rs`
(already correct per review 092 §3).
