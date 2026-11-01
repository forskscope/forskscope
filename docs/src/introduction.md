# ForskScope Documentation

ForskScope is a local-first, cross-platform diff and merge tool for files and
directories.  It is designed for developers and operators who want a practical
WinMerge-style experience on Unix/Linux, macOS, and Windows without being
pulled into an IDE or a Git GUI.

**It also works with AI agents, and leaves the decisions to you.** An agent
can compare files and folders through `forskscope compare` or ForskScope's MCP
server, but it cannot change a file. Its reports and patches are the same ones
the app exports, and it can open the window on what it found. You review and
merge there. See
[Using ForskScope from scripts and AI agents](./intermediate/scripts-and-agents.md).

This documentation is organized by audience:

- **New users** — start with [Quick start](./users/quick-start.md) for a
  five-minute walkthrough of the core workflow.
- **Intermediate users** — covers file types, diff options, the keyboard
  reference, Git, and using ForskScope from scripts and AI agents.
- **Maintainers and contributors** — architecture, design rationale, and local
  development procedures.
