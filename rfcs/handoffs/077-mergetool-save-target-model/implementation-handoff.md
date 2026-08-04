# RFC-077 Developer Handoff: Mergetool Save Target

**Governing RFC.** [RFC-077](../../proposed/077-mergetool-save-target-model.md)
**Program.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md) — milestone M3
**Audit finding.** B3
**Requires.** RFC-075 (complete)

This handoff directs execution. It does not redefine RFC-077. If implementation
evidence contradicts a decision below, amend the RFC first, then update this
handoff to match.

*Refreshed 2026-08-04.* The original was written before R0, M2-A, and M2-B. Its
design substance was sound and is preserved; the sections that assumed an
earlier project state have been corrected, and §4.1, §4.2, §5, and §9–§12 are
new.

## 1. Summary

Compared inputs and save output are different identities. Separate the compared
right/remote input from the actual save destination, prepare the merge target's
explicit match-or-absence precondition and encoding atomically with the
comparison result, then route save, Save As, overwrite, and reload exclusively
through that target model.

The defect is concrete. `app.rs` currently starts an asynchronous local-vs-remote
comparison and then immediately mutates `right_path` to `<merged>` and clears the
fingerprint, while the background load later installs a `right_doc` read from
`<remote>`. Save therefore writes to `<merged>` while checking `<remote>`'s
fingerprint. An existing merge output can raise a false conflict, and the actual
target has no load-time baseline at all.

This closes B3. B4 remains open; v1/public release stays **No-Go**.

## 2. Scope

In scope:

- typed startup and compare requests replacing `STARTUP_PAIR`/`STARTUP_MERGED`;
- `SaveTargetSnapshot`, `TargetExpectation`, and launch mode;
- core `TargetPrecondition` plus no-clobber creation semantics;
- promoting `tempfile` to a core runtime dependency — see §4.1;
- target preparation for existing, missing, and unsupported paths;
- save, Save As, overwrite, and reload integration;
- existing/missing/appeared/deleted/changed/replaced target tests;
- Git/JJ, CLI, and GTK-checklist documentation.

Out of scope:

- base-aware conflict UI, Git invocation, Git exit-code certification, or
  persisting mergetool sessions;
- `README.md` and `docs/src/users/installation.md` — F33 is the architect's. If
  the Git integration text you need lives in `README.md`, say so in the review
  request rather than editing it;
- F23, F24, F25/F25b, F31, F34, F35, F36, F37 — not this milestone's.

## 3. Files changed

Expected areas:

- `crates/forskscope-ui/src/main.rs` — argument parsing into `StartupRequest`
- `crates/forskscope-ui/src/app.rs` — startup wiring; **see §4.2**
- `crates/forskscope-ui/src/state/compare.rs` — `PreparedCompare` commit
- `crates/forskscope-ui/src/state/tab.rs` — `CompareTab.save_target`
- `crates/forskscope-ui/src/ui/view/diff_actions.rs` — `build_request`
- `crates/forskscope-core/src/save.rs` — precondition and no-clobber commit
- diff header and status presentation
- integration tests for prepared compare and save target
- `docs/src/intermediate/git-integration.md`, `cli.md`,
  `docs/src/users/merging.md`, `docs/src/maintainers/gtk-smoke-test.md`

## 4. Design decisions and assumptions

### 4.1 `tempfile` promotion is a dependency-policy change

`forskscope-core` currently carries `tempfile` as a **dev-dependency only**.
RFC-077 requires it at runtime for same-directory `persist_noclobber` commits,
so this is a promotion to a normal dependency — the same class of change as
serde in RFC-076, not an implementation detail:

- run `cargo xtask audit-deps` and `cargo audit` after the change, and record
  both;
- add `tempfile` to the dependency table in
  `docs/src/maintainers/threat-model.md` with its role and risk note;
- it is local filesystem only and introduces no network data flow — state that
  explicitly rather than leaving it inferred.

RFC-077 is explicit about the fallback: if `persist_noclobber` cannot provide
the required semantic on a supported platform, **normal save fails rather than
falling back to replacement**. Do not soften that.

### 4.2 Do not undo review 041's C1 fix

This work restructures the same `app.rs` startup hook that C1 repaired, so read
it before editing. The current shape is load-bearing:

```rust
let (session_resolution, session_notice) = resolve_session(&mut store);   // unconditional
...
if let Some(Some((left, right))) = STARTUP_PAIR.get() { ... }             // CLI
else { restore_tabs(&mut store, &session_resolution); }                    // conditional
```

Session resolution is **outside** the branch on purpose. Before C1, it lived
inside the `else`, so a CLI launch never set `session_write_disabled` and the
first tab-triggered save overwrote a future-version or corrupt `session.json`.
Replacing `STARTUP_PAIR`/`STARTUP_MERGED` with `StartupRequest` must not move
resolution back inside the match. Keep
`future_version_session_stays_byte_identical_through_a_disabled_save` passing.

### 4.3 Preserved design decisions

- `right_path`/`right_doc` always mean the compared right/remote input.
- `save_target` alone supplies output path, precondition, and encoding.
- `MustBeAbsent` is distinct from skipping checks, and uses atomic no-clobber
  commit behaviour.
- `Force` exists only for an explicitly confirmed overwrite; selecting an
  existing Save As destination never implies force.
- Existing merged targets are inspected independently; their content is never
  substituted into the two-way comparison.
- Unsupported targets (binary, XLSX, directory, unreadable) block rather than
  force overwrite.
- Save As changes only the save target, and only after a successful write.
- The prepared comparison and its target snapshot commit together under
  RFC-075's load token — one commit, not two mutations.

### 4.4 Precedent worth reusing

