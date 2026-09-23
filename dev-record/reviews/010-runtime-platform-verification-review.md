# Runtime/Platform Verification Review

Request: `dev-record/review-requests/008-runtime-platform-verification.md`

Date: 2026-07-09

## Verdict

Accept with notes.

The direct `wry/devtools` method-surface exception is acceptable under S-001 as implemented: `dioxus-desktop` remains built without its default `devtools` feature, `dioxus-devtools` is inactive, and the Dioxus default menu entry that can toggle WebView devtools is removed with `Config::with_menu(None)`.

This accepts the implementation change and the local release-build/package-script evidence. It does not close the v1.0 runtime/platform verification lane: startup-only Linux smoke is not enough to replace manual visual and keyboard workflow verification, and macOS/Windows package/runtime verification remains missing.

## Blocking findings

None for this implementation package.

## Non-blocking findings

1. Startup-only Linux GUI evidence is not sufficient to mark runtime/platform verification complete. The request explicitly notes that window automation/screenshot checks failed and that this was not a completed manual visual or keyboard workflow check. Keep v1.0 release blocked on runtime/platform verification until at least a manual non-sandboxed Linux visual workflow is completed, with macOS and Windows package/runtime verification tracked separately.

2. Removing the Dioxus default menu is the right mitigation for the framework devtools menu surface, but it also changes platform UI behavior. Before final release, verify on macOS in particular that losing the default menu does not remove expected app/menu shortcuts or create an unacceptable native-menu gap.

## Requirement checks

- `Cargo.toml:24` keeps `dioxus-desktop` on `default-features = false` with only `tokio_runtime`.
- `Cargo.toml:26`-`Cargo.toml:29` adds direct `wry` only for the `devtools` method surface, with a comment tying the exception to the `dioxus-desktop` 0.7.9 release-build compatibility issue.
- `crates/forskscope-ui/Cargo.toml:25`-`crates/forskscope-ui/Cargo.toml:29` makes `wry` a direct UI dependency, which makes the feature unification explicit instead of hidden in a transitive dependency.
- `crates/forskscope-ui/src/main.rs:56`-`crates/forskscope-ui/src/main.rs:60` launches with `Config::new().with_window(window).with_menu(None)`, replacing the default Dioxus menu.
- Local `dioxus-desktop-0.7.9` source confirms the devtools toggle is handled through the menu event id `dioxus-toggle-dev-tools`, and `Config::with_menu(None)` overrides the default menu.
- `docs/src/maintainers/threat-model.md:194`-`docs/src/maintainers/threat-model.md:208` documents the `wry/devtools` method-surface exception, the inactive `dioxus-devtools` dependency, and the existing `cargo xtask audit-deps` dependency-path gate.
- `packaging/build-release.sh:19`-`packaging/build-release.sh:31` replaces Debian-specific `dpkg` checks with portable `pkg-config --exists webkit2gtk-4.1 gtk+-3.0` checks and distro-specific install hints.

## Reviewer questions

- The direct `wry/devtools` exception is acceptable under S-001 because it enables only the WebView method symbols needed for release compilation. It does not enable `dioxus-desktop/devtools`, does not activate `dioxus-devtools`, and does not add app-authored remote network behavior.
- A separate release gate for `cargo tree -p forskscope-ui -i dioxus-devtools -e features` is not strictly necessary because `cargo xtask audit-deps` already asserts `dioxus-devtools is inactive.` If the team wants a more legible release log, add a printed subcheck to `audit-deps` rather than a second CI command.
- Startup smoke is useful evidence, but it is not sufficient to close the runtime/platform lane. Keep the release blocked until manual visual/keyboard smoke is completed on a real desktop session and the macOS/Windows package/runtime gaps are explicitly resolved or waived.

## Observed evidence

Observed during this review:

- `cargo build --release --locked -p forskscope-ui` passed.
- `cargo xtask audit-deps` passed and printed `dioxus-devtools is inactive.`
- `cargo tree -p forskscope-ui -i dioxus-devtools -e features` printed `warning: nothing to print.`
- `cargo tree -p forskscope-ui -i wry -e features` showed `wry feature "devtools"` enabled by the direct `forskscope-ui` dependency.
- `bash -n packaging/build-release.sh` passed.
- `git diff --check` passed.
- `cargo fmt --check` passed.
- `./target/release/forskscope --diagnostics` printed ForskScope 0.164.0 platform diagnostics and exited successfully.
- `cargo xtask archive-layout target/forskscope-v0.164.0.tar.gz` passed.
- `tar -tzf target/forskscope-v0.164.0-linux-x86_64.tar.gz` printed `forskscope`.
- `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` passed.
- `cargo audit` passed under the checked-in policy with allowed warnings.
- `cargo test -p forskscope-core -p forskscope-ui-logic` passed.
- `cargo clippy --workspace -- -D warnings` passed.
- `bash packaging/build-release.sh` passed and produced `target/forskscope-v0.164.0.tar.gz` plus `target/forskscope-v0.164.0-linux-x86_64.tar.gz`.

Request-provided but not reobserved during this review:

- Unsandboxed GUI launch staying alive for more than 5 seconds with no stderr.
- `makepkg -f --noconfirm --nodeps` from `target/pkgbuild-check`.

## Missing evidence

- No live GitHub Actions run was observed in this review.
- No completed Linux manual visual/keyboard workflow smoke was observed.
- No successful automated screenshot/window check was observed.
- No macOS runtime/package verification was observed.
- No Windows runtime/package verification was observed.

## Recommended next action

Proceed with this implementation package, and keep the release readiness tracker open on runtime/platform verification. The next candidate should be a manual non-sandboxed Linux smoke record with visual confirmation of file compare, hunk navigation/apply, save behavior, and absence of exposed framework devtools UI, followed by macOS and Windows package/runtime verification or explicit release-owner waivers.
