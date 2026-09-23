# Review Request: RFC-075 Documentation Closure

**Date:** 2026-07-15
**Reviewer stance:** focused lifecycle/evidence review
**Repository baseline:** `be5d28e` (`ui: guard async compare completion with load tokens`)
**Repository state:** inspect the current working tree directly

## Summary

Review the documentation-only closure of RFC-075 and stabilization finding B1.
The accepted RFC is moved from `proposed/` to `done/`, its durable wording is
aligned with the implemented exhaustion behavior, the developer handoff now
records actual files and observed evidence, and the roadmap/umbrella program
record M1/B1 complete while retaining the overall v1 No-Go.

No Rust source, dependencies, runtime behavior, release artifact, or RFC-076
implementation is changed in this patch.

## Scope Followed

- Moved RFC-075 to the lifecycle-authoritative `rfcs/done/` folder.
- Changed its status to `Implemented (post-v0.164.0 stabilization)` without
  inventing an unreleased version number.
- Updated the allocator description to the accepted typed root-owned signal.
- Distinguished new-tab ID exhaustion from existing-tab generation exhaustion,
  closing review 029 note N1.
- Added an implementation outcome naming commits `ad2e98e` and `be5d28e`, the
  accepted checkpoint reviews, deterministic race coverage, and B1/M1 closure.
- Replaced the developer handoff's future-tense plan with the actual changed
  files, accepted decisions, observed gates, limitations, review links, and
  next workstream boundary.
- Updated the RFC index from 48/19 to 49 implemented/18 proposed and rewrote the
  RFC-075 link to `done/`.
- Added matching M1/B1 progress records to RFC-074 and `ROADMAP.md`.
- Explicitly retained B2–B4, Gate C, platform evidence, and final architecture
  review as outstanding; v1/public release remains No-Go.

## Files Changed

- `rfcs/done/075-async-compare-identity-and-generation.md` (moved from
  `rfcs/proposed/` and finalized)
- `rfcs/handoffs/075-async-compare-identity-and-generation/implementation-handoff.md`
- `rfcs/proposed/074-v1-release-stabilization-program.md`
- `rfcs/README.md`
- `ROADMAP.md`

## Design Decisions And Assumptions

- The RFC lifecycle folder is authoritative, so closure requires a move to
  `done/` in the same patch as the status/index updates.
- `post-v0.164.0 stabilization` is an honest lifecycle marker until a later
  release assigns the implementation a shipped version.
- B1 is resolved by the accepted implementation and deterministic evidence;
  this does not satisfy the umbrella program's remaining release gates.
- Review records remain under `dev-record/reviews/` and are linked from the
  handoff as repository-local evidence, consistent with RFC-074's audit-source
  convention.
- Review 029's optional direct Store-level exhaustion integration tests remain
  documented limitations, not blockers retroactively added to RFC acceptance.

## Review Questions

1. Does the RFC move/status/index update satisfy the repository lifecycle
   policy without claiming an unreleased version?
2. Does the final exhaustion wording accurately incorporate review 029 N1?
3. Is the completed handoff sufficiently precise about implementation files,
   decisions, observed evidence, accepted reviews, and known limitations?
4. Is B1/M1 closure supported by the accepted implementation evidence while
   remaining clearly distinct from overall v1 Go?
5. Are all remaining dependencies and the RFC-076 next-step boundary stated
   consistently across RFC-074, RFC-075, the index, handoff, and roadmap?

## Tests And Gates Run

Observed for this documentation-only closure:

```text
cargo fmt --check
  pass
git diff --check
  pass
```

Additional documentation checks passed:

- RFC lifecycle file counts: 49 implemented, 18 proposed;
- new `done/` path exists and old `proposed/` path does not;
- no Markdown reference remains to the old proposed RFC-075 path;
- both repository-local checkpoint-review links resolve;
- no trailing whitespace exists in the five changed durable Markdown files.

Rust tests and Clippy were not rerun because this patch changes documentation
only. The handoff preserves the implementation and independent-review evidence
observed before baseline `be5d28e`.

## Generated Artifacts

- This ignored review request is the only review artifact.
- No binary, package, generated documentation tree, test temp directory, or
  release archive was produced.

## Known Limitations

- Git reports the unstaged RFC move as a deletion plus untracked addition;
  normal staging should recognize the rename by content similarity.
- Review links target ignored repository-local evidence and are not intended as
  public documentation URLs.
- RFC-076–078, integrated gates, runtime/platform acceptance, refreshed final
  handoff, and the final Go/No-Go architecture verdict remain outstanding.

## Recommended Next Step

Architect should issue Accept, Accept with notes, or Needs changes for this
documentation closure. If accepted, commit the closure separately, then begin
RFC-076 implementation in a new reviewed workstream.
