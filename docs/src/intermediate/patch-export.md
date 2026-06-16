# Patch export

ForskScope can export the difference between two files as a standard
unified-diff patch file. The patch is compatible with `patch -p1` and
`git apply`.

---

## From the UI

After opening a file comparison, click **More ▼** in the toolbar to expand
the advanced section, then click **Export patch**. A save dialog opens;
choose the output path. The default filename is `<filename>.patch`.

If the two files are identical, the **Export patch** button does nothing
(there are no changes to export).

---

## Applying the patch

The generated file is a standard POSIX unified diff with `git`-compatible
headers. Apply it with the system `patch` tool:

```sh
patch -p1 < changes.patch
```

Or with Git:

```sh
git apply changes.patch
```

---

## Notes

- The patch covers only the current file comparison (not directory trees).
  Directory-level patch export is planned for a future release.
- Paths in the patch file use the original file names from the comparison.
- Files with no trailing newline are correctly annotated with
  `\ No newline at end of file`.
- Context lines follow the **Context lines** setting (default: 3).
