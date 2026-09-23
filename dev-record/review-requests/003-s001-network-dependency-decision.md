# Review Request: S-001 Network-Capable Dependency Decision

**Date:** 2026-07-09
**Reviewer stance:** security/design/implementation review
**Repository state:** inspect the current working tree directly

The reviewer should inspect the project directly. This request intentionally
does not include the whole codebase or full diff.

## Summary

Review the S-001 decision and implementation for Dioxus desktop's
network-capable transitive dependencies.

Previous docs overstated the guarantee by implying that ForskScope had no
network-capable dependency tree. The current implementation removes Dioxus
devtools/logger defaults where practical, then explicitly accepts the remaining
Dioxus desktop loopback WebSocket transport as a constrained framework IPC
dependency.

The accepted residual path is:

```text
native-tls -> tungstenite -> dioxus-desktop -> forskscope-ui
```

This is treated as local WebView transport, not an application feature for
telemetry, cloud upload, remote file access, sync, or remote API calls.

## Scope Followed

- Disabled Dioxus default features at the workspace dependency level.
- Added a direct `dioxus-desktop` dependency with `default-features = false`
  and only `tokio_runtime`.
- Updated the UI entry point to launch through `dioxus_desktop::launch::launch`.
- Removed active Dioxus logger/devtools dependency edges from the release graph.
- Kept the required Dioxus desktop WebView transport dependency.
- Extended `cargo xtask audit-deps` to enforce the reviewed dependency paths.
- Updated maintainer threat-model docs to describe the accepted local WebView
  WebSocket transport.
- Updated user-facing feature wording from absolute "no network access" to
  "no external network service."

## Files Changed

Primary files to inspect:

- `Cargo.toml`
- `Cargo.lock`
- `crates/forskscope-ui/Cargo.toml`
- `crates/forskscope-ui/src/main.rs`
- `xtask/src/main.rs`
- `docs/src/maintainers/threat-model.md`
- `docs/src/users/features.md`

## Design Decisions And Assumptions

- Removing all `tungstenite`/`native-tls` exposure is not practical with
  Dioxus desktop 0.7.9 because `dioxus-desktop` itself uses a loopback WebSocket
  channel for WebView edit/event transport.
- Dioxus devtools are not required for release and should stay inactive.
- The product guarantee should be framed as no telemetry, cloud upload, remote
  resource loading, or app-authored external network workflow.
- The remaining network-capable crates are acceptable only through the reviewed
  Dioxus desktop local WebView transport path.
- `cargo xtask audit-deps` is the enforcement point for this dependency policy.

## Tests And Gates Run

Observed passing after the implementation:

```text
cargo fmt --check
cargo fmt --manifest-path xtask/Cargo.toml --check
cargo check --locked -p forskscope-ui
cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings
cargo test -p forskscope-core -p forskscope-ui-logic
cargo audit
cargo xtask css --check
cargo xtask audit-deps
git diff --check
```

Dependency evidence observed:

```text
cargo tree --prefix depth -i tungstenite
```

showed:

```text
0tungstenite v0.28.0
1dioxus-desktop v0.7.9
2forskscope-ui v0.164.0
```

```text
cargo tree --prefix depth -i native-tls
```

showed:

```text
0native-tls v0.2.18
1tungstenite v0.28.0
2dioxus-desktop v0.7.9
3forskscope-ui v0.164.0
```

```text
cargo tree -i dioxus-devtools
```

reported no active dependency graph output.

`cargo xtask audit-deps` reported:

```text
sheets-diff is absent.
calamine is absent.
dioxus-devtools is inactive.
quick-xml path is limited to wayland-scanner.
network-capable dependency paths are reviewed.
security dependency path check passed.
```

## Generated Artifacts

- `cargo xtask ...` generated `xtask/Cargo.lock`; it was removed from the final
  working tree.
- No generated artifact is intended to be committed for this S-001 change.

## Known Limitations

- `Cargo.lock` may still contain inactive package entries such as
  `dioxus-devtools` because Cargo lockfiles can retain packages from inactive
  target/feature graphs. The enforcement check uses `cargo tree`, not raw
  lockfile name search, to verify active reachability.
- The remaining `tungstenite`/`native-tls` path is accepted, not removed.
- GUI/runtime smoke testing was not run as part of this S-001 implementation.
- This does not complete broader CI/release gate alignment; it only adds the
  dependency policy check to the existing `audit-deps` gate.

## Recommended Next Step

Reviewer should verify:

1. The threat model accurately reflects the product guarantee and the Dioxus
   desktop loopback transport reality.
2. The direct `dioxus-desktop` dependency and launch change are acceptable.
3. `cargo xtask audit-deps` is strict enough for S-001.
4. No user-facing copy still promises an impossible "no network-capable
   dependency tree" guarantee.
5. The residual `tungstenite`/`native-tls` path is acceptable for release.
