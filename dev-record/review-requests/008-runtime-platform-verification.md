# Review request: runtime/platform verification

## Scope

This package covers the v0.164.0 runtime/platform verification follow-up after the docs refresh commit.

Please review the implementation changes and the verification evidence for:

- release UI build compatibility with `dioxus-desktop` 0.7.9 and `wry` 0.53
- desktop devtools exposure policy after the direct `wry` feature adjustment
- Linux release script portability outside Debian-family systems
- local Linux runtime/package verification evidence and remaining platform gaps

## Files to inspect

- `Cargo.toml`
- `Cargo.lock`
- `crates/forskscope-ui/Cargo.toml`
- `crates/forskscope-ui/src/main.rs`
- `docs/src/maintainers/threat-model.md`
- `packaging/build-release.sh`

## Implementation summary

- Added a direct workspace `wry` dependency with `default-features = false` and `features = ["devtools"]`.
- Added `wry` as a direct dependency of `forskscope-ui`.
- Kept `dioxus-desktop` without its default `devtools` feature; `dioxus-devtools` remains inactive.
- Disabled the default Dioxus desktop menu bar with `Config::with_menu(None)` so the framework devtools menu toggle is not exposed as product UI.
- Updated the threat model to document the `wry/devtools` method-surface exception and the absence of `dioxus-devtools`.
- Replaced Debian-specific `dpkg -l` release-script dependency checks with portable `pkg-config --exists webkit2gtk-4.1 gtk+-3.0` checks and distro-specific install hints.

## Why the implementation changed

`cargo build --release --locked -p forskscope-ui` failed because `dioxus-desktop` 0.7.9 calls `WebView::open_devtools` and `WebView::close_devtools` from release builds even when `dioxus-desktop/devtools` is disabled.

Enabling `dioxus-desktop/devtools` would also enable `dioxus-devtools`, which conflicts with the S-001 dependency decision. The narrower compatibility change enables only the `wry` method surface required for compilation while keeping Dioxus devtools inactive and removing the default framework menu surface.

## Observed verification

Passed in this thread:

- `cargo test -p forskscope-core -p forskscope-ui-logic`
- `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings`
- `cargo build --release --locked -p forskscope-ui`
- `cargo xtask audit-deps`
- `cargo tree -p forskscope-ui -i dioxus-devtools -e features`
  - output: `nothing to print`
- `cargo tree -p forskscope-ui -i wry -e features`
  - confirmed the direct `forskscope-ui -> wry/devtools` method-surface path
- `cargo fmt --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo audit`
  - passed under the checked-in policy, with the expected allowed warnings reported
- `./target/release/forskscope --diagnostics`
- `bash -n packaging/build-release.sh`
- `bash packaging/build-release.sh`
  - produced `target/forskscope-v0.164.0.tar.gz`
  - produced `target/forskscope-v0.164.0-linux-x86_64.tar.gz`
- `cargo xtask archive-layout target/forskscope-v0.164.0.tar.gz`
- `tar -tzf target/forskscope-v0.164.0-linux-x86_64.tar.gz`
  - output: `forskscope`
- `makepkg -f --noconfirm --nodeps` from a copied PKGBUILD/source archive in `target/pkgbuild-check`
- `git diff --check`

## Runtime smoke evidence

Linux desktop environment:

- Kernel/OS: Linux x86_64
- GTK: `3.24.52`
- WebKitGTK: `2.52.4`
- Session: Wayland with `DISPLAY=:1`

Observed:

- Sandboxed GUI launch failed with GTK initialization panic.
- Unsandboxed GUI launch of `./target/release/forskscope tests/fixtures/text/left_function.txt tests/fixtures/text/right_function.txt` stayed alive for more than 5 seconds with no stderr output.
- After adding `Config::with_menu(None)`, the unsandboxed GUI launch was repeated and again stayed alive for more than 5 seconds with no stderr output.

Limitations:

- Window automation/screenshot checks could not be completed on this Wayland session: `xdotool` did not find the window and ImageMagick `import` could not capture a useful window image.
- This is startup smoke evidence only, not a completed manual visual or keyboard workflow check.
- macOS and Windows runtime/package verification were not performed in this local environment.

## Packaging notes

- Plain `makepkg -f --noconfirm` failed because local pacman metadata does not know about rustup-installed `cargo`.
- `makepkg -f --noconfirm --nodeps` then passed unsandboxed from `target/pkgbuild-check`, confirming the PKGBUILD build/package flow with the locally available toolchain.

## Reviewer questions

- Is the direct `wry/devtools` method-surface exception acceptable under S-001 when `dioxus-devtools` remains inactive and the default Dioxus menu is removed?
- Should the release gate add an explicit check for `cargo tree -p forskscope-ui -i dioxus-devtools -e features` continuing to print nothing?
- Is startup smoke sufficient for this lane, or should the release remain blocked until manual visual verification is performed on a non-sandboxed desktop session?
