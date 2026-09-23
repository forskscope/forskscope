# RFC-074 R0 Developer Handoff: Stabilization Baseline

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** R0 — Stabilization baseline
**Release.** 0.165.0

This handoff directs execution of one milestone. It does not redefine RFC-074.
If implementation evidence contradicts a decision below, amend RFC-074 first,
then update this handoff to match.

## 1. Summary

R0 is the first milestone of the RFC-074 release-stabilization program after
M1. It changes no product behaviour. It reconciles the release baseline so that
every later milestone carries a version number identifying the code under test,
and so that release automation produces real evidence.

R0 exists because `0.164.0` is a published, immutable tag while the working
tree has advanced 26 commits past it — including an MSRV change, the
fail-closed XLSX decision, the Dioxus dependency-path constraint, release and
CI gate alignment, and the RFC-075 correctness fix — with `Cargo.toml` still
declaring `0.164.0`. A source build therefore reports a version whose published
artifact behaves differently, and `PlatformInfo` propagates that version into
`--diagnostics` and the About panel, so defect reports would be misattributed.

R0 also repairs the release trigger. `.github/workflows/release.yml` fires on
`v[0-9]+.[0-9]+.[0-9]+`, but this project tags without a `v` prefix and none of
its 230 published tags match that pattern. The workflow's gates and artifact
jobs were aligned after the last tag was pushed, so the trigger has never been
exercised. Until corrected, tagging a release runs no gates, builds no
artifacts, and publishes nothing.

R0 releases `0.165.0`. It closes no audit blocker.

## 1a. Precondition: the mainline is unpushed

`main` is **26 commits ahead of `origin/main`**. The delta is not only
unreleased, it is unpushed. CI triggers on push to `main` and on pull requests,
so no CI run has executed against any of this work: not the format gate, not
the tests, not `audit-deps`. Every claim that "CI enforces" these gates
describes workflow configuration, not an observed run.

R0 therefore begins with a standalone push of the current `main`, before any
R0 edit is made:

1. the project owner approves publishing the existing 26 commits — this is an
   outward-facing action and is not the implementer's discretion;
2. push `main` to `origin`;
3. observe the resulting CI run and record its result.

Do this **first and alone**. Pushing 26 commits fires CI across the whole
backlog at once; if it is combined with R0's edits, any environment-specific
failure arrives entangled with them and attribution costs a debugging cycle. If
that first CI run fails, stop and report before starting R0's edits — a red
mainline is a finding about the backlog, not about R0.

## 2. Scope followed

In scope:

- pushing the existing mainline and observing its first CI run;
- workspace version bump to `0.165.0` across every location the version gate
  enforces;
- a `CHANGELOG.md` entry describing the unreleased delta;
- release-workflow trigger correction and an observed run proving it fires;
- three documentation-truth corrections (F1, F2, F3);
- the README three-way merge claim correction (F15);
- a published-tag check in `cargo xtask version-sync` (F5) plus the CI
  configuration that makes it effective (F14);
- release execution and evidence capture;
- post-release version bump that makes the invariant self-sustaining.

Out of scope — do not include any of the following in this workstream:

