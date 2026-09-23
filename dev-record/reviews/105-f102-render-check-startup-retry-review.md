# Review 105 — Request 102: F102, render-check start-up retry

**Reviewer:** architect. **Date:** 2026-09-15. **Reviewed:** `9540254`.
**Verdict:** **Approved retry design. One required follow-up, in handoff
034.** The new stderr capture introduces a pipe hazard on both code paths.
F102 stays open until that lands.

## 1. The retry boundary is right

`launch_until_registered` (`packaging/render_check.py:217`) retries exactly one
case: the process exited before registering. It does not retry a process that
is still running, and it does not retry `wait_for_ready` or `check_pane`. Each
of those runs once, after the function returns.

The real-CI dispatch is the evidence that matters. Run `34932008053`, with
the geometry defect injected, failed and contained **zero** `attempt` lines. I
confirmed that count myself.

`STARTUP_MAX_ATTEMPTS = 3` is fine. You disclosed that `find_app` costs a full
30s per crash, so the worst case is 90s. That is acceptable, and I don't want
it changed now: the change would touch `find_app`, and nothing requires it.

`release.md`'s `gh run rerun <run-id> --failed` entry is correct, and it sits
in the right place, apart from the draft re-cut advice.

## 2. Required: `stderr=subprocess.PIPE` is drained on neither path

Before `9540254`, the app's stderr went straight to the runner log. It now goes
into a pipe that the script reads at most once, and only after a crash.

### 2a. Success path: the pipe is never read

After `return proc, app` (`:242`), nothing reads `proc.stderr` during
`wait_for_ready` and `check_pane`. Once about 64 KiB is buffered, the app's
next write to stderr blocks and its main thread stalls. AT-SPI queries then go
unanswered, and `wait_for_ready` reports *"compare view did not reach the
expected render shape"*.

That is a **false render failure, unretried by design.** The fix would turn
an infrastructure flake into exactly the failure class F102 set out to stop
producing.

Probe (mine): a child that writes 200 KiB to stderr while the parent never
reads it.

```
probe1 child still alive after 3s: True | finished writing: False
```

This hazard is **latent, not observed.** Launched locally, forskscope wrote
0 bytes of stderr in 8s. CI's headless GPU stack may be chattier, but nobody
has measured it. It is still required: the fix is a few lines, and the failure
it prevents would look like a real render defect.

### 2b. Crash path: `proc.stderr.read()` waits for every holder of the pipe

`read()` at `:262` returns only at EOF. EOF arrives when **every** process
holding the write end has closed it, not when forskscope exits.

I checked what holds it by launching the release binary against the
render-check fixtures:

```
descendant WebKitNetworkPr: fd2 -> <app's stderr> | shares app stderr: True
descendant WebKitWebProces: fd2 -> <app's stderr> | shares app stderr: True
```

After a clean `terminate()`, both exited within a second. After a *crash*,
they outlive the main process until they notice their IPC peer has gone. For
that whole time, `read()` blocks with no timeout.

Probe (mine): a child that exits while a background child still holds stderr.

```
probe2 exit 7 | stderr.read() blocked for 6.0s after exit
```

Your stub falsifications were bash scripts with no child processes, so they
could not have shown this. That is not a flaw in how you ran them. It is a
case they did not cover.

### The fix: a file, not a pipe

Open `Path(scratch) / f"forskscope-stderr-{attempt}.log"` and pass that handle
as `stderr=`. `scratch` is already a `TemporaryDirectory`.

- **On a crash,** read the file's last 20 lines. There is no EOF to wait for.
- **On success,** there is nothing to drain.
- **On any failure after registration,** print the same tail. The
  `wait_for_ready` and geometry `FAIL` paths print no app stderr today, and
  that is the log you would want.

Close each handle. Do not use a background reader thread: it adds a lifetime
to manage, and a file removes the problem instead.

## 3. Register

- **F102:** stays at *0.171.0 — handoff 034*.
- **0.171.0:** the cut waits on 034, which also carries review 106's
  follow-ups.
