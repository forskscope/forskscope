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
4. **NUL byte in the first 8 KB** → `Binary`.
5. **Everything else** → `Text`. Encoding is detected separately.

---

## Capabilities by type

| Type | Detected by | Line diff | Inline diff | Merge / Save |
|------|-------------|-----------|-------------|--------------|
| **Text** | No NUL byte in first 8 KB | ✓ | ✓ | ✓ |
| **Binary** | NUL byte found | Hex preview | — | — |
| **Excel `.xlsx`** | `.xlsx` / `.XLSX` extension | Derived text | — | — |
| **Missing** | Path not found | One-sided | — | — |
| **Unsupported** | Not a regular file | — | — | — |

---

## Text encoding

Text files may be encoded in any charset. ForskScope:

1. Tries UTF-8 first (most files on modern systems).
2. If UTF-8 fails, runs `chardetng` byte-pattern detection.
3. Decodes using `encoding_rs` with the detected charset.

The detected encoding label is shown in the status bar (e.g. `UTF-8`,
`Shift_JIS`). **Save preserves the original encoding by default.** A non-UTF-8
file saves as that same encoding. If you add characters outside the charset,
a save guard warns you before writing.

UTF-8 BOM is preserved: a file that has a BOM when loaded will have a BOM when
saved.

---

## Binary comparison

Binary files are shown as a hex preview diff (byte pairs in hex). This is
read-only; merge and save are not available for binary files.

---

## Excel `.xlsx` comparison

Excel files are compared by converting the workbook to a text representation:
sheet names, cell addresses, and cell values are rendered as a structured text
diff. This is read-only and shows content differences only, not formatting.

Requires that both files have the `.xlsx` extension (not `.xls`, `.csv`, or
`.ods`).

---

## Large files

Very large files trigger a pre-diff prompt asking whether to proceed. The
thresholds (and what changes at each level) are:

| Class | Default threshold | Behaviour change |
|-------|------------------|--------------------|
| Medium | > 1 MiB | Inline diff deadline shortened |
| Large | > 4 MiB | Inline diff disabled, progress warning shown |
| Very large | > 16 MiB | Diff deadline shortened; result may be approximate |

These thresholds are controlled by `PerformanceLimits` in the `job` module
(see [Architecture](../maintainers/architecture.md)).

---

## Unsupported files

If a file cannot be classified as any of the above (e.g. a directory, a
named pipe, or a file that fails to open), the comparison shows a notice
explaining why the file cannot be diff'd. No crash occurs and other open
tabs are unaffected.
