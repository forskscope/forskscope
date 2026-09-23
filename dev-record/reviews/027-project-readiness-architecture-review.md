# ForskScope project-readiness architecture review

**Review date:** 2026-07-15  
**Repository baseline:** `a3b31849cf9e1c1ad9aefd818330e868efc61360` (`docs: refresh roadmap i18n count`, 2026-07-10)  
**Review scope:** v0.164.0 handoff archive, older v0.162.0 specifications, project rules, repository documentation/RFCs, Rust workspace, tests, CI, packaging, and observed local gates.  
**Review mode:** Independent audit; no source implementation changes.

## 1. Verdict

**Needs changes.**

**Recommendation:**

- **Go** for continued development and architecture work. The three-crate boundary is useful, the core/view-model test surface is substantial, the repository is cleanly organized, and the current documented build gates are reproducible.
- **No-Go** for a v1.0/public-release claim. In addition to the already acknowledged GTK/WebKitGTK and cross-platform runtime evidence gaps, three implementation issues can affect correctness of user data or advertised workflows: stale asynchronous compare results can be written to the wrong load, the shipping UI persistence path bypasses the specified versioned schemas, and Git mergetool redirects the save path without preserving a matching target fingerprint.

## 2. Risk rating by required dimension

| Dimension | Rating | Evidence-based assessment |
|---|---|---|
| Functional | **High** | The async tab identity race can display or save the wrong comparison result; the advertised three-argument Git mergetool flow can compare the save target against the remote file's fingerprint; active settings/session persistence does not implement the specified compatibility contract. |
| Business | **Medium** | The core product proposition is coherent and differentiated, but calling the workflow “feature-complete” before these correctness issues and platform acceptance are resolved risks user trust in the product's primary promise: safe local merge. Scope remains controlled and the existing test investment lowers remediation cost. |
| Operational | **High** | Linux build/test automation is healthy, but GTK interaction, WebKitGTK rendering, clean-install packaging, and Windows/macOS behavior are not accepted with current runtime evidence. Save durability claims also exceed what the implementation proves. |
| Security | **Medium** | No application-authored external network workflow or shell interpolation was found, and dependency-path enforcement passes. Residual risk remains in stale task write-back, weak same-size/same-mtime external-change detection, and 14 allowed `cargo audit` warnings including two unsoundness advisories requiring explicit disposition. |

## 3. Blocking findings

### B1 — Background load completion is identified only by mutable vector index and `Loading` state

`open_compare` captures `idx = tabs.len()` and later writes to `tabs.get_mut(idx)` if that slot is still `Loading` (`crates/forskscope-ui/src/state/compare.rs:100-130`). `reload_tab` uses the same index/state-only guard (`compare.rs:28-61`). Neither path carries an immutable tab ID, load generation, or cancellation token.

This does not establish the guarantee claimed in `docs/src/maintainers/threat-model.md:40-42`. Two concrete races remain:

1. If a lower-index tab is closed while multiple tabs are loading, vector compaction can move another loading tab into the captured index. The old completion can then populate that different tab.
2. If a tab is reloaded before its prior load completes, both operations target the same index and both observe `Loading`; the older completion can win and overwrite the newer request.

Impact includes incorrect displayed paths/content and the possibility of saving a result derived from the wrong input pair. No UI/state test exercises close-during-load or reload-generation ordering.

**Required before release:** add stable tab identity plus a monotonically changing load generation (or cancellation ownership), validate both on write-back, and add deterministic race tests outside the GTK component layer.

### B2 — Shipping settings/session persistence bypasses the required versioned core schemas

The handoff requires settings and session JSON to use `VersionedEnvelope`, forward migration, and rejection of unknown future schemas (`requirements.md:42-45` inside `.git-exclude/specs/forskscope-v0.164.0-260630-handoffs-files.tar.gz`; also the older authoritative requirement at `.git-exclude/specs/forskscope-v0.162.0-01-requirements.md:112-123`). The core crate implements versioned `UserSettings` and `WorkspaceSession`, but the running UI uses separate types and plain `app_json_settings::ConfigManager`:

- UI settings: `crates/forskscope-ui/src/ui/view/settings.rs:22-29`
- Duplicate UI `AppSettings`: `crates/forskscope-ui/src/state/settings.rs:138-204`
- UI session: `crates/forskscope-ui/src/state/session.rs:11-45`

