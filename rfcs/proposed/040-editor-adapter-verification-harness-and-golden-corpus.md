# RFC 040: Editor Adapter Verification Harness and Golden Corpus

**Status.** Proposed

## Status
Proposed. (Originally proposed in RFC package v0.4.)

## Summary

Define a verification and test harness for the Dioxus editor adapter. The goal is to prove that editor operations, decorations, scroll synchronization, and bridge events behave consistently enough for a diff/merge tool.

## Motivation

The editor adapter is the highest-risk implementation area in the Dioxus migration. A visually working editor is not sufficient. The app needs deterministic operation replay, correct dirty state, stable line mapping, safe bridge behavior, and reliable handling of edge cases.

## Goals

- Test editor operation translation independently from full app UI.
- Maintain a golden corpus of text files and edit sequences.
- Validate decoration mapping after edits.
- Validate scroll sync event behavior.
- Detect regressions across WebView/platform changes where practical.

## Non-Goals

- Formally verify the editor implementation.
- Test every CodeMirror internal behavior.
- Replace manual exploratory UI testing.

## Golden Corpus Categories

```text
text basics:
  empty file
  single line without final newline
  final newline
  CRLF
  LF
  mixed line endings

unicode:
  Japanese text
  combining characters
  emoji
  wide characters
  invalid UTF-8 fallback cases

diff shapes:
  insert-only
  delete-only
  modified lines
  repeated lines
  moved blocks, treated conservatively
  long lines

merge shapes:
  non-conflicting changes
  same-line conflict
  adjacent changes
  manual resolution

scale:
  10k lines
  100k lines where supported
  very long single line
```

## Test Harness Design

### Core Replay Test

```rust
#[test]
fn replay_editor_operations_is_deterministic() {
    let initial = load_fixture("unicode/japanese_lf.txt");
    let ops = load_operations("unicode/japanese_lf.ops.json");
    let result_a = replay(initial.clone(), ops.clone());
    let result_b = replay(initial, ops);
    assert_eq!(result_a.digest(), result_b.digest());
}
```

### Adapter Contract Test

The adapter should be tested at two levels:

```text
Level 1: pure Rust contract test
  simulated editor event -> operation -> core ack -> view command

Level 2: WebView/editor integration test
  launch minimal Dioxus editor fixture
  inject edit sequence
  observe emitted operations
  validate final core state
```

## Bridge Event Schema

All bridge messages should be schema-validated:

```json
{
  "type": "replace",
  "document_id": "doc-1",
  "base_revision": 42,
  "range": { "start": 10, "end": 15 },
  "text": "replacement"
}
```

## Regression Reporting

Failures should produce small reproducible bundles:

```text
fixture file
operation sequence
expected final digest
actual final digest
platform metadata
editor adapter version
```

## Acceptance Criteria

- The golden corpus is part of CI.
- Pure Rust replay tests do not require WebView.
- At least one minimal integration test validates adapter event translation.
- Bridge messages are schema-validated and rejected on mismatch.
- Decoration mapping is tested after representative edits.

## Dependencies

- RFC 016 — Editor Bridge Security and Contract
- RFC 020 — Developer Architecture, CI, and Test Gates
- RFC 032 — Text Editing Operation Model
- RFC 035 — Scroll Sync and Decoration Engine
