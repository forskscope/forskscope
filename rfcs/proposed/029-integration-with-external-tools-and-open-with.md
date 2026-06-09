# RFC 029 — Integration with External Tools and Open With

**Status.** Proposed

## Status

Proposed.

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
