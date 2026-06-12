# Frequently Asked Questions

## Why can't I save after merging?

The **Save** button is only available when both files are plain text. If one
side is a binary file, a spreadsheet, or a missing path, the comparison is
read-only. A notice below the toolbar explains which case applies.

For binary files you can still compare the hex preview — you just can't merge
and save the result.

---

## How do I undo a merge?

Press **Ctrl+Z** (or the **Undo** button in the toolbar). Each merge action is
individually undoable. **Ctrl+Y** redoes the most recently undone action. The
full undo/redo history is preserved until you close the tab or reload the files.

---

## The diff looks wrong — it's splitting lines I expect to stay together

Try a different algorithm. **Myers** (the default) sometimes produces
unintuitive alignments on repetitive code. Switch to **Histogram** or
**Patience** in the advanced toolbar (More ▼ → algorithm dropdown).
Histogram is usually the most human-readable choice for structured code.

---

## My files are identical but ForskScope shows differences

Check whether whitespace differences are the culprit:
- Enable **Ignore WS** in the advanced toolbar and see if the diff disappears.
- Check the encoding shown in the status bar — `UTF-8` vs `UTF-8 BOM` or
  `Windows-1252` means the files have different encodings.
- On Windows vs Linux, files may have `\r\n` vs `\n` line endings. The diff
  will show each line as replaced. Enable Ignore WS, which strips all
  whitespace including `\r`, or use a compare profile that normalises endings.

---

## How do I compare files with different names?

1. Click a file in the left pane to select it (single click — it doesn't need
   to have a counterpart on the right).
2. Click a file in the right pane.
3. Click **Compare** in the bar below the panes.

---

## The deep compare is still "scanning" and seems slow

Deep compare runs file-system digest comparisons for every common file across
both trees. For a project with thousands of files this can take seconds. The
"checking N/total…" counter shows progress. Large binary files (images, object
files) slow down the digest phase most.

You can still browse the partial results while it completes — files that have
finished comparing show their status immediately.

---

## Can I use ForskScope with JJ (Jujutsu)?

Yes. Add to your JJ config:

```toml
[ui]
diff.tool = ["forskscope", "$left", "$right"]
merge-tool = ["forskscope", "$left", "$right", "$output"]
```

See [Git Integration](./intermediate/git-integration.md) for full details.

---

## My session was not restored after restarting

Session restore only works when you launch ForskScope **without arguments**. If
you pass file paths on the command line (e.g. from `git difftool`), the session
is not restored — the explicit arguments take priority.

Also, session restore silently skips tabs where both paths no longer exist. If
only one side is gone the tab opens with that side showing an empty document.

---

## How do I report a bug?

1. Open the **About** panel (ℹ button in the header).
2. Click **Copy diagnostics** to copy version and platform info.
3. Open an issue on the project repository and paste the diagnostics.

---

## How do I export a patch file?

Open a comparison with changes, then click **More ▼** in the toolbar to expand
the advanced options. Click **Export patch**. A save dialog will open — choose
a location and filename (it defaults to `<filename>.patch`). The output is a
standard unified-diff patch that can be applied with `patch -p1` or
`git apply`.

If the two files are identical, the Export patch button does nothing; there
are no changes to export.

---

## Why does Linux require GTK/WebKitGTK?

ForskScope's UI is built with Dioxus Desktop, which renders its interface
using a WebView. On Linux, WebView is provided by WebKitGTK 4.1 — a GTK
library. This means GTK3 and WebKitGTK runtime libraries must be installed
for the app to launch.

Install on Debian/Ubuntu:
```sh
sudo apt-get install libwebkit2gtk-4.1-0 libgtk-3-0
```

On Fedora/RHEL:
```sh
sudo dnf install webkit2gtk4.1 gtk3
```

If you see a "missing shared library" error at startup, the WebKitGTK package
for your distribution is likely version 4.0 instead of 4.1. Check with
`apt-cache search webkit2gtk` or equivalent.

For blank-window issues, NVIDIA DMA-BUF workarounds, Wayland/X11 fallback,
and other platform-specific startup problems, see the full
[Troubleshooting guide](troubleshooting.md).

---

## Can I compare PDF or Word documents?

Not in the current release. ForskScope compares text files, Excel `.xlsx`
workbooks (read-only), and shows a hex preview for binary files. PDF, Word
(`.docx`), PowerPoint, and similar document formats are not supported.

For Word documents, consider exporting to plain text first, then comparing
the text files. For PDF, the same approach (extract text with a CLI tool)
can work.

See [File type support](../intermediate/file-types.md) for the complete list.

---

## What do the ✓ and ⚠ icons in the Explorer mean?

These are **digest comparison** indicators shown when the same filename exists
in both the left and right panes:

| Icon | Meaning |
|------|---------|
| **✓** | Identical — the file content is byte-for-byte the same on both sides |
| **⚠** | Different — the file content differs |
| *(no icon)* | File exists only on one side, or the digest has not yet been computed |
| **⊙** | Comparison is still running in the background |

Digest comparison runs in the background; on large directories the icons
appear progressively as files are scanned. Double-click any file showing **⚠**
to open an immediate comparison.
