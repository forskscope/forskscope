# File Type Support

ForskScope classifies each file once when it is loaded. The classification
determines what operations are available.

---

## Classification rules

Files are classified in this order:

1. **Missing** — the path does not exist. One-sided diffs are valid; the
   missing side is treated as empty.
2. **Not a regular file** — directories, FIFOs, device nodes → `Unsupported`.
   Symlinks are followed; a symlink to a regular file is treated as that file.
3. **`.xlsx` extension** (case-insensitive) → `ExcelXlsx`.
4. **UTF-16 byte-order mark** in the first 8 KB → `Text`. Checked *before* the
   NUL-byte rule below — UTF-16-encoded ASCII is roughly half NUL bytes, and
   would otherwise always be misclassified as binary. A UTF-16 file with no
   BOM is not detected this way and falls through to the rules below (see
   [Known limitations](../users/known-limitations.md)).
5. **NUL byte in the first 8 KB** → `Binary`.
6. **Everything else** → `Text`. Encoding is detected separately.

---

## Capabilities by type

| Type | Detected by | Line diff | Inline diff | Merge / Save |
|------|-------------|-----------|-------------|--------------|
| **Text** | UTF-16 BOM, or no NUL byte in first 8 KB | ✓ | ✓ | ✓ |
| **Binary** | NUL byte found | Hex preview | — | — |
| **Excel `.xlsx`** | `.xlsx` / `.XLSX` extension | Structural (sheets/cells) | — | — |
| **Missing** | Path not found | One-sided | — | ✓ (creates the file) |
| **Unsupported** | Not a regular file | — | — | — |

---

## Text encoding

Text files may be encoded in any charset. ForskScope:

1. Checks for a byte-order mark (BOM). A UTF-8, UTF-16LE, or UTF-16BE BOM
   selects that encoding directly and is stripped before decoding.
2. Otherwise tries UTF-8 (most files on modern systems).
3. If that fails, runs `chardetng` byte-pattern detection.
4. Decodes using `encoding_rs` with the detected (or BOM-selected) charset.

The detected encoding label is shown in the status bar (e.g. `UTF-8`,
`Shift_JIS`) and in the diff toolbar's **Encoding** control. **Save preserves
the original encoding by default.** A non-UTF-8 file saves as that same
encoding.

If detection guessed wrong — the toolbar shows a label but the text renders
as mojibake — open the diff toolbar's advanced panel (**More ▼**) and pick
the correct encoding from the **Encoding** dropdown. This re-decodes the
file's bytes in place (no re-read from disk) and updates the tracked save
label to match, the same way choosing **Save as UTF-8** does below. Choosing
an encoding discards any unsaved merge changes, the same as toggling a diff
option — a confirmation is shown if the tab has unsaved edits.

If you add characters that the saved encoding cannot represent — for example,
typing an emoji into a `Shift_JIS` file — the save is **refused**, not
written. The dialog names the characters that cannot be represented and the
target encoding. Choosing **Save as UTF-8** writes the file in UTF-8 instead;
the file's tracked encoding then becomes UTF-8, so later saves of it stay in
UTF-8 too.

A separate, unrelated case: if a file could not be fully decoded when it was
*opened* — some of its bytes were not valid in the detected encoding and were
replaced with the substitution character (�) — saving that file is refused
entirely, with no offer to save as UTF-8. The bytes those substitutions stood
for were already lost when the file was read, before any edit happened, so no
encoding choice at save time can restore them. **There is no in-app recovery
for this file** — to save its original bytes, use a different tool.

A byte-order mark is preserved: a file that has a BOM when loaded (UTF-8 or
UTF-16) is stripped at load, tracked, and re-added at save; a file loaded
without one stays without one. Choosing a different encoding for a file that
had a BOM clears the tracked BOM — a BOM is a specific encoding's marker,
and carrying an old one forward after switching encodings would prepend
bytes that no longer describe what follows them.

---

## Binary comparison

Binary files are shown as a hex preview diff (byte pairs in hex). This is
read-only; merge and save are not available for binary files.

---

## Excel `.xlsx` comparison

Excel files are recognized by extension and compared structurally: added,
removed, renamed, moved, and modified sheets, and — within a modified sheet —
which cells changed value or formula. The result is projected into the same
diff view as a text comparison, so it scrolls, searches, and navigates hunks
the same way; there is no separate spreadsheet view.

Each side lists the sheets in its own tab order, and a moved sheet is shown with its
position on that side (`moved: tab 1 of 2` against `moved: tab 2 of 2`), so a
workbook whose only change is the order of its tabs still shows a difference.

**Rows are matched by content when that shows less change.** A row inserted or
deleted near the top of a sheet shifts every row below it, and positional
comparison would report each shifted cell as changed. So each sheet is also
compared with its rows matched by content, and the sheet keeps whichever result
shows fewer changed cells. Its header line says which it used:

- `Sheet1 (aligned by content: 1 inserted, 0 removed, 201 matched)`: rows were
  matched by content. The inserted row is shown on the right, at its own row
  number; a removed row is shown on the left.
- `Sheet1 (compared by position: some rows are identical)`: the sheet has rows
  that display identical values, so a match between them would be a guess, and
  positional comparison was kept. This note appears only where matching would
  otherwise have shown less change.
- `Sheet1 (compared by position: too large to align)`: the sheet is past the size
  bound for matching, and the warning above the panes says so.
- `Sheet1 (compared by position: aligning rows would report shifted formulas as
  changed)`: the sheet has formulas that refer to rows the edit moved. Excel
  rewrites those references when a row is inserted or deleted, whether or not they
  use `$`, so a matched comparison would report each of them as changed. Positional
  comparison was kept, and any real formula edit is still shown.

A sheet with no header note was compared by position, as before.

Matching is exact, so **an edited row reads as removed and re-inserted.** A row
whose content changed is not matched to its old self. A sheet where edits
dominate shows the same cells changed in place, because the matched result would
have been larger.

**A warning above the panes means the result may be incomplete or wrong in a way the
diff itself does not show** — a sheet that is not a worksheet (a chart sheet) and so
was not compared, or several sheets that could have been renamed and so were not
matched. It is one line per kind, naming the sheets, and a pair with a warning is
never reported as "Files are identical". Notes that carry no doubt (for example that
charts and images are not compared) are counted by the parser and not shown.

**Comparison is read-only.** Merge and save are not available for `.xlsx`
files, and this does not change with the above — restoring comparison did
not lift that restriction.

This applies to `.xlsx` / `.XLSX` only. `.xls`, `.csv`, and `.ods` are not
spreadsheet-comparison inputs.

---

## Large files

Very large files trigger a pre-diff prompt asking whether to proceed. The
thresholds (and what changes at each level) are:

| Class | Default threshold | Behaviour change |
|-------|------------------|-------------------|
| Small | ≤ 512 KiB | Full diff + inline diff computed eagerly |
| Medium | 512 KiB – 4 MiB | Full diff; inline diff on demand only |
| Large | 4 MiB – 64 MiB | User prompted before diff; inline diff disabled |
| Very large | > 64 MiB | User prompted; diff may be approximate |

These thresholds are controlled by `PerformanceLimits` in the `job` module
(see [Architecture](../maintainers/architecture.md)).


---

## Unsupported files

If a file cannot be classified as any of the above (e.g. a directory, a
named pipe, or a file that fails to open), the comparison shows a notice
explaining why the file cannot be diff'd. No crash occurs and other open
tabs are unaffected.
