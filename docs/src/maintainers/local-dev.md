# Local Development

## Prerequisites

- Rust ≥ 1.85 via [rustup](https://rustup.rs/).
- On Linux: `libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev pkg-config libssl-dev`
- On macOS: Xcode CLT.
- On Windows: MSVC toolchain.

## Build

```sh
cargo build                            # debug build (requires GTK on Linux)
cargo build --release                  # release (LTO, stripped)

# Tests that run WITHOUT GTK / display server:
cargo test -p forskscope-core          # 646 unit + 27 integration tests (corpus + patch)
cargo test -p forskscope-ui-logic      # 189 unit + 5 integration + 1 doctests, 14 view-model modules
cargo test -p forskscope-core -p forskscope-ui-logic  # CI equivalent

# Full workspace (requires GTK):
cargo test --workspace

# Lint (run before every commit):
cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings
```

> **Note:** `forskscope-ui` depends on `dioxus-desktop` which requires GTK3
> at compile time (even for `cargo check` and `cargo test --lib`).  All
> product-logic tests live in `forskscope-core` and `forskscope-ui-logic`
> which have no GUI dependency.  UI-side `#[cfg(test)]` blocks in `state.rs`
> are syntactically complete but require a GTK build environment to run.

## Run

```sh
cargo run -p forskscope-ui
# or, after a release build:
./target/release/forskscope old.txt new.txt
```

## Directory layout

```
crates/
  forskscope-core/src/
    tests.rs          # module root declaring test submodules
    tests/            # one file per domain: diff_tests, merge_tests, …
  forskscope-ui-logic/src/
    compare/          # diff/compare view-model modules
    explore/          # explorer view-model modules
    settings/         # settings form view-model modules
tests/
  fixtures/           # diff acceptance corpus (text/, newlines/, whitespace/)
    README.md         # documents each fixture pair and how to add new ones
```

Files are split at 300 ELOC; splitting is strongly recommended above 500 ELOC.

---

## Adding tests

**Unit tests** live in `crates/forskscope-core/src/tests/` — one file per domain,
registered in `tests.rs`. Add a new `foo_tests.rs` and `mod foo_tests;` in `tests.rs`.

**Corpus tests** (`crates/forskscope-core/tests/diff_corpus.rs`) load fixture files
from `tests/fixtures/` via `load("subdir/name.txt")` and call `compute_diff`.
To add a new case:

1. Create the pair in `tests/fixtures/<subdir>/`.
2. Add a `#[test]` function in `diff_corpus.rs`.
3. Update `tests/fixtures/README.md`.

**CSS coverage tests** (`crates/forskscope-ui-logic/tests/css_coverage.rs`) compile
`main.css` at build time and verify every CSS class token from core is present.

---

## MSRV

The declared minimum supported Rust version is `rust-version = "1.85"` (in `Cargo.toml`).
Verify on MSRV before releasing:

```sh
rustup install 1.85
cargo +1.85 test -p forskscope-core -p forskscope-ui-logic
```
