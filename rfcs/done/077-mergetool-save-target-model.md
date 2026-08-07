# RFC 077: Git Mergetool Save-Target Model

**Status.** Implemented (Milestone M3)
**Tracks.** Release-stabilization audit finding B3.
**Touches.** CLI startup requests, compare preparation, tab state, save/save-as,
reload, external-change handling, Git documentation, and integration tests.

## Summary

Compared inputs and save output are different identities. ForskScope will model
them separately.

Normal two-file mode compares left and right and saves to the right input. Git
mergetool mode compares local and remote but saves to the merged output. The
save target will carry its own load-time precondition, encoding decision, and
existence state. Async compare completion will install the compared documents
and target snapshot as one token-validated result.

## Problem

The current mergetool startup mutates `right_path` to `<merged>` immediately
after starting an async local-vs-remote load. Completion later installs a
`right_doc` loaded from `<remote>`. Save therefore uses the merged path with the
remote document's fingerprint. An existing merge output can produce a false
conflict, and external-change detection does not guard the actual target from a
stable snapshot.

## Goals

- Represent left input, right input, and save target explicitly.
- Capture metadata for the actual save target during compare preparation.
- Use target encoding/policy deliberately in mergetool mode.
- Preserve normal two-file behavior.
- Detect external changes to `<merged>` between load and save.
- Test existing, missing, appeared, deleted, changed, replaced, and unsupported
  target transitions.
- Keep Git/JJ integration local and wrapper-free.

## Non-goals

- Implement a three-way base-aware conflict workspace.
- Parse Git index stages or invoke Git automatically.
- Mark a Git conflict resolved or control Git's exit decisions.
- Persist mergetool tabs in the normal restored session.
- Change left-to-right two-way merge semantics.

## Startup request

Replace loosely coupled `STARTUP_PAIR` and `STARTUP_MERGED` state with one
request:

```rust
pub enum StartupRequest {
    Explorer,
    Compare {
        left: PathBuf,
        right: PathBuf,
    },
    MergeTool {
        local: PathBuf,
        remote: PathBuf,
        merged: PathBuf,
    },
}
```

Argument parsing must reject unsupported arity with diagnostics and a non-zero
exit rather than silently opening Explorer. `--diagnostics` remains separate.

`MergeTool` is converted into a `CompareRequest` before a tab is created:

```rust
pub struct CompareRequest {
    pub left_input: PathBuf,
    pub right_input: PathBuf,
    pub save_destination: SaveDestination,
}

pub enum SaveDestination {
    RightInput,
    Explicit(PathBuf),
}
```

## Prepared result

The blocking preparation step returns:

```rust
pub struct PreparedCompare {
    pub left: LoadedDocument,
    pub right: LoadedDocument,
    pub diff: DiffDocument,
    pub merge: MergeSession,
    pub save_target: SaveTargetSnapshot,
    pub can_save: bool,
}

pub struct SaveTargetSnapshot {
    pub path: PathBuf,
    pub state: SaveTargetState,
}

pub enum SaveTargetState {
    Writable {
        expectation: TargetExpectation,
        encoding_label: String,
    },
    Blocked {
        reason: SaveTargetBlockReason,
    },
}

pub enum TargetExpectation {
    MustMatch(FileFingerprint),
    MustBeAbsent,
}
```

Normal compare derives the snapshot from the loaded right document. Mergetool
mode inspects `<merged>` independently. An `Option<FileFingerprint>` is not an
adequate precondition because `None` conflates “the path must remain absent”
with “skip conflict checking.”

The core save request receives the same explicit contract:

```rust
pub enum TargetPrecondition {
    MustMatch(FileFingerprint),
    MustBeAbsent,
    Force,
}
```

`Force` is constructed only after an explicit overwrite confirmation. It is
never stored as the tab's load-time snapshot and is not used automatically by
Save As.

## Mergetool target preparation

### Existing regular text file

- Read enough information to classify and decode it using the same core loader.
- Record its fingerprint and encoding label.
- Do not use its content as the right-side comparison input.
- Save the merge result in the target's original encoding when representable.

