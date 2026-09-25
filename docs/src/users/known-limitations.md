# Known Limitations

This page documents current limitations of ForskScope and, where relevant,
the recommended workaround.

---

## Diff view

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

### Inline diff skips long lines, and is off for large files

Character-level highlighting is skipped for a changed line pair when either side
is over 2,000 characters; such a row shows a **Long line** badge and the toolbar
warns that some hunks were too large. This is a bound, not a fix: minified files
(one enormous line) get no character-level highlighting at all, only the line
diff. Refining cost is quadratic in line length (release build, `similar` 3.2.0:
about 0.1 ms for a 200-character pair, 1.1–1.4 ms at 1,000, 4.3 ms at 2,000), and
the view refines every changed pair it renders, so a file with hundreds of changed
lines near the limit pauses the window when the toggle is turned on (250 pairs of
1,900 characters: 1.3 s; 1,000 pairs: 4.9 s). Lines under about 200 characters are
unaffected.

For files over 512 KiB the **Inline diff** button is disabled and the load shows
"Large file — inline diff disabled."

---

### Very large files may produce approximate diffs

Files over 4 MiB show a warning banner, and files over 64 MiB ask for
confirmation before loading. The diff itself is time-bounded at 5 seconds
regardless of file size — there is no shortened deadline for large files.
The result is correct for the differences found but may miss some changes
deep in a very large file. A warning is shown in the diff view when this
occurs.

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

### Digest computation restarts when you navigate

Background digest computation restarts when you navigate to a new directory.
There is no persistent cache across sessions. For large directory trees,
repeated navigation re-triggers the scan. Concurrent digest tasks are limited
to 32 at a time to avoid overwhelming the system on very large trees.

---

### Directory merge operations are limited

The Explorer's **Directory Report** mode supports per-file copy (both directions)
and batch copy via the **Copy to right N** / **Copy to left N** toolbar buttons.
Each copy shows a confirmation with the full source and destination path; existing
files receive a `.bak` backup. A restore manifest is written to
`$XDG_DATA_DIR/forskscope/manifests/` after every batch operation.

Delete and full directory-sync operations are not supported and are not planned
for v1 (non-goal NG-004 in the product policy).

---

## File types

### BOM-less UTF-16 is not detected

UTF-16 files are supported when they carry a byte-order mark (BOM) — the
common case for files produced by Windows tools. A UTF-16 file with **no**
BOM is not detected as text and is treated as binary instead.

This is a deliberate limitation, not an oversight: distinguishing BOM-less
UTF-16 from a genuine binary file requires a heuristic, and a wrong guess in
that direction — treating a binary file as editable text — is worse than
refusing a legitimate one. If you have a BOM-less UTF-16 file to compare, add
a BOM to it first with another tool, or convert it to UTF-8.

---

### A spreadsheet with a stray cell far from its data can close ForskScope

A workbook that has a populated cell far below or to the right of its data —
even one stray value at the bottom of the sheet — can make ForskScope use memory
in proportion to the empty area between them, and close the program if that
runs out. The file's size does not show the risk: a workbook of a few kilobytes
can do it. While it happens the comparison cannot be cancelled.

If a spreadsheet comparison hangs or ForskScope closes, look for a stray cell
far from the data (press **Ctrl+End** in the spreadsheet program to jump to the
last used cell) and compare a cleaned copy.

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
