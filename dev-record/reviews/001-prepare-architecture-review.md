# ForskScope Architecture Readiness Review

**Task:** `.git-exclude/tasks/architect/001-prepare.md`  
**Date:** 2026-07-09  
**Reviewer stance:** independent architecture/security/process review  
**Scope reviewed:** v0.164.0 handoff archive, v0.162.0 specs, project rules, repository source/docs/tests/packaging excluding `.git-exclude/` and `target/`.

## Verdict

**Recommendation: No-Go for v1.0 release. Accept with blocking changes for continued development.**

The core architecture is directionally sound: the three-crate split keeps domain logic testable without GTK, the RFC lifecycle is mostly followed, the merge/save/domain logic has strong headless coverage, and current `cargo clippy` plus `cargo test` pass for the mandatory logic crates. However, this repository is not release-ready because current evidence contradicts several release/security/process claims:

- `cargo fmt --check` fails across core and ui-logic.
- `cargo audit` finds 4 active vulnerabilities, including high-severity `quick-xml` advisories in the core XLSX path.
- The dependency graph includes network-capable WebSocket/TLS crates while the threat model and S-001 claim no async HTTP/network-capable dependency tree.
- Release source archives are built with an intermediate top-level directory, directly conflicting with the project rule and v0.164.0 compatibility constraint.
- Documentation and workflow gates are stale or inconsistent with the current repository state.

## Risk Ratings

| Dimension | Rating | Rationale |
|---|---:|---|
| Functional | Medium | Core diff/merge/save/explorer logic is well decomposed and tests passed, but UI/runtime validation remains manual and not observed. Several RFCs in `done/` are core-only with UI deferred, which is acceptable only when public feature claims stay narrow. |
| Business | Medium | The product positioning is coherent, but release credibility is weakened by stale roadmap/test counts, conflicting archive contracts, and unresolved packaging verification. |
| Operational | High | Required gates are not encoded consistently in CI/release workflows; `cargo fmt --check` currently fails; GTK smoke, WebKitGTK visual verification, and multi-platform packaging are not evidenced. |
| Security | High | `cargo audit` currently fails; `quick-xml` vulnerabilities affect the core XLSX path through `sheets-diff -> calamine`; the threat model's "no network dependency tree" control is false for the current UI graph. |

## Blocking Findings

### 1. Current mandatory formatting gate fails

**Evidence:** `cargo fmt --check` exited `1` and printed widespread formatting diffs, including `crates/forskscope-core/src/cancel.rs`, `crates/forskscope-core/src/command.rs`, `crates/forskscope-core/src/command/registry.rs`, and multiple `forskscope-ui-logic` files.

This violates the project rule that release work runs `cargo fmt`, and it contradicts v0.164.0's release-ready posture. The handoff already warned that v0.164.0 was packaged before a fmt normalization; the working tree still has that problem.

**Required action:** Run `cargo fmt`, review the resulting mechanical changes, then re-run all release gates.

### 2. Security audit currently fails with active vulnerabilities

**Observed command:** `cargo audit` exited `1`.

Findings:

- `quick-xml 0.39.4`: `RUSTSEC-2026-0194`, high, duplicate-attribute quadratic runtime; fix `>=0.41.0`.
- `quick-xml 0.39.4`: `RUSTSEC-2026-0195`, high, namespace allocation memory exhaustion; fix `>=0.41.0`.
- `time 0.3.45`: `RUSTSEC-2026-0009`, medium, stack exhaustion DoS; fix `>=0.3.47`.
- `crossbeam-epoch 0.9.18`: `RUSTSEC-2026-0204`; fix `>=0.9.20`.

`quick-xml` is not only a UI-only risk. `cargo tree -p forskscope-core -i quick-xml` shows:

```text
quick-xml v0.39.4
└── calamine v0.35.0
    └── sheets-diff v2.2.3
        └── forskscope-core v0.164.0
```

That means malformed XLSX inputs may traverse a vulnerable XML parser in core. This is directly relevant to the local-file threat model.

**Required action:** Upgrade or patch transitive dependencies, or disable/defer XLSX support until the vulnerable path is resolved. Add `cargo audit` or an explicitly reviewed advisory policy to the release gate.

### 3. Threat model and S-001 dependency claim are false for the current UI graph

