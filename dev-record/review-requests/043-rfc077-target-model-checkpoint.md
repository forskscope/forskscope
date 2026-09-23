# Review Request: RFC-077 Target Model Checkpoint (Patches 1–2)

**Date:** 2026-08-04
**Reviewer stance:** focused implementation/architecture review
**Repository baseline:** `fe234f5` (`core+ui-logic: RFC-077 patches 1-2 -
target model, precondition, no-clobber`) — the handoff's explicit
"request an architecture checkpoint after patch 2, before migrating save
behaviour"
**Governing documents:** `rfcs/handoffs/077-mergetool-save-target-model/implementation-handoff.md`;
RFC-077

## Summary

Review the first two RFC-077 implementation patches: the prepared-comparison/
save-target type model plus its preparation-time derivation (patch 1), and the
save-time precondition model plus a genuinely atomic no-clobber commit (patch
2). This checkpoint is exactly where the handoff asked for one — before either
type is wired into `state/compare.rs`, `state/tab.rs`, `diff_actions.rs`,
`app.rs`, or `main.rs`. Nothing about `save_text`'s existing behavior, the
mergetool startup race in `app.rs`, or the CLI argument handling in `main.rs`
has changed yet.

## Scope Followed

**Patch 1 — request and target model, preparation tests:**

- `StartupRequest` (`Explorer`/`Compare`/`MergeTool`), replacing the
  `STARTUP_PAIR`/`STARTUP_MERGED` `OnceLock` pair's *shape* — not yet its call
  sites. `parse_startup_args` rejects any arity other than 0/2/3 with a typed
  `StartupArgError`, per RFC-077 ("must reject unsupported arity... rather
  than silently opening Explorer"). Lives in `forskscope-ui-logic`
  (`compare::startup`) — pure, no I/O, mirrors RFC-075's precedent of putting
  framework-independent lifecycle types there rather than in `forskscope-core`
  or the UI crate.
- `CompareRequest`/`SaveDestination`, and `StartupRequest::into_compare_request()`
  — the exact place the RFC-077 model boundary lives: normal `Compare` saves
  to `RightInput`; `MergeTool` compares local/remote but saves to
  `Explicit(merged)`. Same module.
- `PreparedCompare`/`SaveTargetSnapshot`/`SaveTargetState`/`TargetExpectation`/
  `SaveTargetBlockReason`, plus `save_target_from_loaded` (normal compare —
  derives the snapshot from an already-loaded document, no extra I/O) and
  `inspect_save_target` (Git mergetool — independently classifies, decodes
  enough to fingerprint, and encodes-labels a path on disk without feeding its
  content into the comparison). Lives in `forskscope-core`
  (`compare_prep.rs`) — per "Core First," and because target inspection
  needs real filesystem access (`file_kind::classify`, `document::load_path`),
  which `forskscope-ui-logic` deliberately never does.

**Patch 2 — core precondition and no-clobber safe-file tests:**

- `TargetPrecondition` (`MustMatch`/`MustBeAbsent`/`Force`) in `save.rs`,
  additive alongside the existing `SaveRequest.expected_fingerprint:
  Option<FileFingerprint>` — not replacing it yet.
- `check_precondition` — built on `document::check_external_state` (RFC-036),
  not a new fingerprint comparison. That function already existed, fully
  tested, with zero production call sites; `save_text`'s own inline check
  duplicates a narrower version of the same logic. Reusing it means
  `MustMatch` gets `ChangedOnDisk`/`DeletedOnDisk`/`ReplacedOnDisk` handling
  for free, more granular than what `save_text` does today.
- `persist_noclobber` / `persist_noclobber_with_hook` — same-directory
  `tempfile::NamedTempFile`, written completely, then
  `.persist_noclobber(target)`. The `_with_hook` variant is the test seam the
  handoff named explicitly: a closure runs after the temp file is fully
  written but before the commit, so the no-clobber race is exercised
  deterministically, no sleeps.
- `tempfile` promoted from `forskscope-core` dev-dependency to a normal one
  (§4.1). `cargo xtask audit-deps` and `cargo audit` both re-run after the
  promotion (output below); `tempfile` introduces zero new advisories.
  `docs/src/maintainers/threat-model.md`'s dependency table has a new row
  stating role and risk explicitly (local filesystem only, no network data
  flow).

## Files Changed

New:
- `crates/forskscope-core/src/compare_prep.rs`
- `crates/forskscope-core/src/tests/compare_prep_tests.rs`
- `crates/forskscope-core/src/tests/save_target_tests.rs`
- `crates/forskscope-ui-logic/src/compare/startup.rs`

Modified:
- `crates/forskscope-core/Cargo.toml` (`tempfile` dependency promotion)
- `crates/forskscope-core/src/lib.rs` (module registration + re-exports)
- `crates/forskscope-core/src/save.rs` (`TargetPrecondition`,
  `check_precondition`, `persist_noclobber`, `persist_noclobber_with_hook`)
- `crates/forskscope-core/src/tests.rs` (register the two new test modules)
- `crates/forskscope-ui-logic/src/compare.rs` (register `startup`)
- `crates/forskscope-ui-logic/src/lib.rs` (module doc + re-exports)
- `docs/src/maintainers/threat-model.md` (`tempfile` dependency-table row)

**Deliberately not touched:** `crates/forskscope-ui/src/main.rs`,
`crates/forskscope-ui/src/app.rs`, `crates/forskscope-ui/src/state/compare.rs`,
`crates/forskscope-ui/src/state/tab.rs`,
`crates/forskscope-ui/src/ui/view/diff_actions.rs`,
`crates/forskscope-core/src/save.rs`'s existing `SaveRequest`/`save_text`
behavior. Review 041's C1 property (session resolution unconditional in
`app.rs`) is therefore untouched by construction, not by a check I ran.

