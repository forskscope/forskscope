# RFC 080: Tiered Directory Comparison in the Explorer

**Status.** Proposed
**Accepted.** 2026-08-21 by the project owner — Gate A cleared; **all four
design questions closed 2026-08-22, so the design is settled and only Gate D
stands between it and implementation.** Stays in
`proposed/` until implemented, per the 4-folder lifecycle (RFC-000 §Folder
layout); it moves to `done/` when the work ships, as RFC-078 does.
**Tracks.** Explorer status column; directory comparison cost model.
**Touches.** `explorer.rs`'s digest effect, `DigestState` (or its F75
replacement), `forskscope-core::dir::recursive`, the status legend in user docs.
**Depends on.** F74's fix, already on `main`, is the
floor this builds on. Interacts with F75 and F76 — see §7.
**Origin.** Owner proposal, 2026-08-21, after re-testing F70 on Windows 11.

## Summary

The Explorer's status column tells the user nothing about directories. Since
F74 it says so honestly — a directory row shows *not compared* rather than a
false `✓` — but honest silence is still silence, and the user's question ("do
these two folders differ?") goes unanswered in the view where they asked it.

This RFC answers it in **two tiers**, because the cheap answer and the certain
answer are different answers and the expensive one should not be compulsory.

The design rests on one asymmetry, and everything else follows from it:

> **A cheap scan can prove two directories *differ*. No cheap scan can prove
> they are *identical*.**

Differing names or differing sizes settle the question immediately and for
free. Matching names and sizes settle nothing — a one-character edit preserves
both. So the tiers are not "rough answer, then better answer"; they are **a
definitive negative, then a definitive positive**, and the labels must say
which one the user is looking at.

## Goals

- Answer "do these directories differ?" in the Explorer, for the common case,
  without reading file contents.
- Never assert equality that was not established. A tier-1 pass finding nothing
  must not claim identity.
- Make the expensive tier deliberate — user-triggered or explicitly bounded,
  never an unannounced recursive read of an arbitrary subtree.
- Reuse the recursive comparison `forskscope-core` already has. No second
  implementation of tree walking.

## Non-goals

- **Replacing Deep Compare.** It remains the place for a full per-file report.
  This RFC gives the Explorer a *verdict*, not a report.
- **Live filesystem watching.** Results are computed on demand and invalidated
  on navigation or refresh, not maintained against external changes (§5).
- **Persisting verdicts across sessions.** Out of scope, and probably a bad
  idea: a stale verdict is worse than none.
- **Changing file-row behaviour.** Files already digest correctly.

## Why this is not simply "recurse and digest"

That is the obvious repair, and handoff 002 forbade it inside F74's bug fix for
a reason worth restating in a design document.

Digesting every subtree of every visible directory row, on every navigation, is
an unbounded read of the user's disk triggered by browsing. A pane showing
thirty directories would fan out into thirty tree walks with full content reads.
On a home directory, a network mount, or a repository with a large build output,
that is seconds to minutes of I/O the user did not ask for and cannot see.

**Correction, 2026-08-21, prompted by the owner asking why only directories are
treated as costly.** The paragraph above was written as though the Explorer does
not already do this. It does — for files. `file_digest_equal` is called on every
common file in the listed directory, on navigation, with no size cap and no
cancellation. Its worst case is two identical large files read in full. So
"browsing must not start unbounded reads" is not a principle this RFC is
introducing; it is a principle the product **already violates**, which nobody had
registered. That is now **F77**, and it changes this RFC's shape rather than
merely adding to it — see §2a.

What remains true is the difference in *bound*, not in kind: a file row's cost is
capped by one pair of files, while a directory row's is the whole subtree. Both
need the control; only one of them has ever been described as needing it.

## Design

### 1. What core already provides

No new engine work. `forskscope-core::dir::recursive` already exports both
tiers, with cancellation:

- `list_recursive_for_display(left, right)` — walks both trees, pairs entries by
  relative path, records `left_size`/`right_size`, and marks files present on
  both sides `Computing`. **No file contents are read.**
