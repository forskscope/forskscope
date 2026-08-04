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

The program is planned as an ordered sequence of tasks with explicit
dependencies and gates. It carries no target dates and no effort estimates:
the owner's decision of 2026-08-01 is that task content and task order are the
schedule. A milestone is complete when its gate evidence exists, and not
before.

- Program start: 2026-07-15.
- One primary Rust developer owns implementation; the architect owns design,
  review, and gate evidence.
- Linux, Windows, and macOS test hosts are confirmed available (owner,
  2026-08-01), so Milestone 5 has no host-access precondition outstanding.
- No feature work shares the files touched by RFC-075–RFC-077 until Milestone 4.
- Each child RFC is reviewed before its implementation begins.
- Ordering may change only by amending this RFC and `ROADMAP.md` together.

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

## Milestones and sequence

| Order | Milestone | Depends on | Exit evidence | Release impact |
|---|---|---|---|---|
| — | M0 — Design approval | — | RFC-074–078 reviewed; owner and architect accept scope and compatibility decisions | Release remains No-Go |
| — | M1 — Async identity | M0 | RFC-075 tests deterministically reject close/reindex and stale reload completions | B1 closed |
| — | R0 — Stabilization baseline | M1 | Version and CHANGELOG represent the unreleased delta; documentation-truth corrections landed; `version-sync` rejects an already-published version; release preflight passes; owner approves the release | No blocker closed; removed version-integrity drift |
| 1 | M2 — Release mechanics and persistence convergence | R0 | **M2-A:** CHANGELOG-sourced release notes, release policy documented, threat-model currency (F19–F22), verified at the next real cut. **M2-B:** RFC-076 UI-v0 and core-v1 migrations, v2 round-trip, future-schema rejection, and runtime-path tests pass | B2 closed |
| 2 | M3 — Mergetool target safety | M1 (hard); sequenced after M2 | RFC-077 existing/missing/appeared/deleted/changed merge-target tests and no-clobber creation pass | B3 closed |
| 3 | M4 — Integrated stabilization gate | M2, M3 | Full documented gates; docs/RFC status synchronized; advisory dispositions recorded; `matrix-plan.md` frozen | Code candidate eligible for runtime QA |
| 4 | M5 — Platform acceptance | M4 | RFC-078 evidence matrix complete for Linux, Windows, and macOS; failures fixed or explicitly waived | B4 closed or release remains No-Go |
| 5 | M6 — Handoff and go/no-go | M5 | Refreshed handoff, release candidate inventory, independent architect review | v1 decision may change |

M2 carries two slices because three consecutive defects were found in release
mechanics — an unfired trigger, a contradictory numbering rule, and inert notes
generation — all sharing the shape of configuration credited as working without
ever being exercised. M2-A treats the pipeline as one unit and lands before
M2-B so that a release-mechanics change is not reviewed under the attention a
production persistence rewrite demands.

R0 is a release-baseline milestone, not a fifth correctness workstream. It
closes no audit blocker. It exists because `0.164.0` is published and immutable
while the working tree has advanced past it, so every later milestone would
otherwise inherit a version number that misidentifies the code under test. R0
pulls the version/CHANGELOG reconciliation and the documentation-truth subset
forward out of M4 without adding scope.

M3's only hard dependency is M1. It is sequenced after M2 because one developer
owns the overlapping UI state files; separate ownership would permit
concurrency without changing any gate.

Sequence is updated only in this RFC and `ROADMAP.md`; developer handoffs refer
to milestone IDs so they do not become competing schedules.

### Progress record

- **2026-07-15 — M1/B1 complete.** RFC-075 moved to `done/` after two accepted
  implementation checkpoints. Stable process-local tab IDs, per-load
  generations, centralized token validation, and deterministic close/reindex
  and stale-reload tests now guard both compare load paths.
- **2026-08-01 — M1 closure committed; program rebaselined.** The RFC-075
  documentation closure was accepted by rereview 031 and committed. Three
  owner decisions were recorded: R0 is inserted as the next milestone and will
  be released as `0.165.0`; the program drops calendar windows and effort
  estimates in favour of task order and dependencies; RFC-078 host access for
  Linux, Windows, and macOS is confirmed available, removing M5's outstanding
  precondition. A new finding drove R0: the workspace version still declares
  the published, immutable `0.164.0` while the tree carries an MSRV change, the
  fail-closed XLSX decision, the dependency-path constraint, release/CI gate
  alignment, and the RFC-075 fix.
- **2026-08-01 — earlier review 001 re-examined.** The 2026-07-09 architecture
  readiness review was re-checked against current evidence. Its five blocking
  findings are closed except one: formatting passes, the four then-active
  advisories are gone (`time` 0.3.47, `crossbeam-epoch` 0.9.20, and the
  `sheets-diff -> calamine -> quick-xml` runtime path removed), the S-001
  network claim is now an explicit reviewed acceptance, and the archive-layout
  contract is enforced in both the script and PKGBUILD. Its finding 5 is only
  partly closed: workflow content was aligned, but the release trigger has
  never matched a real tag, so R0 gains the trigger repair. Its non-blocking
  finding B is adopted as an M4 feature-claim reachability audit, with the
  README three-way merge wording corrected at R0.
