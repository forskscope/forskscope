# RFC-078 M5-C Developer Handoff: Visual/Navigation Cases and Evidence Assembly

**Governing RFC.** [RFC-078](../../proposed/078-platform-runtime-acceptance.md), under [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** M5-C — the last evidence slice before Gate D
**Cases.** P03, P07, P11, plus evidence assembly
**Candidate.** `0.167.0` — same published artifacts and digests as M5-A/M5-B
**Baseline.** `main` at `b27545d`

`matrix-plan.md` is **frozen**. Stop and report rather than amending a row.

## 1. What is different about this slice

M5-A automated launch. M5-B automated interaction. **M5-C is where the limits
this program has been recording come due**, and three of them are prerequisites
rather than caveats:

| Standing item | Why it blocks M5-C |
|---|---|
| **F34's geometry branch has never fired** (review 055 N1) | P03 *is* the alignment case. Only the child-count assertion has ever been observed working |
| **Windows readiness is token-based, not row-count** (review 063 §5.3) | P03 needs row-level precision on Windows; tokens cannot establish alignment |
| **F63 — macOS content stops reaching the AX tree above ~30–100 lines** | P03 and P07 both need multi-hunk and directory content observable on macOS |

Settle these before gathering evidence against them, not during. Evidence
collected through a check that cannot fail is worth nothing at Gate D, and this
is the last slice before that decision.

**P11 is also largely not CI-verifiable** (§6) — that is scoping, not a defect.

## 2. Scope

P03, P07, P11 on the CI-verified rows, plus assembly (§7). Out of scope: the
owner's two manual rows (`linux-wayland` in full, Windows 11's F45 sub-case),
F61's fix, and RFC-079.

## 3. P03 — and its two prerequisites

RFC-078 requires:

- short-row backgrounds span the full widest-line area;
- action rows align with left/right rows across multiple hunks;
- vertical rows remain aligned;
- horizontal scrolling mirrors between panes without feedback or jitter;
- word wrap and narrow window modes remain usable.

Mandatory in full on WebKitGTK; a basic layout observation on WebView2 and
macOS WebKit.

**Prerequisite A — make F34's geometry branch demonstrable.** `check_pane`
compares content-cell x-origins, and that branch has never been observed
failing; review 055 tried twice with CSS mutations and could not produce a
shift. P03 rests on exactly that comparison. Either demonstrate it failing on a
deliberately broken layout, or establish that it cannot fail and say why —
in which case P03's alignment claim needs a different assertion. `render-check.yml`'s
`inject_f32_defect` already proves the injection harness works; a second
injection mode for a geometry-only shift reuses all of it.

**Prerequisite B — Windows row parity.** M5-A's Windows readiness waits for
distinguishing text tokens because the UIA control type WebView2 maps a
table-less `role="row"` div to was never established. Tokens prove content
arrived; they cannot prove rows align. Establish the mapping and assert on rows,
or state plainly that Windows P03 is a weaker observation than Linux's and
record it as such.

**Horizontal scroll mirroring** is the item with no precedent in this program.
Scroll one pane, assert the other's offset follows, and assert it settles — the
"without feedback/jitter" clause means an oscillation is a failure, so a single
sample after scrolling is not sufficient evidence.

## 4. F63 must be settled before macOS P03/P07

M5-B found that a diff pair's content stops reaching the macOS accessibility
tree above a file-size threshold between 30 and 100 lines. It is unresolved
whether that is a **product accessibility defect** or a **harness artifact**.

P03 needs multiple hunks and P07 needs directory content — both plausibly above
that threshold. So on macOS, either:

- it is a harness artifact, and once understood the cases run normally; or
- it is a product defect, in which case **it is a Gate D input in its own
  right** — content invisible to assistive technology above a size is an
  accessibility failure, not a testing inconvenience — and macOS P03/P07 are
  recorded against that reality rather than around it.

**Resolve which before gathering macOS evidence for these two cases.** A
reduced-scope pass like M5-B's P06 is acceptable *if* the reason is understood;
it is not acceptable as a way to avoid finding out.

## 5. P07 — Explorer and directory report

- navigation, history, focused-pane keyboard behaviour;
- equal / different / one-sided statuses;
- deep comparison progress and filters;
- per-file and batch copy: confirmation, backup, manifest, result summary.

