# `.xlsx` comparison fixtures

Each directory is one case: `old.xlsx` and `new.xlsx`, real workbooks read
through the real `sheets-diff` parser by the tests in `src/xlsx.rs`. The test
`every_fixture_pair_renders_two_different_sides_in_both_directions` walks this
directory, so a new case is covered without registering it.

`reorder_only`, `sheet_added`, `sheet_removed` (F129), `stray_far_cell` (F123, F130) and
`chart_sheet_not_compared`, `ambiguous_rename`, `duplicate_row_keys` (F131) were written with
`rust_xlsxwriter` 0.99.0 from a throwaway program, with a fixed creation
timestamp (2026-09-26) so regenerating gives identical files. Sheets and cells:

| case | old | new |
|---|---|---|
| `reorder_only` | `Same` (A1 "unchanged content"), `Other` (A1 "other content") | the same two sheets, tabs swapped |
| `sheet_added` | `Kept` | `Kept`, `Added` (A1 "other content") |
| `sheet_removed` | `Kept`, `Removed` (A1 "other content") | `Kept` |
| `chart_sheet_not_compared` | `Data` (A1 "before", A2 1) and chart sheet `Chart1` | the same with A1 "after" |
| `ambiguous_rename` | `Alpha`, `Beta` | `Gamma`, `Delta` (two removed, two added, none matchable) |
| `duplicate_row_keys` | `Keyed`: column A `k`, `k`, `x`; column B one, two, three | the same with "TWO" (a duplicate key; only a keyed alignment reports it) |
| `stray_far_cell` | `Data`: A1 "origin" and XFD1048576 "x" (5.3 KB) | the same with "y" |

`large` is byte-identical on both sides on purpose: it sizes the bounds and is
cancelled, and its rendered sides are correctly equal.
