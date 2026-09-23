# Review Request: F61 — the real mechanism, this time

**Date:** 2026-08-16
**Request:** follow-up to `062-f61-f62-session-persistence.md`
**Response to:** `dev-record/reviews/065-f61-f62-session-persistence-review.md`
**Baseline:** `main` at `202ab97` (includes `cf27ccb`'s reopening)
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/f61-f62-session-persistence-handoff.md`
**Scope:** F61 only. F62 was already approved (review 065 §4) and is untouched here.

Addresses review 065 §6's five required items directly, in order.

## 1. Register corrected

Already done by `cf27ccb` (the reopening commit) before this session picked the
work back up — F61 was open, with review 065's findings recorded, when this
follow-up started. `ROADMAP.md`'s F61 entry now records the real mechanism and
fix (§3 below), superseding the reopened text.

## 2. The real mechanism, established by observing the real process

Reproduced review 065's Case B (genuinely fresh profile) directly in this
environment first, to confirm starting from the same place: real desktop
process, isolated `XDG_CONFIG_HOME`, `forskscope <left> <right>` — no
`forskscope/` directory, no `session.json`, over 30 real idle seconds. Control
(AT-SPI-invoked tab close) wrote the file immediately, matching review 065
exactly.

**Instrumented the real process, not the harness** — temporary `eprintln!`s
(never committed) in three places: `app.rs`'s effect body, the synchronous tab
push in `open_compare_request`, and the async load task's `commit_load_result`
call. Ran the same fresh-profile launch with stdout/stderr captured:

```text
DIAG-F61 tab pushed synchronously, idx=0
DIAG-F61 async load task about to commit result
DIAG-F61 async load task committed result
```

**The effect's own entry line never printed.** Both writes to `store.tabs`
happen; the effect that's supposed to react to either is never entered — not
"called and returned early," not "called and failed," simply never reached,
over the same 30-second window Case B already established.

**Separately confirmed re-rendering itself is not the problem**: launching
against the `render_check.py` fixture (7 hunks) with the same instrumentation,
the compare view fully renders (7/7 rows reached, confirming the async
commit's write *did* flow through to the screen) while `session.json` still
never appears. So "the view updates" and "the effect runs" are not the same
guarantee in this desktop runtime, for a write made outside a discrete UI
event.

**Ruled out "any subsequent render wakes the stale effect"**: clicked an
unrelated control (the `?` help button, which only touches `store.modal`)
after the same fresh-profile launch. The click succeeds (a real re-render
happens — the keyboard reference opens), and the tabs effect still does not
run. Only a write to `store.tabs` made *from inside a real Dioxus event
handler* — `close_tab`'s `onclick` — reliably flushes it.

**Established, not merely reproduced:** a signal write to `store.tabs` made
outside a discrete UI-event dispatch (synchronously during startup-hook
execution, or from a spawned task's completion callback) does not reliably
flush this desktop runtime's pending-effect queue, even though the identical
write correctly triggers a visual re-render and even after 30 real seconds
idle. A write made from inside an `onclick` handler does. This is not one of
review 065 §3's three listed hypotheses (which were all about *re-triggering*
after one successful run) — the effect does not fire even once.

## 3. Fixed at that level

Per the handoff's own offered resolution — *"The reactive effect either works
or it should be removed in favour of explicit calls everywhere"* — and given
§2 establishes it does not reliably work for exactly the paths that matter:

- **Removed** `app.rs`'s `use_effect` on `store.tabs` entirely.
- **`open_compare_request`** (`state/compare.rs`) now calls `save_session`
  explicitly, synchronously, immediately after the tab push — the same
  statement ordering `close_tab` already used. No corresponding call was added
  after the async load task's `commit_load_result`: the session payload only
  ever holds `left_path`/`right_path` (`build_save_payload`), both already
  final at push time, so the load/diff outcome that task commits doesn't
  change what needs persisting.
- **`swap_sides`** (`state/tab.rs`) also gained an explicit call — it swaps
  `left_path`/`right_path`, the one other place besides `close_tab` and
  `open_compare_request` that changes tab identity. (It was invoked from
  `onclick` too, so per §2's finding it may have already worked in practice;
  added anyway for consistency now that the effect it implicitly relied on no
  longer exists, and because "explicit calls everywhere" is the chosen design,
  not "explicit calls where a gap was proven.")
- Every other `store.tabs.write()` call site (undo/redo, char/word-wrap
  toggles, diff-option changes, save-completion bookkeeping, reload) mutates a
  tab's *internal* state only — none of it is part of `PersistedSession`'s
  payload, so none of it needs a `save_session` call.

## 4. A test that fails on today's `main`

Deleted `crates/forskscope-ui/src/app/tests.rs` — the `VirtualDom`-rendering
harness review 065 correctly identified as diverging from production. It
tested a mechanism (the reactive effect) that no longer exists.

Added `state::compare::tests::opening_a_tab_persists_the_session_without_any_further_render`
(`state/compare/tests.rs`): calls `open_compare_request` directly through
`with_test_store` (F36) against an isolated `XDG_CONFIG_HOME`, then asserts
`session.json` exists and parses with the right tab — synchronously, no
polling, no scheduler-driving, no wait. Runs in 0.00s and is not sensitive to
`--test-threads` (confirmed: `cargo test --workspace`, 3/3 clean runs at
default parallelism).

**Confirmed it fails on the pre-fix code**, per this program's standing
falsifiability rule and review 065 §5.2's explicit requirement: temporarily
commented out the new `save_session(store)` call in `open_compare_request`
(never committed) and re-ran the test —

```text
thread '...opening_a_tab_persists_the_session_without_any_further_render' panicked:
F61: opening a tab must persist session.json synchronously, with no further
render or async completion needed, but no file was written at .../session.json
```

— then restored the call and re-ran clean. The test discriminates for real.

**A genuine capability gap in F36's own harness, found and fixed along the
way:** `with_test_store` dropped its `VirtualDom`'s runtime context before
running the caller's closure, so any function under test that itself calls
`spawn`/`spawn_forever` (as `open_compare_request` does, for its background
load task) panicked with dioxus-core's own `"Components run in the Dioxus
runtime"` message. Fixed by wrapping the closure in `VirtualDom::in_runtime`
— a strict capability addition; every existing `with_test_store` caller is
unaffected since running inside an active runtime context doesn't change
synchronous `Signal` read/write behaviour.

## 5. Real-desktop confirmation

Same real GUI process, same isolated-`XDG_CONFIG_HOME` method as review 065,
against the fixed, cleaned-up build (no diagnostic instrumentation left in):

```text
$ XDG_CONFIG_HOME=<fresh, empty> forskscope <left> <right>
  (5s, no interaction)
session.json:
{
  "payload": { "tabs": [ { "left": "<left>", "right": "<right>" } ] },
  "schema_name": "session", "schema_version": 2, ...
}
```

Case B specifically (genuinely fresh profile, no pre-existing `forskscope/`
directory — the case review 065 called decisive) confirmed working. Restore
also confirmed: relaunching with no CLI args against the same config
directory reaches the compare view with the saved tab's row count.

This sandbox permits real GUI launches via a plain foreground `timeout N
<binary> <args>` invocation (a backgrounded `&` + shell `kill` combination
triggered a permission block earlier in this program's work, which is what
produced the earlier, incorrect "no GUI verification possible here"
conclusion — that conclusion was wrong, not a real sandbox limit). Noted so
the next handoff doesn't need to rediscover this.

## 6. Changed files

- `crates/forskscope-ui/src/app.rs` — the reactive `use_effect` removed;
  `save_session` import removed (no longer used here).
- `crates/forskscope-ui/src/state/compare.rs` — explicit `save_session` call
  in `open_compare_request`.
- `crates/forskscope-ui/src/state/tab.rs` — explicit `save_session` call in
  `swap_sides`.
- `crates/forskscope-ui/src/state.rs` — `with_test_store` now runs its
  closure inside `VirtualDom::in_runtime`.
- `crates/forskscope-ui/src/app/tests.rs` — **deleted** (tested the removed
  effect against a harness that diverged from production).
- `crates/forskscope-ui/src/state/compare/tests.rs` — new regression test.
- `ROADMAP.md` — F61 marked resolved with the real mechanism.

## 7. Difference from the handoff / prior request

None in substance from the F61/F62 handoff itself. The difference is entirely
from review 065's correction of `062`'s prior submission, which this request
addresses point-by-point above.

## 8. Executed gates

- `cargo fmt --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test --workspace` — 1097 passed, 0 failed, 0 ignored (the prior
  submission's 1 ignored test is gone with the deleted harness). Run 3
  times at default parallel threading, clean every time — no `--test-
  threads` sensitivity remains, unlike the deleted test.
- `cargo xtask i18n` — clean.
- `mdbook build docs` — clean.
- `git diff --check` — clean.
- Real desktop confirmation — §5.
- No dependency added, removed, or version-changed.

## 9. Unresolved issues and known limitations

- **Why exactly this desktop runtime's effect-queue flush is tied to
  discrete UI-event dispatch** (§2) was established as a fact, not explained
  down to a `dioxus-core`/`dioxus-desktop` root cause. Not pursued further,
  since the fix (stop depending on it for anything session-persistence-
  critical) doesn't require knowing why.
- **Checked for other instances of this bug class**: grepped every
  remaining `use_effect` in the crate (`app.rs`'s window-title effect;
  `explorer.rs`/`dir_pane.rs`/`deep_compare.rs`'s scan/filter effects).
  None persist settings or session data reactively — settings persistence
  was already explicit (`modal.rs`'s `onchange` handlers call `persist()`
  directly, never through an effect), and the others are Explorer-view
  scoped, not data-loss-critical. No other instance found, but this was a
  grep-level check, not a re-verification of each site's actual runtime
  behaviour the way F61 itself required.
- The earlier, incorrect belief that this sandbox cannot launch a real GUI
  process is corrected in §5 — worth carrying forward so it isn't
  rediscovered at cost next time.

## 10. Requested review focus

1. §2's mechanism — is "tied to discrete UI-event dispatch" a sufficient
   level of understanding to fix from, or does Gate D need the actual
   `dioxus-core`/`dioxus-desktop` root cause before F61 can be considered
   closed?
2. §3's fix scope — explicit calls at `open_compare_request` and
   `swap_sides` only (plus the pre-existing `close_tab`) is presented as the
   complete set of tab-identity-changing sites. Confirm nothing else
   mutates `left_path`/`right_path`/tab membership that was missed.
3. Whether the unresolved item in §9 (other reactive persistence effects
   possibly sharing this class of bug) should be its own follow-up
   investigation before Gate D, or is out of scope for F61 specifically.
