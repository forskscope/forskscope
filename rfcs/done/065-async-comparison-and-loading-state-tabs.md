# RFC 065: Asynchronous Comparison and Loading-State Tabs

**Status.** Implemented (v0.148.0)
**Tracks.** Preventing UI freeze when loading and diffing large or binary files;
a tab lifecycle that opens immediately in a loading state and resolves to a
result (or error) when the background computation completes.
**Touches.** `crates/forskscope-ui/src/state/mod.rs` (tab lifecycle, open
flow), `ui/diff.rs` (loading/loaded/error rendering), `ui/tabs.rs` (tab title
during load), `forskscope-core` (already exposes the load + diff functions; this
RFC moves their invocation off the render thread), and the async task model
already used for directory scans.

## Summary

Opening a comparison currently performs file load + decode + diff synchronously
on the UI thread. For large files — and especially large binary files rendered
as hex — this blocks the app: the reported symptom is the app "stacking"
(freezing) when comparing big binary files.

This RFC defines an **asynchronous comparison lifecycle**: opening a comparison
immediately creates a tab in a **Loading** state, the load+diff runs as a
background task, and when it completes the tab transitions to **Loaded** (showing
the diff) or **Error** (showing a recoverable message). The load is
**cancellable** — closing the tab or the app during load aborts the work.

This directly implements the roadmap's RFC-012 ("Large File, Timeout, and
Performance Policy") concern and the Risk Register entry "Large file diff hangs
UI → use background jobs, deadlines, cancellation, and progress".

## Motivation

A diff/merge tool that freezes on a large file fails at its core promise. The
freeze is also indistinguishable from a crash to the user. The directory-compare
path already uses background jobs with progress and cancellation; the
file-compare path must adopt the same discipline.

## Tab lifecycle

```
                 open comparison
                        │
                        ▼
                 ┌─────────────┐   background load+diff
                 │  Loading    │ ───────────────────────┐
                 │ (cancel ✕)  │                         │
                 └─────────────┘                         ▼
                        │                        ┌───────────────┐
            close tab / app exit                 │ task completes │
                        │                        └───────────────┘
                        ▼                            │        │
                  ┌──────────┐               success │        │ error
                  │ Cancelled │                       ▼        ▼
                  │ (aborted) │               ┌──────────┐ ┌──────────┐
                  └──────────┘               │  Loaded   │ │  Error    │
                                             │ (diff UI) │ │ (message) │
                                             └──────────┘ └──────────┘
```

### States

| State | UI |
|---|---|
| **Loading** | Tab opens immediately with a spinner/loading label and the file names. A **Cancel** control aborts the load. |
| **Loaded** | The diff renders as today. |
| **Error** | A recoverable message (friendly, per RFC-063 C10) with a retry/close action. |
| **Cancelled** | The tab closes, or shows "Load cancelled" with a retry — decide in implementation. |

### Tab title during load

The tab shows the file name with a loading affordance (e.g. a spinner glyph or
"… name"). The dirty marker logic is unaffected (a loading tab is never dirty).

## Background task model

Reuse the existing async task infrastructure used by directory scans
(`tokio` task spawn + a channel/signal the UI observes). The task:

1. Loads both files (decode, classify) off the UI thread.
2. Computes the diff (respecting the existing large-file deadline policy).
3. Sends the result (or error) back to the UI via a signal keyed by the tab's
   session id.

The UI observes completion and transitions the tab state. Because Dioxus
signals are the update mechanism, the completion handler sets the tab's state
signal, which re-renders only that tab.

### Cancellation

Each loading tab holds a cancellation token (or an `Arc<AtomicBool>`/abort
handle). Closing the tab, or app exit, signals cancellation; the task checks it
at safe points (after load, before diff; and ideally within the diff deadline
loop). A cancelled task drops its result without touching UI state.

## Interaction with binary policy (RFC-066)

Large **binary** files are the worst case today. RFC-066 makes binary comparison
off-by-default; when it is enabled and a large binary is compared, this RFC's
async lifecycle is what keeps the hex render from freezing the app. The two RFCs
compose: RFC-066 decides *whether* to compare; RFC-065 decides *how* the compare
runs (off-thread, cancellable).

## Non-goals

- No progress *percentage* for single-file diff (unlike directory scans, a
  single diff has no natural unit count). A spinner + cancel is sufficient.
- No partial/streaming diff rendering — the diff appears when complete.
- No change to the diff algorithm or deadline values (owned by RFC-012).

## Acceptance criteria

- Opening any comparison shows a tab immediately; the app never freezes during
  load or diff, including for large binary files.
- The loading tab can be cancelled; cancelling aborts the background work.
- On completion the tab shows the diff; on failure it shows a friendly,
  recoverable error.
- Closing a loading tab does not leak the background task or panic on completion
  of an already-closed tab.

## Cross-references

- RFC-012 — large file, timeout, performance policy (deadline values).
- RFC-063 C10 — friendly error messages (the Error state uses these).
- RFC-066 — binary comparison policy (composes with this lifecycle).
- Existing directory-scan async model — the pattern to reuse.

## Open questions

- Cancelled state: close the tab outright, or keep it with a "retry" action?
  Leaning toward closing on user-initiated close, and showing "cancelled +
  retry" only if cancellation was implicit. Decide in implementation.
- Should very small files skip the async path (synchronous fast path under a
  size threshold) to avoid a one-frame loading flash? Possibly; measure first.