- any RFC-076 persistence work or RFC-077 mergetool work;
- any change to product behaviour, UI, diff, merge, or save logic;
- refactoring, module splits, or ELOC reduction (F13 is opportunistic and is
  not triggered by R0's edits);
- the nine all-target Clippy test lints (F6 belongs to M4);
- advisory dispositions for the 14 allowed `cargo audit` warnings (F7 belongs
  to M4);
- the systematic feature-claim reachability audit (F16 belongs to M4); R0
  corrects only the single three-way merge claim;
- editing, retagging, or superseding the published `0.164.0` release;
- rewriting historical CHANGELOG entries.

## 3. Files changed

Expected areas:

- `Cargo.toml` — `[workspace.package] version`
- `xtask/Cargo.toml` — package version
- `Cargo.lock` — regenerated, never hand-edited
- `packaging/linux/PKGBUILD` — `pkgver`
- `packaging/windows/AppxManifest.xml` — `Version="X.Y.Z.0"`
- `CHANGELOG.md` — one new release section
- `.github/workflows/release.yml` — tag trigger and ref-name handling
- `.github/workflows/ci.yml` — checkout tag availability
- `xtask/src/main.rs` — `run_version_sync` published-tag check
- `docs/src/maintainers/release.md` — documented tag form and step ordering
- `docs/src/maintainers/testing.md` — test-count table
- `docs/src/maintainers/architecture.md` — UI module table
- `docs/src/maintainers/threat-model.md` — settings persistence section,
  document version line, and audit-history attribution
- `README.md` — three-way merge feature line
- this handoff — completed with observed evidence

`xtask/src/main.rs` is the largest file in the repository at roughly 470 ELOC,
below the 500 hard threshold. R0's addition is small; do not take this as an
occasion to split it. If the addition would cross 500 ELOC, stop and report
rather than splitting opportunistically inside a release-baseline change.

## 4. Design decisions and assumptions

These are settled. Do not re-derive them during implementation.

### Version target

- The release is `0.165.0`. A minor bump, not a patch: the delta changes the
  declared MSRV and the XLSX security posture, which a patch level would
  understate.

### Authoritative version locations

`cargo xtask version-sync` is the authority. It enforces exactly:

| Location | Form |
|---|---|
| `Cargo.toml` `[workspace.package]` | `version = "0.165.0"` — source of truth |
| `xtask/Cargo.toml` | `version = "0.165.0"` |
| `packaging/linux/PKGBUILD` | `pkgver=0.165.0` |
| `packaging/windows/AppxManifest.xml` | `Version="0.165.0.0"` |
| `CHANGELOG.md` | a `## [0.165.0]` section must exist |
| `Cargo.lock` | `forskscope-core`, `forskscope-ui`, `forskscope-ui-logic` at `0.165.0` |

Two traps:

- `xtask` is not a workspace member (DEC-005), so it carries a literal version
  that does not inherit. It is the location most easily missed.
- The three crates use `version.workspace = true` and inherit automatically —
  do **not** add per-crate `version` fields. The `[workspace.dependencies]`
  entries `forskscope-core = { version = "0", ... }` are semver *ranges*, not
  pins — do **not** change them.

The v0.164.0 handoff decision log (DEC-014, "three version bumps: workspace
package plus two path deps") does not describe the current layout. The table
above supersedes it for this work.

Because `version-sync` requires the CHANGELOG section, the version bump and the
CHANGELOG entry must land in the same commit or the gate fails.

### Tag convention

Four sources currently disagree:

| Source | Says |
|---|---|
| Project rules | `X.Y.Z`, no `v` prefix |
| All 230 published tags | no `v` prefix |
| `.github/workflows/release.yml` trigger | `v`-prefixed |
| `docs/src/maintainers/release.md` | `v`-prefixed (`git tag -a v${VER}`) |

The rule and the 230 published tags are authoritative. The unprefixed form
stands; the two `v`-prefixed sources are corrected to match it.

Correcting only the workflow is insufficient. `docs/src/maintainers/release.md`
documents the release procedure, so leaving it uncorrected would instruct the
next releaser to tag `v0.166.0` and reproduce exactly the defect R0 exists to
remove. Both must change together:

- `release.md` step "Tag the commit" drops the `v` from `git tag -a v${VER}`;
- `release.md` prerelease checklist drops `#v` from the documented
  `version-sync` invocation;
- `release.yml` trigger becomes `'[0-9]+.[0-9]+.[0-9]+'`;
- `${GITHUB_REF_NAME#v}` becomes `${GITHUB_REF_NAME}` in both workflow uses, so
  the code stops implying a prefix that is never produced.

While in `release.md`, also fix its step ordering: it currently lists updating
`PKGBUILD` `pkgver` as a step *after* tagging, but `cargo xtask version-sync`
requires `pkgver` to already match before the release gates pass.

The corrected workflow must exist **at the tagged commit**. GitHub resolves
workflow files for tag pushes from the tagged ref, so tagging a commit that
predates the fix still produces no run.

### Published-tag check semantics

The check must enforce "the first commit after a tag bumps the version" without
breaking the release commit itself or CI on `main`.

- **Applies only in no-argument mode** (`cargo xtask version-sync`). In release
  mode (`cargo xtask version-sync <version>`) the tag being released
  necessarily exists, so the check must not run.
- **Failure condition:** the workspace version equals an existing tag **and**
  `HEAD` is not that tag's commit. When `HEAD` *is* the tagged commit, the tree
  is the released state and must pass.
- **Failure message** names the colliding tag and states that the workspace
  version must be bumped.
- **Tag enumeration** uses the local repository's tags.
- **When no tags are available** — a shallow clone, or `git` unavailable — the
  check prints an explicit `SKIPPED` notice naming the reason and exits zero. It
  must never report success as though it had verified something.

Because the skip path exists, CI must be configured so it cannot be taken
silently: `.github/workflows/ci.yml` currently uses `actions/checkout@v7` with
no `fetch-depth` or `fetch-tags`, which defaults to depth 1 and fetches no
tags. Without a configuration change the new check would no-op on every CI run.
Set the checkout to fetch tags, and confirm from an observed CI run that the
check reports a real result rather than `SKIPPED`.

### CHANGELOG

- One new `## [0.165.0]` section above `## [0.164.0]`. The `0.164.0` entry is
  published and must not be edited (M-006).
- Keep a Changelog format, matching the existing file's structure.
- Describe user-visible and operator-visible impact, not commit subjects. The
  26 commits group naturally as: security posture (XLSX fail-closed, MSRV
  1.91, Dioxus dependency path), correctness (async compare load-token guard),
  packaging (archive layout contract, Windows archive root, macOS DMG, helper
  artifact names), CI and release gates (gate alignment, pinned `cargo-audit`,
  artifact action alignment), release automation (trigger correction), and
  documentation.
- The security and correctness entries carry the most user weight. Do not bury
  the MSRV change: it is a build-environment requirement change for anyone
  compiling from source.

The commit range is `0.164.0..HEAD`.

### Documentation corrections

These are corrections to observed truth, not rewrites. Verify each value
yourself; do not copy figures from this handoff.

- `docs/src/maintainers/testing.md` — the counts table reports ui-logic unit
  228 and total 930. Replace with values from an observed
  `cargo test -p forskscope-core -p forskscope-ui-logic` run, and update the
  section's version label.
- `docs/src/maintainers/architecture.md` — the UI module table documents a
  "Shim re-exports" layer of fourteen modules. RFC-073 deleted `bridge/`; no
  such files exist. Remove the row.
- `docs/src/maintainers/threat-model.md` §4 "Settings persistence" — states
  "Residual concerns: none" while audit finding B2 is open. Record the actual
  current behaviour: the running UI serializes its own settings and session
  structs through `app_json_settings::ConfigManager` rather than the core
  versioned envelope, and a corrupt or future-schema file silently resets to
  defaults. Reference B2 and RFC-076 as the tracked remediation. State the gap
  accurately — neither soften it nor overstate it as an exploitable
  vulnerability.
- `docs/src/maintainers/threat-model.md` attribution — the document opens
  "records the security posture of ForskScope at v0.164.0", and its audit
  history credits **v0.164.0** with four changes that landed after the
  `0.164.0` tag: the XLSX parser removal, the Dioxus dependency policy, the
  `wry` release-build compatibility change, and the release/CI gate alignment.
  All four are 0.165.0 content. As written, the security document tells a
  reader that a published release contains mitigations it does not contain.
  Re-attribute those four rows to `0.165.0` and update the document version
  line. This is the same defect class as the version drift and is the reason
  it belongs in R0 rather than M4.
- `README.md` — the three-way merge line says the conflict workspace UI is "in
  progress". It is a deferred post-v1 slice and an explicit RFC-074 non-goal.
  The wording must agree with `ROADMAP.md` Slice 6 and RFC-074's non-goals.

### Post-release bump

After `0.165.0` is tagged, the next commit on the mainline bumps the workspace
version to `0.166.0` and opens a `## [0.166.0]` CHANGELOG section that M2
accumulates entries into as work lands.

This is what makes the invariant self-sustaining, and it is the systemic fix
for the condition that produced R0: a CHANGELOG written continuously cannot
drift into a 26-commit reconstruction.

## 5. Tests and gates run

No implementation commands have been run for this handoff. Required observed
evidence:

```sh
cargo fmt --check
cargo xtask css --check
cargo xtask version-sync
cargo xtask i18n
cargo xtask audit-deps
cargo audit
cargo test -p forskscope-core -p forskscope-ui-logic
cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings
cargo test --workspace
cargo clippy --workspace -- -D warnings
git diff --check
```

R0 changes no Rust product code, so the test suites are regression evidence
that the version and metadata edits broke nothing. Record observed counts
rather than asserting they are unchanged.

`git diff --check` does not inspect untracked files. If R0 adds any new file,
scan it for trailing whitespace separately rather than relying on that command
alone.

Record the CI run from §1a and the CI run for the R0 change itself as separate
evidence. They answer different questions: whether the existing backlog is
green, and whether R0 kept it green.

The published-tag check needs both directions demonstrated, since `xtask` has
no test harness and manual transcripts are its only proof:

- **negative case** — with the workspace version set to a value equal to an
  existing tag on a commit that is not that tag, `cargo xtask version-sync`
  exits non-zero and names the colliding tag;
- **positive case** — with the version bumped, it exits zero;
- **skip case** — in a tag-less checkout it prints `SKIPPED` with a reason and
  exits zero;
- **release mode** — `cargo xtask version-sync 0.165.0` does not apply the
  check.

The stronger advisory `cargo clippy --workspace --all-targets -- -D warnings`
still reports nine pre-existing test-target lints. R0 must not increase that
count and must not attempt to fix it.

## 6. Generated artifacts

The release produces `forskscope-v0.165.0.tar.gz` with files at the archive
root, no intermediate parent directory, and `target/`, `xtask/target/`, `.git/`
and `.git-exclude/` excluded. `cargo xtask archive-layout` verifies this.

Record for the release evidence: workflow run identifier, every artifact name,
and a SHA-256 digest per artifact. The digests are what RFC-078 will later bind
its platform evidence to, so capture them even though R0 performs no runtime
acceptance.

## 7. Known limitations

- R0 closes no audit blocker. B2, B3, and B4 remain open and the v1/public
  release stays **No-Go**.
- The published-tag check depends on tags being fetched. It fails safe by
  announcing a skip, but a misconfigured future workflow could still reduce it
  to a no-op; the skip notice is the only defence.
- R0 proves the release workflow fires and its gates run. It does not
  constitute runtime acceptance of any artifact on any platform — building and
  publishing are not RFC-078 evidence.
- The threat-model correction records the B2 gap; it does not close it.
- Windows and macOS artifacts are produced by the workflow but are not
  installed or executed in R0.
- The first CI run covers 26 commits at once, so it cannot isolate which commit
  introduced any failure it reports. Bisection is a follow-up if needed.
- `forskscope-core`, `forskscope-ui-logic`, and `forskscope-ui` carry no
  `publish = false`, so they are nominally publishable to a registry while no
  documented process publishes them. R0 does not change this; record it for the
  M4 documentation reconciliation.

## 8. Recommended next step

Start with §1a: obtain owner approval to publish the existing 26 commits, push
`main`, and observe the first CI run. Treat a failure there as a backlog
finding and report it before touching R0's edits.

Then prepare the change and request review before tagging. Tagging is a release
action: the project owner approves every release, and the tag is pushed only
after that approval.

If the corrected workflow still does not fire on the pushed tag, **stop and
report**. Do not fall back to `packaging/build-release.sh` and present a
locally built archive as release-workflow evidence — the reason R0 exists is
that inspected automation was credited as working automation across three
separate documents.

After the release is evidenced and the post-release bump has landed, begin M2
(RFC-076) as the next single-developer workstream. Keep its persistence
adapters from installing legacy persisted IDs as runtime `CompareTabId` values.

## 9. Acceptance criteria

- The existing mainline is pushed and its first CI run is observed and
  recorded, before any R0 edit.
- `cargo xtask version-sync` passes at `0.165.0` across all six enforced
  locations.
- `CHANGELOG.md` carries a `## [0.165.0]` section describing the delta; the
  `0.164.0` entry is byte-identical to its published form.
- The release workflow fires on the unprefixed `0.165.0` tag, and its preflight
  gates and artifact jobs are observed to run.
- The published-tag check fails on a version equal to a published tag from a
  non-tag commit, passes on the tagged commit itself, does not run in release
  mode, and reports an explicit skip when tags are unavailable.
- CI is configured so the check reports a real result, confirmed from a run.
- `testing.md` counts match an observed test run.
- `architecture.md` no longer documents the removed shim layer.
- `threat-model.md` records the settings-persistence gap and references B2 and
  RFC-076, and no longer attributes post-tag security work to `0.164.0`.
- No `v`-prefixed tag instruction remains anywhere in the repository; a
  repository-wide search for the prefixed form returns only historical
  CHANGELOG or RFC prose, never an instruction.
- `release.md` lists `pkgver` synchronization before tagging, not after.
- `README.md` describes the conflict workspace UI as deferred post-v1,
  consistent with `ROADMAP.md` and RFC-074.
- All gates in §5 pass with recorded output.
- The source archive satisfies the layout contract with recorded digests.
- The post-release bump to `0.166.0` has landed with an open CHANGELOG section.

## 10. Prohibited shortcuts

- Hand-editing `Cargo.lock` instead of regenerating it.
- Editing, retagging, moving, or replacing the published `0.164.0` release.
- Adding a `v` prefix to the tag convention to match the workflow.
- Weakening the published-tag check so it passes vacuously, or letting the skip
  path stand in for a real result in CI.
- Bundling RFC-076 or RFC-077 work, refactoring, or unrelated lint fixes.
- Presenting a locally built archive as evidence that the release workflow ran.
- Reporting gate results that were not observed in this workstream.

## 11. Compatibility and security constraints

Compatibility:

- `0.164.0` is published and immutable; recovery from any release defect is a
  forward patch, never a retag.
- MSRV `1.91` and edition 2024 are unchanged by R0.
- The archive naming and layout contract is unchanged.
- No persisted schema changes, so no migration or downgrade concern arises.

Security:

- No dependency is added, removed, or version-changed.
- `cargo audit` and `cargo xtask audit-deps` must still pass with the reviewed
  `.cargo/audit.toml` exceptions intact.
- No real user paths, host names, or secrets may appear in documentation,
  CHANGELOG text, or recorded evidence.

## 12. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. addressed items (R0 tasks, F1–F5, F14, F15);
3. changed files;
4. important implementation decisions, especially the published-tag check's
   HEAD-versus-tag comparison and the CI tag-fetch configuration;
5. any difference from this handoff or from RFC-074;
6. executed gates with observed output, including all four published-tag check
   cases;
7. release workflow run identifier, artifact names, and SHA-256 digests;
8. unresolved issues and known limitations;
9. requested review focus.
