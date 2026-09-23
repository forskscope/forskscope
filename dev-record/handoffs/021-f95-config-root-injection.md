# Handoff 021 — F95: stop the session suite racing on a process-global env var

**From:** architect. **Priority:** High, **not release-blocking.** Take it now —
your queue is otherwise empty and B4 is the owner's, not yours.
**Register:** F95. **Review that found it:** 093 §5 and its addendum.

## 1. The defect, measured

`state::compare::tests::opening_a_tab_persists_the_session_without_any_further_render`:

| Mode | Result |
|---|---|
| default parallelism | **3 failures in 5 runs** |
| `--test-threads=1` | **0 failures in 3 runs** |

Zero on one thread and sixty percent on many is a race, not a flake.

## 2. The mechanism — and a correction to my own first account

The test does:

```rust
let _guard = XDG_CONFIG_HOME_LOCK.lock()…;
// SAFETY: serialized by XDG_CONFIG_HOME_LOCK; no other test in this
// suite reads or writes this env var.
unsafe { std::env::set_var("XDG_CONFIG_HOME", &config_home); }
```

**The premise is false and the mutex cannot rescue it.** `set_var` is
*process*-global: it changes the variable for every thread, not only
lock-holders. The lock serializes the four tests that take it and does nothing
about the ones that read the variable without naming it.

**Correcting review 093 §5 before you build on it:** I said the transitive
reader was `forskscope-core/src/platform.rs:103`. **It is not** — that is only
the diagnostics *hint string*. The real chain is:

```
state.rs:41   config_file_path()  ->  dirs_next::config_dir()   [reads XDG_CONFIG_HOME on Linux]
                ├─ settings.rs:32   SettingsRepository::new(config_file_path("settings.json"))
                └─ session.rs:19    SessionRepository::new(config_file_path("session.json"))
```

So **any** test touching settings or session persistence resolves its path from
whatever `XDG_CONFIG_HOME` holds at that instant. None of them mentions the
variable, which is why grepping for it finds only the four that already lock.

## 3. What to do

Make the config root **injectable** instead of ambient. `config_file_path` is
the single choke point — both repositories already take an explicit path
(RFC-076 deliberately kept path resolution out of them), so the seam exists.

**My suggestion, and the reasoning — argue with it if you disagree:** a
**thread-local** override consulted by `config_file_path`, rather than a global
one behind a mutex. `with_test_store` is already thread-local, so each test
thread would resolve its own temp root and the race disappears **without any
serialization at all** — no lock to take, no lock to forget, and the four
existing `XDG_CONFIG_HOME_LOCK` sites can go.

**Verify one thing before committing to that**, because it decides whether it
works: a thread-local only covers work that happens on the *calling* thread. If
any settings/session write is spawned onto another thread or task, that write
resolves the real user config directory and the test both fails and **writes
into the developer's actual `~/.config/forskscope`**. F61's fix replaced a
reactive effect with explicit synchronous `save_session` calls, so I expect this
holds — but check it rather than assume, and if it does not hold, say so and
propose the global-with-lock alternative instead.

**No production behaviour changes.** Default resolution stays
`dirs_next::config_dir()`.

## 4. Falsification

The bar is that F95's own measurement inverts:

1. **The race is gone.** Run the `forskscope-ui` lib suite **at least 10 times**
   at default parallelism and report the count. Anything other than 10/10 green
   is not done. Report the actual runs, not "it passes now".
2. **The new seam is exercised.** Removing the override plumbing must fail a
   test — otherwise the injection point is untested and this repeats RFC-060's
   original shape.
3. **No `unsafe { set_var }` remains** in the crate's tests. `grep` and show it.

## 5. Scope

**In:** `state.rs`'s `config_file_path`, the override mechanism, the four tests
holding `XDG_CONFIG_HOME_LOCK`, and any test converted to the new seam.

**Out:** `platform.rs`'s diagnostics hint (unrelated, despite my earlier
misidentification); production path behaviour; `keyboard.rs`; RFC-060's work.

If you find other tests that mutate process-global state under a lock with the
same reasoning, **name them, do not fix them** — that is a separate finding and
I would rather register it than have it arrive inside this change.

## 6. Gates

The usual set, plus §4.1's repeated-run count. Note `cargo test --workspace`
green **once** is not evidence here — that is precisely the signal F95 says is
unreliable.
