# RFC 080: Tiered Directory Comparison in the Explorer

**Status.** Accepted — review complete; implementer may start. Moves to `done/` when the work ships (RFC-000, 5-folder variant, adopted 2026-09-02).
**Scheduling.** Post-Gate-D — accepted; design settled, all four questions closed. See `ROADMAP.md` § "Remaining proposed RFCs", which must list every file in `proposed/` and `accepted/` and nothing else (F83).
**Accepted.** 2026-08-21 by the project owner — Gate A cleared, and
**re-confirmed 2026-08-22 after a self-review found nine defects in it**, one of
which (§1's “no new engine work”) was load-bearing and false. The corrections
changed this RFC's scope — core must gain an error state first, now registered
as **F79** — so the re-confirmation is an acceptance of the amended design, not
a restatement of the original; **all four
design questions closed 2026-08-22, so the design is settled and only Gate D
stands between it and implementation.** Stays in
`proposed/` until implemented, per the 4-folder lifecycle (RFC-000 §Folder
layout); it moves to `done/` when the work ships, as RFC-078 does.
**Tracks.** Explorer status column; directory comparison cost model.
**Touches.** `explorer.rs`'s digest effect, `RowStatusKind` (F75 lands first —
see §7), **`forskscope-core::dir::recursive`, which needs an error state before
this RFC's error handling is expressible (§1)**, and the status legend in user
docs.
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

### 1. What core already provides — and the one thing it does not

**Correction, 2026-08-22, found while re-reviewing this RFC.** An earlier draft
of this section opened "No new engine work." **That is false**, and the reason
matters more than the sentence: this RFC's error handling rests on an assumption
about core that does not hold.

**`RecStatus` has no error variant.** Its five states are `Equal`, `Changed`,
`LeftOnly`, `RightOnly`, `Computing`, `Symlink` — and nothing else. A file the
walk cannot read is not marked; it is `continue`d past and **absent from the
result entirely**. A *directory* the walk cannot open is worse: every recursive
call is made as `let _ = walk_and_merge_fast(..)`, so the error is discarded and
that entire subtree silently vanishes. **This holds at the top level too**
(`recursive.rs:99` and `:121`): if the right root itself cannot be opened, the
walk returns a result containing only left-side entries.

The consequences for the tiers, stated plainly because each is a defect this RFC
would otherwise ship:

- **An unreadable file makes a differing pair look matching.** It vanishes, so
  tier 1 sees matching names and sizes among what remains and reports the
  tier-1 match state — a false "no difference" *caused by* the error.
- **An unreadable subdirectory removes its whole subtree from consideration**,
  with the same result and a larger blast radius.
- **An unreadable right root reports every left entry as `LeftOnly`**, which
  tier 1 reads as a confident **Different** — a definite verdict produced by a
  failed read.

So core work **is** required, and it is a precondition rather than a nicety:
`RecStatus` needs an error state, per-entry errors must populate it instead of
being skipped, and the discarded `let _ =` results must be surfaced. Core's
`EqualityEvidence::Error { message }` already exists as the vocabulary; the
recursive walk simply does not use it.

**This is F76's family a third time** — a status asserting more than was
measured — and it is why §"Relationship" now says F76 comes first rather than
merely making this RFC smaller.

Otherwise core provides both tiers already. `forskscope-core::dir::recursive` already exports both
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
| Any entry that could not be read | **Unknown** — never a verdict | — *(requires the core change in §1; not expressible today)* |
| Any entry reported `Symlink` on either side | **Unknown** — never a verdict | — |
| Otherwise | **No difference found without reading contents** | No |

The first two are free certainty and they cover the case that prompted this
RFC: the owner's test directories differed in file *content*, and content edits
usually move the size. Tier 1 would very likely have answered correctly, at the
cost of a directory walk and no reads.

**On symlinks.** Core reports `RecStatus::Symlink` and does not follow the link,
so tier 1 knows a link exists and nothing about what it points at. A symlink and
a regular file of the same name are neither proven equal nor proven different by
anything the walk did, so the honest tier-1 answer is *unknown* — not
*different*, which would be a verdict, and not silence, which would let a
symlink-versus-file mismatch pass as a tier-1 match. This was unaddressed until
the 2026-08-22 re-review.

The last row is the one that must not be overstated. Same names and same sizes
is the expected state of two directories that differ by one edited character.

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

- **Applies to file rows too (§2a).** F77 and F78 have since given both views a
  shared `DigestEpoch` carrying the cancellation token and the concurrency
  bound, so those two halves are done. **What remains for this RFC is the size
  cap** — deliberately deferred here because a file exceeding it needs somewhere
  to rest, and the only honest resting state is the tier-1 state §4 defines.
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
  **Ordering decided (Q4): F75 lands before this RFC**, so this RFC targets
  `RowStatusKind` and `DigestState` is already gone by then. An earlier draft
  said "either order works; doing them in one change is cheaper than either" —
  superseded by the owner's decision, and left stale until the 2026-08-22
  re-review.
