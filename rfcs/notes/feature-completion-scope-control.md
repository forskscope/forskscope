# Feature Completion Scope Control

## 1. Why this note exists

A diff/merge application can accidentally become an IDE, a file manager, a Git client, or a general text editor. ForskScope should avoid that expansion. The product value is not “many file features.” The value is safe, understandable comparison and merge work for local files and directories.

## 2. Product identity

ForskScope should be described as:

```text
A cross-platform local diff and merge workstation tool for users who need a practical WinMerge-like experience, especially on Unix/Linux desktops.
```

It should not be described as:

```text
A code editor, repository manager, cloud collaboration tool, or project IDE.
```

## 3. Feature admission rule

A feature may enter the roadmap when it satisfies at least one condition:

1. It improves comparison accuracy.
2. It improves merge safety.
3. It improves user understanding of differences.
4. It improves recoverability after file operations.
5. It improves cross-platform reliability.
6. It reduces migration risk from the current application.

A feature should be rejected or deferred when it mainly supports unrelated editing, project management, or developer ecosystem integration.

## 4. Examples

### Accept

```text
- copy hunk left to right
- copy selected file from left directory to right directory
- ignore whitespace option
- export comparison report
- atomic save with backup
- compare session persistence
- external editor launch
```

### Defer

```text
- embedded terminal
- Git commit graph
- LSP-based symbol navigation
- multi-cursor editing
- collaborative merge session
- cloud share links
- plugin marketplace
```

## 5. Roadmap discipline

Each RFC should include a “Non-Goals” section. Any feature that cannot be cleanly defended under the feature admission rule should be moved to a future-options appendix rather than implemented in the first Dioxus migration.
