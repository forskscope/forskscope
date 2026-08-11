# Local Development

## Prerequisites

- Rust ≥ 1.91 via [rustup](https://rustup.rs/).
- On Linux: `libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev pkg-config libssl-dev`
- On macOS: Xcode CLT.
- On Windows: MSVC toolchain.

## Build

```sh
cargo build                            # debug build (requires GTK on Linux)
cargo build --release                  # release (LTO, stripped)

# Tests that run WITHOUT GTK / display server:
cargo test -p forskscope-core          # 695 tests: 643 unit + 27 diff corpus + 16 merge corpus + 2 patch apply + 7 doctests
cargo test -p forskscope-ui-logic      # 235 tests: 228 unit + 6 CSS coverage + 1 doctest
cargo test -p forskscope-core -p forskscope-ui-logic  # headless gate (930 total)

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

**Testing `Store`-dependent UI logic (F36).** `Store::new` needs a live Dioxus
runtime — `Signal::new_in_scope` panics without one, which a bare `#[test]` fn
doesn't have. Two ways to get real test coverage instead of relying on
AT-SPI/runtime evidence alone:

1. **Prefer extracting a pure predicate.** If the logic under test is really a
   decision — "does this row get a screen-reader label," "would this action
   discard unsaved work" — pull it into a plain function over owned/borrowed
   values with no `Store`/`Signal` involved, and unit-test that directly.
   `wants_replace_label` (`ui/view/hunk.rs`, F35) and `recompute_diff`'s
   destructive-contract test (`state/tab/tests.rs`, F40) are the examples to
   copy. This is the default — reach for it first.
2. **When the logic genuinely needs a `Store`** (dirty-check-then-modal
   guards, tab mutation call sites), use `state::with_test_store` (`#[cfg(test)]`,
   `pub(crate)`): it spins up a headless `dioxus_core::VirtualDom` — no
   renderer, no WebView, no GTK — runs a trivial root component to get a real
   `Store` backed by real `Signal`s, and hands it to your closure. See
   `change_diff_options_defers_to_confirmation_when_the_tab_is_dirty` in
   `state/tab/tests.rs` for a working example: push a fixture tab, call the
   action function, assert on `store.modal`/`store.tabs` afterward, same as
   any other unit test.

Neither covers rendering, event dispatch from a real click, or visual
correctness — those still need AT-SPI runtime evidence (or a rendering check,
see F34). `with_test_store` closes the gap for *state mutation* logic only.

---

## MSRV

The declared minimum supported Rust version is `rust-version = "1.91"` (in `Cargo.toml`).
Verify on MSRV before releasing:

```sh
rustup install 1.91
cargo +1.91 test -p forskscope-core -p forskscope-ui-logic
```
