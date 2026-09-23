# Review Request: v1 Release Stabilization Design

**Date:** 2026-07-15
**Reviewer stance:** architecture/design/release-plan review
**Repository state:** inspect the current working tree directly

The reviewer should inspect the proposed RFCs, roadmap, and handoffs directly.
This is a design gate before implementation; it does not claim that any of the
release blockers have been remediated.

## Summary

Review the proposed v1 stabilization program created from the blocking and
non-blocking findings in the project-readiness architecture review.

The package turns the four blocking findings into ordered workstreams:

1. stable compare-tab identity and generation-checked asynchronous completion;
2. versioned core-owned runtime persistence with migration and recovery;
3. a typed mergetool save-target model independent of the remote comparison
   document;
4. reproducible Linux, Windows, and macOS runtime acceptance evidence.

The proposed schedule assumes one primary Rust developer, begins with design
approval, and ends with an independent v1 go/no-go architecture review.

## Scope Followed

- Added an umbrella stabilization program with dependencies, milestones,
  schedule, cross-workstream invariants, and release gates.
- Drafted one detailed RFC for each blocking finding.
- Defined deterministic test designs for asynchronous close/reload races.
- Defined versioned settings/session schemas, legacy import, future-version
  handling, corrupt-data recovery, and migration safety.
- Separated compare inputs from save destination for mergetool mode.
- Defined an artifact-bound platform acceptance matrix and evidence format.
- Added optional implementation/acceptance handoffs for delegated work.
- Updated the roadmap and RFC index without marking any proposed work complete.

## Primary Files To Inspect

- `ROADMAP.md`
- `rfcs/README.md`
- `rfcs/proposed/074-v1-release-stabilization-program.md`
- `rfcs/proposed/075-async-compare-identity-and-generation.md`
- `rfcs/proposed/076-versioned-runtime-persistence.md`
- `rfcs/proposed/077-mergetool-save-target-model.md`
- `rfcs/proposed/078-platform-runtime-acceptance.md`
- `rfcs/handoffs/075-async-compare-identity-and-generation/implementation-handoff.md`
- `rfcs/handoffs/076-versioned-runtime-persistence/implementation-handoff.md`
- `rfcs/handoffs/077-mergetool-save-target-model/implementation-handoff.md`
- `rfcs/handoffs/078-platform-runtime-acceptance/acceptance-handoff.md`

## Design Decisions And Assumptions

- RFC-075 precedes RFC-077 because mergetool loading must use the same stable
  asynchronous completion contract as normal compare tabs.
- RFC-076 is logically independent and may run in parallel only when a second
  owner can avoid overlapping the same UI state files; the baseline schedule is
  sequential.
- Runtime acceptance begins only after the three correctness workstreams pass
  their focused and integrated gates.
- Platform evidence is tied to an exact commit and artifact digest; screenshots
  alone are not sufficient.
- The release target is conservative: ambiguous persistence data or ambiguous
  save-target state must fail visibly without overwriting user data.
- Dates are planning targets, not evidence that a milestone is complete.

## Review Questions

1. Do RFCs 075-078 fully close blocking findings B1-B4 from the architecture
   review, or is any blocker only partially addressed?
2. Is the RFC-075 identity-plus-generation contract sufficient for every
   close, reorder, reload, and stale-completion race?
3. Does RFC-076 put the durable schema at the correct ownership boundary, and
   are its migration/recovery rules safe against silent data loss?
4. Does RFC-077 cleanly separate `remote` comparison content from the merge
   result destination in load, fingerprint, reload, overwrite, and save paths?
5. Is RFC-078's platform matrix strong enough to support a public v1 go/no-go
   decision, including native overwrite semantics?
6. Are milestone dependencies, estimates, contingency, and exit gates credible
   for one primary Rust developer?
7. Are any non-blocking audit findings missing a disposition or scheduled
   decision point?
8. Are the developer handoffs narrow and concrete enough to delegate without
   introducing design drift?

## Observed Evidence

Observed for this documentation-only package:

```text
git diff --check
```

A repository-local link scan reported no broken relative Markdown links in the
new or modified files. The RFC index count matched the 19 proposed RFC files.

No Rust implementation, unit test, integration test, runtime smoke test,
package build, or platform gate was run for this design package.

## Known Limitations

- The proposed schedule depends on reviewer availability and platform access.
- Windows and macOS executor ownership is intentionally left as a pre-M5
  planning decision rather than assigned to an unconfirmed person.
- Exact code shapes in proposed RFCs may change during implementation review,
  but the safety invariants and acceptance outcomes should remain stable.
- The package does not itself remediate B1-B4 and must not change the current
  v1/public-release No-Go status.

## Acceptance Criteria For This Review

- Accept, revise, or reject the RFC dependency order and milestone plan.
- Confirm that each blocking finding has a testable closure condition.
- Confirm the persistence ownership and migration policy before implementation.
- Confirm the mergetool source/destination model before modifying startup and
  save paths.
- Confirm the platform matrix and evidence-retention policy before reserving
  platform test time.
- Record any required RFC changes; implementation begins only after the design
  gate is accepted.
