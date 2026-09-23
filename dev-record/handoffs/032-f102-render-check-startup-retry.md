# Handoff 032 — F102: release builds that survive a start-up flake

**From:** architect. **Release:** `0.171.0` (`ROADMAP.md` § *Release plan*).
**Register:** F102. **Small.**

## 1. What happened

`0.170.0`'s cut **failed** at `release.yml`'s F34 step. Under
`xvfb-run --auto-servernum dbus-run-session`, the app panicked in `tao` with
`Failed to initialize GTK` **before any application code ran**, and the script
then reported:

```
FAIL: forskscope never registered on the accessibility bus within 30s
```

It was environmental, established by evidence rather than assumed:
`render-check.yml` dispatched against the same `main` **passed**, its invocation
is byte-identical to `release.yml`'s, a `gh run rerun --failed` of the same
commit passed, and `0.170.1` passed the same step first time.

**Two defects, not one.** The step has no retry, so an infrastructure hiccup
fails a whole cut. And the message is misleading: *"never registered within
30s"* describes a slow app, when the process had already exited.

## 2. The fix belongs in the script, not the workflow

Both `release.yml:112-113` and `render-check.yml:85-86` call
`packaging/render_check.py`. A workflow-level retry would re-run the **geometry
assertions** as well — retrying genuine render failures until one happened to
pass. That is the one outcome this check exists to prevent.

In `render_check.py`, around the single `Popen` at `:217`:

- **Distinguish the two start-up failures.** A process that has **exited**
  (`poll()` is not `None`) before registering is a start-up failure. A process
  still running but never registered is the existing timeout. Report them
  differently, including the child's exit code and the tail of its stderr.
- **Retry only the first**, a small fixed number of times, logging each attempt.
- **Never retry anything after the app is found** — not `wait_for_ready`'s
  timeout, not a geometry mismatch.

## 3. Falsification

1. **A binary that dies at start-up on the first attempts and then runs** passes
   after retrying, and the log shows each failed attempt with its exit code.
   A stub script is fine for this; say what you used.
2. **A binary that always dies at start-up** fails after the bound with the
   *exited during start-up* message — and does not hang.
3. **A real render defect is not retried.** Use the existing
   `inject_geometry_defect` path (a `render-check.yml` input): it must fail
   **on the first attempt**, with no relaunch in the log. This one proves the
   retry cannot mask the defect class the check exists for.

## 4. Also — write the recovery down

`release.md` should say what to do when a tag is pushed and no draft appears:
`gh run rerun <run-id> --failed`, which keeps the green platform jobs, and
re-cut the tag only if code has to change. The register's *Release cycle*
table now states this; `release.md` is where a person doing a cut will look.

## 5. Scope

**In:** `packaging/render_check.py`, `docs/src/maintainers/release.md`.
**Out:** the other evidence harnesses, and F97's question of gating them all —
that is an owner disposition, not this handoff. Keep the retry logic small enough
that §3's stub test is its whole proof.

## 6. Gates

The usual set. Run `render-check.yml` by dispatch for §3.3 and give the run IDs;
this script cannot be exercised locally in a way that proves anything about CI.
