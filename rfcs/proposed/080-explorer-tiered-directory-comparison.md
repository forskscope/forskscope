# RFC 080: Tiered Directory Comparison in the Explorer

**Status.** Proposed
**Accepted.** 2026-08-21 by the project owner — Gate A cleared. Stays in
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

Nobody has ever decided the Explorer should do that. It would arrive as a side
effect of fixing a status glyph, which is how performance decisions get made by
accident. This RFC exists so the decision is taken deliberately, with the cost
stated.

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

**Trigger.** Owner decision, §8 Q1. The options, with my recommendation:

- **(a) User-initiated per row** — a control on a row already at *no difference
  found*. Cheapest to build, most predictable, zero surprise I/O.
  **Recommended.**
- **(b) Automatic when tier 1 finds nothing, under a size/count budget** —
  answers more without asking, but needs a budget nobody has calibrated, and
  the budget becomes a claim of its own.
- **(c) Automatic always** — the thing §"Why this is not simply" argues against.

### 4. Status vocabulary

Five states, each naming what was measured rather than what is probably true:

| State | Meaning | From |
|---|---|---|
| **Not compared** | Nothing was examined | current post-F74 default |
| **Comparing…** | A tier is running | either tier |
| **Different** | Proven — names or sizes differ, or digests differ | tier 1 or 2 |
| **No difference found** | Names and sizes match; contents unread | tier 1 only |
| **Identical** | Proven — contents compared | tier 2 only |

**On the owner's proposed "Maybe equal".** The intent is right — the label must
be weaker than `Equal` — and the wording is worth one more turn. "Maybe equal"
describes a probability the app has not computed; *"no difference found"*
describes what the app did. The distinction matters here more than it usually
would, because this project has spent a program's worth of effort on markers
credited with more than they measure, and this is the same failure at the
product surface. Final wording is the owner's call (§8 Q2); the requirement is
that the label describes a measurement.

**A tier-1 "Different" and a tier-2 "Different" are the same claim** and should
render identically. Certainty differs between *equal* verdicts, not *different*
ones.

### 5. Cost control and invalidation

- **One tier-1 pass per directory row, on demand — never a fan-out across every
  visible row.** Trigger is the owner's call (§8 Q3): on selection, on expand,
  or on an explicit control.
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

## Open questions for the owner

- **Q1 — tier-2 trigger.** (a) user-initiated per row, (b) automatic under a
  budget, or (c) automatic always? Recommendation: (a).
- **Q2 — the tier-1 label.** "No difference found", the owner's "Maybe equal",
  or another wording? The requirement is that it describe a measurement rather
  than a probability.
- **Q3 — tier-1 trigger.** On row selection, on expand, or on an explicit
  control? This decides whether browsing costs anything at all.
- **Q4 — ordering against F75/F76.** Do these three land as one change after
  Gate D, or separately? One change is cheaper; three are individually
  reviewable.
