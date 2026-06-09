# RFC-020 — Developer Architecture, CI, and Test Gates

**Status.** Proposed

## 1. Summary

This RFC defines developer-facing architecture rules, CI stages, and release gates for the Dioxus migration.

The migration touches core logic, UI framework, editor bridge, file safety, and packaging. Without strong developer gates, regressions will be hard to detect.

## 2. Motivation

The project is moving from a Tauri/Svelte architecture to a Dioxus architecture. This shift can easily become a large rewrite with unclear correctness. CI must enforce architecture boundaries and behavioral tests from the start.

## 3. Goals

- Define target crate/module boundaries.
- Define test layers.
- Define CI stages.
- Define migration gates.
- Define release blocking criteria.
- Define developer commands.

## 4. Non-Goals

- This RFC does not require every platform package to be built on every PR.
- This RFC does not define paid code signing.
- This RFC does not require exhaustive GUI automation in the first milestone.

## 5. Target Repository Shape

```text
forskscope/
  crates/
    forskscope-core/
      src/
        diff/
        file/
        merge/
        session/
        save/
        jobs/
        errors/
    forskscope-dioxus/
      src/
        app/
        components/
        workspaces/
        editor/
        commands/
        settings/
    forskscope-editor-bridge/
      src/
        protocol/
        adapter/
        mock/
    forskscope-cli/
      src/
        parity.rs
        diagnostics.rs
  tests/
    fixtures/
    parity/
    integration/
  docs/
    rfcs/
    design/
```

The exact directory names may change, but the boundary intent must remain.

## 6. Dependency Rules

```text
forskscope-core
  must not depend on Dioxus, WebView, CodeMirror, or platform UI crates

forskscope-editor-bridge
  may depend on protocol types and editor integration support
  must not own merge/save truth

forskscope-dioxus
  may depend on core and editor bridge
  must not implement duplicate diff/merge logic

forskscope-cli
  may depend on core for parity and diagnostics commands
```

## 7. Test Layers

### 7.1 Unit Tests

Scope:

- encoding detection helpers;
- line splitting;
- hunk identity;
- merge transactions;
- save preflight;
- error conversion.

### 7.2 Core Integration Tests

Scope:

- file-pair diff;
- directory comparison;
- text/binary classification;
- save plan creation;
- undo/redo replay.

### 7.3 Parity Tests

Scope:

- compare old fixture expectations to new core output;
- record intentional changes.

### 7.4 Editor Bridge Tests

Scope:

- protocol validation;
- mock editor operations;
- offset conversion;
- decoration updates;
- stale revision handling.

### 7.5 UI Smoke Tests

Scope:

- app starts;
- open sample file pair;
- open sample directory pair;
- navigate hunks;
- execute merge command;
- open save dialog.

Automated UI tests may begin minimal and expand later.

## 8. CI Stages

Recommended stages:

```text
format
  cargo fmt
  frontend formatting if applicable

lint
  cargo clippy
  deny warnings for core crates when practical

unit
  cargo test -p forskscope-core
  cargo test -p forskscope-editor-bridge

parity
  cargo run -p forskscope-cli -- parity tests/fixtures/parity

build
  build Dioxus app on primary CI OS

package-smoke
  create package artifact on release branches
```

## 9. Architecture Boundary Checks

At minimum, CI should block:

- `forskscope-core` importing Dioxus or editor bridge crates;
- UI crates defining duplicate hunk/diff models not derived from core;
- public APIs returning unstructured `String` errors where `AppError` is required;
- save APIs that bypass save preflight.

These checks can begin as review rules and later become scripts.

## 10. Developer Commands

```text
cargo xtask parity
cargo xtask generate-fixtures
cargo xtask check-boundaries
cargo xtask smoke-dioxus
cargo xtask package-dev
cargo xtask diagnostics-sample
```

Using `xtask` is recommended but not mandatory.

## 11. Release Gates

A release candidate must pass:

- core unit tests;
- parity suite;
- save safety tests;
- editor bridge protocol tests;
- large-file smoke test;
- directory background job cancellation test;
- Linux desktop smoke test;
- at least one Windows smoke test if Windows artifact is published;
- at least one macOS smoke test if macOS artifact is published.

## 12. Manual QA Checklist

Manual QA must include:

- open two text files;
- open two directories;
- compare Japanese text if supported;
- compare binary files;
- compare Excel files if retained;
- copy hunk left-to-right;
- undo/redo merge;
- edit text manually;
- save as new file;
- attempt save after external modification;
- cancel background directory comparison;
- use keyboard-only hunk navigation.

## 13. Diagnostics Artifact

Release builds should include a way to copy diagnostics:

```text
App version
Build profile
Platform
Dioxus/WebView runtime summary if available
Core schema version
Session schema version
Recent error IDs
Feature flags
```

Do not include file contents.

## 14. Acceptance Criteria

- Core crate is UI-independent.
- CI can test core behavior without Dioxus.
- Parity fixtures exist.
- Save safety tests block release.
- Editor bridge has protocol tests.
- Release gates are documented and enforced.

## 15. Risks

| Risk | Severity | Mitigation |
|---|---:|---|
| Rewrite proceeds without parity | High | Make parity a migration gate |
| UI crate duplicates core logic | High | Boundary checks and review |
| CI too slow | Medium | Separate fast PR gates from release gates |
| UI tests are flaky | Medium | Start with core and bridge tests; keep UI smoke minimal |
| Packaging failures appear late | Medium | Package smoke on release branches early |

## 16. Open Questions

- Should CI preserve the old Tauri app temporarily for comparison?
- Should Dioxus UI tests use WebDriver-style automation or internal state smoke tests?
- Should package artifacts be produced for every PR or only release branches?
