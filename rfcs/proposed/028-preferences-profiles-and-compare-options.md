# RFC 028 — Preferences, Profiles, and Compare Options

**Status.** Proposed

## Status

Proposed.

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