### Missing path

- Record `Writable { expectation: MustBeAbsent, ... }`.
- Use the remote input's encoding as the initial output encoding.
- Saving creates the file and required parent directories through a
  no-clobber commit path. If any filesystem entry appears after preparation,
  normal save returns a conflict and preserves that entry.

### Binary, XLSX, directory, or unreadable target

- Do not silently replace it.
- Produce a blocking, user-visible target error with Save As and Cancel where
  safe. Overwrite is unavailable until the user deliberately chooses a valid
  text/new target.

### Existing conflict-marker content

It is still an existing text target. ForskScope records its encoding and
fingerprint but does not parse markers or use them as the merge model.

## Tab state

`CompareTab` retains compared input paths for headers/reload:

```rust
pub left_path: Option<PathBuf>,
pub right_path: Option<PathBuf>,
pub save_target: Option<SaveTargetSnapshot>,
pub launch_mode: CompareLaunchMode,
```

`right_path` always means the compared right/remote input. It is never mutated
to mean output.

Mergetool presentation shows a quiet, explicit output line such as
`Result: /path/to/MERGED`; it must not expose a destructive control without the
normal save guards. The tab title may retain a localized merge marker.

## Async integration

RFC-075 is a prerequisite. `PreparedCompare` is committed only for the same
tab ID/load generation that produced it. The save-target snapshot is installed
in the same commit as the compared documents, eliminating the current
post-spawn mutation race.

## Save behavior

`build_request` uses only `tab.save_target` for:

- target path;
- expected target precondition;
- output encoding.

It never reads `right_doc.fingerprint_at_load` as an implicit save target.

On success:

- replace the expectation with `MustMatch(outcome.new_fingerprint)`;
- mark the merge session saved;
- retain compared right input identity for reload.

On Save As:

- validate/inspect the selected destination and derive `MustBeAbsent` or
  `MustMatch`; selecting an existing file does not imply force;
- save with the same conflict/backup policy;
- replace `save_target` only after success;
- do not change the compared right input path.

On confirmed overwrite:

- construct `TargetPrecondition::Force` only for that deliberate request;
- create the configured backup;
- do not mutate the stored snapshot before the save succeeds.

## Core commit semantics

The precondition is checked immediately before backup/write:

- `MustMatch(fingerprint)` conflicts when the path is missing, is no longer a
  regular text target, or its current fingerprint differs;
- `MustBeAbsent` conflicts when any entry exists at the path;
- `Force` follows only a user confirmation and still rejects unsupported
  target kinds unless the user chose a different valid path.

For `MustBeAbsent`, a check followed by an overwriting rename is still racy.
The safe-file primitive therefore needs an atomic no-clobber commit on each
supported platform. The planned first implementation is a same-directory
`tempfile::NamedTempFile` written completely and committed with
`persist_noclobber`; `tempfile` moves from core dev-dependencies to normal core
dependencies. An already-existing error maps to `CoreError::Conflict`, and the
temporary file is cleaned without touching the competing target. If that API
cannot provide the required semantic on a supported platform, normal save
fails rather than falling back to replacement. RFC-078 exercises this on every
primary platform, and dependency gates run after the dependency-scope change.

`MustMatch` retains the current best-effort fingerprint preflight contract;
RFC-074 N1/N2 govern any stronger digest/durability claim. This RFC does not
claim portable filesystem transactions beyond the explicit precondition and
commit behavior above.

## Reload behavior

Reload re-reads local and remote inputs and recomputes the diff. It preserves
the save target path but must re-check the target against the previous target
snapshot:

- unchanged target: install a refreshed equivalent snapshot;
- changed target: surface reconciliation before allowing future save;
- missing/replaced target: update the blocking state visibly.

Reload must not silently adopt a changed target as the new baseline when the
session has unsaved work.

## Exit behavior

The current CLI exits 0 on normal window close. This RFC does not introduce a
Git-specific success code because Dioxus lifecycle coordination needs separate
design. Documentation must continue to state that Git inspects the merged file
and that closing without save does not certify resolution.

## Test design

