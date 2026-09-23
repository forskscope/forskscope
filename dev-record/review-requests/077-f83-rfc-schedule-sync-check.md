# Review Request 077: F83 — RFC schedule/folder sync check

**Governing task.** `dev-record/handoffs/009-f83-rfc-schedule-sync-check.md`
**Register.** F83 (fixed here). Also discharges the narrow part of F13 named in §7a.
**Baseline.** `main` at `ff40035` (docs: F83 handed off (handoff 009))
**Commit.** `ece3f52`

## An unrelated, pre-existing CI failure blocking full pipeline confirmation — read this first

**`main` is currently CI-red, since before this commit, for a reason this handoff explicitly forbids me from touching.** `ff40035` (the handoff commit itself, one before mine) already fails "Version metadata sync check" — `version 0.167.2 is already tagged; bump it`. The `0.167.2` tag sits at `36e8ee2`, four commits behind current `main`; nothing since has bumped `[workspace.package]` version. I confirmed this is not something my change caused or interacts with: same failure, same message, on the commit immediately before mine, before I touched anything.

Because that step runs before mine in the job (`css` → `version-sync` → `i18n` → `rfc-sync` → ...) and the job has no `continue-on-error`, **my new "RFC schedule sync check" CI step has not actually executed in either CI run** — it shows `-` (skipped), same as every step after `version-sync`. I have not silently claimed CI-green for a step CI never reached. What I have instead:

- **Direct local invocation**, matching CI's exact command (`cargo xtask rfc-sync`), passing on the real tree and failing correctly on all six falsifications below.
- **Confirmation the CI step is wired and would run** once `version-sync` is unblocked — it's a plain `run:` step, no conditional logic, positioned correctly.

Per §11 ("do not touch ... version-sync") I have not touched it and am not proposing to. This is not mine to fix, but leaving it unmentioned would misrepresent what "gates green" means for this request. Flagging it plainly rather than working around it or staying silent.

## The six falsifications, run against the real tree

### §8.1 — a proposed RFC with no table row

```
$ cat > rfcs/proposed/099-scratch.md   # minimal file, no Scheduling line
$ cargo xtask rfc-sync
RFC schedule sync check failed:
  - RFC 099 exists in rfcs/proposed/ but has no row in ROADMAP.md's "## Remaining proposed RFCs" table
  - rfcs/proposed/099-scratch.md has no "**Scheduling.**" line
```

Named `099` as required (also correctly caught the missing-Scheduling-line condition on the same throwaway file — expected, not a bug). Removed the file; `cargo xtask rfc-sync` passes again.

### §8.2 — a table row for an RFC that's in `done/` (the real `077` case)

```
$ # added "| 077 | F83 falsification scratch row | temporary |" to the table
$ cargo xtask rfc-sync
RFC schedule sync check failed:
  - RFC 077 has a row in ROADMAP.md's "## Remaining proposed RFCs" table but is in rfcs/done/, not rfcs/proposed/
```

`git checkout -- ROADMAP.md`; passes again.

### §8.3 — a table row for a number that exists nowhere

```
$ # added "| 999 | F83 falsification scratch row | temporary |" to the table
$ cargo xtask rfc-sync
RFC schedule sync check failed:
  - RFC 999 has a row in ROADMAP.md's "## Remaining proposed RFCs" table but does not exist anywhere under rfcs/
```

`git checkout -- ROADMAP.md`; passes again.

### §8.4 — a proposed RFC missing `**Scheduling.**`

```
$ # removed the Scheduling line from rfcs/proposed/060-*.md
$ cargo xtask rfc-sync
RFC schedule sync check failed:
  - rfcs/proposed/060-global-keyboard-scope-and-modal-input-safety.md has no "**Scheduling.**" line
```

`git checkout -- rfcs/proposed/060-*.md`; passes again.

### §8.5 — the tree as it stands passes, unmodified

```
$ cargo xtask rfc-sync
RFC schedule sync check passed: 18 RFCs agree between ROADMAP.md and rfcs/proposed/.
```

Ran before and after every falsification above, always from a clean `git status`.

### §8.6 — a renamed heading fails rather than passing vacuously

```
$ sed -i '753s/.*/## Remaining proposed RFCs RENAMED/' ROADMAP.md
$ cargo xtask rfc-sync
ROADMAP.md has no "## Remaining proposed RFCs" heading - the RFC sync check has nothing to anchor its table parse to. A renamed or removed heading must fail this check, not silently pass with zero rows found (F83 §7c).
```

`git checkout -- ROADMAP.md`; passes again.

## 1. `xtask/src/main.rs`'s growth

**+6/-1**: `mod rfc_sync;`, one dispatch arm (`Some("rfc-sync") if args.len() == 1 => rfc_sync::run(&workspace_root())`), one usage line, one doc-comment line, and `workspace_root()` changed from private to `pub(crate)` so `rfc_sync.rs` can call it — nothing else moved or was restructured. The check itself (roughly 175 lines with its doc comments) lives entirely in the new `xtask/src/rfc_sync.rs`.

## 2. Every violation is reported, not just the first

`diff_violations` collects into a `Vec`, doesn't return early on the first match; the `Scheduling`-line check is appended to the same vector before the single `if !violations.is_empty()` exit point. §8.1's falsification run above shows two simultaneous violations reported in one invocation, which is the same mechanism a maintainer moving three RFCs at once would see: all three, in one run.

## 3. When the heading is missing

`table_section` fails with a message naming what's missing and why it matters, then `run` prints it and exits 1 — before any table numbers, proposed-folder listing, or Scheduling-line check happens. It does not fall through to "zero rows found, nothing to report." §8.6 above is the direct demonstration.

## 4. Parsing approach (§7c)

Anchored to the exact heading text (`## Remaining proposed RFCs`, trimmed-line equality) and sliced to the next line starting with `## ` (or EOF). Within that slice, a line only counts as a table row if it starts with `|` **and** its first cell is non-empty and all-ASCII-digit — which is what excludes the header row (`RFC`, not digits) and separator row (`----`, not digits) without needing to special-case either, and can't match `ROADMAP.md`'s other pipe tables (findings register, milestone table) since those live outside the sliced section entirely. Unit-tested with fixture strings covering: normal extraction, the missing-heading failure, an empty-but-valid section (heading at EOF), and header/separator rows correctly excluded.

RFC numbers are carried as literal digit-string text end to end (from both the table cells and the `rfcs/*/NNN-slug.md` filenames) — no numeric parsing, no assumed digit count, per §13's explicit instruction not to hard-code today's three digits.

## 5. Scope discipline

- `ROADMAP.md` and every RFC file are exactly as they were before this commit — the six falsifications above were all reverted with `git checkout`/`rm`, confirmed by a clean `git status` before committing.
- `css`, `i18n`, `audit-deps`, `version-sync` — not touched (version-sync's pre-existing failure is reported above, not patched).
- No RFC content changes, no lifecycle moves.
- `xtask/src/main.rs`'s existing four checks — not restructured.

## 6. Executed gates

`cargo fmt --manifest-path xtask/Cargo.toml --check`, `cargo clippy --manifest-path xtask/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path xtask/Cargo.toml` (17, +11 new), `cargo fmt --check` (workspace), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (unchanged: core 697, ui-logic 257, ui 76), `cargo xtask css --check`, `cargo xtask i18n` (236 keys, unchanged), `cargo xtask audit-deps`, `git diff --check`. `cargo xtask rfc-sync` run directly (not just via CI) per §1 above. Pushed as `ece3f52`; CI blocked before reaching the new step, for the pre-existing reason described at the top of this request.
