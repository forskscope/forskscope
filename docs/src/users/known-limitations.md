# Known Limitations

This page documents current limitations of ForskScope and, where relevant,
the recommended workaround.

---

## Diff view

### Horizontal scroll is shared between both panes

Both panes scroll together horizontally. If the left file has a very long line
in one area and the right file has long lines in a different area, you may
need to scroll horizontally to see each side. Independent per-pane horizontal
scroll is planned (ISSUE-001, Approach A) but not yet implemented.

**Workaround:** Enable **Wrap** in the toolbar (More ▼ → Wrap: on) to wrap
long lines within each pane instead of scrolling.

---

### Three-way merge workspace not yet available

`ThreeWayMergeSession` and the conflict resolution model are fully implemented
in core (v0.40.0), but the conflict resolution UI workspace has not shipped
yet (RFC-034). Three-way merge is available as a `git mergetool` workflow
(see [Git integration](../intermediate/git-integration.md)), where the merge
is performed left-to-right and saved; it is not yet available as a
graphical conflict navigator.

---

### Inline character diff is only available for Replace hunks

Inline highlighting is computed only for lines that appear on both sides but
differ (Replace hunks). Pure insertions and deletions show the full line
without inline highlighting because there is no counterpart line to compare
against.

---

### Very large files may produce approximate diffs

Files over 64 MiB trigger a time-bounded diff with a shortened deadline.
The result is correct for the differences found but may miss some changes
deep in a very large file. A warning is shown in the diff view when this
occurs. Files over 4 MiB disable inline character diff automatically.

---

## Explorer

### GTK clipboard warning on Linux

On some Linux systems, pressing **Ctrl+C** in the diff view produces a GTK
warning in the terminal:

```
Gdk-WARNING: Error writing selection data: Error writing to file descriptor: Broken pipe
```

This is a known GTK/WebKitGTK clipboard pipe issue and does not affect
functionality. The copy operation itself may or may not succeed depending
on the desktop environment and clipboard manager. If clipboard operations
are unreliable, try installing or restarting a clipboard manager such as
`xclip`, `xsel`, or `wl-clipboard` (Wayland).

---

Background digest computation restarts when you navigate to a new directory.
There is no persistent cache across sessions. For large directory trees,
repeated navigation re-triggers the scan. Concurrent digest tasks are limited
to 32 at a time to avoid overwhelming the system on very large trees.

---

### Directory merge operations are limited

The Explorer's **Directory Report** mode supports per-file copy (both directions)
and batch copy via the **Copy N →** / **← Copy N** toolbar buttons. Each copy
shows a confirmation with the source and destination path; existing files receive
a `.bak` backup.

Delete and full directory-sync operations are not supported and are not planned
for v1 (non-goal NG-004 in the product policy).

---

## File types

### Excel comparison is derived text, not semantic

`.xlsx` comparison converts the workbook to a text representation (sheet name
+ cell address + cell value). It does not understand formulas, formatting,
charts, or macros. Two cells with the same displayed value but different
underlying formulas will show as equal.

---

### Binary merge is not available

Binary files (files with NUL bytes in the first 8 KB) show a hex preview diff
but cannot be merged or saved from within ForskScope.

---

## Platform

### Linux: display server dependency

ForskScope requires WebKitGTK 4.1 on Linux. Some distributions package
WebKit2GTK 4.0 (`libwebkit2gtk-4.0`) and 4.1 (`libwebkit2gtk-4.1`) separately;
the 4.1 variant is required. On systems with only 4.0, the binary will fail
to launch with a missing-library error.

Install on Debian/Ubuntu:
```sh
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev
```

---

### macOS: unsigned binary warning

The release binary is not notarized. macOS Gatekeeper will show a warning on
first launch. Right-click → Open to bypass it, or run:
```sh
xattr -d com.apple.quarantine ./forskscope
```

---

### Windows: long path support

On Windows, paths longer than 260 characters may cause issues if long-path
support is not enabled. Enable it via Group Policy or the registry:
`HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled = 1`.

---

## Not limitations — intentional non-goals

The following are not planned features. They are out of scope by design:

- Cloud or network file access
- Git GUI (commit, branch, push)
- Plugin system
- AI-assisted merge
- Collaborative editing

See the [non-goals policy](../../rfcs/notes/feature-completion-scope-control.md)
for the full list.
