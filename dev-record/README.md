# Development record

The working record behind this project's changes: what was asked for, what was
built, and what was checked before it was accepted.

Until 2026-09-24 these documents lived outside the repository, in an untracked
directory. That meant the reasoning behind roughly 150 changes existed on one
machine, with no history and no backup, while `rfcs/README.md` still advertised
a tracked handoff convention that had stopped in August (register entry
**F113**). They are tracked from now on.

## What is here

| Directory | Written by | Contents |
|---|---|---|
| `handoffs/` | architect | Implementation instructions for work that comes from the **register** in `ROADMAP.md` — an `F`-numbered finding rather than an RFC. |
| `review-requests/` | dev team | The report back: what was built, the falsifications that were run, what could not be tested, and what was deliberately left out. |
| `reviews/` | architect | The verdict on a review request: what was verified independently, what is required before it closes, and which register entries it moves. |

**Handoffs derived from an RFC are not here.** They live beside their RFC, in
`rfcs/handoffs/NNN-slug/`, under the convention RFC-000 defines: a handoff has
no lifecycle state of its own and inherits its RFC's. `rfcs/README.md` lists
them.

## Numbering

Handoffs share one sequence regardless of where the file ends up, so handoff
039 is RFC-080's tier-1 handoff in `rfcs/handoffs/` while handoff 038 is in
`handoffs/` here. Reviews and review requests each have their own sequence.
The numbers are the way these documents refer to each other, so they do not
change when a file moves.

## How to read a change

Most changes leave a trail of four documents:

1. the register entry in `ROADMAP.md`, which says why the work exists;
2. a handoff, here or under `rfcs/handoffs/`, which says what to build and
   what evidence will be required;
3. a review request, which reports what was built and shows the checks
   failing before they pass;
4. a review, which records what the architect verified independently — often
   by re-running the falsification rather than taking the report's word.

`CHANGELOG.md` states what shipped; this directory states how it was decided
and what was checked.

## Two conventions worth knowing

**Falsification.** A check is not accepted because it passes. It is accepted
when it has been shown to fail against the defect it claims to catch, and not
merely against a helper the fix introduced. Most review requests here contain
the output of a deliberately broken run.

**Say what was not done.** Review requests are expected to name what could not
be tested and why, rather than leaving a gap to be inferred. Several of the
most useful entries here are reports of a check that could not be performed.

## Note on these files

Absolute home directory paths were replaced with `/home/<user>` when these
documents were brought into the repository on 2026-09-24. Nothing else was
edited: the text is as it was written at the time, including the errors it
records and the corrections that followed.

Owner-facing task notes stay untracked. They hold account-setup detail that
does not belong in a public repository.
