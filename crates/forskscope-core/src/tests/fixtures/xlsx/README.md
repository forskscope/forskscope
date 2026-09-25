# `.xlsx` comparison fixtures

Each directory is one case: `old.xlsx` and `new.xlsx`, real workbooks read
through the real `sheets-diff` parser by the tests in `src/xlsx.rs`. The test
`every_fixture_pair_renders_two_different_sides_in_both_directions` walks this
directory, so a new case is covered without registering it.

`reorder_only`, `sheet_added` and `sheet_removed` (F129) were written with
`rust_xlsxwriter` 0.99.0 from a throwaway program, with a fixed creation
timestamp (2026-09-26) so regenerating gives identical files. Sheets and cells:

| case | old | new |
|---|---|---|
| `reorder_only` | `Same` (A1 "unchanged content"), `Other` (A1 "other content") | the same two sheets, tabs swapped |
| `sheet_added` | `Kept` | `Kept`, `Added` (A1 "other content") |
| `sheet_removed` | `Kept`, `Removed` (A1 "other content") | `Kept` |

`large` is byte-identical on both sides on purpose: it sizes the bounds and is
cancelled, and its rendered sides are correctly equal.