Add integration tests around compare preparation and save request construction.
Use temporary paths and real file mutations; no GTK event loop or sleeps.

### Existing merged target

1. local, remote, and merged exist;
2. prepare mergetool comparison;
3. assert compared right content/fingerprint belong to remote;
4. assert save-target path/fingerprint/encoding belong to merged;
5. apply a hunk and save successfully without a false conflict;
6. assert `.bak` contains the previous merged bytes.

### Missing merged target

- snapshot expectation is `MustBeAbsent`;
- save creates merged and parent directories;
- subsequent save uses the new fingerprint.

### Target-state transition races

- prepare a missing target, create it externally, then save: conflict; the
  external bytes remain unchanged;
- prepare an existing target, delete it externally, then save: conflict; the
  path is not recreated;
- prepare an existing target, replace it with another file of different
  fingerprint, then save: conflict;
- select an existing Save As destination: overwrite confirmation is required;
  the selection itself never constructs `Force`;
- exercise the no-clobber commit with a competing creator at the core safe-file
  boundary without sleeps. A test-only before-commit seam creates the competing
  file after preparation/temp write but before `persist_noclobber`.

### Externally modified merged target

- prepare, mutate merged, then save;
- save returns external-modification conflict;
- original external bytes remain intact;
- confirmed overwrite creates backup of the externally changed version.

### Replaced/unsupported target

- replace merged file with a directory after prepare;
- save is blocked without deleting/replacing the directory;
- binary/XLSX target at startup is rejected as a save target.

### Identity and UI-state cases

- right header remains remote while result label shows merged;
- reload preserves save-target identity;
- Save As changes only save target;
- obsolete async completion cannot replace a newer target snapshot (RFC-075
  token test).

## Compatibility

- Two-argument CLI remains normal compare.
- Three-argument CLI remains Git mergetool compatible, with corrected safety.
- Existing session format does not persist mergetool tabs.
- No settings schema change is required beyond RFC-076's canonical migration.

## Security and safety impact

This closes a file-integrity gap: conflict detection and backup apply to the
actual output. No shell is invoked, paths remain `PathBuf`/`OsStr` values, and
no external network workflow is added.

## Documentation updates

- README and CLI/Git integration describe local/remote as compared inputs and
  merged as a distinct result.
- Safe-save documentation states which target is fingerprinted.
- GTK checklist includes visible result path and mergetool save.
- Known limitations do not describe the two-way workflow as a graphical
  base-aware conflict resolver.

## Implementation sequence

1. Land RFC-075 identity/generation support.
2. Add startup/request/target model and unit tests.
3. Refactor blocking preparation to return `PreparedCompare` atomically.
4. Migrate normal compare and verify no behavior change.
5. Migrate three-argument mergetool startup.
6. Change save/save-as/overwrite/reload to use `save_target` exclusively.
7. Add integration tests and documentation.
8. Execute RFC-078 platform cases, especially Windows replacement semantics.

## Acceptance criteria

- Compared right input and save output cannot share one ambiguous field.
- Mergetool preparation fingerprints the actual merged target.
- Existing/missing/appeared/deleted/changed/replaced target integration tests
  pass.
- A path expected to be absent is committed with no-clobber semantics.
- Save As never bypasses conflict checks merely because a path was selected.
- Normal compare continues saving to the right input with its fingerprint.
- Save As and reload preserve compared input identity.
- Git/JJ documentation matches observed behavior.

## Implementation outcome

Implemented across five reviewed patches plus two review-driven corrections:

- **Patches 1–2** (`fe234f5`) added the target model types (`SaveTargetSnapshot`,
  `SaveTargetState`, `TargetExpectation`, `TargetPrecondition`), the
  precondition check built on RFC-036's `check_external_state`, and the
  no-clobber commit primitive (`persist_noclobber`, `tempfile` promoted to a
  normal core dependency). Review 046 N1 — `MustBeAbsent` must not swallow a
  read failure — was fixed separately in `5500b81`.
- **Patch 3** (`064584f`) changed normal-compare preparation to return
  `PreparedCompare` atomically, verified to change no observable behavior.
