# RFC 075: Async Compare Identity and Load Generations

**Status.** Proposed
**Tracks.** Release-stabilization audit finding B1.
**Touches.** `forskscope-ui-logic::compare`, UI tab/store state, comparison
lifecycle, close/reload behavior, and deterministic race tests.

## Summary

Every asynchronous compare or reload operation will carry a stable tab identity
and a monotonically increasing load generation. A completion may mutate UI
state only when both values still match the target tab. Vector position and
`TabState::Loading` are not sufficient identity.

This prevents an old task from populating a different tab after vector
compaction and prevents an earlier reload from overwriting a later reload.

## Problem

The current lifecycle captures a mutable `Vec` index. Closing a lower-index tab
changes subsequent indices. Reloading reuses the same index and returns the tab
to `Loading`. In both cases, an obsolete completion can pass the current guard.

The required property is:

> A result created for load token `(tab_id, generation)` can be committed only
> to that exact live token.

## Goals

- Give each compare tab a unique identity for the lifetime of the process.
- Give each load/reload attempt a new generation for that tab.
- Resolve completions by identity, never by captured position.
- Make acceptance/rejection logic pure and unit-testable without GTK.
- Preserve current background loading and responsive tab behavior.
- Define close, reload, swap, and session-restore interactions explicitly.

## Non-goals

- Persist tab IDs or generations across application restarts.
- Add cross-process cancellation or durable background jobs.
- Redesign Dioxus signals, tab presentation, or diff computation.
- Guarantee that obsolete blocking I/O stops immediately; rejecting its result
  is the correctness requirement. Cancellation is an optional optimization.

## Model

Add framework-independent lifecycle types to
`forskscope-ui-logic::compare::load_identity`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CompareTabId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LoadGeneration(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadToken {
    pub tab_id: CompareTabId,
    pub generation: LoadGeneration,
}

pub enum CompletionDecision {
    Accept,
    RejectTabMissing,
    RejectGenerationMismatch,
    RejectNotLoading,
}
```

Constructors expose values needed for logging/tests but prevent arbitrary
mutation. `LoadGeneration::next()` uses checked addition; exhaustion returns a
recoverable internal error instead of wrapping and reusing an identity.

`CompareTab` gains:

```rust
pub id: CompareTabId,
pub load_generation: LoadGeneration,
```

`Store` gains a process-local allocator. The preferred representation is a
root-owned `Signal<u64>` so test stores are deterministic. IDs start at 1 and
are never reused after close. ID 0 is reserved for invalid/uninitialized data.

## State transitions

### Open

1. Allocate a new `CompareTabId`.
2. Create the tab with generation 1 and `Loading` state.
3. Capture `LoadToken { id, generation }` in the task.
4. On completion, search the current vector for `tab.id == token.tab_id`.
5. Accept only when the generation matches and state remains `Loading`.

### Reload

1. Find the live tab by the caller's current index.
2. Increment its generation before spawning.
3. Set `Loading`; capture paths/options and the new token atomically from the
   same mutable borrow.
4. Commit only if the token remains current.

The previous load may continue consuming I/O/CPU, but its completion is
rejected. A later optimization may associate a cancellation token with each
generation.

### Close

Removing a tab invalidates every outstanding token for that ID. Vector
compaction is irrelevant because completion resolution searches by ID.

### Swap and option recomputation

Operations that synchronously recompute from already-loaded documents do not
change the generation. Any operation that triggers new file I/O must increment
the generation and use the token path.

### Session restore

Restored tabs receive new process-local IDs. Nothing persists the old identity.

`CompareTabId` is specifically a runtime concurrency identity. It is not the
legacy `forskscope_core::session::TabId`, which was designed as persisted
workspace metadata. RFC-076 schema v2 does not persist either runtime tab IDs
or load generations; its core-v1 migration consumes legacy IDs only to parse
old envelopes. Keeping the names/types separate prevents a restored identifier
from accidentally validating a task created by another process lifetime.

## Completion API

Centralize result installation in one helper rather than duplicating guards in
`open_compare` and `reload_tab`:

```rust
fn commit_load_result(
    tabs: &mut [CompareTab],
    token: LoadToken,
    result: LoadResult,
) -> CompletionDecision;
```

The helper locates by ID, calls pure `completion_decision`, and installs the
result only on `Accept`. Rejected completions are expected lifecycle events;
they do not show a user-facing error.

## Error and observability policy

- Join/load failures for the current token become the existing tab error state.
- Failures for an obsolete token are discarded without changing the new load.
- Debug logging may record the rejection reason without paths or file content.
- ID/generation allocation failure becomes `TabState::Error` and blocks new
  work rather than reusing an identifier.

## Test design

### Pure unit tests in `forskscope-ui-logic`

- same ID, same generation, Loading -> Accept;
- missing ID -> RejectTabMissing;
- same ID, older generation -> RejectGenerationMismatch;
- same token but Ready/Error -> RejectNotLoading;
- generation increments and never wraps silently.

### Deterministic UI-state tests

Use prepared `LoadResult` values; do not depend on task scheduling or sleeps.

1. Create loading tabs A and B, capture B's token, close A, commit B: B receives
   B's result despite its changed vector position.
2. Capture A generation 1, begin reload generation 2, commit generation 1:
   rejected; commit generation 2: accepted.
3. Close A, create C in a reused vector position, commit A: rejected by ID.
4. Current-token failure changes only its own tab.
5. Obsolete failure does not replace a newer Ready result.

If Dioxus types prevent GTK-free UI-state tests, extract the tab collection
transition into `forskscope-ui-logic`; do not replace deterministic tests with
timing-based GUI tests.

## Compatibility and migration

IDs and generations are in-memory only, so settings/session schemas do not
store them. RFC-076 independently advances the persistence envelope to schema
v2 for model convergence, not because of these tokens. Public CLI behavior and
restored path pairs remain unchanged.

## Security and safety impact

The change prevents integrity failures in which content from one user-selected
path pair is displayed or saved under another tab identity. It introduces no
new data flow, network capability, or persisted sensitive data.

## Implementation sequence

1. Add lifecycle types and pure tests to `forskscope-ui-logic`.
2. Add tab fields and the store allocator.
3. Add centralized commit helper and deterministic state tests.
4. Migrate `open_compare`.
5. Migrate `reload_tab`.
6. Audit every `TabState::Loading` assignment for generation ownership.
7. Run workstream and workspace gates.

## Acceptance criteria

- No async completion uses a captured vector index as identity.
- Every file-I/O load attempt owns a unique `(tab_id, generation)` token.
- Close/reindex and stale-reload regression tests pass deterministically.
- Existing compare, reload, session restore, close, and multiple-tab tests pass.
- Threat-model wording is updated to describe the token guard precisely.
- The developer handoff under `rfcs/handoffs/075-.../` is completed with
  observed gate output.

## Alternatives considered

### Index plus state guard

Rejected: it is the current bug; neither value is stable identity.

### Stable ID without generation

Rejected: it fixes close/reindex but not two overlapping reloads of one tab.

### Cancellation token only

Rejected as the correctness boundary. Blocking work may complete after
cancellation, and cancellation delivery itself races. Identity validation is
still required.

### Store tabs in a map

Not required. The ordered vector remains useful for rendering; lookup by ID is
small and bounded by the number of open tabs. A map can be reconsidered only
from measured scale evidence.

## Dependencies

- Parent: RFC-074.
- Required before RFC-077.
- Related: RFC-065 asynchronous loading, RFC-036 external changes.
