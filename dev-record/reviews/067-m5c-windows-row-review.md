# M5-C Windows row review — P03, P07, P11

**Review date:** 2026-08-16
**Request:** `dev-record/review-requests/064-m5c-windows-visual-navigation-and-assembly.md`
**Governing document:** `rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md`
**Review mode:** Independent verification, including reproducing defect B's premise on a real Linux desktop.

## 1. Verdict

**Approved as evidence.** Prerequisite B is resolved properly, P03 exceeds its
required floor with real proof, and both failures are genuine findings rather
than harness defects or given-up cases.

**Both candidate defects are real and now tracked** — F69 and F70. I verified
defect B's premise independently (§2), which is what turns it from a plausible
report into a finding.

The concurrency incident in §6 is more serious than it reads and is registered
as **F71** (§5.3).

Gate D is unaffected in direction: **F44 still blocks it**, and these add two
inputs rather than changing the outcome.

## 2. Defect B verified — Linux is correct, so the platform framing holds

The claim rests on Linux behaving correctly, which the request cites as
"recorded elsewhere." I tested it directly rather than taking the
cross-reference, on a real desktop with an isolated profile:

```text
applied a hunk (tab dirty) → clicked "Reload files from disk"
modal buttons: ['Cancel', 'Discard and Reload']
FOCUSED:       [('button', 'Cancel')]
```

**Focus lands on Cancel on WebKitGTK.** So the app's reliance on the HTML
`autofocus` attribute for a dynamically-mounted modal holds on one engine and
not the other, and your report is a genuine WebView2 behavioural difference —
not a harness artifact and not a general defect we had simply never noticed.

That matters for how it gets fixed: the app is not doing anything obviously
wrong, so the remedy is an explicit focus call rather than a corrected attribute.
And because **every destructive modal in the app uses this same pattern** — you
read all seven rather than sampling — it is the pattern failing, not one modal.

Registered as **F69**, a Gate D input.

Your closing observation deserves the weight you gave it: RFC-078's P11 warns
about focus starting on the *destructive* action, and what you found is worse —
focus never enters the modal at all, staying on a control the modal covers. The
inference you drew but correctly refused to assert (that global shortcuts may
not be inert behind a modal on Windows either) is the right shape: flagged,
unconfirmed, and untestable given the keyboard gap.

## 3. Defect A — registered, with a cheaper resolution than a rebuild

Five CI runs, four independent trigger mechanisms, and a 150-second poll that
never moved the accessible-text-node count off the empty state's 54. That is
thorough, and ruling out slowness, fixture-specific paths, and mount-time races
individually is what makes it credible rather than a shrug.

**Registered as F70.** Two additions:

**The product-versus-environment question is itself a Gate D input**, not a
precondition for recording one. If it is a product defect, P07 fails on a
supported platform; if environmental, the Windows row has an unverifiable case.
Either outcome belongs in the Gate D list, so it does not wait for resolution.

**There is a cheaper resolution than a rebuild with instrumentation:** the owner
has a Windows 11 manual host. Opening the Explorer there and seeing whether
directories list answers it in seconds. Worth doing before anyone builds an
instrumented binary — and note the significance you already identified, that **no
M5-A or M5-B case touched Explorer at all**, so there is no prior Windows
evidence either way.

## 4. Prerequisite B and P03 — resolved better than asked

`ClassName` mirroring the DOM `class` attribute literally is a genuinely useful
finding, and you established it empirically against the published artifact rather
than assuming it. The consequence is worth stating: Windows now has a **more
precise** row filter than Linux's ARIA-role-derived match, so P03 on Windows is
not the "basic layout observation" RFC-078 sets as its floor — it is full
parity, achieved rather than waived.

Doing this before gathering any P03/P07 evidence is exactly what the handoff
asked and the reason the evidence is worth anything.

## 5. Answers to the requested review focus

### 5.1 Are A and B real? — B yes, verified; A undetermined but tracked

See §2 and §3. B is real. A is genuinely open, and its openness is recorded
rather than resolved by assumption in either direction.

### 5.2 The scroll-mirror methodology — sound, and stronger than the alternative

Sound. The concern this program has expressed about input synthesis was
specifically about **keyboard events under a bare Xvfb with no window manager**,
where `XSendEvent` is ignored and X11 focus does not imply widget focus. None of
that applies to a mouse-wheel message on a real Windows desktop session, and the
evidence is *positive*: both panes' rectangles moved by an identical 1,000px,
twice, cumulatively. An accessibility-pattern read would have been weaker — it
would have shown a capability, not a mirrored outcome.

Polling for settling rather than sampling once is the right reading of "without
feedback/jitter," and the off-screen-target bug you found and fixed in your own
diagnostic (a cell rectangle not clipped to the viewport, midpoint at x=7743 on a
1044px window) is the kind of thing that would otherwise have produced a
confident wrong answer.

One limitation to record, not to fix: this proves the mirror under a synthetic
horizontal wheel. A trackpad gesture or drag-scroll exercises the same
`install_hscroll_sync` mechanism, so the coverage is real, but the input path is
one of several.

### 5.3 The concurrency issue — yes, standing practice, and it has a precedent

This needs more than a note. **The bundled-commit incident is identical to
2026-08-04**, when my own `git commit` without a pathspec swept the dev team's
staged `git rm`/`git mv` work into a docs commit. Same cause, same remedy
(explicit trailing `-- <pathspec>`), rediscovered at cost by someone who had no
way to know it had happened before.

The second half is worse and newer: `windows_harness.py` repeatedly containing
P03/P07/P11 implementations this effort had not written, **citing this effort's
own CI run IDs**. Two agents on one assignment in one working directory. That it
produced usable output is luck, not design — and the verification instinct
(dispatching real CI to confirm or refute rather than trusting by inspection,
which caught two real breakages) is what kept it safe.

Adopted as **F71**, standing practice: parallel per-row agents get separate git
worktrees, or the row assignment is explicit and exclusive before work starts;
and `git commit` always carries a pathspec. No further remediation is needed for
the incident itself — no work was lost, and the disclosure is complete.

### 5.4 Both defects are tracked now, not held for my reproduction

F69 and F70, registered. Holding a defect until the reviewer reproduces it is
how a finding gets lost between two people who each think the other has it.

## 6. Notable quality observations

- Settling Prerequisite B first, with committed diagnostics that carry **no case
  ID, no `--break`, no assertion** — an explicit non-evidence artifact, which
  keeps a probe from being mistaken for a result later.
- Reading all seven destructive modals rather than sampling one, so "this modal
  is representative" is a checked claim.
- Reporting that `--break` cannot demonstrate P11's falsifiability *while the
  defect persists*, instead of quietly accepting a failing check as a passing
  demonstration. That is a genuinely awkward thing to write down.
- Not attempting P07's remaining sub-checks after the listing blocker, and
  saying they are **unreached** rather than unverified. Those are different
  words and you chose the accurate one.
- Disclosing the bundled commit in full, including which files and how many
  lines, when nothing would have surfaced it.

## 7. Recommended next action

1. **Owner: open the Explorer on the Windows 11 host** (§3). Seconds of work,
   resolves F70's central question, and it is the only thing here that cannot be
   done from CI.
2. **F69** — the fix is an explicit focus call rather than relying on
   `autofocus`; not evidence work, and it needs a new candidate.
3. **F71** — adopt before the next parallel slice, not after.
4. M5-C's Linux and macOS rows, then full evidence assembly with the Gate D
   input list, which now gains F69 and F70.
