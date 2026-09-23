# M5-A review — evidence harness and the launch/CLI cases

**Review date:** 2026-08-15
**Request:** `dev-record/review-requests/059-m5a-harness-and-launch-cases.md`
**Baseline:** `6746080`
**Governing documents:** `rfcs/handoffs/078-platform-runtime-acceptance/m5a-harness-and-launch-cases-handoff.md`, RFC-078, frozen `matrix-plan.md`
**Review mode:** Independent verification, including re-running five CI runs across all three platforms and both modes. No implementation changes made.

## 1. Verdict

**Approved.** The evidence is honest, the harness is falsifiable, and the three
known-bad outcomes are recorded as findings rather than laundered into a pass.

One new finding I am raising, which neither the slice nor the frozen plan
surfaced: **the declared Windows floor has no runtime evidence and none is
planned** (§4, registered as F60).

Gate D remains unassessable, as expected. B4 open; **v1 No-Go** stands.

## 2. Verified

| Check | Result |
|---|---|
| Normal-mode runs `31852372407` (Linux P01), `31852248989` (Linux P09) | **success** |
| Break-mode runs `31852312861` (Linux P09), `31853350190` (Windows P02), `31853267021` (macOS P10) | **failure** — all three, spanning all platforms |
| Artifact digests | Match §5 and my own independent hashes from review 062 |
| `cargo test --workspace` | 1095, unchanged — no Rust touched |
| `cargo fmt --check`, `mdbook build docs` | Pass |
| F59 registered | Confirmed |
| `matrix-plan.md` unedited | Confirmed |

I re-ran five runs myself across three platforms and both modes rather than
sampling one. Every one matched its claimed outcome.

## 3. What this slice got right, specifically

**The workflow structure *is* the F59 experiment.** `m5-evidence-linux.yml`
installs the *documented* user prerequisites (`libwebkit2gtk-4.1-0`,
`libgtk-3-0`) at line 41 and the harness's own dependencies — including
`libxdo3` — separately at line 84. That separation is what made an undocumented
prerequisite discoverable at all. Installing everything in one block would have
produced a clean P01 pass and hidden F59 completely.

**P01 is recorded as two sub-results, not one.** "Pass, but only after
installing an undocumented prerequisite" and "Fail on a real libxdo-4 host" are
both there, with real `readelf`/`pacman` output from an actual machine rather
than a simulation. That is the difference between evidence and a status table.

**`windows-10.md` refuses to overclaim.** It records the resolved image
(`win25-vs2026`, `20260810.198.2`, NT `10.0.26100.0`), states plainly that this
is not a literal Windows 10 install, and quotes the plan's own stand-in
language. A row that could easily have read "Windows 10: Pass" says what was
actually observed.

## 4. New finding — F60: the declared Windows floor has no evidence, and none is planned

Following the `windows-10` row's honesty to its conclusion surfaces something
the frozen plan sanctioned without anyone noticing the consequence.

- `AppxManifest.xml` declares `MinVersion=10.0.17763.0` — **Windows 10 1809** —
  and that is a live Microsoft Store constraint, deciding who can install.
- `installation.md` tells users **"Windows 10, version 1809, or later."**
- The `windows-10` row's evidence comes from a Server-2025-based image running
  kernel **NT 10.0.26100**, seven years newer.
- There is **no Windows 10 host anywhere** in the execution model — no CI runner
  offers one, and the owner's manual host is Windows 11.

So the oldest Windows the project *claims to support* has never been observed
running the application, and nothing planned will change that. F45 compounds it:
the "prerequisites missing" case is also unexecuted, so the claim rests entirely
on a machine that is neither old nor clean.

This is not a defect in this slice — the plan sanctioned the stand-in and the
evidence reports it accurately. It is a **Gate D input that should be visible
before the go/no-go**, not discovered while reading a verdict. Three coherent
resolutions, all the owner's call: narrow the published floor to something
evidenced, obtain a Windows 10 host, or state explicitly that 1809 is a declared
compatibility floor carrying no runtime evidence.

