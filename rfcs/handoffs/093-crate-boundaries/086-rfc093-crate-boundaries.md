> **Updated 2026-10-20: one more step, by the owner's decision (RFC-093,
> *Amended*).** The GUI package `forskscope-ui` is renamed **`forskscope`**.
>
> - **Only the package name changes.** The directory stays
>   `crates/forskscope-ui`. The `[lib]` target keeps `name = "forskscope_ui"`,
>   and the `[[bin]]` stays `forskscope`.
> - **Nothing reads `CARGO_PKG_NAME`,** so behaviour cannot change. Confirm the
>   grep is still empty after the move.
> - **Do it as step 8,** after `forskscope-core` is removed, in its own commit.
> - **Update every `-p forskscope-ui`:**
>   - the workflows: `ci.yml`, `release.yml` (all three platforms),
>     `windows-check.yml`, `render-check.yml`;
>   - `store-build.ps1`;
>   - `README.md`, `installation.md`, `local-dev.md`, `gtk-smoke-test.md`.
> - **Update the package-name lists in `xtask`:** the `audit-deps` dependents
>   (`"forskscope-ui "` becomes `"forskscope "`) and the clippy package list.
> - **Leave alone:** directory paths (`crates/forskscope-ui/...`) and
>   `forskscope-ui-logic`.
> - **Do not add `default-members` to the workspace.** Plain `cargo test` would
>   then test only the app crate.
> - **Grep for `-p forskscope-ui`, excluding `-ui-logic`,** and the request
>   says it is empty except history.
> - **Show `cargo build --release -p forskscope` producing
>   `target/release/forskscope`.**

# Handoff 086 — 0.195.0, part 1: crate boundaries

**From:** architect. **Date:** 2026-10-20. **Release:** `0.195.0`. It lands on
`main`, with no release of its own.
**Implements:** RFC-093 (`rfcs/accepted/093-crate-boundaries.md`), accepted by
the owner on 2026-10-20.
**Then:** RFC-094 (the headless program). I write its handoff after this one is
approved, because it builds on the new crates.

**The RFC is the authority.** Where this document and the RFC differ, the RFC
wins. If the code does not fit the RFC's table, **stop and tell me before
changing course.** Examples:
- a module that cannot go to one crate without a cycle;
- a coupling the RFC does not name.

Do not change the design yourselves, and do not edit any RFC.

**This is a move, not a rewrite.** No behaviour changes, and no API is redesigned
on the way, except the three couplings RFC-093 §1 names. If you see something
worth improving, list it in the request. Do not do it here.

---

## 1. Before moving anything: the map

**Produce the module-to-crate map from the code** and put it in the request's
first section, **before you change any code**:

- **every module and sub-module** of `forskscope-core`, with its destination
  crate;
- **every `crate::` edge that will cross a crate boundary,** with its
  direction;
- **every edge that would point the wrong way.** Expect three, which RFC-093
  §1 names: `report` → `merge` history, `dir`'s batch copy and plan/execute,
  and `compare_prep`'s save target. **Any other one is a stop-and-tell.**

Then proceed, unless the map shows a surprise.

## 2. The move, in the RFC's order

Follow RFC-093 §3:
1. `forskscope-compare`;
2. `forskscope-report`;
3. `forskscope-edit`;
4. `forskscope-desktop`;
5. the view-models into `forskscope-ui-logic`;
6. **shared operations moved down** (RFC §1): `build_dir_report`,
   `BuildOutcome`, `checksum_if_fingerprinted` and the file report's assembly,
   into `forskscope-report`. `write_export_file` stays in the GUI;
7. remove `forskscope-core`.

- **One commit per step,** each green: build, tests, clippy (native and
  Windows GNU), and the `cargo xtask` gates.
- **`git mv` for files,** so history follows them. Say in the request that
  `git log --follow` works on a moved module.
- **No re-export shims, and no `pub use` of one crate's modules from another,**
  except the public API a crate legitimately owns.
- **The workbook stack becomes `forskscope-compare`'s `spreadsheet` feature,**
  on by default.

## 3. The gate: `cargo xtask crate-boundaries`

As RFC-093 §2. In a new `xtask/src/crate_boundaries.rs`, with only a dispatch
arm in `main.rs`, and wired into CI.

- **Rule 1:** no file writes in `forskscope-compare` or `forskscope-report`.
  The calls are listed in the RFC.
- **Rule 2:** no `std::process::Command` in either crate.
- **Rule 3:** the workspace dependency graph equals RFC-093 §1's table.
  - Read it from `cargo metadata`.
  - **An unknown crate fails it too.** RFC-094 adds `forskscope-cli` to the
    expected table then, not now.
- **Read the source text, as `ui-logic-connectivity` does,** and skip
  `#[cfg(test)]` modules and `tests/` folders. Say how the skip is done.
- **A test-only helper that writes** (fixtures) must sit under `#[cfg(test)]`
  or in `tests/`. Otherwise the gate is right to fail it.
- **Falsify each rule** on a temporary commit, reverted, with the run or
  output quoted:
  - a `std::fs::write` in `forskscope-report`;
  - a `Command::new` in `forskscope-compare`;
  - a `forskscope-edit` dependency added to `forskscope-report`.

## 4. Everything that names a crate

- **`audit-deps`:** the expected dependents (`sheets-diff`, `calamine`, `zip`,
  `sha2`, `quick-xml`) name the new crates. Each changed expectation must still
  be shown failing once.
- **`ui-logic-connectivity` and `ui-logic-docs`:** cover the four modules that
  move into `forskscope-ui-logic`.
- **`xlsx-fixtures`:** its path follows the fixtures.
- **CI:** the headless clippy step (`-p forskscope-core -p
  forskscope-ui-logic`) names the new crates.
- **`docs/src/maintainers/architecture.md`:** the crate diagram, and a module
  table per crate. **`testing.md`:** the same.
- **`CONTRIBUTING.md` and any other doc that names `forskscope-core`.** Grep for
  it, and the request says the grep is empty except history (CHANGELOG, RFCs,
  the register).

## 5. Verification

- **The test count:** report it before (`72bff1c`: 1,810) and after. They are
  equal, apart from the gate's own tests. Name any difference.
- **`forskscope-compare` builds and tests with `--no-default-features`.**
- **The GUI binary,** release build, before and after:
  - its size, within 1 %;
  - on Linux, `readelf -d` shows the same 16 libraries;
  - the Windows import gate passes in a Windows Check dispatch.
- **The no-behaviour-change check:**
  - every test passes;
  - **plus one live smoke run, through AT-SPI only:** a file comparison, a
    folder comparison, and one report export, with the result shown.
- **Quote the lines you rely on.** Describe the screenshots after looking at
  them.

## Scope

- **In:** RFC-093 entire; the gate; the docs.
- **Out:**
  - RFC-094;
  - behaviour changes;
  - API redesigns beyond the three named couplings;
  - CHANGELOG, ROADMAP, the version.

## Gates

- **The standard set,** and the new `crate-boundaries` gate.
- **A green CI run** with its ID.
- **A Windows Check dispatch** with its ID.

Reply in `.git-exclude/review-request/161-crate-boundaries.md`.
