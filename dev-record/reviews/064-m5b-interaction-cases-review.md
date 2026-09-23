# M5-B review — the interaction cases, and F61

**Review date:** 2026-08-16
**Requests:** `dev-record/review-requests/061-m5b-interaction-cases.md`, with `060-m5b-macos-interaction-cases.md` as the macOS record
**Governing document:** `rfcs/handoffs/078-platform-runtime-acceptance/m5b-interaction-cases-handoff.md`
**Review mode:** Independent verification, including reproducing F61 end-to-end on a real desktop. No implementation changes made.

## 1. Verdict

**Approved as evidence.** The work is sound and F61 is real — I reproduced it
myself, both halves.

**But F61 is un-waivable, and the slice under-rates it.** RFC-078's waiver
policy names *"silent settings/session loss"* as one of five things no waiver
may turn into a release pass. F61 is that, exactly. It is a **second hard Gate D
blocker alongside F44**, not a severity question (§4).

Two corrections follow from that: Windows P12's result misrepresents a
cross-platform defect as platform-specific (§5.4), and F61 belongs in the
evidence verdict beside F44 (§5.5). Both are your own questions 4 and 5 — your
instincts were right in each case.

B4 open; **v1 No-Go** stands, now for two independent reasons.

## 2. F61 — independently reproduced, both halves

Not taken on three harness reports. Run on a real desktop with an isolated
`XDG_CONFIG_HOME`, so nothing touched the real profile:

```text
$ forskscope <left> <right>     # CLI-opened tab, isolated profile
  … 14s later …
  session.json present? NO

$ (same process, AT-SPI invoke "Close f61-l.txt ↔ f61-r.txt")
  session.json after closing the TAB? YES
```

That is precisely your diagnosis and it discriminates cleanly: the reactive
`use_effect` on `store.tabs` (`app.rs:86`) does not produce a write for a
CLI-opened tab, while `close_tab`'s direct `save_session` call does, in the same
process and environment.

The first close button I invoked was the wrong one — there are two, and only
`Close <left> ↔ <right>` is the tab's. Worth recording because it is exactly the
kind of thing that would make a harness report a false negative.

**Three independent implementations finding the same defect on three platforms,
then a fourth reproduction here, is as solid as this program gets.**

## 3. Verified

| Check | Result |
|---|---|
| F61 reproduced, both halves | **Confirmed** (§2) |
| `cargo test --workspace` | **1095** — see the correction below |
| RFC-078 waiver policy includes "silent settings/session loss" | Confirmed verbatim |
| `matrix-plan.md` untouched | Confirmed |
| P08 Exit asserts process death, not dialog dismissal, on all three rows | Confirmed from the reports: `proc.poll()`, `psutil.pid_exists`, `os.kill(pid,0)` |

**Gate-report correction:** §10 says `cargo test --workspace` ran "252
unit/integration tests plus doc-tests." It is **1095**, and has been since
M5-A. 252 is one crate's count, not the workspace's. Nothing turns on it — no
Rust changed — but a gate line that misreports its own number is the kind of
thing this program has learned not to wave through.

## 4. F61's standing — un-waivable, and it changes the Gate D picture

You asked whether a silently-lost CLI session blocks Gate D or is a
post-release fix. It is neither a judgement call nor mine to make: RFC-078
already decided it.

> **No waiver may turn these into a release pass:** … **silent settings/session
> loss** …

F61 is silent session loss in the plainest sense. `forskscope <left> <right>` —
the `git difftool` workflow, the most ordinary CLI invocation this tool has —
opens a session that is never written. The user quits, and it is gone with no
error. The `let _ =` discards in `persist_session`/`persist_settings` make the
silence structural rather than incidental.

So Gate D now has **two independent un-waivable blockers**: F44 (inability to
launch on a claimed supported platform) and F61 (silent session loss). F44's
resolution is an upstream release nobody here controls; **F61's is a fix this
project can make.**

That reframes the critical path. Until now the honest summary was "v1 waits on
dioxus." It is now "v1 waits on dioxus *and* on a persistence fix we own."

**One consequence worth recording.** `README.md:96` claims *"Session persistence
— open tabs are restored on next launch."* That is false for CLI-opened tabs.
F16's feature-claim audit passed all seventeen bullets, and its method was
right — it audited against the UI crate. It could not have caught a claim that
holds on one path and fails on another, which is a real limit of claim-auditing
worth knowing rather than a mistake in that slice.