## Design Decisions And Assumptions

- **`inspect_save_target` classifies before calling `load_path`, rather than
  going through `load_path` alone.** `load_path` collapses `FileKind::Unsupported`
  (e.g. a directory) into `Err(CoreError::Unsupported)`, which would otherwise
  land in the same generic `SaveTargetBlockReason::Unreadable` bucket as an
  actual I/O failure. RFC-077's test design wants "replaced by a directory"
  distinguishable, so I call `file_kind::classify` first and only fall through
  to `load_path` for the `Text` case (where reading is genuinely needed to get
  a fingerprint and encoding label). Found this the hard way — my first draft
  used `load_path` alone and a directory test failed with `Unreadable` instead
  of the expected `NotAPlainFile`.
- **`SaveTargetBlockReason` has no separate `Directory` variant** — it's
  `NotAPlainFile { reason: String }`, carrying whatever `FileKind::Unsupported`
  says (currently always `"not a regular file"`, which covers directories and
  any other non-regular-file entry `fs::metadata` can see, e.g. a socket or
  FIFO on Unix). I chose not to special-case directories specifically since
  `classify()` itself doesn't distinguish them from other non-regular-file
  cases — inventing a distinction `classify()` doesn't make felt like scope
  creep. Flagging this as a place you might want a sharper `Directory` variant
  if the presentation layer (a later patch) needs to say "this is a folder"
  specifically rather than "this isn't a plain file."
- **`check_precondition` reuses `check_external_state` rather than duplicating
  `save_text`'s inline fingerprint check.** This is a real behavior
  *refinement*, not yet exercised in production: today, `save_text` with
  `Some(fingerprint)` silently skips its check if the target is missing
  (`if ... && target.exists()`); under `MustMatch`, a missing target now
  correctly conflicts. This only affects `check_precondition`'s own callers —
  none exist yet outside its tests — so it changes nothing observable until a
  later patch routes `save_text`/`diff_actions.rs` through it.
- **`persist_noclobber` creates missing parent directories before attempting
  the no-clobber commit**, mirroring `save_text`'s existing Save-As-to-new-path
  behavior, so RFC-077's "missing path... required parent directories"
  acceptance criterion is met without a second convention.
- **The hook parameter is `pub(crate)`, not `pub`.** Only this module's own
  tests need it; making the race seam part of the public API seemed like
  unnecessary surface for something that exists purely to make one race
  deterministic in tests.

## Review Questions

1. Is `compare_prep.rs` the right new module, or would you rather this live
   inside `save.rs` itself (given how tightly `TargetExpectation` and
   `TargetPrecondition` mirror each other), or somewhere under a `save/`
   directory once `save.rs` needs splitting anyway?
