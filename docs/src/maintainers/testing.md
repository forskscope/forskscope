# Testing Strategy

Tests validate **design specifications** (RFC-001 §10, RFC-002 §11), not merely
the written code.  Each test references the behaviour promised by an RFC.

## Core unit tests

Located in `crates/forskscope-core/src/tests/`:

| File | Covers |
|---|---|
| `encoding_tests` | UTF-8 detection, legacy decoding, round-trip encode, newline styles. |
| `document_tests` | File kind classification, load, fingerprint, hex preview. |
| `diff_tests` | Hunk kinds, ranges, stable IDs, newline markers, inline Unicode safety, large-file policy. |
| `merge_tests` | Apply, undo, redo, double-apply rejection, dirty state, mark_saved. |
| `save_tests` | Atomic write, `.bak` backup, conflict detection. |
| `dir_tests` | Listing sort, file digest equality, recursive directory equality. |

Run: `cargo test -p forskscope-core`

## UI build verification

`cargo build -p forskscope-ui-dioxus` is the current UI gate.  Integration and
screenshot tests are planned in RFC-020 and RFC-040.
