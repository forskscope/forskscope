# RFC-078 M5-B Developer Handoff: The Interaction Cases

**Governing RFC.** [RFC-078](../../proposed/078-platform-runtime-acceptance.md), under [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** M5-B — the second slice of platform acceptance
**Cases.** P04, P05, P06, P08, P12
**Candidate.** `0.167.0` — the same published artifacts and digests M5-A used
**Baseline.** `main` at `2558cb3`

`matrix-plan.md` is **frozen**. If execution shows a row needs to change, stop
and report. §3 below is a case-definition question that must be settled *before*
you execute, not during.

## 1. What changes from M5-A

M5-A's four cases needed only launch and command-line arguments. **All five here
need in-app interaction**, which makes this the harder slice and the one where
M5-A's central lesson matters most:

> Invoke the platform's native accessibility action directly — AT-SPI
> `Action.do_action`, Windows UIA Invoke, macOS `AXPress` — rather than
> synthesizing keyboard or mouse input.

M5-A established that over five failed CI iterations: under a bare Xvfb there is
no window manager, `XSendEvent` is ignored by GTK, and X11 focus does not imply
widget focus. Start from accessibility actions on every platform.

The evidence set already exists under
`docs/src/maintainers/release-evidence/0.167.0/`. **Append to the existing row
files**; do not restructure them or re-record the header fields.

## 2. Scope

P04, P05, P06, P08, P12 on the CI-verified rows. Out of scope: P03, P07, P11 and
the owner's manual passes (M5-C), and **F59's documentation fix** — now
unblocked, but ordinary work against a later candidate, not part of an evidence
pass tied to fixed artifacts.

## 3. Settle this before executing — P04's "keyboard and mouse"

RFC-078 P04 requires:

> apply focused hunk **with keyboard and mouse**

**M5-A's findings say CI cannot do that**, and pretending otherwise would be the
exact laundering this milestone exists to prevent. Accessibility-action
invocation is neither a keystroke nor a click — it calls the handler directly,
bypassing the input path the case names.

So one of these is true, and you must say which before running P04:

1. The case's intent is *"both invocation paths reach the same handler"*, and
   accessibility invocation demonstrates the handler while the input paths go
   unverified on CI — in which case P04's CI result is **partial**, with the
   keyboard/mouse portion recorded as manual-outstanding like F45's sub-case.
2. The case means literal input synthesis, in which case P04 **cannot** be
   CI-verified at all and belongs to the owner's manual rows.

**Do not choose silently.** Report your reading with the reasoning; if it means
amending RFC-078's case text, that is an RFC amendment and comes back to me
before evidence is gathered under it. Evidence collected against a case whose
meaning was changed mid-run is what `matrix-plan.md`'s freeze exists to prevent.

The same question may apply in weaker form to P11 in M5-C. Settling it here
settles it there.

## 4. P08 is the highest-value case in this slice

F37's amendment requires **all three recovery-dialog choices — Exit, Continue
(either variant), Reset — exercised on every platform row**, not only where a
fixture happens to trigger the dialog.

**Exit is the one that matters.** It terminates the process from inside a modal
during startup, while a WebView-hosted event loop is running — precisely the
path that can hang, orphan a process, or differ across WebKitGTK, WebView2 and
WKWebView. Linux-only evidence says nothing about the other two.

A row is not P08-complete until each choice has been observed to resolve the
dialog *and* leave the process in the expected state:

| Choice | Expected state |
|---|---|
| Exit | fully exited, **no orphaned process** |
| Continue (either) | running normally, dialog dismissed, nothing written |
| Reset | dialog dismissed, file reset, original backed up |

Assert the process state, not just the dialog's disappearance. An Exit that
dismisses the dialog and leaves a zombie is a failure that looks like a pass.

The rest of P08 — legacy migration without loss, backup and versioned envelope,
future-schema fixture preserved, corrupt fixture preserved until explicit reset
— is filesystem-observable and should automate cleanly. Use **sanitized**
fixtures; no real paths or usernames, per RFC-078's schema.

## 5. The other three

- **P05 — External modification.** Mostly filesystem assertions and therefore
  the easiest here. Verify `.bak` *bytes* equal the externally changed version,
  not merely that a `.bak` exists, and that Save As leaves the original target
  untouched.
- **P06 — Async identity.** Needs at least two deliberately slow comparisons.
  RFC-078 says the deterministic tests remain the primary proof and this case
  confirms runtime integration — so a light but real exercise is right; do not
  build elaborate timing machinery. Note review 056 §5.3's standing rule: **any
  P06 defect on any row upgrades every P06 spot-check on that platform to
  Required.**
- **P12 — Session/settings restart.** Change theme/language/font, restart,
  verify restoration; verify tabs restore only when no explicit CLI paths were
  given. The Japanese-label portion is a practical-workflow check, not a
  translation audit.

## 6. Falsifiability

Unchanged and non-negotiable: for every case, demonstrate the assertion failing
against a deliberately broken condition, then revert. M5-A's `break_case`
mechanism already provides the shape — extend it rather than inventing a second
one.

Two of these are easy to write vacuously, so state explicitly what would make
them fail: **P06** (a check that passes whenever two tabs merely exist) and
**P08's Exit** (a check that passes whenever the dialog closes, regardless of
whether the process died).

## 7. Constraints

- `0.167.0`'s published artifacts only, digest-verified before each run. Same
  digests as M5-A.
- No dependency is added, removed, or version-changed — including
  `dioxus-desktop`. If it releases mid-slice, **stop and report**: new artifacts
  mean new digests and re-run cases.
- No product behaviour changes. A defect found is registered and reported, not
  fixed.
- `matrix-plan.md` is frozen. §3 is the one place a definition question is open,
  and it comes back to me.
- Do not weaken a case to make it pass.

## 8. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. **§3's resolution and reasoning** — before anything else, since everything
   about P04 depends on it;
2. implementation summary — how each case runs on demand;
3. cases executed, per row, with results;
4. **P08's three-choice evidence per row, including process state** (§4);
5. falsifiability demonstrations with observed output, including §6's two;
6. created and changed files;
7. any product defect found, registered not fixed;
8. any difference from this handoff, RFC-078, or the frozen plan;
9. executed gates with observed output;
10. unresolved issues and known limitations;
11. requested review focus.

## 9. After this slice

M5-C: P03, P07, P11, the owner's `linux-wayland` manual pass, F45's Windows
sub-case, and evidence assembly. Then Gate D — which on current knowledge cannot
pass while F44 is open, and which F60's Windows-floor question also feeds.
