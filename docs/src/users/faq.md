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
