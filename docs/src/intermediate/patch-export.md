# Patch export

ForskScope can export the difference between two files (or two directory
trees) as a standard unified-diff patch file. The patch is compatible with
`patch -p1` and `git apply`.

---

## From the UI

After opening a comparison, click **Export Patch** in the toolbar (or use
the **File → Export Patch** menu item). A save dialog opens; choose the
output path.

For a directory comparison the patch covers every differing file: modified
files produce `Modify` hunks, files that exist only on the right produce
`Add` entries, and files that exist only on the left produce `Delete`
entries.

---

## From the command line

```sh
# Export a patch from two files
forskscope --export-patch changes.patch old/src/main.rs new/src/main.rs

# Export a patch from two directories
forskscope --export-patch changes.patch old/ new/
```

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

## Patch options

| Option | Default | Effect |
|--------|---------|--------|
| Context lines | 3 | Unchanged lines kept around each change block. |
| Include file additions and deletions | on | Emit `Add`/`Delete` entries for files present on only one side. |
| Include binary file notices | off | Emit a notice line for binary files that differ but cannot be text-patched. |

---

## Notes

- Paths in the patch file always use forward slashes, making the file
  portable across Linux, macOS, and Windows.
- Files with no trailing newline are correctly annotated with
  `\ No newline at end of file`.
- The guarded **apply** workflow (preflight check, backup, atomic write)
  is planned for a future release. For now, use the system `patch` or
  `git apply` tools to apply exported patches.
