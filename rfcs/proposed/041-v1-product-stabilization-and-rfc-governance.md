# RFC 041: v1.0 Product Stabilization and RFC Governance

**Status.** Proposed — checklist updated v0.87.0

## Current state (v0.87.0)

### Must-Stabilise targets — all complete

| Target | Status |
|---|---|
| Session schema versioning | ✓ `VersionedEnvelope` + `SESSION_SCHEMA_VERSION=1` (v0.51.0, v0.56.0) |
| Document operation model | ✓ `TextEditOperation` + `RevisionId` + `EditTransaction` (v0.62.0) |
| Save and backup safety | ✓ Atomic write + `BackupPolicy` + `ExternalFileState` (v0.27.0, v0.53.0) |
| Dirty/external-change semantics | ✓ `check_external_state` + `blocks_save()` (v0.53.0) |
| Command IDs and shortcut registry | ✓ `CommandRegistry` + 25 `cmd::*` const IDs (v0.63.0) |
| Basic two-way text diff/merge | ✓ Core shipped; UI wiring is the remaining work |
| Directory comparison basics | ✓ `DirectoryIndex` + `EqualityEvidence` + `pair_entries` (v0.58.0) |
| Editor adapter safety boundary | ✓ `TextEditOperation` revision contract; UI adapter is remaining work |

### Release readiness checklist (v0.87.0)

```text
Product:
  [ ] Two-way file compare works end to end         (UI wiring remaining)
  [x] Result buffer save works with backup policy
  [ ] Directory compare works for practical trees   (UI wiring remaining)
  [ ] Basic keyboard navigation is complete         (UI wiring remaining)

Safety:
  [x] Save checks external modifications            (check_external_state)
  [x] Undo/redo covers merge operations             (TransactionLog + ThreeWayMergeSession)
  [x] Error messages are actionable                 (AppError + RecoveryAction)
  [x] Large-file mode prevents UI lockups           (FileSizeClass + PerformanceLimits)

Engineering:
  [x] Core tests pass                               (599 unit + 2 integration + 6 doctest, 0 failures)
  [x] ui-logic tests pass                           (189 unit tests, 14 modules, 0 failures)
  [ ] Editor harness tests pass                     (RFC-040 deferred)
  [ ] Packaging smoke tests pass                    (RFC-010 deferred)
  [x] Session schema migration tests pass           (persist_tests, session_tests)

Documentation:
  [x] Architecture and testing docs current
  [ ] User guide covers common workflows            (RFC-030 deferred)
  [ ] Recovery/backup behavior documented
  [ ] Known limitations documented
```

### RFC inventory at v0.78.0

**Done: 38** — all core data-layer RFCs complete.
**Proposed: 10** — editor adapter track (4), platform/packaging (2), process/governance (3), documentation (1).

The remaining 10 proposed RFCs are all correctly scoped to the UI implementation phase or are process documents. No further core-only RFC work is needed before beginning UI implementation.


### Must-Stabilise targets — status

| Target | Status |
|---|---|
| Session schema versioning | ✓ `VersionedEnvelope` + `SESSION_SCHEMA_VERSION=1` shipped (v0.51.0, v0.56.0) |
| Document operation model | ✓ `TextEditOperation` + `RevisionId` + `EditTransaction` shipped (v0.62.0) |
| Save and backup safety | ✓ Atomic write + `BackupPolicy` + `ExternalFileState` shipped (v0.27.0, v0.53.0) |
| Dirty/external-change semantics | ✓ `ExternalFileState`, `check_external_state`, `blocks_save()` shipped (v0.53.0) |
| Command IDs and shortcut registry | ✓ `CommandRegistry` + all `cmd::*` const IDs shipped (v0.63.0) |
| Basic two-way text diff/merge workflow | ✓ Core shipped; UI wiring is the remaining work |
| Directory comparison basics | ✓ `DirectoryIndex` + `EqualityEvidence` + `pair_entries` shipped (v0.58.0) |
| Editor adapter safety boundary | ✓ `TextEditOperation` revision contract + `CommandRegistry` shipped; UI adapter is remaining work |

### Release readiness checklist

