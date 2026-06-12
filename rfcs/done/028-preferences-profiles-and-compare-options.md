# RFC 028 — Preferences, Profiles, and Compare Options

**Status.** Implemented (v0.50.0 + v0.60.0 + v0.66.0) — core fully complete; toolbar profile selector UI deferred post-v1

## Status

Core implementation complete across three releases:

- **v0.50.0**: `CompareProfile` with four named presets (`default_profile`,
  `ignore_whitespace`, `code_review`, `large_file_safe`), `WhitespaceMode`,
  `NewlineCompareMode`, `CaseSensitivity`, `InlineMode`, `DiffAlgorithm`.
  `CompareProfile::to_diff_options()` maps the profile to `DiffOptions`.
- **v0.60.0**: Profile persistence via `UserSettings.diff.compare_profile`
  (profile name serialised to JSON, resolved from `all_presets()` on load;
  RFC-009 §10 fallback to default on unknown name).
- **v0.66.0**: `NewlineCompareMode::IgnoreDifference` wired into the diff
  engine — `DiffOptions::ignore_newlines` field added; `line_key()` strips the
  newline from the comparison key when the flag is set; LF and CRLF lines
  compare equal. 7 tests verify the end-to-end behaviour.

**Remaining (deferred post-v1):** toolbar profile selector UI dropdown;
user-defined custom profiles stored outside the preset list. These are purely
UI concerns; the data model and persistence are complete.

Partially implemented in v0.50.0:

- **`WhitespaceMode`**, **`NewlineCompareMode`**, **`CaseSensitivity`** — typed enums replacing bare booleans at the profile layer. All default to the "significant / sensitive" value consistent with existing `DiffOptions` defaults.
- **`CompareProfile`** — named preset (`name`, whitespace, newlines, case, inline_mode, algorithm) with four built-in presets: `default_profile`, `code_review` (Histogram algorithm), `loose_text` (ignore trailing WS + newline diff), `large_file_safe` (inline disabled). `all_presets()` returns them in display order. `to_diff_options()` bridges to the engine layer. 14 tests.

Remaining open: profile persistence (serialization, RFC-011 dependency), toolbar profile selector UI, user-defined custom profiles, and `NewlineCompareMode::IgnoreDifference` wired into the diff engine.

## Summary

Define user-facing compare options and reusable comparison profiles. This RFC turns comparison behavior into explicit, inspectable product settings rather than hidden implementation choices.

## Goals

- Provide clear compare options for text and directory comparison.
- Support named profiles.
- Persist last-used and default profile.
- Store compare options in session and report metadata.
- Avoid overwhelming users with advanced settings.

## Non-goals

- Language-specific semantic comparison.
- User-defined scripting rules.
- Per-project config files in the first release.

## Compare options

```rust
pub struct CompareOptions {
    pub whitespace: WhitespaceMode,
    pub newline: NewlineCompareMode,
    pub case_sensitivity: CaseSensitivity,
    pub encoding: EncodingCompareMode,
    pub binary: BinaryCompareMode,
    pub inline_diff: InlineDiffMode,
    pub large_file: LargeFileMode,
    pub directory: DirectoryCompareOptions,
}

pub enum WhitespaceMode {
    Significant,
    IgnoreTrailing,
    IgnoreAllWhitespace,
    IgnoreBlankLines,
}

pub enum NewlineCompareMode {
    Significant,
    IgnoreDifference,
    PreserveButDoNotMark,
}

pub enum CaseSensitivity {
    Sensitive,
    Insensitive,
}
```

## Default profiles

```text
Default
  whitespace significant
  newline significant
  case sensitive
  inline diff enabled where safe

Code Review
  whitespace significant
  newline preserve
  case sensitive
  inline diff enabled

Loose Text
  ignore trailing whitespace
  ignore newline difference
  case sensitive

Large File Safe
  line diff only
  inline diff disabled
  strict memory limit

Binary/Archive Scan
  content hash and metadata focused
  no text decode attempt unless safe
```

## UI design

Compare profile selector in toolbar:

```text
Profile: [Default v]  Options...  Recompare
```

Options dialog:

```text
+----------------------------------------------------------+
| Compare Options                                          |
+----------------------------------------------------------+
| Profile: [Default v] [Save as Profile]                   |
|                                                          |
| Text                                                     |
|   Whitespace: [Significant v]                            |
|   Newlines:   [Significant v]                            |
|   Case:       [Sensitive v]                              |
|   Encoding:   [Auto-detect, ask on ambiguity v]          |
|                                                          |
| Diff Rendering                                           |
|   [x] Inline character diff                              |
|   [x] Show whitespace-only changes                       |
|                                                          |
| Large Files                                              |
|   Mode: [Auto safe mode v]                               |
|                                                          |
| Directory                                                |
|   [x] Compare file content                               |
|   [x] Compare size/time metadata                         |
|   Symlink policy: [Do not follow v]                      |
+----------------------------------------------------------+
| [Cancel] [Apply and Recompare]                           |
+----------------------------------------------------------+
```

## Profile persistence

```rust
pub struct CompareProfile {
    pub id: ProfileId,
    pub name: String,
    pub built_in: bool,
    pub options: CompareOptions,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}
```

Profiles should be stored in local settings, not embedded in the application binary except for built-ins.

## UX rules

- Provide safe defaults.
- Advanced options are hidden behind an expandable section.
- Recompare should be explicit after option changes if recomputation is expensive.
- Active options should be visible in status or report metadata.
- If an option disables inline diff, show why.

## Acceptance criteria

- User can select a built-in profile.
- User can edit compare options and recompare.
- User can save a custom profile.
- Session stores profile/options used.
- Reports include profile/options used.
- Large-file safe profile disables expensive features.

## Test strategy

- Unit tests for option normalization.
- Snapshot tests for option serialization.
- Diff tests across whitespace/newline settings.
- UI tests for profile selection and recompare flow.

## Dependencies

- RFC 002 similar v3 diff engine.
- RFC 012 Encoding/newline/binary policy.
- RFC 013 Large-file performance.
- RFC 027 Report/export.
