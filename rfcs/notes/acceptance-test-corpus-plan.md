# Acceptance Test Corpus Plan

> **Note — v0.114.0 (2026-06-12).** The corpus planned here uses paths like
> `text/basic/`, `text/newline/` etc. The actual shipped corpus uses a flatter
> structure: `tests/fixtures/text/`, `tests/fixtures/newlines/`,
> `tests/fixtures/merge/`. The fixture files are documented in
> `tests/fixtures/README.md`. This planning document is preserved as a
> historical record.

## 1. Purpose

ForskScope needs a repeatable test corpus because diff/merge correctness is highly sensitive to edge cases. Manual visual testing is not enough.

The test corpus should be used by unit tests, integration tests, UI smoke tests, and manual QA.

## 2. Corpus categories

### 2.1 Text basics

```text
text/basic/identical.txt
text/basic/one-line-changed-left.txt
text/basic/one-line-changed-right.txt
text/basic/insertions.txt
text/basic/deletions.txt
text/basic/reordered-blocks.txt
```

### 2.2 Newline variants

```text
text/newline/lf.txt
text/newline/crlf.txt
text/newline/cr.txt
text/newline/mixed-newline.txt
text/newline/no-final-newline.txt
```

### 2.3 Encoding variants

```text
text/encoding/utf8.txt
text/encoding/utf8-bom.txt
text/encoding/shift-jis.txt
text/encoding/euc-jp.txt
text/encoding/invalid-utf8.bin
```

### 2.4 Whitespace and normalization

```text
text/whitespace/spaces-tabs.txt
text/whitespace/trailing-spaces.txt
text/whitespace/indentation-only.txt
text/whitespace/blank-lines.txt
```

### 2.5 Large files

```text
large/100k-lines-small-diff.txt
large/1m-lines-head-tail-diff.txt
large/long-lines.txt
large/minified-json.txt
```

### 2.6 Binary and unsupported files

```text
binary/small.bin
binary/large.bin
binary/image.png
binary/pdf.pdf
binary/excel.xlsx
```

### 2.7 Directory comparison

```text
dirs/simple-added-deleted-modified/
dirs/nested-changes/
dirs/case-conflict/
dirs/symlink-policy/
dirs/permission-differences/
```

### 2.8 Merge operations

```text
merge/copy-left-to-right/
merge/copy-right-to-left/
merge/manual-edit-then-save/
merge/conflicting-external-change/
merge/undo-redo-hunks/
```

## 3. Expected outputs

Each scenario should include:

```text
input/
expected/
  summary.json
  hunks.json
  result-left-to-right.txt
  result-right-to-left.txt
  report.md
README.md
```

`summary.json` should be stable enough to use in snapshot tests.

## 4. Test dimensions

The same corpus should be executed under these option combinations:

- default comparison,
- ignore whitespace,
- ignore line endings,
- binary detection enabled,
- large-file safe mode,
- read-only output mode,
- backup enabled.

## 5. Manual QA scripts

Each release candidate should run manual scripts for:

1. Open two files and inspect line/inline diff.
2. Copy one hunk left to right.
3. Undo and redo the operation.
4. Save with backup.
5. Reopen saved result.
6. Compare two directories.
7. Apply a batch copy to a temporary directory.
8. Export a comparison report.
9. Confirm keyboard shortcuts.
10. Confirm failure diagnostics by intentionally making one side read-only.