2. Does reusing `check_external_state` (RFC-036, previously unwired) for
   `MustMatch` read as the right call, or does the precondition model deserve
   its own dedicated comparison now that it has a real semantic (missing =
   conflict) that `save_text`'s narrower inline check doesn't have?
3. `SaveTargetBlockReason::NotAPlainFile` vs. a dedicated `Directory` variant
   — worth the extra type now, or wait until the presentation-layer patch
   shows whether the distinction is actually needed in UI copy?
4. `StartupRequest`/`CompareRequest` in `forskscope-ui-logic` vs.
   `forskscope-core` — I read RFC-075's precedent (pure UI-orchestration
   lifecycle types in ui-logic) as the closer analogy than RFC-076's precedent
   (file-safety types in core), since parsing `argv` and deciding "what gets
   compared, what gets saved" isn't itself a file-safety concern. Confirm or
   correct before patch 3 builds on top of this placement.
5. May implementation proceed to patch 3 (normal-compare migration, proving
   unchanged behavior) and patch 4 (mergetool startup + save-path migration)?

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1063 (680+27+16+2 core, 267 ui-logic, 58 forskscope-ui, 6 css, 7 doctests), exactly 1031 + 32
cargo clippy --workspace -- -D warnings                          pass
cargo xtask audit-deps                                           pass — tempfile promotion re-audited, no new findings
cargo audit                                                      pass — 14 pre-existing allowed warnings (GTK3/glib/rand transitive advisories); no tempfile entries anywhere in the report
cargo xtask i18n                                                 pass — 220 keys (unchanged; this checkpoint adds no user-visible strings)
cargo xtask css --check                                          pass
cargo xtask version-sync                                         pass — v0.165.1
git diff --check                                                 pass
```

CI run `30879738819`: Test & Lint green, on `fe234f5`.

The stronger advisory command `cargo clippy --workspace --all-targets -- -D
warnings` did not pass — it reports the same 9 pre-existing test-target lint
locations noted in RFC-075's own checkpoint review (024/028): two
`manual_contains` in `diff_corpus.rs`, one `type_complexity` in
`search_index.rs`, one `manual_contains` in `diff_tests.rs`, one `cmp_owned`
in `dir_index_tests.rs`, four `assertions_on_constants` in `job_tests.rs`.
None are in any file this checkpoint touches.

**Test-count delta, itemized:** +32 over baseline 1031.

| Source | Delta | Reason |
|---|---:|---|
| `compare_prep_tests.rs` (core) | +10 | `save_target_from_loaded`/`inspect_save_target` across text/missing/binary/xlsx/directory/conflict-marker-content/encoding-fallback cases |
| `save_target_tests.rs` (core) | +12 | `check_precondition` × 3 modes (8 tests) + `persist_noclobber` (4 tests, including the before-commit-hook race) |
| `compare::startup` (ui-logic) | +10 | `parse_startup_args` arity handling (0/1/2/3/4) + `into_compare_request` conversion, including a test that the mergetool save destination is never aliased to the compared right input |

## Generated Artifacts

- This review request is the only review artifact.
- No binary, package, generated documentation tree, or runtime evidence —
  none of this is wired into anything runnable yet.

## Known Limitations

- **F36 doesn't apply here** — nothing in this checkpoint constructs a
  `Store`; both new modules are pure/file-I/O-only with no Dioxus dependency.
- This checkpoint does not yet close B3 in production: `app.rs`'s mergetool
  startup race (synchronously overriding `right_path`/clearing
  `fingerprint_at_load`, then the async `open_compare` completion potentially
  clobbering it — flagged by the research pass before I started this patch)
  is completely untouched. `main.rs` still populates
  `STARTUP_PAIR`/`STARTUP_MERGED` exactly as before.
- `save_text` still uses its own narrower inline conflict check;
  `TargetPrecondition`/`check_precondition` have no production callers yet.
- RFC-077 and M3 remain incomplete; v1/public release remains **No-Go**.

## Recommended Next Step

Architect should issue Accept, Accept with notes, or Needs changes for this
type/precondition boundary. After acceptance, implementation proceeds to
patch 3 (refactor `state/compare.rs`'s blocking preparation to construct and
return `PreparedCompare`, migrating normal two-file compare and proving no
behavior change) and patch 4 (migrate the three-argument mergetool startup
and the save/save-as/overwrite/reload paths onto `save_target` exclusively,
closing the actual B3 race).
