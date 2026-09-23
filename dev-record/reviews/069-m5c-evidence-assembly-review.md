# M5-C evidence assembly review — Gate D input list and README verdict

**Request:** `dev-record/review-requests/067-m5c-evidence-assembly.md`
**Review date:** 2026-08-16
**Governing document:** `rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md` §7
**Review mode:** Independent completeness check of the input list against the full register, not against the request's own account.

## 1. Verdict

**Approved. M5-C is complete.**

The Gate D input list is the document the handoff asked for and it does the job:
it separates what the checks found from what the findings mean, which is the
distinction that makes a go/no-go assessable at all.

**One completeness gap — F49b (§3).** Everything else checks out, including
every exclusion.

## 2. Verified

| Check | Result |
|---|---|
| Input list covers every open register entry bearing on Gate D | **One gap** — F49b (§3) |
| The three blockers are correctly identified as un-waivable | Confirmed against RFC-078's five named categories |
| F68 folded into F73 rather than double-counted | Correct — one root cause, one fix (review 068 §5) |
| README verdict leads with blocking facts | Confirmed — opens *"This evidence cannot support a Gate D pass"* |
| F49 (platform version claims) genuinely resolved | Confirmed — `MACOSX_DEPLOYMENT_TARGET: "13.0"` matches `LSMinimumSystemVersion 13.0` |
| `artifacts.md` unchanged and complete | Confirmed |
| Exclusions (F65, F66, F67, F68, F71) | All defensible — see §4 |

I enumerated the register's open entries independently rather than checking your
list against its own account. That is how F49b surfaced.

## 3. The gap — F49b belongs in "considered and not included"

`AppxManifest.xml` still declares `MaxVersionTested = 10.0.19041.0` — Windows 10
2004, predating Windows 11 entirely. **F49b's own text says it is bumped "at M5
as an *output* of the Windows evidence,"** and that evidence now exists (M5-A/B/C
ran on NT 10.0.26100).

So F49b is not a decision input — you are right that it does not bear on the
go/no-go — but it is **an action M5 just enabled**, and it currently appears on
no list at all. That is precisely how something gets lost between milestones.

It is also asymmetric as it stands: **F60 (the Windows floor) is listed and F49b
(the ceiling) is not**, and they are two halves of the same manifest question.

Add it to "Considered and not included" with the reason — *an action enabled by
M5's evidence, not an input to the decision* — rather than to the input tables.
One line.

## 4. The exclusions are right, and one is right for a subtle reason

F65, F66, F67, F71 are all correctly out: a post-Gate-D dependency decision, and
three findings about *how the evidence was produced* rather than about the
product being evidenced.

**F68's exclusion is the one worth endorsing explicitly.** Folding it into F73
rather than listing both is correct, and the reasoning you gave is the right one:
counting one root cause as two blockers would overstate the blocker count. A
Gate D list that inflates from three to four because two entries share a fix
would misrepresent how much work stands between here and a pass — which is
exactly the number the owner will use to decide what to do next.

## 5. Answers to the requested focus

### 5.1 Completeness — yes, apart from F49b

See §3. The structure is right too: un-waivable blockers separated from
weighted inputs, with an explicit exclusions section so absence reads as
judgment rather than oversight. That third section is what makes the list
auditable, and most such documents omit it.

### 5.2 Drift risk between README and input list — real, and cheap to close

Your concern is well founded. The README's verdict currently **restates** the
count ("three independent, un-waivable reasons"), so a fourth blocker would
require both documents to change, and nothing enforces that.

Cheapest fix that keeps both documents useful: have the README's verdict **name
the blockers and link to the input list as authoritative**, rather than assert a
count of its own. The verdict then stays correct even if the list grows, and
there is one place to update.

Do not merge the two — the split is right. A verdict a reader meets first and a
detailed record they consult second are different jobs.

### 5.3 Is M5-C complete? — yes

Handoff §10 anticipated exactly this state. Nothing in the assembly is
outstanding.

**But note what that does and does not mean**, because it is the point where a
milestone's completion gets over-read:

- **Gate D's verdict is already determined.** Three un-waivable blockers,
  and neither the owner's manual rows nor F70's Windows check can change that —
  they can only refine the picture.
- **Gate D is not formally assessable yet.** RFC-078 requires the evidence
  matrix complete across all three platforms, and `linux-wayland` and F45's
  sub-case are outstanding. So the matrix stays open even though the outcome is
  known.

Those are compatible, and stating both is more honest than picking one. The
practical consequence: **completing the matrix is still worth doing**, because
the fixes for F44/F61/F73 will need re-verification against a new candidate
anyway, and a matrix already complete except for the fixed rows is much cheaper
to re-run than one with unexecuted rows.

## 6. Notable quality observations

- Catching your own cross-document error before committing — attributing the
  `--break` limitation to P07 when it belongs to P11, and P07's being a
  different and simpler thing ("not reached at all"). That is precisely what an
  assembly pass exists to find, and finding it in your own draft is better than
  my finding it in the artifact.
- Applying the walker hardening pre-emptively **and** stating the three places
  you deliberately did not apply it, with the reason (nodes an enclosing retry
  just confirmed present, not deep walks over a long-lived subtree). A blanket
  application would have been easier to defend and less accurate.
- Re-dispatching all three Linux cases after the hardening to confirm no
  regression, rather than reasoning that a defensive change cannot break
  anything.
- Moving the VoiceOver caveat from F63 to Finding 3, and adding the parallel
  caveat about the click technique. Both now sit where the open question
  actually lives rather than attached to a closed finding.
- The case-result summary explicitly labelled "context, not a separate input,"
  which stops it being read as a second, competing verdict.

## 7. Recommended next action

1. **§3's F49b line** and **§5.2's README/link change.** Two small edits; no
   re-run needed.
2. **The F68+F73 fix** — pass `left_root`/`right_root` into `DeepRow`. One
   un-waivable blocker, one fix, and F68 comes free.
3. **A new candidate** carrying F61's and F73's fixes, then re-run P07 and P12
   against it.
4. **Owner:** F70's Windows Explorer check, F60, and the two manual rows.
5. **F44 remains upstream.** It is now the only blocker no one here can act on.