- `recursive_diff(left, right)` — the same walk plus per-file digest comparison.
- `list_recursive_for_display_with_cancel` / `recursive_diff_with_cancel` — the
  cancellable variants.

Symlinks are reported as `RecStatus::Symlink` rather than followed, so neither
tier can loop on a cycle.

**One implementation detail that must not be assumed away.** The fast pass marks
every file present on both sides `RecStatus::Computing` **regardless of size** —
verified in `walk_and_merge_fast`, which records `left_size`/`right_size` and
sets `Computing` without comparing them. Core does **not** flag a size mismatch
for you. Tier 1's verdict in §2 is therefore caller-side logic over the returned
`Vec<RecEntry>`, not a status core hands back. An implementer who reads
`Computing` as "undetermined" and stops there gets no tier-1 answer at all.

### 2a. The tiers are a property of comparison, not of directories

The owner's question — *if thorough file comparison also costs much, could
directory comparison be names and sizes only?* — points at a real inconsistency,
and resolving it downward would be the wrong repair.

**Files already have both tiers; the cheap one is just not named.**
`file_digest_equal` returns `false` immediately when the two sizes differ, before
opening either file. That is precisely tier 1's logic — a free, definitive
*different* — already implemented and already correct. What follows it, the
streamed content comparison, is tier 2.

So the model is not "directories are special". It is:

| | Tier 1 — metadata | Tier 2 — contents |
|---|---|---|
| **File** | sizes differ → **Different** (free) | streamed compare → **Identical** / **Different** |
| **Directory** | names or sizes differ → **Different** (walk, no reads) | recursive compare → **Identical** / **Different** |

Same asymmetry in both rows: tier 1 can prove *different*, never *identical*.

**Making file comparison names-and-sizes only is rejected**, and the reason is
the product's purpose. A one-character edit preserves file size, so a size-only
file verdict would report *identical* for the single most common edit a diff tool
exists to find. That is a false negative in the direction this project has twice
ruled un-waivable. The consistency the owner is asking for is real and should be
achieved by **raising directories to the file model, not lowering files to the
directory one** — the same five states, meaning the same thing on both row kinds,
so a glyph does not silently change strength depending on what it sits beside.

**The cost control in §5 therefore applies to file rows as well**, which is new to
this RFC and is the substance of what the owner's question changed.

### 2. Tier 1 — the cheap pass, and exactly what it can conclude

Run `list_recursive_for_display_with_cancel`. Reading only the result's shape
and sizes, three outcomes are possible:

| Observation | Conclusion | Certain? |
|---|---|---|
| Any entry present on one side only | **Different** | Yes |
| Any common file whose `left_size` ≠ `right_size` | **Different** | Yes |
| Any entry with an error status | **Unknown** — not "different" | — |
| Otherwise | **No difference found without reading contents** | No |

The first two are free certainty and they cover the case that prompted this
RFC: the owner's test directories differed in file *content*, and content edits
usually move the size. Tier 1 would very likely have answered correctly, at the
cost of a directory walk and no reads.

The fourth row is the one that must not be overstated. Same names and same sizes
is the expected state of two directories that differ by one edited character.

### 3. Tier 2 — the certain pass

Run `recursive_diff_with_cancel`. This reads contents and returns a definitive
`Identical` or `Different`.

**Trigger — decided by the owner, 2026-08-22 (§8 Q1 closed): user-initiated per
row.** A control appears only on a row already sitting at the tier-1 state. Zero
surprise I/O, and the cost is always attributable to something the user asked
for.

Two consequences to build to, not to rediscover:

- **The control appears only where tier 1 finished and found nothing.** A row
  proven *Different* needs no verification, and a row still at *Not compared*
  has no tier-1 result to escalate from. Offering it everywhere would invite the
  expensive pass exactly where it is least useful.
- **Tier 2 is per row, so several may run at once.** They must share the same
  bound and cancellation as everything else — see §5. A user clicking *verify* on
  six directories has asked for six answers, not for six unbounded fan-outs.