- **F76** — statuses that overstate their evidence. **Decided (Q4): F76 lands
  first**, and §1 raises the stake: F76 is no longer only *"would make this RFC
  smaller"*, it is the entry under which core's walk learns to report an error at
  all. Without that, this RFC's §2 error row and acceptance criterion 4 cannot be
  implemented.
- **F77 and F78** — both fixed (reviews 074 and 075), and together they produced
  `DigestEpoch`: one shared generation guard, cancellation token and concurrency
  bound, already used by both views. §5's cost control is written against it,
  which is why §5 can require cancellation and bounding without specifying the
  mechanism.

## Acceptance criteria

1. A directory pair differing only in a file's **size** is reported *Different*
   by tier 1, with no file contents read.
2. A directory pair differing only in a file's **contents at identical size** is
   reported at the **tier-1 match** state by tier 1 — *“Names and sizes match;
   contents not compared”* — and *Different* by tier 2.
3. An identical pair is at the **tier-1 match** state after tier 1 and
   *Identical* after tier 2 — and **never `Identical` from tier 1 alone**.
   *(Criteria 2 and 3 said “no difference found” until the 2026-08-22
   re-review — the exact phrasing §4 rejects, left behind when the label was
   settled. Acceptance criteria are what get implemented and tested, so the
   contradiction would have shipped the rejected wording.)*
4. A subtree containing an unreadable entry, an unreadable subdirectory, or an
   unreadable root reports *unknown*, not a verdict — in particular **not** a
   tier-1 match and **not** `Different`. **Requires the core change in §1**; it
   is not satisfiable against `RecStatus` as it stands.
5. Navigating away cancels an in-flight comparison; no result lands in a view
   the user has left.
6. Every state has a non-empty localised accessible label, and completion is
   announced.
7. Browsing costs nothing beyond §5: **moving through rows starts no walk** —
   only resting on one past the debounce interval does — and no tier-2 pass
   ever starts without a click. *(This criterion read “no comparison is
   triggered by browsing alone” before the 2026-08-22 re-review, which
   contradicted Q3's decision to trigger tier 1 on selection: selecting a row
   **is** browsing. The debounce is what makes the two compatible, so the
   criterion has to name it.)*

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

**After Gate D.** This is a feature, and Gate D is blocked on **F44** (upstream)
and **F60** (an owner decision). Adding an Explorer *feature* now would put new code into a
stabilization program for a question Deep Compare already answers by another
route.

**One of this section's two original reasons is void, corrected 2026-08-22.** It
also said adding code now "would put new code into the candidate that the
acceptance matrix has not covered". **The matrix already does not cover `main`:**
M5's evidence is tied to `0.167.1`, and seven code commits have landed since it
(F74, F77, F78, F79), spanning all three crates. A re-cut and a full matrix
re-run are therefore **already mandatory** and are not a cost this RFC can avoid
by waiting. F44's fix will force the same re-run again when it lands, since a
`dioxus-desktop` bump changes every artifact's digest.

What survives is the other reason, and it is the one that matters: **this is a
feature, and defect work and feature work carry different risk during a
stabilization program.** Deferring RFC-080 is still right. Deferring the *defect*
entries it depends on — F76, and F75's wiring — no longer has the evidence
argument behind it.

Nothing is lost by waiting: F74's fix means the Explorer is currently *honest*
about not knowing, which is a correct state to ship, not a placeholder.

## Open questions — all closed

- **Q1 — tier-2 trigger. CLOSED 2026-08-22** — user-initiated per row. See §3.
- **Q2 — the tier-1 label. CLOSED 2026-08-21** — state the evidence outright.
  See §4 for the settled vocabulary and the reasoning that replaced the earlier
  recommendation.
- **Q3 — tier-1 trigger. CLOSED 2026-08-22** — on selection, debounced and
  cancellable. See §5.
- **Q4 — ordering. CLOSED 2026-08-22, and its first half corrected the same day.**
  **Decided: F75's wiring (with F76 folded into it), then this RFC.** The
  original answer was *F76, then F75, then this RFC*, on the architect's advice
  that F76 would bring `TypeMismatch` and `Error` across from core. That is
  incoherent — it would bring them into `DigestState`, the enum F75 deletes.
  **F76 is a consequence of the wiring, not a predecessor:** making the Explorer
  emit `EqualityEvidence` fixes both F76 instances by construction. The owner's
  decision to do this work now stands; only the internal order changed.
  The superseded reasoning, kept because it is what was decided against: F76 stops the statuses overstating their evidence and
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
