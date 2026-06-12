# Contributing to ForskScope

Thank you for your interest in contributing. This guide explains how to set up
the project, what the constraints are, and where to start.

---

## Code of conduct

Be respectful, constructive, and patient. Substantive technical disagreements
are welcome; personal attacks are not.

---

## Setting up

**Prerequisites:** Rust ≥ 1.85 via [rustup](https://rustup.rs/).

```sh
git clone https://github.com/forskscope/forskscope
cd forskscope
cargo test -p forskscope-core -p forskscope-ui-logic
```

Tests in `forskscope-core` and `forskscope-ui-logic` run without GTK. The UI
crate (`forskscope-ui`) requires GTK3/WebKitGTK on Linux — see
[Local development](docs/src/maintainers/local-dev.md) for the system
package list.

---

## Project layout

```
crates/
  forskscope-core/        # GUI-independent domain logic; no Dioxus dependency
  forskscope-ui-logic/    # Pure view-model layer; no GTK dependency
  forskscope-ui/          # Dioxus desktop shell (requires GTK to build)
docs/src/                 # mdBook documentation
rfcs/                     # Design documents (RFC lifecycle: rfcs/done/000-…)
tests/fixtures/           # Diff acceptance test corpus
```

The critical constraint: **`forskscope-core` and `forskscope-ui-logic` must
never gain a Dioxus or GTK dependency.** All product logic — file loading,
diff computation, merge decisions, save safety — lives in core, tested without
a display server.

---

## Before you write code

For any non-trivial change, read the relevant RFC in `rfcs/done/`. The RFC
describes the design contract your change must satisfy. If no RFC covers the
area, open an issue to discuss scope before investing time.

File a bug or feature request in the issue tracker before opening a pull
request for anything beyond a one-line fix.

---

## Making a change

1. **Branch** from `main`.
2. **Write the test first** (or alongside the code). Tests live in
   `crates/forskscope-core/src/tests/` (unit) or
   `crates/forskscope-core/tests/` (integration).
3. **Run the test suite:**
   ```sh
   cargo test -p forskscope-core -p forskscope-ui-logic
   cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings
   ```
   Both must pass with zero failures and zero warnings before opening a PR.
4. **Check file size.** Files over 300 ELOC should be split; over 500 ELOC
   splitting is required.
5. **Update docs** if the change affects user-visible behaviour or the
   public API.

---

## Adding diff corpus fixtures

The acceptance corpus lives in `tests/fixtures/`. When you fix a diff edge
case or add a feature, add a fixture pair that exercises it:

1. Create matching `left_*.txt` and `right_*.txt` files in the appropriate
   subdirectory (`text/`, `newlines/`, `whitespace/`).
2. Add a test in `crates/forskscope-core/tests/diff_corpus.rs` that loads
   the pair and asserts the expected diff behaviour.
3. Update `tests/fixtures/README.md` with a description of the pair.

Fixture files should be minimal — the smallest input that demonstrates the
edge case.

---

## Adding a view-model module

If you need to expose new presentation logic:

1. Create `crates/forskscope-ui-logic/src/<area>/<module>.rs`.
2. Register it in the `mod.rs` for that area and re-export from `lib.rs`.
3. Add a shim file in `crates/forskscope-ui/src/ui/<module>.rs` with
   `pub use forskscope_ui_logic::...`.
4. Register the shim in `crates/forskscope-ui/src/ui/mod.rs`.
5. Write at least one test per public function.

---

## RFC governance

RFC numbers are never reused. Lifecycle changes (moving an RFC between
`proposed/`, `done/`, `archive/`) are maintainer decisions, not contributor
decisions — flag in the PR or issue that an RFC should be closed/archived and
the maintainer will do it.

See [RFC 000](rfcs/done/000-rfc-lifecycle-policy.md) for the full lifecycle
policy.

---

## Commit messages

```
Short summary (≤ 72 chars, imperative mood)

Optional body explaining *why*, not just what. Reference RFC numbers
when the change implements or affects a design decision.

Refs: RFC-024, RFC-035
```

---

## Pull request etiquette

- One logical change per PR; split unrelated fixes.
- Link to the relevant issue or RFC.
- The CI-equivalent command (`cargo test -p … && cargo clippy -p … -- -D warnings`)
  must be green before requesting review.
- Reviewers may ask for tests, documentation, or scope reduction — this is
  normal and not a rejection.

---

## Licence

By contributing you agree that your contributions are licensed under the
project's [Apache-2.0 licence](LICENSE).
