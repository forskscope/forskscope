# Contributing to ForskScope

Thank you for your interest in contributing. This guide explains how to set up
the project, what the constraints are, and where to start.

---

## Code of conduct

Be respectful, constructive, and patient. Substantive technical disagreements
are welcome; personal attacks are not.

---

## Setting up

**Prerequisites:** Rust ≥ 1.91 via [rustup](https://rustup.rs/).

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
6. **Record what must survive.** A reason, a limit you discovered, or
   evidence that a check can actually fail belongs in a tracked document —
   an RFC, the register in `ROADMAP.md`, the maintainer documentation, or a
   comment at the line it explains. See
   [What gets recorded, and where](docs/src/maintainers/what-gets-recorded.md).

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

If you need to expose new presentation logic, follow the shape of two real
modules added this way — `crates/forskscope-ui-logic/src/explore/sync_panes.rs`
and `crates/forskscope-ui-logic/src/explore/classify_pair.rs` — rather than
this list alone; if the two disagree with the steps below, trust the code
and open an issue about this file.

1. Create `crates/forskscope-ui-logic/src/<area>/<module>.rs`.
2. Add `pub mod <module>;` to `<area>.rs` (e.g. `explore.rs`) — **there is no
   `mod.rs` anywhere in this codebase**; the area's own `<area>.rs` file is
   where its submodules are declared.
3. Re-export from `crates/forskscope-ui-logic/src/lib.rs`'s crate root:
   `pub use <area>::<module>::{...};`.
4. Add a row for the module to **both**
   `docs/src/maintainers/architecture.md`'s `` `ui-logic` modules (N) `` table
   (and bump `N`) **and** `docs/src/maintainers/testing.md`'s
   `` `forskscope-ui-logic` test modules `` table. `cargo xtask ui-logic-docs`
   checks these two documents against the module files on disk — not
   `lib.rs`'s own doc comment, which is unchecked prose.
5. **Consume it directly** from `forskscope-ui` — no shim file exists
   anywhere in this codebase, and none should. `cargo xtask
   ui-logic-connectivity` requires every crate-root export to have a real
   consumer somewhere in `forskscope-ui/src`, so export a name only once
   something actually uses it, not speculatively ahead of a consumer.
6. Write at least one test per public function, inline in the module file
   (`#[cfg(test)] mod tests`), matching every other module in this crate.

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