- **2026-08-01 — R0 complete; `0.165.0` released.** The version and CHANGELOG
  were reconciled with the 26-commit delta past the published `0.164.0` tag,
  the release-workflow trigger was corrected to the project's unprefixed tag
  form, `cargo xtask version-sync` gained a published-tag check, and five
  documentation-truth defects were fixed. Review 032 conditionally approved the
  work with six documentation-currency follow-ups carried into `0.166.0`.

  R0's defining outcome was procedural: requiring an *observed* release-workflow
  run rather than a configuration review immediately exposed F17, a Windows
  build failure in the `app-json-settings` dependency, which was reported
  upstream, fixed in 2.4.1, and re-verified before tagging. Configuration
  review could not have found it. Two rules follow for every future
  release-bearing handoff, and neither was stated in R0's:

  1. a red platform job in a release run is a stop-and-report condition, not an
     occasion to complete the release by another route;
  2. release actions divide into three, and only the last is manual: CI builds
     the artifacts and creates the draft; CI composes the release notes from
     the tag's CHANGELOG section; a human publishes the draft. Publishing stays
     manual deliberately — draft state is the owner's approval gate and the
     inspection window that made R0's red Windows job recoverable. Creating or
     composing by hand bypasses evidence; publishing by hand is the control.

- **2026-08-02 — `0.165.0` published; release-cycle numbering corrected.** The
  release is out of draft. An owner challenge exposed that the program's
  numbering rule (`0.MINOR.0` unconditionally) contradicted
  `docs/src/maintainers/release.md`'s content-driven scheme, and that R0's
  post-release bump had applied the former automatically — pre-committing the
  next release to a minor level before its scope existed. `release.md` is
  reaffirmed as authoritative: the post-release bump now defaults to the next
  patch level, and promotion to minor happens at release time from observed
  content with owner confirmation. The tree was re-bumped `0.166.0` → `0.165.1`
  accordingly. Registered as F19–F21 against M2.

- **2026-08-04 — M2-B/B2 complete.** RFC-076 moved to `done/` after six
  reviewed patches. The running app reads and writes settings/session
  exclusively through core's versioned schema-v2 repositories; a legacy
  UI-v0 file migrates with a durable `.pre-v2.bak` backup; a future-version or
  corrupt file is preserved byte-identical and reported through a blocking
  recovery dialog (Exit/Continue with defaults/Continue without
  saving/Reset and back up) instead of silently collapsing to defaults.
  M2-A (release-notes composition, release policy documentation,
  threat-model currency) remains open within M2.

- B3, B4, Gate C, runtime/platform evidence, and the final architecture
  verdict remain outstanding. The v1/public-release decision remains
  **No-Go**.

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

At R0:

- ensure the workspace version and CHANGELOG represent the unreleased delta;
- correct the maintainer test-count table to observed values;
- remove the architecture description of the shim re-export layer deleted by
  RFC-073;
- record the known settings-persistence gap in the threat model instead of
  claiming no residual concern while B2 is open;
- extend `cargo xtask version-sync` to reject a workspace version equal to an
  already-published tag;
- correct the release workflow trigger so it matches the project's unprefixed
  `X.Y.Z` tag convention, and confirm from an observed run that gates and
  artifact jobs actually execute;
- correct the README three-way merge claim so it states the conflict workspace
  UI is deferred post-v1 rather than in progress.

At M4:

- amend RFC-041's checklist and current counts;
- annotate RFC-058 with the current fail-closed security suspension;
- reconcile current architecture paths and persistence claims;
- decide whether fully shipped RFC-062 moves to `done/`;
- update save durability wording to match observed guarantees;
- audit every public feature claim in `README.md` and the user documentation
  for core-complete versus user-reachable status, so a shipped domain model is
  never presented as a usable feature;
- freeze `matrix-plan.md` with exact OS versions before M5 begins.

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
| Working tree carries a published version number, misidentifying binaries and defect reports | R0 reconciles the version and CHANGELOG; `version-sync` gains a published-tag check so the drift cannot recur silently |
| RFC-076 rewrites the production persistence path | Design review after the schema/fixture patch, before any production load/save call is switched |
| RFC-077 promotes `tempfile` from a dev-dependency to a normal dependency | Re-run `cargo xtask audit-deps` and `cargo audit` at that workstream gate |
| Release automation is treated as evidence without ever having executed | R0 requires an observed workflow run, not workflow content review; any later gate added to release automation must be evidenced by a run before it counts |

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
2. R0: reconcile the release baseline so later milestones carry a version that
   identifies the code under test.
3. RFC-076: converge persistence and migration while the release remains gated.
4. RFC-077: build mergetool output identity on the stable tab/load model.
5. Integrated gates and documentation reconciliation.
6. RFC-078 platform matrix.
7. Refreshed handoff and architecture review.