Mostly automatable through the accessibility-action approach M5-A and M5-B
established. Two notes:

- **"focused-pane keyboard behaviour"** hits §6's limitation — record it the
  same way.
- **Batch copy writes files.** Assert the manifest contents and the backup
  bytes, not merely that the operation reported success. This is the one place
  in P07 where a wrong result is silent and destructive, and F62's lesson
  applies: an operation that reports success without its output verified is a
  claim, not evidence.

## 6. P11 — mostly the owner's, and say so precisely

RFC-078 requires:

- execute the maintained keyboard checklist;
- modal focus starts on the safe/cancel action for destructive operations;
- global shortcuts do not affect the background view while a modal is open;
- Escape behaviour is consistent.

M5-B §3 established that keyboard input is structurally not CI-verifiable: no
accessibility API can invoke a global `onkeydown` handler bound to no element.
That resolution applies here and covers **three of the four items**.

But the decomposition matters — do not mark the whole case manual:

| Item | CI-verifiable? |
|---|---|
| Keyboard checklist | **No** — manual |
| Modal focus starts on safe/cancel | **Yes** — focus position is readable through the accessibility tree without synthesizing anything |
| Global shortcuts inert behind a modal | **No** — requires a keystroke |
| Escape behaviour | **No** — requires a keystroke |

The focus-position item is worth automating precisely because it is the one
with a **data-safety consequence**: a destructive modal whose focus starts on
the destructive action is a real hazard, and it is checkable today.

Record the other three as owner-executed, mirroring F45's shape. Then say the
consequence plainly in the evidence: **the documented keyboard interface has no
automated runtime coverage on any platform**, and keyboard operability is a
claim this project makes in its README and its accessibility RFCs.

## 7. Evidence assembly

The last deliverable, and the one Gate D actually consumes.

- **Complete every row file** — M5-A/B/C results together, per RFC-078's schema,
  with the **resolved runner image versions** rather than rolling labels.
- **`artifacts.md`** — the three published assets with digests and source commit.
- **`README.md`** — the verdict, ordered per review 063 §5.1: **blocking facts
  first**, pass rate as context.
- **A Gate D input list.** This is the thing I will assess against, and it does
  not exist yet. Everything that bears on the decision, in one place, each with
  its status:

| Input | Status to record |
|---|---|
| F44 | Fail, un-waivable, upstream schedule dependency |
| F61 | Reopened; un-waivable; **fix in progress, not in this candidate** |
| F45 | Manual-only, owner-executed, outstanding |
| F46 | Blocked — cannot be verified under current resourcing |
| F60 | Windows floor unevidenced; owner decision open |
| F63 | Product defect or harness artifact — per §4 |
| Keyboard interface | No automated coverage on any platform — per §6 |
| `linux-wayland` | Owner's manual row — outstanding |

If an input's status changes while you assemble, record what is true at
assembly time and date it. Do not predict.

## 8. Constraints

- `0.167.0`'s published artifacts only, digest-verified. F61's fix is **not** in
  this candidate; do not test against a rebuilt binary.
- No dependency added, removed, or version-changed.
- No product behaviour changes. A defect found is registered and reported.
- `matrix-plan.md` is frozen.
- Do not weaken a case to make it pass — least of all here, where the output is
  a go/no-go input.

## 9. Required review-request content

1. **The three prerequisites' outcomes** (§3 A and B, §4) — before any P03/P07
   evidence is presented;
2. cases executed per row with results;
3. **P11's decomposition as executed**, and the keyboard-coverage statement (§6);
4. falsifiability demonstrations with observed output, including the geometry
   branch (§3);
5. the assembled evidence set and the **Gate D input list** (§7);
6. any product defect found, registered not fixed;
7. any difference from this handoff, RFC-078, or the frozen plan;
8. executed gates;
9. unresolved issues and known limitations;
10. requested review focus.

## 10. After this slice

M5's CI-verified rows are complete. What remains for Gate D is the owner's two
manual rows and the open inputs in §7 — and on current knowledge Gate D cannot
pass, because F44 and F61 are both un-waivable. **That is the expected outcome
and not a failure of this work**: the milestone's job is to establish what is
true about the candidate, and it will have done that.
