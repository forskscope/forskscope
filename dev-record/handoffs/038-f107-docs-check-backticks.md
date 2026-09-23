# Handoff 038 — F107: a table row without backticks must not be skipped

**From:** architect. **Date:** 2026-09-24. **Release:** 0.172.0.
**Register:** F107 in `ROADMAP.md`. **Evidence:** review 107 §"F107".

Small and self-contained. One commit is fine.

## Why

`cargo xtask ui-logic-docs` (F93's fix) compares the module tables in
`architecture.md` and `testing.md` against the modules on disk. It reads each
row's first backticked cell.

`first_backtick_cell` returns `None` for a row whose first cell has no
backticks, and `table_rows_until_next_heading` then **drops that row
silently**. I added this row to `testing.md`:

```
| compare/ghost_module | unbackticked stale row | RFC-000 |
```

and the check **passed**. Every current row uses backticks, so no document is
wrong today. But a stale row written in a slightly different style is exactly
the drift this check exists to catch, and it would pass.

## 1. Report the row instead of dropping it

In `xtask/src/ui_logic_docs.rs`:

- Every **table body row** must yield a backticked module name. A row that
  does not is a problem the check reports, naming the document and the row's
  text.
- A body row is a line starting with `|` that is neither the header row
  (`| Module | Purpose |`, `| File | Covers | RFC |`) nor the separator
  (`|---|---|`). Decide how to tell those apart, and say in a comment what
  rule you used.
- Keep reporting every problem in one run, as the check does now. Do not stop
  at the first.
- No allowlist.

## 2. Say what the check enforces

`docs/src/maintainers/architecture.md` gained this sentence with F93's fix:

> `cargo xtask ui-logic-docs` now checks this table (and `testing.md`'s)
> against the module tree on every push, so a third silent drift is not
> possible.

That claims more than the check measures — the pattern this project keeps
recording. Replace it with what is actually enforced: the module names listed
in both tables, and the heading count, must match the leaf modules on disk, and
a row the check cannot read a module name from is reported rather than ignored.

State it plainly, without the "not possible" flourish.

## Verification

1. **The probe above must now fail.** Add that exact row to `testing.md`, show
   the check failing and naming it, then revert.
2. **A row with no backticks in `architecture.md` must fail too**, since the
   two documents use different table shapes. Show it, then revert.
3. **The real tree still passes**, with no allowlist.
4. **The three F93 falsifications still fail**, so this change did not weaken
   them: delete a row; add a fictitious backticked row; change the heading
   count. Show each failing on its own.
5. Unit tests for the row-classification rule, including a header row, a
   separator row, a normal row and an unbacktacked row.

## Scope

- **In:** `xtask/src/ui_logic_docs.rs`, `docs/src/maintainers/architecture.md`
  (that sentence only).
- **Out:** the module tables' contents, `ui-logic-connectivity`, `ci.yml`
  (the check is already wired), `CHANGELOG.md` and `ROADMAP.md`, RFC-080.

## Gates

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and the Windows-target clippy now in `ci.yml`.
- `cargo test --workspace` and `cargo test --manifest-path xtask/Cargo.toml`.
- `cargo xtask` with `ui-logic-docs`, `ui-logic-connectivity`, `i18n`,
  `rfc-sync`, `audit-deps`, `version-sync`.
- `mdbook build docs`.
- A green CI run for the final commit, with its run ID.

Reply in `dev-record/review-requests/110-f107-docs-check-backticks.md`.
