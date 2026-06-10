# Local Development

## Prerequisites

- Rust ≥ 1.85 via [rustup](https://rustup.rs/).
- On Linux: `libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev pkg-config libssl-dev`
- On macOS: Xcode CLT.
- On Windows: MSVC toolchain.

## Build

```sh
cargo build                            # debug build
cargo build --release                  # release (LTO, stripped)
cargo test -p forskscope-core          # core tests only
cargo test --workspace                 # all tests
cargo clippy --workspace -- -D warnings
```

## Run

```sh
cargo run -p forskscope-ui
# or, after a release build:
./target/release/forskscope old.txt new.txt
```

## Directory layout

```
crates/forskscope-core/src/
  tests.rs          # module root declaring test submodules
  tests/            # one file per domain: diff_tests, merge_tests, …
```

Files are split at 300 ELOC; splitting is strongly recommended above 500 ELOC.