It also settles part of **F49b**: that entry deferred `MaxVersionTested` until
"M5's Windows evidence" existed. That evidence is from NT 10.0.26100 — so it
supports raising the *ceiling*, and says nothing whatsoever about the floor.

Registered as **F60**.

## 5. Answers to the requested review focus

### 5.1 F44's framing — honest, but the ordering works against the reader

The content is right: the F44 bullet is unambiguous, and **"This does not mean
Gate D can pass"** is stated in bold with the un-waivability reasoning and the
handoff quote behind it. Nobody who reads the section is misled.

The problem is that the section *opens* with **"Every case this slice covers
passes on every CI-verified row."** For this document's actual audience — someone
skimming toward a v1 decision — the first bold line is the one that lands, and
the blocker arrives third.

**Invert it.** Lead with the blocking fact, then the pass rate as context:

> **This evidence cannot support a Gate D pass.** F44 fails Linux P01
> un-waivably on a supported platform… Within that constraint, every case this
> slice covers passes on every CI-verified row.

Same facts, and a skimmer cannot come away with the wrong impression. That was
your own instinct in asking — trust it.

### 5.2 Delegation — legitimate here, for a reason worth naming precisely

Acceptable, and the justification is not "you checked afterwards" but something
structural: **the falsifiability protocol was defined before the work was
delegated.** A harness that passes normal mode and fails break mode has
demonstrated it discriminates, whoever wrote it. That property is externally
checkable without reading the code — which is why I could confirm it from five
runs, and why your two spot-checks plus 24 dispatched combinations were
sufficient rather than merely reassuring.

The principle to carry, since this is the first delegation in the program:
**delegate work whose correctness criterion is already externally checkable.**
Harness construction under a pre-agreed break/normal protocol qualifies.
Judgement work — deciding what a case should assert, or how a failure is
characterised — would not, because no run can verify it.

Two honest notes in your own account support this rather than undercut it:
naming that the three-way convergence on native accessibility actions was *not*
independent (you briefed the Linux lesson forward), and finding a real dangling
cross-reference in `windows-11.md` that only a full read would catch.

### 5.3 Windows's token readiness — sufficient for P02, a dependency for P03

Sufficient. The asymmetry matters only where a *partial* tree could produce a
wrong answer rather than no answer. Linux's exact row count exists because F34
compares alignment across rows, and a partial row set would compare a subset and
pass. P02 asserts content presence, where early satisfaction costs nothing.

But it becomes a real gap at **P03 (compare layout and scrolling)** in M5-C,
which is the alignment case. Windows will need row-level precision there, and
the reason you could not do it now — WebView2's UIA mapping for a table-less
`role="row"` div was never empirically established — is exactly the thing to
settle before P03, not during it.

Record it as a known dependency of M5-C rather than a limitation of M5-A.

## 6. Notable quality observations

- Separating documented prerequisites from harness dependencies in the workflow,
  which is what made F59 findable.
- The Linux harness's five-iteration history, written down in enough detail that
  the next platform author can skip it — including *why* each approach failed
  (no window manager, `XSendEvent` ignored by GTK, X11 focus ≠ widget focus).
  Most teams delete that.
- Recording `linux-wayland` as outstanding rather than omitting the file.
- Sanitising the F44 reproduction to distribution family without host identifiers,
  per RFC-078's schema, while keeping the output that makes it verifiable.
- Registering F59 instead of applying a one-line doc fix mid-evidence-gathering.
  The constraint existed to keep the evidence tied to a fixed artifact, and it
  was honoured when breaking it would have been trivially easy to justify.

## 7. Recommended next action

1. **§5.1's inversion** in `README.md` — one paragraph reordered.
2. **§5.3's dependency** noted for M5-C.
3. **F59's doc fix** — now unblocked; it belongs with ordinary work, not this
   evidence set.
4. **Owner: F60** — decide what the Windows floor claims, before Gate D rather
   than during it.
5. **M5-B** — P04, P05, P06, P08, P12. Handoff to follow.
