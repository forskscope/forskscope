# Review Request: v1 Release Stabilization Design Rereview

**Date:** 2026-07-15
**Reviewer stance:** focused architecture/design rereview
**Repository state:** inspect the current working tree directly

## Summary

Rereview the v1 stabilization design after correcting the two blocking findings
from the first architecture pass:

1. RFC-076 no longer reinterprets existing core schema v1 payloads. The
   converged settings/session contract is schema v2, with distinct migration
   routes for shipping UI-v0 plain JSON and existing core-v1 envelopes.
2. RFC-077 no longer represents expected target absence as an unchecked `None`.
   Save preconditions distinguish `MustMatch`, `MustBeAbsent`, and explicitly
   confirmed `Force`, with atomic no-clobber creation for absent targets.

No production Rust implementation is included in this package.

## Scope Followed

- Defined settings schema v2 as the union of UI-v0 and core-v1 fields.
- Kept core appearance font and shipping UI diff font separate, preserving all
  five current diff-font choices.
- Defined session schema v2 around restorable compare/explorer paths and an
  active ordering hint.
- Declared RFC-075 `CompareTabId`/generation runtime-only and distinct from
  legacy persisted `SessionId`/`TabId` values.
- Added deterministic routing for UI-v0, core-v1, current-v2, future, wrong
  schema, and corrupt inputs.
- Changed migration backup naming to `.pre-v2.bak`.
- Replaced optional target fingerprints with explicit target preconditions.
- Required missing-to-created, existing-to-deleted, replacement, Save As, and
  competing-creator tests.
- Selected same-directory `NamedTempFile::persist_noclobber` as the planned
  first no-clobber implementation, with fail-closed platform behavior.
- Required an exact, owner-backed `matrix-plan.md` before platform M5 starts.
- Updated the roadmap, RFC index, umbrella invariants, and all affected
  developer/QA handoffs.

## Files Changed

Primary files to inspect:

- `rfcs/proposed/074-v1-release-stabilization-program.md`
- `rfcs/proposed/075-async-compare-identity-and-generation.md`
- `rfcs/proposed/076-versioned-runtime-persistence.md`
- `rfcs/proposed/077-mergetool-save-target-model.md`
- `rfcs/proposed/078-platform-runtime-acceptance.md`
- `rfcs/handoffs/075-async-compare-identity-and-generation/implementation-handoff.md`
- `rfcs/handoffs/076-versioned-runtime-persistence/implementation-handoff.md`
- `rfcs/handoffs/077-mergetool-save-target-model/implementation-handoff.md`
- `rfcs/handoffs/078-platform-runtime-acceptance/acceptance-handoff.md`
- `ROADMAP.md`
- `rfcs/README.md`

Reference contracts:

- `crates/forskscope-core/src/settings.rs`
- `crates/forskscope-core/src/settings/display.rs`
- `crates/forskscope-core/src/session.rs`
- `crates/forskscope-core/src/session/tab.rs`
- `crates/forskscope-core/src/save.rs`
- `rfcs/done/011-workspace-session-persistence.md`

## Design Decisions And Assumptions

- Schema numbers describe immutable payload contracts; schema v1 is a migration
  input and cannot be assigned a new meaning.
- A v2 migration preserves represented preference values and restorable paths,
  but not IDs, dirty summaries, or unavailable unsaved content.
- Runtime load tokens are never restored, preventing cross-process identity
  reuse from validating stale work.
- Expected absence is a write precondition, not permission to skip conflict
  detection.
- Selecting an existing Save As destination requires overwrite confirmation;
  path selection alone never creates `Force`.
- No-clobber creation must fail closed if a supported platform cannot provide
  the required commit semantic.

## Review Questions

1. Does RFC-076 now preserve the immutability of existing settings/session
   schema v1 while defining complete UI-v0/core-v1-to-v2 migrations?
2. Is the v2 settings field union sufficient to prevent value loss, including
   core-only settings and UI-only font/profile/explorer settings?
3. Is the separation between legacy persisted IDs and RFC-075 runtime identity
   explicit enough to implement without accidental reuse?
4. Are the session-v1 fields deliberately discarded by migration limited to
   data that cannot or should not be restored?
5. Does RFC-077's `MustMatch`/`MustBeAbsent`/`Force` model close appearance,
   deletion, and Save As overwrite gaps?
6. Is `persist_noclobber` plus fail-closed platform acceptance an acceptable v1
   design for targets expected to be absent?
7. Are the revised handoffs aligned closely enough to begin RFC-075 only after
   this rereview accepts the design package?

## Tests And Gates Run

Observed after the documentation revisions:

```text
cargo fmt --check
git diff --check
```

A repository-local relative Markdown link scan passed. The proposed RFC file
count and RFC-index count both remained 19. A trailing-whitespace scan over all
modified design/handoff files passed.

No Rust unit/integration tests, runtime smokes, package builds, or platform
acceptance cases were run because this remains a design-only correction.

## Generated Artifacts

- This ignored rereview request is the only review artifact.
- No binary, package, generated documentation tree, or runtime evidence was
  produced.

## Known Limitations

- The exact Rust DTO/module layout remains an implementation decision within
  the accepted ownership and compatibility boundaries.
- `persist_noclobber` behavior still requires automated core tests and real
  Windows/macOS acceptance before release.
- Exact platform versions and executor roles are intentionally frozen in the
  future `matrix-plan.md`; M5 cannot begin without it.
- v1/public release remains No-Go. This request seeks design acceptance only.

## Recommended Next Step

Architect should issue Accept, Accept with notes, or Needs changes for the
corrected RFC-074–078 package. Begin RFC-075 implementation only after the
blocking design findings are recorded closed.
