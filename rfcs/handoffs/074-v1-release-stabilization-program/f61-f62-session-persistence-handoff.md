# F61/F62 Developer Handoff: Session Persistence and Its Silence

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md), touching [RFC-076](../../done/076-versioned-runtime-persistence.md)
**Register items.** F61, F62
**Baseline.** `main` at `92fb9e3`
**Standing.** F61 is an **un-waivable Gate D blocker** per RFC-078's waiver policy ("silent settings/session loss"). This is the one on the v1 critical path that this project can actually fix.

This handoff directs execution of one slice. It does not redefine RFC-074. This
is **product work, not evidence work** — it is deliberately outside M5, and it
requires a new candidate before M5's P12 rows can be re-run.

## 1. What is known, and what is not

**Known — reproduced four times independently** (three M5-B harnesses on three
platforms, plus review 064 on a real Linux desktop with an isolated
`XDG_CONFIG_HOME`):

```text
forskscope <left> <right>        → no session.json, 14s later
close that tab (AT-SPI invoke)   → session.json appears immediately
```

So a tab opened from CLI startup arguments is never persisted, while
`close_tab`'s direct `save_session` call persists correctly in the same process
and environment.

**Not known — the mechanism.** `app.rs:86` has:

```rust
// Persist the session whenever the tab list changes.
use_effect(move || {
    let _tabs = store.tabs.read(); // subscribe to the tabs signal
    save_session(&store);
});
```

That effect is *supposed* to cover every mutation site. Why it produces no write
for a CLI-opened tab is unestablished. Plausible shapes, none confirmed:

- the effect runs once while `tabs` is still empty and never re-runs when the
  startup tab arrives asynchronously (RFC-075's load path);
- it does re-run and calls `save_session`, but the write fails and **F62 eats
  the error**;
- it re-runs but `save_session` reads an empty or stale tab list;
- `session_write_disabled` is set during startup for a reason nobody intended.

**Do not skip to a fix.** Adding a direct `save_session` call to the startup
path would make the symptom disappear and leave the reactive mechanism broken —
so the *next* mutation site added by anyone would silently fail to persist, and
nobody would find out until another platform matrix. The reactive effect either
works or it should be removed in favour of explicit calls everywhere. Establish
which.

## 2. Do F62 first — it is the diagnostic

`persist_session` and `persist_settings` discard their write `Result` with
`let _ =`. Every persistence failure is therefore invisible: no error, no toast,
no log.

Fix that first, and it may tell you what F61 is. With the `Result` handled, a
run of the CLI path either reports a failed write (answering the question
immediately) or reports nothing at all (proving `save_session` is never reached,
which points at the effect rather than the write).

That ordering turns a guessing exercise into an observation.

**What "handled" means is a design decision, not a mechanical change.** A
persistence failure at startup is not the same as one during a save the user
asked for. Options include surfacing through the existing recovery/notice path
RFC-076 built, a toast, or a diagnostic-only log. Choose deliberately, and say
what a user sees in each case. What is not acceptable is a second silent
discard wearing different syntax.

Note the constraint RFC-076 already establishes: a document whose source could
not be established is **not written to**, and that must stay true. Reporting a
failure must not become a reason to overwrite something.

## 3. Then F61, with the mechanism established

State the cause before the fix. Then fix it at the level the cause sits at:

- If the effect never re-runs, the bug is in the subscription and the fix is
  there — not a compensating call elsewhere.
- If the effect runs but writes an empty list, the bug is ordering between
  startup tab creation and the effect.
- If the write is attempted and fails, F62's fix already exposed it and the
  cause is in the repository/path layer.

**Tests, and this is the part that matters.** RFC-076's persistence layer is
well covered at the core level and none of it caught this, because the defect
lives in the UI's reactive wiring — the `Store`-dependent seam F36 named. You
now have `with_test_store` (F36, M4-B), built precisely for this. A regression
test that opens a startup tab through the real production path and asserts the
session was persisted is the deliverable that stops this recurring; the fix
without it is half the work.

If `with_test_store` cannot reach the startup path, say so explicitly rather
than substituting a weaker test — that is itself a finding about F36's harness.

## 4. Scope

In scope: F61, F62, their tests, and correcting `README.md:96`'s claim if the
fix does not make it true (see §5).

Not in scope:

- **F63** — macOS's accessibility size threshold. Unresolved whether product or
  harness; separate investigation.
- **F44** — upstream.
- M5-C, and re-running M5's P12 rows. That happens against a new candidate,
  after this lands.
- Any other persistence redesign. RFC-076 is implemented and accepted; this is a
  defect in its UI wiring, not a reopening.

## 5. A claim to check afterwards

`README.md:96` states:

> **Session persistence** — open tabs are restored on next launch

That is currently false for CLI-opened tabs. F16's audit passed it because the
claim holds on the Explorer path — a real limit of claim-auditing, not a mistake
in that slice.

After the fix, verify the claim is true **on both paths**, or narrow it. Do not
leave a claim that is true only when the user arrives one particular way.

## 6. Constraints

- `0.165.0`, `0.166.0`, `0.167.0` are published and immutable.
- No dependency is added, removed, or version-changed — `dioxus-desktop`
  included.
- Do not weaken RFC-076's write-disable guarantees (§2).
- No evidence files are edited. M5's records describe `0.167.0` and stay as they
  are; this fix produces a *new* candidate, and re-running P12 is later work.

## 7. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. **F61's established mechanism** — what actually prevented the write, and how
   you established it, before any fix is described;
2. F62's chosen handling and what a user sees when a persistence write fails;
3. the fix for each, at the level the cause sits at;
4. **the regression test through the real startup path**, or an explicit
   statement of why `with_test_store` could not reach it;
5. runtime confirmation that a CLI-opened tab now persists — the same
   observation review 064 made, now passing;
6. §5's claim check;
7. changed files;
8. any difference from this handoff;
9. executed gates with observed output;
10. unresolved issues and known limitations;
11. requested review focus.

## 8. After this slice

A new candidate is cut, M5's P12 rows re-run against it, and F61 clears as a
Gate D blocker. **F44 remains**, so v1 stays No-Go until the upstream
`dioxus-desktop` release lands — but at that point the blockers this project
controls are closed.
