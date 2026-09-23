# RFC-076 patch 4 — C1 follow-up review

**Review date:** 2026-08-03
**Request:** `dev-record/review-requests/038-rfc076-patch4-c1-followup.md`
**Baseline:** `15f3227` (`persist: resolve session file unconditionally at startup (review 041 C1)`)
**Responds to:** review 041, correction C1
**Review mode:** Independent verification; no implementation changes made.

## 1. Verdict

**Approved.** C1 is closed, and with it **audit finding B2 is closed** — the
first of the three correctness blockers from the 2026-07-15 audit to be fully
resolved through to reachable production code.

One note for patch 6, not a finding against this work.

B3 and B4 remain open; v1/public release stays **No-Go**.

## 2. Verification

### The split is correct and the ordering is enforced

`app.rs` now resolves before it branches:

```rust
let (session_resolution, session_notice) = resolve_session(&mut store);
...
if let Some(Some((left, right))) = STARTUP_PAIR.get() {
    open_compare(...);                                   // CLI mode
} else {
    restore_tabs(&mut store, &session_resolution);       // needs the resolution
}
```

`resolve_session` is outside the branch, so `session_write_disabled` is set on
every launch path. `restore_tabs` takes `&SessionRuntimeResolution`, so it cannot
be called without a resolution having been produced — the ordering is enforced by
the signature rather than by convention.

The comment above it records *why*, with the review reference. That is the right
place for it: the next person to restructure this hook needs to know that
resolution being outside the branch is load-bearing, not incidental.

### The regression test proves the property

`future_version_session_stays_byte_identical_through_a_disabled_save` writes a
`schema_version: 99` envelope, confirms `write_disabled`, then calls
`save_session_if_allowed` with a tab-pair payload — exactly what `open_compare`'s
effect produces in CLI mode — and asserts the file is byte-identical.

Extracting `save_session_if_allowed(write_disabled, payload, repo)` is what makes
this testable: the gate itself is now exercised, not merely its plumbing. Given
`Store::new` panics outside a Dioxus runtime, this is the strongest available
proof, and the reasoning for not attempting a full runtime test is sound and was
verified rather than assumed.

### Manual verification reproduced the actual scenario

Replacing the real `session.json` with a future-version envelope, running the
real binary in CLI mode, confirming the toast, then confirming the file
byte-identical afterwards — and restoring the original config from its
`.pre-v2.bak` — is exactly the exercise C1 described. Checking the file rather
than trusting the toast is the part that matters.

## 3. Answers to the requested review focus

### 3.1 Two functions, or one with a flag?

**Two functions, as implemented.** A boolean parameter meaning "also restore
tabs" would encode the coupling that caused C1 rather than remove it — the bug
was precisely that one function did two jobs with different preconditions.

Three concrete reasons the split is better than a flag:

- resolution must happen **exactly once, unconditionally**; restoration is
  conditional. A flag lets a future caller pass `false` and skip both again.
- `restore_tabs(&resolution)` makes the dependency a type-level fact. A flag
  makes it a comment.
- the invariant is now checkable by reading `app.rs` alone — resolve outside the
  branch, restore inside. With a flag you would have to read the callee to know
  whether resolution happened.

### 3.2 Toast priority

**Still correct.** `use_context_provider`'s initialiser runs before `use_hook`'s
on first render, so a settings notice is already in `store.toast` when the
session notice is considered, and the `is_none()` guard lets settings win.
Unchanged in behaviour despite `resolve_session` moving earlier within the hook.

## 4. Note for patch 6 — not a finding here

Both documents can be write-disabled on the same launch: a corrupt `settings.json`
alongside a future-version `session.json` is entirely possible. Today the session
notice is dropped when a settings notice exists, which you disclosed in patch 4
as an accepted trade-off for toasts.

That trade-off must not survive into the recovery dialogs. A user in that state
would be told their settings will not save and told nothing about their session,
while both are silently read-only. **Patch 6's recovery UI must be able to
communicate both.**

Recorded against F28, which already covers the recovery UI telling the user the
truth about write-disabled state.

## 5. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test --workspace` | Pass — **1065** (709+27+16+2+21+21+255+6+7+1), exactly 1063 + 2 |
| `cargo clippy --workspace -- -D warnings` | Pass |
| CI run `30797216154` | `success` on `15f32276` |
| `resolve_session` runs outside the `STARTUP_PAIR` branch | Confirmed |
| `restore_tabs` requires a resolution by signature | Confirmed |
| `save_session_if_allowed` gates on the flag | Confirmed |
| Regression test asserts byte-identity | Confirmed |
| Toast priority — settings wins | Confirmed |

## 6. A process note that is mine, not yours

The RFC-076 amendment, the convergence-cleanup handoff, and the ROADMAP register
updates were sitting uncommitted in the working tree and were carried into your
commit. You disclosed that accurately and attributed it correctly.

The fault is mine: after review 032 I said architect planning changes should land
as their own commit before a handoff goes out, and `bd37dd9` did exactly that.
This time I wrote those documents and left them uncommitted before handing over.
I will commit architect changes separately before the next handover.

## 7. Recommended next action

1. Treat C1 as closed and **B2 as closed**.
2. Begin patch 5 — convergence cleanup, per
   `rfcs/handoffs/076-versioned-runtime-persistence/convergence-cleanup-handoff.md`.
3. Then patch 6 — recovery UI and documentation, carrying F28 as extended in §4.
   Its dialog remains release-blocking for M2's cut.
4. Review 041 §4.3 (`Store::new`'s signature) is now unblocked and may be
   revisited during patch 5 if a natural shape emerges; it is not required.
