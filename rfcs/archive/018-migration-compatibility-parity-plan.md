# RFC-018 — Migration Compatibility and Parity Plan

**Status.** Withdrawn — migration complete; superseded by delivered implementation

## 1. Summary

**This RFC is archived.** The Dioxus migration is complete through v0.40.0.
Parity with the v0.22 baseline has been demonstrated by the shipped feature
set (RFC-001–007, RFC-021, RFC-033, RFC-039, RFC-054–057). The formal
parity-proof approach described below was not needed; incremental vertical
slices and test coverage proved correctness more practically. Archived per
RFC-000 §"Granularity of transitions".

---

*Original content preserved below for historical reference.*

---

This RFC defines how the project should prove that the Dioxus migration preserves intended behavior from the current Tauri/Svelte application while allowing intentional improvements.

The migration must not rely on visual similarity alone. It needs fixtures, compatibility checks, and an intentional-change register.

## 2. Motivation

The existing app already encodes many decisions:

- line diff grouping;
- inline character diff behavior;
- binary display behavior;
- Excel diff rendering;
- directory listing order;
- binary-comparison-only flags;
- startup path handling;
- merge action expectations.

When moving to Dioxus and `similar` v3, behavior may change. Some changes are desirable. Silent changes are not.

## 3. Goals

- Define a parity fixture suite.
- Capture current behavior before replacing it.
- Separate preserved behavior from intentional changes.
- Provide migration gates.
- Enable CLI/test-only comparison independent of UI.

## 4. Non-Goals

- This RFC does not require pixel-perfect UI parity.
- This RFC does not freeze every current bug as required behavior.
- This RFC does not require Tauri and Dioxus to coexist permanently.

## 5. Parity Categories

| Category | Preserve? | Notes |
|---|---:|---|
| Text file line diff | Yes, unless improved intentionally | Must document changed hunk grouping |
| Inline character diff | Mostly | May change due to `similar` v3 or algorithm options |
| Binary classification | Yes | Must avoid binary corruption |
| Excel comparison | Preserve if retained | Can be read-only in first Dioxus version |
| Directory listing sort | Yes | Stable user expectation |
| Startup two-path behavior | Yes | Should be wired more completely |
| Save behavior | Improve | Must become safer than current behavior |
| Merge model | Improve | Must become core-owned and undoable |
| Visual CSS details | No | Dioxus design may differ |

## 6. Fixture Structure

```text
tests/fixtures/parity/
  text-basic/
    old.txt
    new.txt
    expected-lines.json
  text-japanese-shiftjis/
    old.sjis
    new.sjis
    expected-meta.json
  newline-crlf/
    old.txt
    new.txt
    expected-newline.json
  binary-null/
    old.bin
    new.bin
    expected-kind.json
  excel-basic/
    old.xlsx
    new.xlsx
    expected-summary.json
  directory-basic/
    old/
    new/
    expected-tree.json
```

## 7. Golden Output Model

Golden output should not expose unstable UI details. Prefer normalized core output.

```json
{
  "schemaVersion": 1,
  "kind": "text-diff",
  "leftEncoding": "UTF-8",
  "rightEncoding": "UTF-8",
  "hunks": [
    {
      "tag": "equal",
      "leftStart": 1,
      "rightStart": 1,
      "leftLineCount": 2,
      "rightLineCount": 2
    }
  ]
}
```

## 8. Intentional Change Register

Every behavior change must be recorded:

```markdown
## ICR-0001 — Save behavior becomes preflight-guarded

Old behavior:
- Save command writes content directly using selected charset.

New behavior:
- Save command creates a save preflight plan and blocks unsafe writes.

Reason:
- Prevent data loss and external modification overwrite.

Migration impact:
- Users see an additional dialog in risky cases.
```

## 9. Migration Gates

### Gate 1 — Current Behavior Capture

Before replacing code, create fixtures from the current app/core.

### Gate 2 — Core Parity

The new core must pass fixture tests or produce an intentional-change entry.

### Gate 3 — UI Workflow Parity

Dioxus UI must support the same main workflows:

- open file pair;
- open directory pair;
- open diff from explorer;
- navigate hunks;
- copy hunk;
- save guarded result;
- handle binary/unsupported files.

### Gate 4 — Release Candidate Parity

Manual QA must verify cross-platform behavior on Linux, Windows, and macOS if release assets target those platforms.

## 10. Compatibility CLI

Provide a developer command:

```text
forskscope-core parity --fixture tests/fixtures/parity/text-basic
```

Expected result:

```text
PASS text-basic lines
PASS text-basic inline
PASS text-basic metadata
```

## 11. Test Matrix

| Feature | Fixture | Required Before |
|---|---|---|
| UTF-8 diff | text-basic | M1 |
| Japanese encoding | text-japanese-shiftjis | M2 |
| Newline preservation | newline-crlf | M2 |
| Binary classify | binary-null | M1 |
| Excel render | excel-basic | M5 if retained |
| Directory compare | directory-basic | M5 |
| Save conflict | external-modification | M6 |
| Editor bridge edit | editor-edit-basic | M4 |

## 12. Acceptance Criteria

- Parity fixture suite exists before removing the old implementation.
- New core can run fixture tests without Dioxus.
- Intentional behavior changes are documented.
- Release candidate cannot ship with unexplained parity failures.

## 13. Risks

| Risk | Severity | Mitigation |
|---|---:|---|
| Old behavior not captured before rewrite | High | Gate migration on fixture capture |
| Golden files overfit unstable internals | Medium | Normalize output schema |
| Intentional changes become excuses | Medium | Require reason and user impact |
| Excel behavior is hard to preserve | Medium | Mark read-only/deferred explicitly |

## 14. Open Questions

- Should parity fixtures be generated from the existing Tauri backend or manually curated?
- Should the old app remain buildable during migration until Gate 3?
- How many real-world sample files can be included without licensing/privacy concerns?
