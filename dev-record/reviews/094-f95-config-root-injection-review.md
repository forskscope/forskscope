# Review 094 — Request 091: F95 config root injection

**Reviewer:** architect. **Date:** 2026-09-02. **Reviewed:** `f74c34f`.
**Verdict:** **Approved. F95 is closed.** No follow-up.

## 1. The measurement, reproduced independently

Ten runs of the `forskscope-ui` lib suite at default parallelism, on the same
machine that produced F95's original 3-failures-in-5:

```
PASS: 10 / FAIL: 0
```

The race is gone, not quieter.

## 2. The test still bites — which mattered more than the race

A racy test can always be "fixed" by making it assert less. So I checked the
migrated test still detects the defect it exists for. Neutralising
`save_session(store)` at `compare.rs:375`:

```
opening_a_tab_persists_the_session_without_any_further_render ... FAILED
F61: opening a tab must persist session.json synchronously … but no file was
written at /tmp/fsk-ui-load-and-diff-f61-open-compare-persists-…/config/forskscope/session.json
```

Two things at once: F61's guarantee is still enforced, **and** the path in the
failure is the temp override root — so the seam is genuinely in effect rather
than the test passing because it stopped looking.

## 3. The RAII guard anticipated a hazard the handoff did not

I asked for an override. You built `ConfigRootOverrideGuard` with `Drop`, and
gave the reason: cargo reuses worker threads across tests, so a test panicking
between a manual set and clear would **leak the override onto whatever test runs
next on that thread**. That is a real failure mode, it is exactly the shape of
the bug being fixed — ambient state outliving its owner — and a manual
set/clear pair would have reintroduced it in a subtler form. Not asked for,
correctly identified.

## 4. You checked my prediction instead of trusting it

§3 of the handoff said *"I expect this holds — but check it rather than
assume."* You traced every `save_session` / `persist_session` / `resolve_session`
/ `persist_settings` call site and established none runs inside a spawned task,
including the one that decides it: `save_session` at `compare.rs:375` runs
**before** `spawn_forever` at `:383`.

That mattered. Had any write been spawned, the thread-local would have silently
resolved the developer's real `~/.config/forskscope` — the test would fail *and*
write into it. You confirmed the precondition rather than discovering it.

Your §4.2 falsification shows the same care: the fallback does resolve to the
real config path, and the test only compares `PathBuf`s and never writes, so
falsifying against the real function was safe. Stating why is what makes it
reviewable.

## 5. Correcting my error, which you caught

The handoff's §5 said *"the four tests holding `XDG_CONFIG_HOME_LOCK`."* There
was **one**. I ran `grep -c`, which counts matching **lines** — a static
declaration, a lock acquisition and two `SAFETY:` comments, all inside a single
test — and reported the number as a count of tests without opening the file.

That is the same instrument error as F83's heading probe, which I also hit
earlier today: **counting matches instead of reading them.** Recorded here
because it is now twice.

## 6. Verified

Production resolution unchanged — `config_file_path` falls back to
`dirs_next::config_dir()` and the override is set only by a `#[cfg(test)]` type,
so it is `None` in every shipped build. `grep` for `set_var` across `crates/`
and `xtask/`: **no output**. Scope held to `state.rs` and
`state/compare/tests.rs`.

**F95 is closed.** Nothing deferred.
