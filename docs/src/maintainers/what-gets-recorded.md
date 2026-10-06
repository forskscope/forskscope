# What gets recorded, and where

**The rule: a tracked document must contain the substance it relies on.**

Work on this project generates two kinds of writing. One is the record: it is
committed, it is read by people who were not there, and it must still make
sense years later. The other is working material — instructions for a piece of
work, the report back, the review of it. That material is useful while the work
is in flight and is not kept.

The failure this page exists to prevent is putting record-grade content into
working material. It is invisible at the time, because everyone involved can
still read both.

## Where each kind of knowledge belongs

| Knowledge | Home |
|---|---|
| Why a design is the way it is: alternatives weighed, questions closed | An RFC under `rfcs/`. [RFC-000](../../../rfcs/done/000-rfc-lifecycle-policy.md) governs their lifecycle. |
| A defect or risk: what it is, the evidence, the disposition, how it was resolved | The register in `ROADMAP.md` |
| What shipped, and what changed for a user | `CHANGELOG.md` |
| How the system works, and **what a gate actually guarantees** | This maintainers section |
| Evidence that a release was fit to publish | `docs/src/maintainers/release-evidence/` |
| Why one line is the way it is — a constant's value, a workaround, an ordering | A comment at that line |
| How to implement one unit of work, and the review of it | Working notes, untracked |

## The test

> If deleting the untracked working notes would lose something, it was
> recorded in the wrong place.

Apply it while the work is in flight, not afterwards. A falsification that
proves a check can fail, a reason a bound has the value it has, a limit
discovered while implementing — each belongs in one of the tracked homes above
**at the moment it is established**.

## Citing working material

A tracked document may name a review or a handoff as **attribution**:

> Found in review 058, while diagnosing F74.

It must not **defer its substance** to one:

> See F55's review request for the falsifiability demonstration.

The second sentence sends a reader to something they cannot open. If the
demonstration matters enough to mention, it matters enough to state here; if it
does not, drop the sentence. The register entries in `ROADMAP.md` follow this
rule: they name the review that found a thing, and then say what was found.

## A change to a module's behaviour updates its rows

When a change alters what a module does, that module's row in
`architecture.md` and `testing.md` is in scope for the same change. The review
request says whether the row changed, or why it is still accurate.

The documentation gates check which modules are listed, not what a row says
(`cargo xtask ui-logic-docs` states this in its own module doc). So a row's
wording is a review step, and a reviewer applies it like this:

1. Find each module the change touches, and its row in the two tables. The
   module path is the row's name: `explore/sync_panes` is the row for
   `crates/forskscope-ui-logic/src/explore/sync_panes.rs`.
2. Read the row against the new behaviour. A row that describes behaviour the
   change removed or replaced is stale, and the change is not complete until
   the row is corrected.
3. The review request states either *row changed: what changed* or *row
   unchanged: why it is still accurate*.

Why this rule exists: F165. `testing.md`'s row for `explore/sync_panes` still
described the pre-anchor `mirror_target` after the behaviour had changed, and
the gate stayed green throughout.

## What is tracked, and what is not

Tracked: RFCs, the register, the changelog, this documentation, release
evidence, and **handoffs that are companions to an RFC**, under
`rfcs/handoffs/NNN-slug/`.

RFC-000 is explicit about the limits of that last folder. Only *"current,
reviewed, implementation-useful companion documents"* belong there, and every
directory corresponds to an existing RFC number. Review notes, intermediate
discussion and handoffs for register findings are therefore not tracked; they
are working material, and the rule above applies to them.

## Why this page exists

In September 2026 an architect proposed publishing several hundred working
documents because the reasoning behind roughly 150 changes existed only in
them. The reasoning was the problem, not the storage: content that deserved to
survive had been written into material that was never meant to. Register entry
F113 records the episode.