The actual load calls use `load_or_default().unwrap_or_default()`. A corrupt or future-incompatible file silently resets to defaults rather than producing the required user-visible incompatibility error. This also contradicts the core-ownership rule in `docs/src/maintainers/architecture.md:13-17,92-98`: the core models are tested, but they are not the models that own runtime persistence.

**Required before release:** select one canonical persisted model, migrate the UI to the core serialization contract (including legacy plain-JSON import), surface future-schema errors, and test the exact runtime load/save path rather than only the unused core model.

### B3 — Git mergetool save target and fingerprint are not updated atomically with load completion

The CLI advertises `forskscope <local> <remote> <merged>` (`crates/forskscope-ui/src/main.rs:5-9,42-45`). `App` starts an asynchronous local-vs-remote comparison, then immediately changes `right_path` to `<merged>` and clears the fingerprint (`crates/forskscope-ui/src/app.rs:30-41`). When the background load completes, it replaces `right_doc` with the document loaded from `<remote>` (`crates/forskscope-ui/src/state/compare.rs:123-130`). Normal save then uses `<merged>` as the target but `right_doc.fingerprint_at_load`—now belonging to `<remote>`—as the expected target fingerprint (`crates/forskscope-ui/src/ui/view/diff_actions.rs:101-123`).

For the usual case where `<merged>` already exists, the first save can raise a false external-modification conflict; worse, the model has no stable load-time fingerprint for the actual merge target. This is inconsistent with the README's Git mergetool compatibility claim and G-003/S-006 safe-save expectations.

**Required before release:** model the merge output path and its load-time fingerprint as distinct fields from the compared right/remote document, populate them in the background result or a dedicated startup request, and add integration tests for existing, missing, and externally modified `<merged>` files.

### B4 — Required runtime/platform acceptance evidence is still absent

The handoff itself identifies GTK smoke tests, WebKitGTK visual confirmation, and end-to-end platform packaging as release blockers (`project-summary.md:60-78` inside `.git-exclude/specs/forskscope-v0.164.0-260630-handoffs-files.tar.gz`). Current CI builds all three platform artifacts, which is useful progress, but build/package success is not runtime acceptance. No observed evidence in this review establishes:

- the documented GTK interaction checklist on a real display;
- WebKitGTK table-row layout and scroll synchronization;
- clean-install execution of Linux, macOS, and Windows artifacts;
- Windows overwrite/backup/external-change behavior;
- WebView2 prerequisite behavior or macOS Gatekeeper behavior.

**Required before release:** execute and retain the platform/runtime matrix with artifact hashes, host versions, checklist results, and failures/waivers.

## 4. Non-blocking findings

### N1 — External-change detection stores a digest but never uses it at save time

`FileFingerprint` documents and captures an optional digest (`crates/forskscope-core/src/document.rs:35-60`), but `save_text` captures metadata only and compares only length and modification time (`crates/forskscope-core/src/save.rs:56-66`). `check_external_state` likewise treats matching length/mtime as unchanged. The test explicitly permits a same-size external edit to be reported clean on a coarse-mtime filesystem.

This meets the narrow handoff wording of “mtime + size,” but it is weaker than the code documentation and the trust posture imply, especially on coarse timestamps or network/virtual filesystems. Either hash current bytes when metadata is inconclusive or document the limitation prominently.

### N2 — “Atomic/power-loss safe” wording is stronger than the implementation

The save path writes a deterministic sidecar with `fs::write` and renames it (`save.rs:86-93`) but does not `sync_all` the file or parent directory and does not preserve the original file's permissions/extended metadata. Rename prevents readers from seeing a partially written target during ordinary process failure; it does not by itself prove durability across power loss. Narrow the threat-model claim or implement and test the intended durability/metadata contract per platform.

### N3 — Handoff and durable design documents have material drift

Examples:

- Handoff MSRV is 1.85 while the repository declares 1.91 (`Cargo.toml:9-14`).
- Handoff S-001 forbids any WebSocket while the current reviewed design accepts Dioxus loopback WebSocket IPC (`docs/src/maintainers/threat-model.md:9-22,186-210`).
- Handoff reports 937 tests; current headless evidence is 930 and full workspace is 938.
- Handoff says XLSX comparison is implemented; current behavior intentionally fails closed.
- `docs/src/maintainers/architecture.md:90` describes shim re-exports that do not exist; current UI imports `forskscope-ui-logic` directly.

The repository is correctly treated as authoritative, but the archive cannot safely onboard a new implementer without an explicit supersession note or refreshed bundle.

### N4 — RFC-058's lifecycle/status no longer describes the shipping behavior

