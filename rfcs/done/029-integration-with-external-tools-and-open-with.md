# RFC 029 — Integration with External Tools and Open With

**Status.** Implemented (v0.55.0 + v0.70.0) — core complete; settings UI for custom editor commands and external-change reload flow deferred to UI layer

## Status
Partially implemented in v0.55.0. `forskscope-core::external_tool` ships:

- **`ExternalToolCommand`** — `id: ToolId`, `name`, `executable: PathBuf`,
  `args: Vec<ExternalToolArg>`.
- **`ExternalToolArg`** — `Literal(String)` or `Placeholder(ExternalToolPlaceholder)`.
- **`ExternalToolPlaceholder`** — `Path | LeftPath | RightPath | Line | Column`.
  `token()` returns the `{token}` string; `from_token()` parses it;
  `all()` returns the display-order list.
- **`ExpandContext`** — `path`, `left_path`, `right_path`, `line`, `column`,
  all `Option`. Missing values cause the corresponding argument to be **omitted**,
  not replaced with `"None"`.
- **`expand_args(cmd, ctx) → Vec<String>`** — expands the argument template
  into a concrete array. No shell string expansion. Callers pass the result
  directly to `std::process::Command::args`.
- **`parse_arg(s) → Result<ExternalToolArg, UnknownTokenError>`** — used by
  the settings UI to validate user-supplied argument strings. Rejects
  apparent `{token}` strings that are not in the supported set (protects
  against typos like `{pat}` silently becoming a literal).
- **20 tests** covering: literal pass-through, all five placeholders,
  mixed literal+placeholder, the security contract (spaces, semicolons,
  `$HOME`, backticks all arrive as single intact arguments), missing-context
  omission, `parse_arg` acceptance/rejection, token round-trips.

Remaining open: the file manager reveal command (`open_with_file_manager` in
the UI already handles this via `opener`; RFC-029 needs it wired to a
configurable `ExternalToolCommand` entry), the settings UI for custom editor
commands, and the external-change reload flow when a file is opened
externally.

## Summary

Define safe integration with external editors, terminals, file managers, and system open actions. This gives users escape hatches without turning ForskScope into a full IDE.

## Goals

- Open selected files in external editor.
- Reveal files in system file manager.
- Copy paths safely.
- Allow user-configured external diff/editor commands.
- Detect when external changes require reload.

## Non-goals

- Plugin marketplace.
- Embedded terminal.
- Git client integration.
- Arbitrary shell script automation in v3.

## User-facing actions

Context menu on file/diff:

```text
Open With...
Reveal in File Manager
Copy Path
Copy Relative Path
Reload from Disk
Compare Again
```

Directory comparison context menu:

```text
Open File Diff
Open Left Externally
Open Right Externally
Reveal Left
Reveal Right
Copy Relative Path
Plan Copy Left → Right
Plan Copy Right → Left
```

## External tool configuration

Settings page:

```text
+----------------------------------------------------------+
| External Tools                                           |
+----------------------------------------------------------+
| Default editor: [System default v]                       |
| Custom editor command: [code --goto {path}:{line}]       |
| File manager: [System default v]                         |
| Terminal: [Disabled v]                                   |
|                                                          |
| Placeholders:                                            |
|   {path} {left} {right} {line} {column}                  |
+----------------------------------------------------------+
| [Test Command] [Save]                                    |
+----------------------------------------------------------+
```

## Security policy

- Do not pass paths through a shell by default.
- Use argument arrays, not shell string expansion.
- Only allow known placeholders.
- Show the expanded command in test mode.
- Do not execute custom commands without explicit user configuration.

## API sketch

```rust
pub struct ExternalToolCommand {
    pub id: ToolId,
    pub name: String,
    pub executable: PathBuf,
    pub args: Vec<ExternalToolArg>,
}

pub enum ExternalToolArg {
    Literal(String),
    Placeholder(ExternalToolPlaceholder),
}

pub enum ExternalToolPlaceholder {
    Path,
    LeftPath,
    RightPath,
    Line,
    Column,
}
```

## External change detection

When a file is opened externally, ForskScope should watch or re-stat the file on focus return where practical.

If changed:

```text
The file changed outside ForskScope.

Left: /path/file.rs
Opened fingerprint: abc123
Current fingerprint: def456

[Ignore] [Reload] [Save As Current Buffer]
```

## Acceptance criteria

- User can reveal selected file in file manager.
- User can open selected file in default external editor.
- Custom command supports safe placeholders.
- External changes are detected before save or on reload.
- Shell injection through paths is prevented by argument-array execution.

## Test strategy

- Unit tests for placeholder expansion.
- Tests for no-shell execution where supported.
- Manual tests on Linux/Windows/macOS.
- External modification tests.

## Dependencies

- RFC 017 Error taxonomy.
- RFC 021 Document model.
- RFC 023 Atomic file operations.
- RFC 026 Cross-platform compatibility.