**The `let _ =` discard pattern deserves its own entry** (your §11). It is not
F61 — it is why F61 and any future write failure are invisible. Register it
separately; fixing F61 without it leaves the next persistence failure just as
silent.

## 5. Answers to the requested review focus

### 5.1 §3's resolution — correct, and no RFC amendment needed

Your reading is right and the distinction is the important part: the "Use"
button has a real `onclick`, so accessibility invocation fires the same handler
a click does — the equivalent-path argument M5-A already established. The
Enter shortcut is a global `onkeydown` in `app.rs` bound to no actionable
element, so **no accessibility API on any platform has anything to invoke**.
That is categorically different from M5-A's Linux struggles, where a delivery
mechanism existed but was unreliable.

Recording the keyboard path as manual-outstanding, mirroring F45, narrows what
CI verified without altering what the case requires. Correct not to amend
RFC-078.

**Add one line to the evidence, though:** Enter-to-apply is a documented
keyboard shortcut with **no automated runtime coverage on any platform**, and
keyboard operability is an accessibility claim this project makes. That belongs
in the Gate D inputs, not only in a case's fine print.

### 5.2 F61's severity — see §4

Un-waivable. Not a post-release fix.

### 5.3 macOS P06's reduced scope — acceptable as evidence, but you found something bigger

The reduced scope is disclosed, the reasoning is sound, and Linux and Windows
both exercise genuinely overlapping tabs — so P06 is not resting on the weakest
row. Acceptable.

**But the root cause you found is more interesting than the workaround.**
"A generated diff pair's content never reaches the accessibility tree once the
file crosses a size threshold between 30 and 100 lines" is, if it holds, a
**macOS accessibility defect in the product**, not a harness limitation. Content
invisible to assistive technology above some size is exactly the class RFC-061
and RFC-019 exist for.

It may also be a harness artifact — a timeout, a lazily-rendered virtual list,
an AX tree that populates on demand. That distinction matters and is currently
unresolved.

**Register it as its own finding** rather than leaving it as a footnote about
fixture sizing. If it is a product defect it is a Gate D input; if it is a
harness artifact it is a note. Right now nobody knows which, and that is the
worst state for it to be in.

### 5.4 P12's inconsistent results — correct this

Your instinct is right, and it is more than a presentation issue.

Windows P12 reads **Pass**; Linux and macOS read **Fail**. A reader scanning the
results tables concludes F61 is platform-specific and Windows is unaffected.
**It is not** — Windows avoided it by seeding `session.json` directly instead of
exercising the CLI-launch path.

The prose discloses this. A results table does not get read alongside its prose,
and this table feeds a v1 decision. **Record Windows P12 as Fail, or as Partial
with the tab-restore path explicitly not exercised.** A row that passes by
routing around the defect misrepresents the platform.

This is the one place in an otherwise scrupulous slice where the evidence, taken
at face value, says something untrue.

### 5.5 Yes — F61 belongs in the verdict

For the same reason F44 leads it: a confirmed, cross-platform, un-waivable
defect is not a footnote. And per review 063 §5.1, the verdict should open with
the blocking facts before any pass rate. Two blockers now, one of them ours to
fix.

## 6. Notable quality observations

- Settling §3 before executing rather than during, and arriving at the same
  reading on three platforms independently.
- Asserting P08's Exit on process death by three different mechanisms
  (`proc.poll`, `psutil.pid_exists`, `os.kill`) — each appropriate to its
  platform, all answering the same question the handoff asked.
- Letting Linux and macOS P12 **fail honestly** rather than seeding around F61,
  and separating the restore half so the working part is still evidenced.
- Chasing macOS P06 past two failed designs to a root cause, instead of
  accepting the pre-approved fallback that would have "passed."
- Substituting real workflow dispatch for the unavailable `actionlint` and
  saying so, rather than reporting a gate that did not run.
- `proc.poll()` polled rather than checked once, after the same lesson was
  learned earlier in this program.

## 7. Recommended next action

1. **§5.4** — correct Windows P12's recorded result. Highest priority; the
   evidence currently misstates a fact.
2. **§5.5 and §5.1** — F61 into the verdict; the Enter-key coverage gap into the
   Gate D inputs.
3. **Register two findings:** the `let _ =` discard pattern, and macOS's
   accessibility size threshold (product defect or harness artifact — unknown,
   and that is the point).
4. **§3's gate-report number.**
5. **F61's fix is now on the v1 critical path** and is not evidence work. It
   needs its own slice against a later candidate — I will write that handoff.
6. M5-C (P03, P07, P11, manual passes, assembly) continues in parallel.
