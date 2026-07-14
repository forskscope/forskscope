# RFC 074: v1 Release Stabilization Program

**Status.** Proposed
**Tracks.** Architect audit blockers B1–B4; v1 release sequencing and evidence.
**Touches.** RFC-041, RFC-075–RFC-078, `ROADMAP.md`, release gates, and the
next project handoff.

## Summary

ForskScope is approved for continued development but is not approved for a
v1/public-release claim. This RFC converts the 2026-07-15 project-readiness
architecture audit into one bounded stabilization program. The program fixes
three correctness risks, then closes the platform-evidence gap:

1. stale asynchronous compare completions can target the wrong load;
2. runtime settings/session persistence bypasses the versioned core contract;
3. Git mergetool conflates the compared remote file with the merge output;
4. platform artifacts lack retained runtime acceptance evidence.

The detailed designs live in RFC-075 through RFC-078. This RFC owns ordering,
milestones, release gates, and the final go/no-go transition. It does not
replace the detailed RFCs.

## Source and authority

The triggering evidence is the architect-reviewed package
`.git-exclude/reviewed/027-project-readiness-architecture-review.md`. That file
is review evidence; this RFC and its child RFCs are the durable design source
of truth. If implementation evidence changes a design assumption, amend the
relevant RFC before changing scope.

RFC-041 remains the product-stabilization policy. RFC-074 is the executable
release-blocking program under that policy.

## Goals

- Remove the three identified correctness risks with deterministic tests.
- Preserve the three-crate architecture and model-backed merge boundary.
- Migrate existing user settings/session files without silent data loss.
- Make normal compare and mergetool save identities explicit and testable.
- Define retained, auditable runtime evidence for supported platforms.
- Produce a refreshed handoff and a new architect go/no-go review.

## Non-goals

- Three-way conflict workspace UI, command palette, or editor adapter work.
- New diff, directory, spreadsheet, VCS, or cloud capabilities.
- Broad visual redesign unrelated to acceptance failures.
- Claiming power-loss durability or metadata preservation beyond the contract
  explicitly tested during this program.
- Automatically accepting all `cargo audit` warnings; advisory disposition is
  an explicit release task.

## Planning assumptions

The calendar below is a planning envelope, not a promise. It assumes one
primary Rust developer, owner review at each design/implementation gate, and
access to Linux, Windows, and macOS test hosts by Milestone 5. If staffing or
host access differs, preserve dependency order and rebaseline the dates in
`ROADMAP.md`.

- Program start: 2026-07-15.
- No feature work shares the files touched by RFC-075–RFC-077 until Milestone 4.
- Each child RFC is reviewed before its implementation begins.
- A milestone completes only from observed evidence, not elapsed time.

## Workstreams

| RFC | Workstream | Audit mapping | Depends on |
|---|---|---|---|
| RFC-075 | Async compare identity and load generations | B1 | RFC-074 |
| RFC-076 | Versioned runtime settings/session persistence | B2 | RFC-074 |
| RFC-077 | Mergetool save-target model | B3 | RFC-075 |
| RFC-078 | Platform runtime acceptance and release evidence | B4 | RFC-075–077 |

RFC-076 may be implemented in parallel with RFC-075 when separate developers
own the overlapping UI state files. With one developer, the planned sequence
is RFC-075, RFC-076, then RFC-077. RFC-078 starts only after all correctness
workstreams pass their acceptance gates.

## Milestones and schedule

| Milestone | Target window | Exit evidence | Release impact |
|---|---|---|---|
| M0 — Design approval | 2026-07-15 to 2026-07-17 | RFC-074–078 reviewed; owner and architect accept scope and compatibility decisions | Release remains No-Go |
| M1 — Async identity | 2026-07-20 to 2026-07-24 | RFC-075 tests deterministically reject close/reindex and stale reload completions | B1 closed |
| M2 — Persistence convergence | 2026-07-27 to 2026-08-07 | RFC-076 UI-v0 and core-v1 migrations, v2 round-trip, future-schema rejection, and runtime-path tests pass | B2 closed |
| M3 — Mergetool target safety | 2026-08-10 to 2026-08-14 | RFC-077 existing/missing/appeared/deleted/changed merge-target tests and no-clobber creation pass | B3 closed |
| M4 — Integrated stabilization gate | 2026-08-17 to 2026-08-21 | Full documented gates; docs/RFC status synchronized; advisory dispositions recorded | Code candidate eligible for runtime QA |
| M5 — Platform acceptance | 2026-08-24 to 2026-09-11 | RFC-078 evidence matrix complete for Linux, Windows, and macOS; failures fixed or explicitly waived | B4 closed or release remains No-Go |
| M6 — Handoff and go/no-go | 2026-09-14 to 2026-09-18 | Refreshed handoff, release candidate inventory, independent architect review | v1 decision may change |

Dates are updated only in this RFC and `ROADMAP.md`; developer handoffs refer
to milestone IDs so they do not become competing schedules.

## Gate model

### Gate A — Child design approval

Before implementation of each workstream:

- its RFC is accepted by the project owner;
- blocking architecture-review findings against its design are closed;
- persistence/security/compatibility impact is explicit;
- tests are specified before production edits;
- non-goals prevent opportunistic feature work.

