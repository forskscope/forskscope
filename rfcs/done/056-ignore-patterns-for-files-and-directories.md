# RFC 056: Ignore Patterns for Files and Directories

**Status.** Implemented (v0.36.0)

**Tracks.** Comparison filtering. Adds user-defined ignore rules for file
extensions and directory names, applied across directory listing, digest
comparison, and recursive (deep) comparison.

**Touches.** New core module (proposed `crates/forskscope-core/src/ignore.rs`),
`crates/forskscope-core/src/dir/listing.rs`,
`crates/forskscope-core/src/dir/recursive.rs`,
`crates/forskscope-ui-dioxus/src/state.rs` (settings),
`crates/forskscope-ui-dioxus/src/ui/settings.rs` (the input UI).

## Summary

Users frequently compare project trees that contain noise: build output
(`*.o`, `*.class`), editor temp files (`*~`, `*.swp`), and whole
directories like `target/`, `node_modules/`, `.git/`. This RFC lets users
define two ignore lists in Settings:

- **file extensions to ignore** (e.g. `o`, `class`, `tmp`);
- **directory names to ignore**, where a simple wildcard is allowed
  (e.g. `target`, `node_modules`, `*.cache`, `.git`).

Ignored entries are excluded from directory listings, from digest
equality computation, and from recursive comparison. The matching engine
lives in `forskscope-core` so it is testable without the GUI and shared
by every comparison path.

## Motivation

Without ignore rules, comparing two checkouts of the same project shows
hundreds of differences in `target/` and `node_modules/` that drown out
the handful of real source changes. Every WinMerge-class tool provides
filter/ignore rules for exactly this reason. The non-goals policy permits
filters (it is listed as in-scope compare ergonomics, distinct from the
out-of-scope synchronization features).

## Goals

- A core ignore-matching engine: given an entry name (and whether it is a
  directory), decide if it is ignored.
- File-extension ignore: case-insensitive match on the extension, no dot
  required in the user's input (`o` and `.o` both mean "ignore `*.o`").
- Directory-name ignore: exact name or a single-`*` wildcard glob
  (`*.cache`, `tmp*`, `*backup*`). Case sensitivity follows the platform
  convention (case-insensitive on Windows/macOS-default, case-sensitive
  on Linux) — or, simpler and proposed for v1, always case-sensitive with
  a documented note.
- Apply ignore rules in: directory listing (explorer), digest equality
  (a directory that differs only in ignored entries is **equal**),
  recursive/deep comparison.
- Persist the lists in settings (`ConfigManager`, alongside existing
  preferences).
- Clear UX: a placeholder in the directory-name field demonstrates
  wildcard syntax (e.g. `node_modules, target, *.cache`).

## Non-Goals

- Full `.gitignore` semantics (negation, anchoring, `**` globs, per-
  directory ignore files). A single `*` wildcard covers the common cases;
  gitignore parity is a possible later RFC.
- Path-based ignore rules (ignoring `src/generated/` specifically rather
  than any directory named `generated`). v1 matches on the entry *name*,
  not its path.
- Ignoring by file size or content.

## External Design

### Data model

```rust
// forskscope-core
pub struct IgnoreRules {
    /// Lowercased extensions without a leading dot, e.g. ["o", "class"].
    pub file_extensions: Vec<String>,
    /// Directory-name patterns; each may contain a single '*' wildcard.
    pub dir_patterns: Vec<String>,
}

impl IgnoreRules {
    pub fn is_file_ignored(&self, name: &str) -> bool { /* extension match */ }
    pub fn is_dir_ignored(&self, name: &str) -> bool  { /* glob match */ }
}
```

Input normalization: a user entry of `.o`, `o`, or `O` all normalize to
the extension token `o`. Directory patterns are stored as entered.

### Wildcard semantics

A directory pattern matches a directory name if:
- the pattern contains no `*` and equals the name exactly, or
- the pattern contains one `*` acting as "any run of characters",
  anchored at both ends (so `*.cache` matches `build.cache` and
  `.cache`; `tmp*` matches `tmp` and `tmpfiles`; `*backup*` matches
  `mybackup1`).

A pattern with more than one `*` is accepted but only the first is
treated as a wildcard for v1, with the rest literal — documented as a
limitation. (Implementation may instead reject multi-`*` patterns; the
RFC leaves this to the detailed design, preferring lenient acceptance.)

### Where ignore rules apply

| Surface | Effect |
|---------|--------|
| Explorer listing | Ignored files/dirs are hidden from the pane. |
| Digest equality | Two directories equal if they differ only in ignored entries. |
| Recursive compare | Ignored entries are not walked or reported. |
| File diff (open) | No effect — opening a specific file always honors the user's explicit choice, even if its extension is in the ignore list. |

The last row is important: ignore rules filter *discovery*, never an
explicit open. If a user explicitly opens `a.o` vs `b.o`, the comparison
proceeds regardless of the ignore list.

### Settings UI (owned by this RFC)

Two inputs in the Settings dialog:

```text
Ignore
  File extensions   [ o, class, tmp                         ]
  Directory names   [ node_modules, target, *.cache         ]
                      ^ placeholder shows wildcard example
```

Comma-separated entry, trimmed and normalized on change, persisted
immediately. Empty list means "ignore nothing" (current behavior).

## Alternatives Considered

- **Per-session ignore rules instead of global settings.** Rejected for
  v1: most users want the same noise (`target/`, `node_modules/`) ignored
  everywhere. A per-session override is a reasonable later addition.
- **Full glob/gitignore engine via a crate.** Deferred: heavier
  dependency and semantics than the common case needs. A single-`*`
  matcher is a few lines and fully testable.

## Testing

- `is_file_ignored`: extension match is case-insensitive; dot-prefixed and
  bare inputs behave identically; non-matching extensions pass through.
- `is_dir_ignored`: exact match; `*`-prefix, `*`-suffix, and `*`-infix
  globs; non-matching names pass through.
- Digest equality: two dirs differing only by an ignored file are equal;
  differing by a non-ignored file are not.
- Recursive compare: ignored directories are not descended into.
- Explicit open is unaffected by ignore rules.

## Open Questions

- Case sensitivity policy (platform-aware vs. always case-sensitive).
  Proposed v1: extensions case-insensitive, directory names
  case-sensitive, documented.
- Multi-`*` pattern handling (lenient first-`*`-only vs. reject).
  Proposed v1: lenient.
