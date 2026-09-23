# Review Request 091 — F95: config root injection instead of a process-global env var

Handoff: `dev-record/handoffs/021-f95-config-root-injection.md`
Commit: `f74c34f` (pushed to `main`)

## §4.1 — the race is gone, measured

`forskscope-ui`'s lib test suite, 10 runs at default parallelism:

```
run 1: test result: ok. 115 passed; 0 failed
run 2: test result: ok. 115 passed; 0 failed
run 3: test result: ok. 115 passed; 0 failed
run 4: test result: ok. 115 passed; 0 failed
run 5: test result: ok. 115 passed; 0 failed
run 6: test result: ok. 115 passed; 0 failed
run 7: test result: ok. 115 passed; 0 failed
run 8: test result: ok. 115 passed; 0 failed
run 9: test result: ok. 115 passed; 0 failed
run 10: test result: ok. 115 passed; 0 failed
```

**10/10 green** (was 3 failures in 5 runs before this fix — the handoff's
own measurement, reproduced before I touched anything). Full `cargo test
--workspace` also re-run three times afterward, all green (717 core / 29 /
16 / 2 / 115 ui-lib / 115 ui-bin / 200 ui-logic / 4 / 6, one ignored
doctest unchanged).

## §4.2 — the seam is exercised, falsified without touching a real path

`config_file_path` now consults a thread-local `CONFIG_ROOT_OVERRIDE`
before falling back to `dirs_next::config_dir()`. I wrote
`config_file_path_honors_the_override` to test this directly — it only
computes and compares a `PathBuf`, calling `config_file_path` itself, with
no repository write and no real file ever touched.

I temporarily replaced the override lookup with `None::<PathBuf>` (skipping
`CONFIG_ROOT_OVERRIDE` entirely) and re-ran it:

```
thread 'state::tests::config_file_path_honors_the_override' panicked at crates/forskscope-ui/src/state.rs:378:9:
assertion `left == right` failed
  left: "/home/<user>/.config/forskscope/session.json"
 right: "/f95-override-root/forskscope/session.json"
test result: FAILED. 0 passed; 1 failed
```

Worth being explicit about what that output shows: `config_file_path`
resolved to my *real* `~/.config/forskscope/session.json` once the override
was skipped — proving the fallback genuinely reaches the real platform
directory — but the test never calls anything that writes there; the
assertion only compares the returned `PathBuf`s. Safe to falsify against the
real function for exactly that reason. Restored, re-ran green.

## §4.3 — no `unsafe { set_var }` remains

```
$ grep -rn "unsafe.*set_var\|set_var" --include="*.rs" . | grep -v /target/ | grep -v /.git-exclude/
(no output)
```

## The mechanism

`CONFIG_ROOT_OVERRIDE` is a `thread_local! { RefCell<Option<PathBuf>> }`,
consulted first in `config_file_path`, falling back to
`dirs_next::config_dir()` exactly as before — production resolution is
unchanged. It's set only by `ConfigRootOverrideGuard`
(`#[cfg(test)]`-only), an RAII guard rather than a manual set/clear pair:
cargo's default test harness reuses worker threads across tests, so a test
that panicked between a manual set and clear would leak the override onto
whatever test runs next on that same thread. `Drop` restores exactly what
was there before `set`.

`opening_a_tab_persists_the_session_without_any_further_render` now uses
`ConfigRootOverrideGuard::set(config_home.clone())` instead of the
lock/`unsafe`/`set_var`/`remove_var` block — the `XDG_CONFIG_HOME_LOCK`
static is gone.

## Verifying §3's thread requirement, not assuming it

Before committing to the thread-local, I traced every `save_session` /
`persist_session` / `resolve_session` / `persist_settings` call site
(`app.rs`'s startup hook, `compare.rs:375`, `tab.rs:121`'s `swap_sides`,
`session.rs:174`'s `close_tab`, `settings.rs`'s `persist`) — all called
directly from a synchronous event handler or the startup `use_hook`, none
from inside `spawn`/`spawn_forever`/`tokio::spawn`/`spawn_blocking`.
`open_compare_request_with_options` (`compare.rs:314-398`) is the case that
matters for the migrated test: `save_session(store)` runs synchronously at
line 375, *before* `spawn_forever` is reached at line 383, and the spawned
task's own comment already states it doesn't call `save_session` at all
("the session payload only ever holds left_path/right_path ... already
final at the synchronous push above"). So the thread-local holds for
everything this crate does today; the handoff's expectation was right, and
I checked it against the actual call graph rather than trusting the
prediction.

## Correcting the handoff's "four tests" before it propagates further

§5's scope line says "the four tests holding `XDG_CONFIG_HOME_LOCK`."
Checked with a repo-wide grep for `XDG_CONFIG_HOME`, `set_var`, and the
lock's own name: there was exactly **one** `#[test]` function using it —
`opening_a_tab_persists_the_session_without_any_further_render` — with two
`set_var` calls and one `remove_var` inside that single test (the set, and
the two branches of restoring it), which is likely what got counted as
"four." No other test in the crate touched the lock or the env var
directly. I converted the one that existed; there was no second, third, or
fourth site to find.

## Scope

Only `state.rs` (the seam + guard + two new unit tests) and
`state/compare/tests.rs` (the one migrated test). No change to
`platform.rs`, `keyboard.rs`, or any RFC file. No other test found mutating
process-global state under a lock with the same reasoning — none exist to
name.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo xtask css --check`, `version-sync`, `i18n`, `rfc-sync`,
`git diff --check` — all green. `cargo test --workspace` green on three
repeated full runs, plus the §4.1 10/10 lib-only count above (this
handoff's own bar: single-run green is not evidence here). CI run 33603262372
for `f74c34f` confirmed green.