The alternatives were an automatic pass under a size budget (rejected: nobody has
calibrated the budget, and the budget becomes a claim of its own) and automatic
always (rejected for the reason in §"Why this is not simply").

### 4. Status vocabulary — settled

**Decided by the owner, 2026-08-21 (§8 Q2 closed).** Five states. Each names what
was measured, and the tier-1 state names it outright rather than summarising it.

The status column renders **only a glyph**; the wording below is the `title` and
`aria_label`, so there is no column width to economise against and no reason to
compress the truth into two words. F74's existing label is already a full
sentence, which is the precedent.

| State | Label (file row) | Label (directory row) | From |
|---|---|---|---|
| **Not compared** | Contents not compared | Directory contents not compared — use Deep Compare | post-F74 default |
| **Comparing…** | Comparing… | Comparing… | either tier |
| **Different** | Different | Different | tier 1 or 2 |
| **Tier-1 match** | Size matches; contents not compared | Names and sizes match; contents not compared | tier 1 only |
| **Identical** | Identical — contents compared | Identical — contents compared | tier 2 only |

**Why not "no difference found", which this RFC previously recommended.** It is
the phrasing of a *completed* search: it implies something looked and came back
empty. Tier 1 compared names and sizes and never opened a file, so the phrase
claims a thoroughness the check did not have — the same defect this program has
been cataloguing as *a marker credited with more than it measures*, arriving at
the product surface. The axis that matters to a user is **complete versus
incomplete**, not *measurement versus probability*, which is the axis the earlier
recommendation argued on and got wrong.

**Why not "Maybe equal", the owner's first proposal.** It carries the
uncertainty, which is the important half, and it was a better answer than the one
this RFC originally preferred. It was set aside only because "maybe" reads as an
estimate nobody computed. Stating the evidence outright does the same job without
implying an assessment.

**Two consequences of stating the evidence, both deliberate:**

- **The two row kinds carry different words for the same state.** A file pair is
  already matched by name, so only its size was checked; a directory pair had
  both names and sizes compared. The *state* is identical and the glyph is
  identical — the label differs because the evidence differs.
- **"Identical" is a statement about the moment of comparison, not a property.**
  It is invalidated by §5's rules on navigation and refresh, and is never
  persisted across sessions. A label outliving its evidence is the same failure
  in slow motion.

**A tier-1 "Different" and a tier-2 "Different" are the same claim** and render
identically. Certainty differs between *equal* verdicts, not *different* ones —
which is why only one row in the table above splits by tier.

### 5. Cost control and invalidation

- **Applies to file rows too (§2a).** Today's eager, uncancellable, uncapped file
  comparison is F77; the same token and the same bound cover both row kinds.
- **One tier-1 pass per directory row — never a fan-out across every visible
  row.** **Trigger decided by the owner, 2026-08-22 (§8 Q3 closed): on selection,
  debounced and cancellable.** Selecting a directory row starts a timer; the walk
  begins only if the row is still selected when it expires, and is cancelled the
  moment selection moves or the pane navigates.

  **The debounce is not a nicety and must not be dropped as one.** Selection
  changes on every keypress during keyboard navigation, so without it, arrowing
  down twenty directories starts twenty recursive walks. With it, only the row
  the user rests on costs anything. Choose the interval deliberately and record
  it; it is the difference between this feature being free and being a hazard.
- **Cancellable, and cancelled on navigation.** Both core entry points take a
  token. A pane that navigates away must cancel outstanding work rather than let
  it complete into a discarded view.
- **Results are cached per (left_root, right_root, rel_path) and dropped on
  navigation or refresh.** No cross-session persistence.
- **Errors are their own state, never "different".** Core skips per-entry I/O
  errors during the walk; an unreadable subtree must surface as *unknown*, not
  as a verdict. This is F76's second instance appearing one level up (§7).

### 6. Accessibility

Every state needs a localised accessible label, as F74's states now have, and
whichever ARIA pattern is chosen must match the one the Explorer already uses —
`role="img"` + `aria_label`. Note the app currently carries two correct patterns
(`hunk.rs` uses `sr-only` + `aria_hidden`); this RFC does not reconcile them,
but adds no third.