The threat model says ForskScope has no network code and that no async HTTP crate exists in the dependency tree, "Verified by `cargo tree`" ([docs/src/maintainers/threat-model.md:11](../../docs/src/maintainers/threat-model.md#L11), [docs/src/maintainers/threat-model.md:145](../../docs/src/maintainers/threat-model.md#L145)). The workspace enables Dioxus desktop with default features ([Cargo.toml:22](../../Cargo.toml#L22)). Current dependency tracing shows:

```text
tungstenite v0.28.0
├── dioxus-desktop v0.7.9
│   └── dioxus v0.7.9
└── dioxus-devtools v0.7.9
```

and:

```text
native-tls v0.2.18
└── tungstenite v0.28.0
```

This does not prove telemetry or outbound network behavior, but it invalidates the documented control "no network-capable dependency tree." S-001 requires security review for dependencies with network capability.

**Required action:** Either remove/disable the devtools/WebSocket/TLS path for release builds, or update the threat model and security review to explicitly accept and constrain the Dioxus desktop transport surface.

### 4. Release archive layout violates the project rule

The project rule and v0.164.0 handoff require release tarballs to contain files at archive root with no intermediate parent directory. The local release script instead transforms paths into `forskscope-v$VER/` ([packaging/build-release.sh:35](../../packaging/build-release.sh#L35), [packaging/build-release.sh:39](../../packaging/build-release.sh#L39)). The GitHub release workflow does the same ([.github/workflows/release.yml:30](../../.github/workflows/release.yml#L30), [.github/workflows/release.yml:32](../../.github/workflows/release.yml#L32)). The PKGBUILD expects `cd "$pkgname-$pkgver"` after extraction, reinforcing the opposite convention.

This is not cosmetic. It means release automation and packaging docs currently implement a different artifact contract than the governing rule.

**Required action:** Decide the artifact contract once, update the rule/spec or the packaging scripts, and add an archive-layout check to release gates.

### 5. CI/release gates do not match documented mandatory gates

Documented mandatory gates include `cargo fmt --check`, clippy for core/ui-logic, tests for core/ui-logic, and `cargo xtask css --check`. The current CI workflow runs `cargo test -p forskscope-core`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `cargo build -p forskscope-ui`, but does not run `cargo fmt --check` or `cargo xtask css --check` ([.github/workflows/ci.yml:42](../../.github/workflows/ci.yml#L42), [.github/workflows/ci.yml:45](../../.github/workflows/ci.yml#L45), [.github/workflows/ci.yml:48](../../.github/workflows/ci.yml#L48), [.github/workflows/ci.yml:51](../../.github/workflows/ci.yml#L51)).

The release workflow builds and packages artifacts but does not run fmt, tests, clippy, CSS staleness, audit, or archive-layout verification before upload ([.github/workflows/release.yml:64](../../.github/workflows/release.yml#L64), [.github/workflows/release.yml:95](../../.github/workflows/release.yml#L95), [.github/workflows/release.yml:125](../../.github/workflows/release.yml#L125)).

**Required action:** Align CI/release workflows with the documented release gate before using them as release evidence.

## Non-Blocking Findings

### A. Public documentation is stale and over-claims some deferred work

`ROADMAP.md` still says last updated v0.140.0 ([ROADMAP.md:3](../../ROADMAP.md#L3)), cites 39 of 48 RFCs and 936 tests ([ROADMAP.md:10](../../ROADMAP.md#L10)), and says docs/UI are complete ([ROADMAP.md:14](../../ROADMAP.md#L14)). The v0.164.0 handoff and current tree have moved past this. `docs/src/maintainers/testing.md` also labels counts as v0.135.0 and reports 5 CSS coverage tests while the observed run executed 6 CSS integration tests ([docs/src/maintainers/testing.md:23](../../docs/src/maintainers/testing.md#L23), [docs/src/maintainers/testing.md:32](../../docs/src/maintainers/testing.md#L32)).

`README.md` claims "GitHub Actions CI/CD — Linux x86_64, macOS aarch64, Windows x64 release builds on tag push" ([README.md:84](../../README.md#L84)). A workflow exists, but it does not run release gates before artifact publication, so the claim is operationally incomplete.

### B. RFC lifecycle is mostly followed, but "done" does not always mean user-reachable

The repository preserves RFCs under `rfcs/done/`, `rfcs/proposed/`, and `rfcs/archive/`, matching the lifecycle policy. Many `done` RFC statuses explicitly state "core complete; UI deferred", which is allowed by the policy if deferred work is recorded. The risk is communication: feature claims must distinguish core model completeness from user-reachable UI.

Examples include three-way merge/conflict UI and command palette. `README.md` handles three-way merge reasonably by saying core shipped and UI in progress ([README.md:70](../../README.md#L70)), but the broader release/readiness docs need the same discipline.

### C. GTK/WebKitGTK acceptance remains unobserved

I observed `cargo check -p forskscope-ui` passing locally, but I did not run the GUI or GTK smoke checklist. The handoff identifies GTK smoke, WebKitGTK visual confirmation, and multi-platform packaging verification as release blockers. Those remain missing evidence.

## Positive Evidence

Observed in this review:

- `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` passed.
- `cargo test -p forskscope-core -p forskscope-ui-logic` passed.
- `cargo xtask css --check` passed and reported `assets/main.css is up to date`.
- `cargo check -p forskscope-ui` passed in this environment.
- `find . -name mod.rs -not -path './target/*' -not -path './.git-exclude/*' -not -path './.git/*'` found no `mod.rs`.
- Direct source search did not find application `reqwest`, `hyper`, `ureq`, `std::net`, `sh -c`, `sudo`, `setuid`, or raw `unsafe` usage in active source paths; the main network concern is transitive Dioxus desktop/devtools transport capability.

## Missing Evidence

- No GTK smoke test execution evidence in this session.
- No WebKitGTK visual confirmation for the compare-view CSS/table layout in this session.
- No Windows or macOS build/install/smoke evidence in this session.
- No current i18n audit was run in this session.
- No release archive was built and inspected in this session; the finding is based on script/workflow behavior.

## Recommended Next Actions

1. Fix formatting and make `cargo fmt --check` pass.
2. Resolve `cargo audit` vulnerabilities, prioritizing the core `quick-xml` path.
3. Decide and enforce the S-001 interpretation for transitive network-capable dependencies; either remove release network-capable paths or update the threat model with explicit acceptance.
4. Resolve the archive-layout contract conflict between project rules, release scripts, workflow, PKGBUILD, and docs.
5. Align CI and release workflows with mandatory gates: fmt, core/ui-logic clippy, core/ui-logic tests, CSS staleness, audit/advisory policy, version sync, i18n audit, and archive-layout check.
6. Refresh `ROADMAP.md`, testing docs, threat model, and README feature/release claims to match v0.164.0 and current evidence.
7. Run GTK smoke tests and platform packaging verification before any v1.0/public-release Go decision.