- **Patch 4a** (`b387f59`) replaced `STARTUP_PAIR`/`STARTUP_MERGED` with
  `StartupRequest`/`CompareRequest`, and made argument-arity errors exit
  non-zero instead of silently opening Explorer.
- **Patch 4b** (`5be4086`) routed save/save-as/overwrite/reload through
  `tab.save_target` exclusively, eliminating the post-spawn `right_path`
  mutation that produced the false-conflict/wrong-fingerprint bug this RFC
  exists to close. **This closes finding B3.**
- **Patch 5** (`c789636`, preceded by review 048 C1/C2's fix in `b95ed39` —
  `ConfirmOverwrite` now carries the exact attempted target, and a blocked
  save destination reports instead of failing silently) added the quiet
  mergetool `Result:` presentation line, the Save-As-destination-exists
  confirmation dialog, a default-path regression review 048 had missed (Save
  As on a mergetool tab was defaulting to the remote input, not the merge
  target), and the Git/JJ/merging documentation updates.
- Review 050 §3.2's fix (`7595802`) replaced that confirmation dialog's plain
  `Path::exists()` gate with the same `inspect_save_target` classification
  `build_request` already used, so a destination that can never be written to
  (a directory, a binary file) is reported immediately instead of asking to
  overwrite something the next step would then refuse.

Acceptance criteria:

| Criterion | Status |
|---|---|
| Compared right input and save output cannot share one ambiguous field | Met — typed `CompareRequest`/`SaveDestination`, `CompareTab.launch_mode` |
| Mergetool preparation fingerprints the actual merged target | Met — `inspect_save_target` |
| Existing/missing/appeared/deleted/changed/replaced target tests pass | Partially — see below |
| A path expected to be absent is committed with no-clobber semantics | Met — `persist_noclobber`, tested and runtime-verified |
| Save As never bypasses conflict checks merely because a path was selected | Met |
| Normal compare continues saving to the right input with its fingerprint | Met |
| Save As and reload preserve compared input identity | Met — `reload_tab` derives from `launch_mode`, never touches `left_path`/`right_path` |
| Git/JJ documentation matches observed behavior | Met |

The "partially": every named target transition (missing→appeared,
existing→deleted, existing→changed, existing→replaced) is covered by
`check_precondition`'s own tests, and `check_precondition` is exactly what
`build_request`/`save_text` call at save time — not a parallel
implementation exercised only in tests. Two of the four additionally have
direct runtime evidence gathered against a running process (an existing-target
round trip, and a target replaced by a directory). What is not independently
re-verified is each transition specifically through a live mergetool process
racing against a real concurrent external actor, as opposed to the core-level
tests' direct function calls. The shared code path is the reason this is
judged sufficient rather than a gap; it is a judgment call, not something
proven to the same standard as the two runtime-evidenced transitions.

Windows replacement semantics for `persist_noclobber` — whether
`NamedTempFile::persist_noclobber` provides the same atomic no-clobber
guarantee there as on the platforms exercised so far — are explicitly
deferred to RFC-078's platform runtime matrix, per this RFC's own
"Dependencies" section.

This closes release-stabilization finding B3. It does not by itself make the
v1/public release Go: RFC-078, integrated gates, and platform evidence remain
outstanding, and finding F38 (the `0o644` permission fix in
`persist_noclobber` ignores umask, registered by review 048) remains open
against Milestone M3 — moving this RFC to `done/` records that its design has
shipped, not that M3's gate is satisfied.

## Alternatives considered

### Clear the fingerprint again after load completion

Rejected: it avoids one false conflict but leaves the actual target without a
load-time external-change baseline.

### Compare local directly against merged

Rejected: it changes the documented local-vs-remote review meaning and ignores
the explicit remote input supplied by Git.

### Treat merged as the right document

Rejected for the same reason and because it discards remote identity.

### Force-save merged without fingerprint checks

Rejected: it violates the product's primary safe-save promise.

## Dependencies

- Parent: RFC-074.
- Requires RFC-075.
- Runtime migration and target save behavior are accepted under RFC-078.
