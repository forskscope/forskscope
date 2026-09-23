# Review 107 — Request 104: 0.171.0 follow-ups

**Reviewer:** architect. **Date:** 2026-09-15. **Reviewed:** `5fc777f`, `f6f5089`, `dd07037`.
**Verdict:** **Approved. F102, F54 and F93 are closed.** I found one small
gap in the new documentation check, recorded as **F107** for 0.172.0. It does
not block 0.171.0, which is being prepared now.

## A. F102: closed

The change is what review 105 asked for:
- a per-attempt file, with the handle closed once `Popen` returns;
- the stderr tail printed on every failure path;
- no change to `find_app`.

I checked run `34937588375`'s log myself. The geometry `FAIL` is followed by
the `libEGL` stderr tail, and the log has **zero** `attempt` lines.

A1 shows the blocking before the fix and the prompt report after it, which is
the evidence that matters. For A2 you measured under 7 KiB and said so instead
of inventing a before-and-after. That was right, and it is recorded as a
latent risk, not a demonstrated one.

## B. F54: closed

I reproduced your probe, then went further. I planted the export and put its
name **only** inside the forms most likely to confuse a hand-written scanner:

- `'"'`, `'\''` and `b'"'`;
- lifetimes `<'a, 'b>` next to identifiers;
- a string with an escaped quote;
- `r#"quote " …"#`;
- a nested block comment.

The gate **failed** and named the symbol. I then added one real call
**after** all of those forms, and the gate **passed**. That shows the scanner
does not lose its place and swallow real code. The scanner falls back to
plain text for `'\x41'`/`'\u{…}'`, and neither form can contain a literal
`"`, so that fallback cannot open a false string.

## C + D. F93: closed

The tables match the disk: 11 modules. I checked the rewritten *Covers* rows
against the modules, and `StatusGlyph` exists (`explore/status.rs:38`). Both
comment corrections are right, and `all_presets` was not touched.

You found the `load_identity/tests.rs` misclassification before the check
ever ran against the tree. That is the falsification standard applied to your
own tool, and it is appreciated.

### F107: rows without backticks are skipped silently (0.172.0)

`first_backtick_cell` returns `None` for a row whose first cell has no
backticks, and `table_rows_until_next_heading` then **drops that row
silently**. My probe added `| compare/ghost_module | unbackticked stale row |
RFC-000 |` to `testing.md`, and the check **passed**.

Every current row uses backticks, so nothing is wrong today. But a stale row
written in a slightly different style would pass the check, and hiding that
kind of drift is the one thing it exists to prevent.

Relatedly, the new paragraph in `architecture.md` says *"a third silent drift
is not possible."* That claims more than the check measures, which is a
pattern this project has recorded before.

**For the 0.172.0 handoff:**
- Every table body row, meaning any row that is neither the header nor the
  separator, must yield a backticked module name. A row that doesn't is
  reported as an error.
- Falsification: the probe above must fail.
- Change the sentence to say what the check actually enforces.
