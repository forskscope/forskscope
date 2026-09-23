# S-001 network dependency decision review

Reviewed:

- `dev-record/review-requests/003-s001-network-dependency-decision.md`
- `Cargo.toml`
- `Cargo.lock`
- `crates/forskscope-ui/Cargo.toml`
- `crates/forskscope-ui/src/main.rs`
- `xtask/src/main.rs`
- `docs/src/maintainers/threat-model.md`
- `docs/src/users/features.md`
- Local registry source for `dioxus-desktop 0.7.9`

## Verdict

Accept with notes.

The implementation correctly changes the product guarantee from impossible
"no network-capable dependency tree" wording to a narrower and supportable
"no app-authored external network service" guarantee. The remaining
`native-tls -> tungstenite -> dioxus-desktop -> forskscope-ui` path is real,
reviewed, and constrained to Dioxus desktop's local WebView edit/event
transport.

## Blocking findings

None.

## Non-blocking findings

1. The top threat-model summary still contains one stale absolute statement
   around secrets/crypto material.

   `docs/src/maintainers/threat-model.md:24` says there is no authentication
   surface, no session tokens, no cookies, no cryptographic material to protect,
   and no user-account data. The user-account/session/cookie parts remain true,
   but the new accepted Dioxus transport uses ephemeral client/server keys for
   local WebSocket authentication. The later S-001 section documents this
   transport, so this is not a blocker; the top summary should be narrowed to
   "no product/user authentication surface or persisted user secrets" to avoid
   contradicting the accepted framework IPC.

2. `cargo xtask audit-deps` enforces the reviewed residual path, but it is not a
   generic future network-crate detector.

   `xtask/src/main.rs:196` checks `tungstenite` immediate dependents and
   `native-tls` immediate dependents, and `xtask/src/main.rs:136` checks that
   `dioxus-devtools` is inactive. That is strict enough for the specific S-001
   residual path. If the release policy expects automation for "no app-authored
   external network crate" more broadly, add explicit absence checks for known
   HTTP/client/server crates such as `reqwest`, `hyper`, and `ureq`, or document
   that S-001 review is required when new network-capable crates appear.

## Requirement checks

- Threat model reflects the real product guarantee: pass with note. The main
  S-001 wording in `docs/src/maintainers/threat-model.md:9` and
  `docs/src/maintainers/threat-model.md:182` accurately describes local-only
  operation plus accepted local WebView IPC. The stale top-summary phrase above
  should be cleaned up.
- Direct `dioxus-desktop` dependency and launch change: pass. `Cargo.toml:22`
  disables `dioxus` defaults and uses `features = ["lib"]`;
  `Cargo.toml:24` adds `dioxus-desktop` with `default-features = false` and
  `tokio_runtime`; `crates/forskscope-ui/src/main.rs:56` launches through
  `dioxus_desktop::launch::launch`.
- Devtools removal from the active graph: pass. `cargo tree -i dioxus-devtools`
  reports nothing active, and `cargo xtask audit-deps` reports
  `dioxus-devtools is inactive.`
- Residual `tungstenite`/`native-tls` path: pass. Observed active paths are
  exactly `tungstenite -> dioxus-desktop -> forskscope-ui` and
  `native-tls -> tungstenite -> dioxus-desktop -> forskscope-ui`.
- User-facing copy avoids impossible "no network access" wording: pass.
  `docs/src/users/features.md:129` now says "no external network service".

## Supporting source evidence

Local `dioxus-desktop 0.7.9` source supports the accepted framework IPC model:

- `src/edits.rs:103` builds `ws://127.0.0.1:{port}/{webview_id}/{key}`.
- `src/edits.rs:148` binds the edit socket to `127.0.0.1` on an ephemeral port.
- `src/edits.rs:137` documents the client key used to keep external
  applications from connecting to the edit socket.
- `src/edits.rs:257` validates the client key during WebSocket handshake.
- `src/edits.rs:287` sends a server key back for the WebView to authenticate the
  server.
- `dioxus-desktop 0.7.9` default features include `devtools`, but the workspace
  dependency disables defaults and enables only `tokio_runtime`.

## Observed evidence

Commands observed during this review pass:

- `cargo tree --prefix depth -i tungstenite` - passed; path was
  `tungstenite -> dioxus-desktop -> forskscope-ui`.
- `cargo tree --prefix depth -i native-tls` - passed; path was
  `native-tls -> tungstenite -> dioxus-desktop -> forskscope-ui`.
- `cargo tree -i dioxus-devtools` - reported `nothing to print`.
- `cargo xtask audit-deps` - passed and reported:
  - `sheets-diff is absent.`
  - `calamine is absent.`
  - `dioxus-devtools is inactive.`
  - `quick-xml path is limited to wayland-scanner.`
  - `network-capable dependency paths are reviewed.`
  - `security dependency path check passed.`
- `cargo check --locked -p forskscope-ui` - passed.
- `cargo fmt --check` - passed.
- `cargo fmt --manifest-path xtask/Cargo.toml --check` - passed.
- `git diff --check` - passed.
- `cargo tree -i reqwest`, `cargo tree -i hyper`, and `cargo tree -i ureq`
  each reported that the package did not match any active package.

## Missing evidence

- GUI/runtime smoke testing was not run in this review pass.
- I did not independently rerun the broader clippy, unit-test, `cargo audit`,
  or CSS gates listed in the request; this review focused on the S-001 network
  dependency decision and its directly affected compile/enforcement checks.

## Recommended next action

Accept the S-001 decision for release-readiness purposes. Before final release
docs freeze, narrow the top threat-model sentence about cryptographic material
and decide whether `cargo xtask audit-deps` should also maintain an explicit
denylist for common external-network crates.