### Gate B — Workstream implementation

Each workstream must pass its RFC-specific acceptance tests plus:

```sh
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

The stronger `--all-targets` clippy command is advisory until its existing test
lint debt is separately resolved. New code must not add new warnings to that
command.

### Gate C — Integrated release-core gate

After RFC-075–077:

```sh
cargo fmt --check
cargo xtask css --check
cargo xtask version-sync
cargo xtask i18n
cargo xtask audit-deps
cargo audit
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

`cargo audit` exit success is not sufficient by itself. Every unsoundness
advisory must have a reachability statement, owner, review date, and upgrade
trigger in the release evidence.

### Gate D — Runtime/platform acceptance

RFC-078 defines the matrix. Artifact build success is necessary but does not
count as runtime acceptance. Evidence must identify the exact artifact digest,
host/runtime versions, executed cases, result, and any waiver.

### Gate E — Final architecture review

The architect receives:

- the refreshed requirements/design/implementation/testing handoff;
- RFC-075–078 final status and deferred notes;
- integrated gate logs;
- platform evidence and artifact hashes;
- open risks, advisory dispositions, and waivers.

Only an explicit Go verdict may remove the v1 release block.

## Cross-workstream invariants

1. A background completion is accepted only for the same immutable tab and
   same load generation that created it.
2. Compared input identity and save-target identity are separate concepts.
3. Save conflict detection compares against the snapshot of the actual target.
4. A target expected to be absent is never replaced if an entry appears before
   commit; force is an explicit confirmed operation.
5. Runtime persistence has one canonical schema owner and never silently
   overwrites an unknown future schema.
6. Existing UI-v0 plain JSON and core-v1 envelopes remain importable; migration
   failure preserves the
   original file.
7. Runtime compare/load IDs are process-local and never restored from persisted
   workspace IDs.
8. No new application-authored external network workflow is introduced.
9. Runtime evidence must be reproducible from committed instructions.

## Documentation and lifecycle updates

At M4:

- amend RFC-041's checklist and current counts;
- annotate RFC-058 with the current fail-closed security suspension;
- reconcile current architecture paths and persistence claims;
- decide whether fully shipped RFC-062 moves to `done/`;
- update save durability wording to match observed guarantees;
- ensure the workspace version/changelog represent post-0.164.0 work.

At M6, create a new handoff bundle. Do not amend the historical v0.164 bundle
in place.

## Non-blocking audit disposition

The advisory findings do not block B1–B4 implementation, but each needs a
recorded outcome before the final architecture review.

| Finding | Milestone | Required disposition |
|---|---|---|
| N1 — digest not used for save conflict checks | M4 | Either use the digest when metadata is inconclusive, or document the same-size/same-mtime limitation in user and threat-model guidance |
| N2 — atomic/power-loss wording overclaims | M4/M5 | Narrow the durability claim unless file and parent synchronization plus metadata behavior are implemented and evidenced per platform |
| N3 — handoff drift | M6 | Generate a new bundle from repository truth; do not revise the historical archive |
| N4 — RFC-058 lifecycle drift | M4 | Add a security-suspension note and link the current fail-closed decision while preserving its historical implementation record |
| N5 — audit warning debt | M4 | Record reachability, owner, review date, and upgrade trigger for each unsoundness advisory; keep policy-pass distinct from a clean advisory set |
| N6 — clippy/ELOC scope | M1–M4 | Avoid growth in touched large Rust files, split on natural boundaries, and record both mandatory and stronger all-target clippy results |
| N7 — VCS temporary-directory assumption | M4 | Add a discovery-boundary test seam or explicitly document the non-hermetic `TMPDIR` constraint and track the test fix |

An advisory may become release-blocking if investigation shows reachable data
loss, unsoundness on a supported runtime, or a false public guarantee.

## Risks and controls

| Risk | Control |
|---|---|
| UI state changes overlap and cause review-heavy patches | Land RFC-075 before RFC-077; keep RFC-076 persistence adapters isolated |
| Legacy migration silently loses unfamiliar fields | Preserve original, create backup before rewrite, test fixtures from current production JSON |
| Platform QA begins before code stabilizes | RFC-078 depends on completion of RFC-075–077 and Gate C |
| Schedule pressure turns waivers into silent omissions | Every skipped case has owner, reason, expiry, and release impact |
| Historical RFCs conflict with current behavior | Amend status notes without deleting historical decisions |

## Acceptance criteria

- RFC-075–078 are reviewed and implemented or explicitly deferred with the v1
  release remaining No-Go.
- B1–B3 have deterministic regression tests on the shipping code paths.
- Gate C passes from observed output on the release candidate.
- RFC-078's matrix is complete or every missing case is a release-blocking
  failure.
- The new handoff matches the repository's actual MSRV, schemas, dependency
  policy, module map, test counts, and XLSX posture.
- An independent architect issues a new go/no-go verdict.

## Recommended implementation sequence

1. RFC-075: establish stable identity tokens before further async state work.
2. RFC-076: converge persistence and migration while the release remains gated.
3. RFC-077: build mergetool output identity on the stable tab/load model.
4. Integrated gates and documentation reconciliation.
5. RFC-078 platform matrix.
6. Refreshed handoff and architecture review.
