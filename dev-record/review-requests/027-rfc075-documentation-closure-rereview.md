# Review Request: RFC-075 Documentation Closure Rereview

**Date:** 2026-07-15
**Reviewer stance:** focused lifecycle correction rereview
**Repository baseline:** `be5d28e` (`ui: guard async compare completion with load tokens`)
**Prior review:** `dev-record/reviews/030-rfc075-documentation-closure-review.md`
**Repository state:** inspect the current working tree directly

## Summary

Rereview the RFC-075 documentation closure after applying review 030's one
blocking correction and both recommended non-blocking clarifications.

The RFC-075 implementation remains unchanged and accepted. This correction is
documentation-only and does not begin RFC-076.

## Review 030 Corrections

### B1 — Resolved

Removed RFC-075 from `ROADMAP.md`'s **Remaining proposed RFCs** table. The
roadmap now agrees with the lifecycle-authoritative `rfcs/done/` location, the
Implemented status, the RFC index, and the M1/B1 completion record.

### N1 — Resolved

Updated the roadmap's current headless inventory from 930 to 943 tests and from
228 to 241 UI-logic unit tests, preserving the independently observed
arithmetic:

```text
643 core unit + 45 core integration + 241 UI-logic unit
+ 6 CSS integration + 8 doctests = 943
```

### N2 — Clarified

Changed review 028/029 Markdown links in the completed handoff to backticked
paths under an explicit “Ignored workspace-local review evidence
(not committed/public links)” label.

## Files Changed Since Review 030

- `ROADMAP.md`
- `rfcs/handoffs/075-async-compare-identity-and-generation/implementation-handoff.md`

The full closure patch still includes the RFC move/status, RFC index,
RFC-074 progress record, and completed handoff described by review request 026.

## Review Questions

1. Is the lifecycle contradiction closed now that RFC-075 is absent from the
   remaining-proposed roadmap table?
2. Are the 943 total and 241 UI-logic counts stated consistently with review
   030's observed inventory?
3. Is the local/non-public nature of review 028/029 evidence now unambiguous?
4. May the RFC-075 documentation closure be accepted and committed separately
   before RFC-076 begins?

## Checks Run

Observed after the corrections:

```text
cargo fmt --check
  pass
git diff --check
  pass
```

Focused documentation checks also passed:

- 49 RFCs in `done/` and 18 in `proposed/`;
- new RFC-075 done path exists and old proposed path is absent;
- no reference remains to the old proposed RFC-075 path;
- RFC-075 is absent from the roadmap's remaining-proposed table;
- roadmap contains 943 total and 241 UI-logic tests, with no stale 930/228
  inventory text;
- the handoff labels review paths as ignored workspace-local, non-public
  evidence;
- no trailing whitespace in the five durable closure documents.

No Rust tests or Clippy commands were rerun because the corrections are
documentation-only. Accepted implementation evidence remains unchanged.

## Known Limitations

- RFC-076–078 and later stabilization/release gates remain outstanding.
- Overall v1/public release remains No-Go.

## Recommended Next Step

Architect should issue Accept, Accept with notes, or Needs changes for the
corrected RFC-075 closure. If accepted, commit this closure before beginning
RFC-076 as a separate workstream.
