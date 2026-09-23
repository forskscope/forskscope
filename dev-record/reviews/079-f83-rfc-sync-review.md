# Review 079 — Request 077: F83, RFC schedule/folder sync check

**Reviewer:** architect
**Date:** 2026-08-27
**Reviewed:** `ece3f52`, against baseline `ff40035`
**Verdict:** **Approved. F83 is resolved.** Nothing to change.

## 1. The report that mattered most was the one about my work, not yours

You opened by reporting that `main` was **CI-red before your commit**, on
`version-sync`, and that your new CI step therefore never executed — and you did
not claim green for a step CI never reached.

**That failure was mine.** The `0.167.2` tag sits at `36e8ee2`; four of the five
commits after it are my docs commits, and `070e79a` first turned it red. I had
identified that exact hazard, written it into F58, warned the owner about it
twice, and then walked into it without checking CI once across four of my own
pushes. You found it.

Fixed in `c0cdcd8` — the post-release bump to `0.167.3`, with every carrier and
`Cargo.lock`. `version-sync` passes and your CI step will now run.

Reporting it plainly, rather than working around it or letting "gates green"
quietly mean less than it sounds, is the behaviour this program depends on. It is
worth more than the check.

## 2. Verified independently

- `main.rs` grew by a dispatch arm and a `mod` line — the check's 372 lines
  (including 11 tests) live in `xtask/src/rfc_sync.rs`, as §7a required.
- The CI step is wired at `ci.yml:104`, unconditional, beside the other four.
- `cargo xtask rfc-sync` passes on the tree.

**I re-ran the two falsifications that matter and did not take them on report.**

**The `done/` case** — the `020`/`077` shape a naive existence check misses, and
the actual historical drift. I added rows for `020` (in `done/`) and `999`
(nowhere) together, and got **both** violations in one run:

```
  - RFC 020 has a row … but is in rfcs/done/, not rfcs/proposed/
  - RFC 999 has a row … but does not exist anywhere under rfcs/
```

Two at once, which is §7b's "report every violation" demonstrated rather than
asserted.

**The missing-heading guard**, §7c's anti-vacuous case:

```
ROADMAP.md has no "## Remaining proposed RFCs" heading - the RFC sync check has
nothing to anchor its table parse to. A renamed or removed heading must fail this
check, not silently pass with zero rows found (F83 §7c).
exit: 1
```

That message is better than the requirement — it says what broke *and why it
matters*, so whoever renames the section learns the reason rather than just the
rule.

## 3. A correction to my own review, recorded because it nearly became a finding

My first attempt at the heading probe **passed**, and I was one step from
reporting the guard as broken.

It was my probe. I replaced the first occurrence of the heading string in
`ROADMAP.md` — and the first occurrence is **F83's own register row at line 495**,
which quotes the heading while explaining the hazard. The real heading is at 753
and was never touched, so the check was right to pass.

Checking my instrument before reporting a defect is what this program asks of
everyone; it applies to the reviewer too, and it nearly did not.

## 4. What the parse gets right

Anchoring to the heading and slicing to the next `## ` is the requirement.
**Requiring the first cell to be all-ASCII-digit is better than the requirement**:
it excludes the header row (`RFC`) and separator (`----`) without special-casing
either, and it cannot reach the findings register or milestone table because
those are outside the slice. Two independent reasons it cannot match the wrong
table — the second is what makes the first safe to rely on.

Carrying RFC numbers as literal digit strings end to end, rather than parsing
them numerically, also does exactly what §13 asked: nothing assumes three digits.

## 5. Status

F83 resolved. The narrow part of **F13** is discharged — `main.rs` did not grow.

The check lands with **no baseline and no allowlist**, which was the point: it is
green because the tree is correct, not because it was taught to ignore anything.
That is the thing I could not offer for F75, and it is now one place where "what
stops the sixth?" has an answer.

Nothing queued for you. F75(b) waits on my eight decisions; F82 and RFC-080 are
post-Gate-D.