`rfcs/done/058-spreadsheet-xlsx-structural-diff.md:3-23,284-320` says the sheets-diff adapter and derived-text XLSX comparison are implemented and shipped. Current architecture and threat model say XLSX parsing is removed and comparison fails closed. Preserve the historical RFC, but amend its status with the security suspension and link to the decision that disabled it; otherwise the RFC folder/status is misleading under RFC-000.

### N5 — Security audit passes by policy but has unresolved warning debt

Observed `cargo audit` exited successfully with 14 allowed warnings: 12 unmaintained crates and two unsoundness advisories. Notably:

- `RUSTSEC-2024-0429` reaches the UI runtime through GTK/glib.
- `RUSTSEC-2026-0097` reaches the build graph through `rand 0.7.3 -> phf_generator -> selectors -> kuchikiki -> wry`.

`.cargo/audit.toml` documents only the two quick-xml exceptions (`.cargo/audit.toml:1-10`). These warnings may be acceptable given upstream Dioxus/GTK constraints, but each unsoundness advisory needs reachability analysis, owner, review date, and upgrade trigger. “cargo audit passes” must not be interpreted as “no advisory findings.”

### N6 — Clippy coverage and maintainability gates are narrower than the headline

The documented `cargo clippy --workspace -- -D warnings` passes. A stronger `cargo clippy --workspace --all-targets -- -D warnings` fails on nine test-target lints. This is not a violation of the currently documented command, but it means test code is outside the zero-warning claim.

Approximate nonblank/non-comment counts also put 13 Rust files above the 300-ELOC soft threshold; `xtask/src/main.rs` is approximately 470 ELOC, below the 500 hard threshold. No `mod.rs` or Rust `unsafe` blocks were found. Plan incremental splits when these files next change.

### N7 — VCS tests assume the OS temp directory is outside any repository

The two “outside repo” tests fail if `TMPDIR` is placed below the checkout because `find_git_root` deliberately walks parents. This appeared during sandboxed MSRV verification and passed when rerun with real `/tmp`. Use `tempfile` plus a discovery boundary/test seam if hermetic execution inside nested workspaces is a supported developer scenario.

## 5. Positive evidence

- The workspace separation is real: `forskscope-core` and `forskscope-ui-logic` remain GTK-free, while Dioxus stays in the UI crate.
- No `mod.rs`, Rust `unsafe`, application HTTP client, shell-based path execution, privilege escalation call, or secret material was found in the reviewed source.
- Diff, merge, directory, save, encoding, persistence-model, patch, VCS, accessibility-view-model, and security-boundary tests are broad and fast.
- Generated CSS, version metadata, i18n coverage, dependency-path assertions, source archive logic, and cross-platform build workflows are automated.
- Current documentation accurately acknowledges the loopback IPC exception and the fail-closed XLSX decision, even though the handoff/RFC history has not been fully reconciled.

## 6. Observed gates in this review

| Command | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo xtask css --check` | Pass |
| `cargo xtask version-sync` | Pass for v0.164.0 |
| `cargo xtask i18n` | Pass; 203 UI keys covered |
| `cargo xtask audit-deps` | Pass; reviewed dependency paths enforced |
| `cargo test -p forskscope-core -p forskscope-ui-logic` | Pass; 930 tests including doctests |
| `cargo test --workspace` | Pass; 938 executed tests including doctests, plus one ignored UI doctest |
| `cargo +1.91 test -p forskscope-core -p forskscope-ui-logic` | Pass; observed on declared MSRV using system temp isolation |
| `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Fail; nine test-target lints (non-mandatory stronger check) |
| `cargo audit` | Exit 0 with 14 allowed warnings; not a clean advisory set |

## 7. Missing evidence

- Deterministic async close/reload race tests.
- Runtime tests of the exact UI settings/session serialization path and migration from existing plain JSON.
- Git mergetool end-to-end tests with an existing and externally changing merge target.
- GTK/WebKitGTK smoke and visual evidence.
- Windows/macOS runtime and clean-install artifact evidence.
- Platform-specific save semantics, metadata preservation, and crash/power-loss testing.
- Explicit disposition of current unsoundness advisories.
- A refreshed handoff bundle aligned to the repository's MSRV, dependency policy, test counts, XLSX posture, and current module map.

## 8. Recommended next action

Create one narrowly scoped stabilization RFC (or amend RFC-041) that treats B1-B3 as release-blocking correctness work, with tests first. After those land, rerun the complete gates and execute B4's platform matrix. Only then refresh the handoff and reconsider the v1.0 Go/No-Go decision.
