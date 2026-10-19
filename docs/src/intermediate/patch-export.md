# Exporting a comparison

ForskScope can save what a comparison found, as an **HTML report** or a
**unified-diff patch**, from a file comparison or a folder comparison
(RFC-092).

- The **HTML report** is one self-contained file: no scripts, nothing
  loaded from elsewhere. It records the SHA-256 of every file it describes.
- The **unified-diff patch** is compatible with `patch -p1` and
  `git apply`.

---

## From the UI

- **A file comparison:** open it, click **More ▼** in the toolbar, then
  **Export report…** (beside the existing **Export patch**).
- **A folder comparison (Deep Compare):** its own toolbar offers
  **Export report…** and **Export patch…** directly.
- **A spreadsheet or binary comparison** shows **Export report…** disabled
  — reports cover text comparisons for now.
- Either way, a dialog opens:
  - **Format:** *HTML report* (the default) or *Unified diff (.patch)*.
  - **More options ▸** (collapsed by default): the **Paths** choice — *File
    names only* (the default), *paths relative to the compared folders*,
    or *full paths*. Full paths never appear in a report unless chosen
    here.
- A save dialog follows. For a file comparison the default filename is
  `<right file name>-report.html`; for a folder comparison,
  `<right folder name>-report.html`. The patch keeps its own default name.
- After saving, a notice offers **Open**, which opens the file in its
  system default program.
- If the compared file(s) changed since the comparison was made, or a file
  comparison has unsaved edits, the export refuses — compare again (or
  save/undo first), then export.

---

## What the checksums prove, and what they do not

Each file's SHA-256 sits beside its name in the report. Run `sha256sum` on
the file to confirm the report describes exactly that file's bytes. This
does **not** prove the report itself was not altered after it was saved —
no unsigned file format can promise that on its own.

---

## Printing a report to PDF

The HTML report is written to print cleanly: every change keeps its `−`/`+`
mark and a visible label even without colour, and a print stylesheet keeps
a change's lines together where it can. Open the saved `.html` file in any
browser and use **Print → Save as PDF**.

---

## Applying the patch

The generated file uses the unified-diff format with `git`-compatible
headers, including git's own path-quoting conventions for names
containing spaces or special characters. Apply it with the system `patch`
tool:

```sh
patch -p1 < changes.patch
```

Or with Git:

```sh
git apply changes.patch
```

---

## Notes

- A folder comparison's patch covers every entry Deep Compare could read;
  an entry it could not read fails the whole export, naming it, rather
  than producing a patch that looks complete and is not.
- A folder comparison's HTML report caps the files whose changes it shows
  in full at 50 files or 5 MiB of change text, whichever comes first —
  every file left out by the cap is still listed, by name, never silently
  omitted. The patch export has no such cap.
- Paths in the patch file use the original file names from the comparison.
- Files with no trailing newline are correctly annotated with
  `\ No newline at end of file`.
- Context lines follow the **Context lines** setting (default: 3).
