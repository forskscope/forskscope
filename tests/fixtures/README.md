# Test Fixtures

This directory contains the corpus of test files used by `tests/diff_corpus.rs`
and any future corpus-driven tests.

Fixtures document the expected diff behavior for well-defined input cases.
They serve as both test inputs and readable specifications.

## Structure

```
fixtures/
  text/         Core text comparison pairs
  newlines/     Newline variant files (LF, CRLF, no-final-newline, mixed)
  whitespace/   Whitespace normalization test files
```

## text/

| File | Pair | What it tests |
|------|------|---------------|
| `left_identical.txt` / `right_identical.txt` | identical | No changes detected |
| `left_one_changed.txt` / `right_one_changed.txt` | one changed line | `charlie` → `CHARLIE`; tests case detection and `ignore_case` suppression |
| `left_insertions.txt` / `right_insertions.txt` | insertions | Right side has two extra lines |
| `left_deletions.txt` / `right_deletions.txt` | deletions | Right side is missing two lines |
| `left_reordered.txt` / `right_reordered.txt` | reordered blocks | Section A and B swapped |
| `left_function.txt` / `right_function.txt` | single-line code change | `return a` → `return a + 1` |
| `empty.txt` | (with `nonempty.txt`) | Empty vs non-empty comparison |
| `nonempty.txt` | (with `empty.txt`) | Non-empty vs empty comparison |

## newlines/

| File | Content |
|------|---------|
| `lf.txt` | Unix line endings (`\n`) |
| `crlf.txt` | Windows line endings (`\r\n`) |
| `no_final_newline.txt` | LF content, no trailing newline |
| `crlf_no_final_newline.txt` | CRLF content, no trailing newline |
| `mixed_newlines.txt` | Mix of LF and CRLF in the same file |

## whitespace/

| File pair | What it tests |
|-----------|---------------|
| `left_spaces.txt` / `right_extra_space.txt` | Extra space between words; tests `ignore_whitespace` |
| `left_trailing.txt` / `right_no_trailing.txt` | Trailing whitespace; tests `ignore_whitespace` |
| `tab_indent.txt` / `space_indent.txt` | Tab vs 4-space indent; tests whitespace normalization |

## Adding fixtures

When adding a new fixture pair:
1. Add the files to the appropriate subdirectory.
2. Add a test in `tests/diff_corpus.rs` that loads and exercises the pair.
3. Update this README with a description of what edge case the pair covers.
4. Prefer minimal files — the smallest input that demonstrates the case.

---

## `merge/` — Three-way merge corpus

Each scenario is a base/left/right triple (`base_*.txt`, `left_*.txt`, `right_*.txt`).
Loaded by `crates/forskscope-core/tests/merge_corpus.rs`.

| Scenario | Description |
|----------|-------------|
| `noconflict` | Left and right change different lines — both auto-merge, no conflicts |
| `conflict` | Left and right change the same line differently — one conflict produced |
| `both_same` | Left and right make identical changes — auto-merges, no conflict |
| `left_insert` | Left inserts a new line, right is unchanged — auto-merges |
| `crlf` | Base/left/right all use CRLF line endings — terminators preserved in result |
| `multi` | Left and right each change three lines (divergently) — three conflicts |

### Adding a new merge fixture

1. Create `base_<scenario>.txt`, `left_<scenario>.txt`, `right_<scenario>.txt`
   in `tests/fixtures/merge/`.
2. Add a `#[test]` in `crates/forskscope-core/tests/merge_corpus.rs` using
   `session("<scenario>")` to load the triple.
3. Add a row to the table above.