**A running comparison must announce completion**, not silently swap a glyph. A
status that changes without notification is invisible to a screen-reader user,
who has no reason to re-read the row.

### 7. Relationship to F74, F75, F76

- **F74** — fixed, on `main`, unreleased. This RFC is the feature that F74's
  honest *not compared* was holding the place for. **It does not depend on this
  RFC**, and shipping this RFC is not a precondition for anything in v1.
- **F75** — nine unwired `ui-logic` view-models, `explore/status.rs` among them.
  `RowStatusKind` needs the states in §4 whichever order these land in. **If F75
  is done first, this RFC targets `RowStatusKind`; if this lands first, it adds
  variants to `DigestState` and F75's wiring absorbs them.** Either order works;
  doing them in one change is cheaper than either.
- **F76** — statuses that overstate their evidence (type mismatch labelled
  "only on this side"; unreadable file labelled "different"). §5's error state
  is the same requirement at directory level. **Fixing F76 first would make this
  RFC smaller**, since `EqualityEvidence` already carries `TypeMismatch` and
  `Error`.

## Acceptance criteria

1. A directory pair differing only in a file's **size** is reported *Different*
   by tier 1, with no file contents read.
2. A directory pair differing only in a file's **contents at identical size** is
   reported *no difference found* by tier 1, and *Different* by tier 2.
3. An identical pair is *no difference found* after tier 1 and *Identical* after
   tier 2 — and **never `Identical` from tier 1 alone**.
4. A subtree containing an unreadable entry reports *unknown*, not a verdict.
5. Navigating away cancels an in-flight comparison; no result lands in a view
   the user has left.
6. Every state has a non-empty localised accessible label, and completion is
   announced.
7. No comparison is triggered by browsing alone beyond what §5 permits.

## Testing

Each check demonstrated failing against a deliberately broken input, per this
program's standing requirement — and specifically **not against a helper the fix
introduces**, which is the failure review 072 caught.

Criteria 1–4 are unit tests over real temporary directory trees; the
`temp_dir(tag)` pattern in `explorer.rs`'s tests is the precedent. Criterion 5
needs the cancellation token observed, not assumed — the pattern F61 established.
Criterion 6's label existence is unit-testable; that it *reaches* the
accessibility tree is a P07 assertion, as recorded for F74.

## Sequencing

**After Gate D.** This is a feature, and v1 stabilization is blocked on F44
alone. Adding an Explorer feature now would put new code into the candidate that
the acceptance matrix has not covered, for a question Deep Compare already
answers by another route.

Nothing is lost by waiting: F74's fix means the Explorer is currently *honest*
about not knowing, which is a correct state to ship, not a placeholder.

## Open questions — all closed

- **Q1 — tier-2 trigger. CLOSED 2026-08-22** — user-initiated per row. See §3.
- **Q2 — the tier-1 label. CLOSED 2026-08-21** — state the evidence outright.
  See §4 for the settled vocabulary and the reasoning that replaced the earlier
  recommendation.
- **Q3 — tier-1 trigger. CLOSED 2026-08-22** — on selection, debounced and
  cancellable. See §5.
- **Q4 — ordering. CLOSED 2026-08-22** — **F76, then F75, then this RFC.** Each
  makes the next smaller: F76 stops the statuses overstating their evidence and
  brings `TypeMismatch` and `Error` across from core; F75 then wires
  `RowStatusKind` and deletes `DigestState`; this RFC then adds the tiers to a
  status type that already has the vocabulary for them. Three independently
  reviewable units rather than one diff spanning three crates.

  **F77 and F78 are no longer part of this question.** Both were defects rather
  than design items, both were fixed ahead of this RFC (reviews 074 and 075), and
  the `DigestEpoch` they produced is the mechanism §5's cost control now
  assumes — one shared generation guard, cancellation token and concurrency
  bound, already used by both views. That is why §5 can require cancellation
  without specifying how.

**Nothing in this RFC is open.** The design is settled; only Gate D stands
between it and implementation.