RFC-076 built machinery that solves adjacent problems; reuse rather than
reinvent:

- `verify_unchanged(path, expected_bytes)` in
  `persist/schema/repository.rs` is the same shape as `MustMatch` — read, compare,
  refuse on mismatch.
- `PersistenceCommitError::{Conflict, Io}` established that a stale-read conflict
  and an I/O failure must stay distinguishable. Review 038's C1 was about
  collapsing them; do not repeat it in the save path.
- `save::atomic_replace` was extracted in RFC-076 patch 2 precisely so
  temp-then-rename has one implementation. Extend it for no-clobber rather than
  adding a second writer — RFC-077 says so directly.

### 4.5 Patch sequence

1. Request and target model, preparation tests.
2. Core precondition and no-clobber safe-file tests.
3. Normal compare migration, proving unchanged behaviour.
4. Mergetool startup and save-path migration.
5. Target-transition cases, presentation, and docs.

**Request an architecture checkpoint after patch 2**, before migrating save
behaviour. That is the boundary where the precondition model becomes load-bearing
for real files, and it is the analogue of the RFC-076 pause that caught the
schema issues early.

## 5. Release context

M3 completes into a release.

- **Version level is decided at release time, from content.** A save-path
  correctness fix is a significant internal change, so a minor bump is expected —
  but the owner confirms it with the content visible. See
  `docs/src/maintainers/release.md`.
- **Write the CHANGELOG entry as work lands.** Release notes are composed in CI
  from the `## [X.Y.Z]` section and the job fails closed on an empty one.
- **Release-bearing rules from R0 apply.** A red platform job is stop-and-report.
  CI builds artifacts and creates the draft; CI composes the notes; a human
  publishes. Never create or publish a release by hand.

## 6. Tests and gates

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo xtask audit-deps
cargo audit
cargo xtask i18n
cargo xtask version-sync
git diff --check
```

Baseline is **1031**. Report the delta and what each new test covers.

Named evidence required for:

- existing merged target, including `.bak` holding the pre-save bytes;
- missing merged target created on save, with parent directories;
- missing target created externally after preparation — conflict, external bytes
  intact;
- existing target deleted externally after preparation — conflict, path not
  recreated;
- external merged-target mutation — conflict, original bytes intact, confirmed
  overwrite backs up the changed version;
- target replaced by a directory — blocked, directory not removed;
- normal two-argument compare — unchanged behaviour;
- reload and Save As preserve compared-input identity;
- Save As to an existing destination requires confirmation and never constructs
  `Force`;
- the no-clobber race at the core safe-file boundary, using RFC-077's
  before-commit test seam rather than sleeps.

**Runtime evidence.** This changes what the mergetool writes to disk. Exercise
the three-argument CLI against real files under an isolated `HOME`, confirm the
result label shows `<merged>` while the right header still shows `<remote>`, and
confirm the saved bytes and `.bak`. F32 shipped a visible defect through two
releases with every gate green; the technique is in that handoff's §4.

**Testability note.** `Store` cannot be constructed in a test — this is F36, and
it will bite the `app.rs` and `CompareTab` work here. Split `Store`-dependent
wrappers from testable cores as patches 4 and 6 of RFC-076 did, and say plainly
what rests on runtime evidence rather than claiming coverage that is not there.

## 7. Generated artifacts

None before RFC-078. Tests create only temporary files and must clean them
through `tempfile` ownership.

## 8. Known limitations

- The mode remains a two-way local-vs-remote workflow, not graphical diff3.
- Git determines resolution from the merged file; normal window exit remains 0.
- Windows replacement semantics require real runtime evidence in RFC-078.
- `MustMatch` keeps the existing best-effort fingerprint contract; F8 (digest
  unused at save time) and F9 (durability wording) remain M4's.

## 9. Acceptance criteria

- Compared right input and save output cannot share one ambiguous field.
- Mergetool preparation fingerprints the actual merged target.
- All six target-transition cases pass.
- A path expected to be absent is committed with no-clobber semantics, or save
  fails — never silent replacement.
- Save As never bypasses conflict checks because a path was selected.
- Normal compare still saves to the right input with its fingerprint.
- `tempfile` promotion is recorded in the threat model and re-audited.
- Review 041's C1 property holds: session resolution stays unconditional.
- Git/JJ and CLI documentation match observed behaviour.
- All gates in §6 pass, with runtime evidence for the mergetool path.

## 10. Prohibited shortcuts

- Falling back to replacement when no-clobber cannot be guaranteed.
- Constructing `Force` from a Save As path selection.
- Substituting `<merged>` content into the comparison.
- Collapsing conflict and I/O failure into one indistinguishable outcome.
- Adding a second temp-then-rename writer instead of extending `atomic_replace`.
- Moving session resolution back inside the startup branch.
- Editing `README.md` or `docs/src/users/installation.md`.
- Reporting gate results that were not observed.

## 11. Compatibility and security constraints

- Two-argument CLI remains normal compare; three-argument remains Git
  mergetool-compatible, with corrected safety.
- No settings or session schema change — RFC-076 settled that.
- MSRV 1.91 and edition 2024 unchanged.
- `tempfile` is the only dependency change; no shell is invoked and paths stay
  `PathBuf`/`OsStr`.
- This closes a file-integrity gap: conflict detection and backup must apply to
  the actual output.

## 12. Required review-request content

Standard format, plus:

1. which of the six target transitions are covered, and how the no-clobber race
   was exercised without sleeps;
2. runtime evidence for the three-argument mergetool path;
3. `audit-deps` and `cargo audit` output after the `tempfile` promotion;
4. confirmation that session resolution remains unconditional in `app.rs`;
5. what rests on runtime evidence rather than automated tests, stated plainly.
