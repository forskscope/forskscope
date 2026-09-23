# Review Request 102 — F102: release builds that survive a start-up flake

Handoff: `dev-record/handoffs/032-f102-render-check-startup-retry.md`
Commit: `9540254` (pushed to `main`). CI run `34931436849` confirmed green.

## Falsifications, run for real

### 1 — a binary that dies twice then runs, retried and passes

A real stub (`.git-exclude/tmp/f102-falsify/stub_dies_then_runs.sh`, not
kept — untracked scratch): a bash script that increments a counter file
each invocation, exits 7 on attempts 1 and 2, and `exec`s the real
`target/release/forskscope` binary on attempt 3. Run locally (this
sandbox has a real desktop session with a working AT-SPI bus — confirmed
before relying on it):

```
attempt 1/3: forskscope exited during start-up (exit code 7) before registering on the accessibility bus
  stderr tail:
stub: simulated start-up crash on attempt 1
attempt 2/3: forskscope exited during start-up (exit code 7) before registering on the accessibility bus
  stderr tail:
stub: simulated start-up crash on attempt 2
OK: 7 left rows + 7 right rows all aligned within their pane.
```

Exit 0. The third attempt reached the real app, and the check proceeded
through `wait_for_ready` and `check_pane` normally — proving the retry
boundary doesn't leak past registration.

### 2 — a binary that always dies, fails after the bound, does not hang

Same approach, a stub that always exits 9:

```
attempt 1/3: forskscope exited during start-up (exit code 9) before registering on the accessibility bus
  stderr tail:
stub: simulated start-up crash, always
attempt 2/3: ...
attempt 3/3: ...
FAIL: forskscope exited during start-up on all 3 attempts
```

Exit 1, and it terminated — `time` showed 1:30.24 total, matching 3
attempts × `APP_TIMEOUT_S`'s unshortened 30s each (see the disclosed
non-optimization below), not an unbounded hang.

### 3 — a real render defect is not retried, on real CI

Handoff §3.3 explicitly asked for `render-check.yml` dispatch, not a local
proxy, and for run IDs. `gh workflow run render-check.yml
-f inject_geometry_defect=true`, run `34932008053`:

```
FAIL: F34 rendering check found misalignment:
  - left pane: a row's content starts at x=46, other rows start at x=26 - a column shift, the visual symptom of F32
  - right pane: a row's content starts at x=661, other rows start at x=641 - a column shift, the visual symptom of F32
```

No `attempt N/M` line anywhere in the log — the app registered on the
first try, and the geometry failure was reported and the job ended without
a second launch. This is the falsification the whole handoff exists to
protect: retrying a real defect until it happens to pass would be worse
than not retrying at all.

A plain dispatch with no injection, run `34931681014`, confirms the fix
doesn't add retry noise to the normal passing case either — one line,
first attempt:

```
OK: 7 left rows + 7 right rows all aligned within their pane.
```

## Design notes

**`launch_until_registered` owns the only retry.** It distinguishes
`find_app` returning `None` because the process exited (`proc.poll()` is
not `None` — F102's flake, retried) from `None` because the process is
still running and simply hasn't registered yet (the pre-existing timeout,
left exactly as it was — killed, reported, not retried). Everything after
registration — `wait_for_ready`'s timeout, `check_pane`'s geometry
assertions — is unchanged code, called exactly once, matching §2's
instruction that a workflow-level retry would have re-run the assertions
themselves.

**`STARTUP_MAX_ATTEMPTS = 3`**, a judgment call — the handoff asked for
"a small fixed number," not a specific value. Three matches this
project's other bounded-retry conventions loosely and kept falsification
2's local run under two minutes.

**Disclosed, deliberate non-optimization**: `find_app`'s own 30-second
poll loop was left untouched — it does not check whether the child process
has already exited, so each start-up-crash attempt still costs the full
`APP_TIMEOUT_S` before `launch_until_registered` notices the exit and
retries (confirmed by falsification 2's 90-second total). Shortening that
per-attempt cost was not asked for and would have meant touching
`find_app`'s signature/behavior beyond what a "small fixed number of
retries" required — flagged here rather than fixed silently, since it
means the worst case for a real, live-CI 0.170.0-shaped flake is still
up to 3× the original 30s budget before the plain-rerun fallback in
`release.md` would ever be needed.

**stderr capture**: `Popen(..., stderr=subprocess.PIPE, text=True)`,
tail-cut to the last 20 lines. stdout was left uncaptured (unbuffered to
the runner's own log, as before) — the handoff asked for "the tail of its
stderr" specifically, and GTK/WebKit's own crash chatter goes to stderr.

## Scope

In: `packaging/render_check.py`, `docs/src/maintainers/release.md`.
`release.md` now documents `gh run rerun <run-id> --failed` as the first
recovery step when a tag is pushed and no draft appears, distinct from the
existing draft-re-cut advice, which assumes a draft already exists. Out,
untouched: `render-check.yml`, `release.yml` (both already called
`render_check.py` the same way; no workflow edit was needed), the other
evidence harnesses, F97's gating question. `ROADMAP.md` left for the
architect.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (unchanged — no Rust touched),
`cargo xtask css/version-sync/i18n/rfc-sync/audit-deps`, `git diff
--check`, `mdbook build docs` — all clean. `python3 -m py_compile
packaging/render_check.py` — clean (no Python lint gate exists in `ci.yml`
to run beyond this). `actionlint` — clean, no workflow file changed. CI
run `34931436849` for `9540254` confirmed green. Two `render-check.yml`
dispatches (`34931681014` plain, `34932008053` geometry-defect) exercised
the real fix on real CI infrastructure, as the handoff required for §3.3.