```text
Product:
  [ ] Two-way file compare works end to end (UI wiring remaining)
  [x] Result buffer save works with backup policy
  [ ] Directory compare works for practical project trees (UI wiring remaining)
  [ ] Basic keyboard navigation is complete (UI wiring remaining)

Safety:
  [x] Save checks external modifications (check_external_state)
  [x] Undo/redo covers merge operations (TransactionLog + ThreeWayMergeSession)
  [x] Error messages are actionable (AppErrorKind + UserMessage + RecoveryAction)
  [x] Large-file mode prevents UI lockups (FileSizeClass + PerformanceLimits)

Engineering:
  [x] Core tests pass (536 unit + 2 integration = 538 core tests, 0 failures)
  [ ] Editor harness tests pass (RFC-040 not yet implemented)
  [ ] Packaging smoke tests pass (RFC-010 not yet implemented)
  [x] Session schema migration tests pass (persist_tests, session_tests)

Documentation:
  [x] Architecture and testing docs updated to v0.64.0
  [ ] User guide covers common workflows (RFC-030 not yet implemented)
  [ ] Recovery/backup behavior is documented
  [ ] Known limitations are documented
```

## Status
Proposed. (Originally proposed in RFC package v0.4.)

## Summary

Define the stabilization policy for ForskScope v1.0 and the governance rules for accepting, deferring, or rejecting further RFCs.

## Motivation

The migration scope has grown from a framework change into a serious product redesign. Without explicit stabilization rules, the project risks endless feature expansion before v1.0. This RFC protects product coherence and release readiness.

## Goals

- Freeze critical contracts before v1.0.
- Define what must be stable for users.
- Define what may still evolve internally.
- Prevent late-stage scope creep.
- Establish RFC states and acceptance requirements.

## Non-Goals

- Freeze every internal implementation detail.
- Prevent post-v1 innovation.
- Promise full WinMerge feature parity in v1.0.

## v1.0 Stability Targets

### Must Stabilize

```text
- session schema versioning
- document operation model
- save and backup safety behavior
- dirty/external-change semantics
- command IDs and shortcut registry
- basic two-way text diff/merge workflow
- directory comparison basics
- editor adapter safety boundary
```

### May Evolve After v1

```text
- advanced three-way merge heuristics
- VCS provider depth
- patch apply sophistication
- report templates
- theme details
- performance optimizations
- optional future Iced UI backend
```

## RFC States

```text
Draft
  idea captured, not accepted

Accepted
  design direction approved, implementation may begin

Implemented
  merged and covered by tests

Deferred
  valuable but not needed for current milestone

Rejected
  intentionally out of scope or superseded

Reopened
  accepted/implemented RFC needs correction due to evidence
```

## Acceptance Requirements

Each implementation RFC must include:

```text
- user-facing behavior
- internal model impact
- data persistence impact
- security/safety impact
- accessibility impact where UI is affected
- tests/acceptance criteria
- migration notes from current Tauri/Svelte behavior
```

## Scope Control Rules

1. v1.0 prioritizes trustworthy two-way diff/merge over broad editor features.
2. Three-way merge may ship only if unresolved conflict safety is complete.
3. VCS integration must remain optional and read-mostly before v1.0.
4. Patch apply must not ship without preflight and backup policy.
5. Any feature that can overwrite files requires a dedicated safety review.

## Release Readiness Checklist

```text
Product:
  [ ] Two-way file compare works end to end
  [ ] Result buffer save works with backup policy
  [ ] Directory compare works for practical project trees
  [ ] Basic keyboard navigation is complete

Safety:
  [ ] Save checks external modifications
  [ ] Undo/redo covers merge operations
  [ ] Error messages are actionable
  [ ] Large-file mode prevents UI lockups

Engineering:
  [ ] Core tests pass
  [ ] Editor harness tests pass
  [ ] Packaging smoke tests pass
  [ ] Session schema migration tests pass

Documentation:
  [ ] User guide covers common workflows
  [ ] Recovery/backup behavior is documented
  [ ] Known limitations are documented
```

## Iced Reconsideration Gate

Iced should be reconsidered only if all conditions are met:

```text
- Dioxus/WebView shows unacceptable platform risk
- core/editor operation model is stable
- a viable Iced editor widget/prototype exists
- migration cost is estimated with evidence
- the Iced path does not regress merge safety
```

## Acceptance Criteria

- All existing RFCs are assigned a state before v1.0 planning freeze.
- v1.0 must-have and may-evolve lists are approved.
- New feature RFCs after freeze require explicit milestone justification.
- Safety-affecting RFCs require acceptance tests before implementation is marked done.
