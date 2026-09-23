# Review Request 092 — RFC-085: restore .xlsx comparison on sheets-diff 2.5.0

Handoff: `rfcs/handoffs/085-spreadsheet-comparison-resumption/022-rfc085-xlsx-resumption.md`
Commit: `d492557` (pushed to `main`)

## §3's decision — falsified, not just asserted

**`Unchanged`**: dropped entirely. Fixture `unchanged_sheet` has two sheets,
"Same" (identical both sides) and "Changed" (differs). Falsified by making
the `Unchanged` arm push a `Modified` entry instead of `continue`:

```
thread 'xlsx::tests::an_unchanged_sheet_is_dropped_from_the_sheet_list' panicked:
assertion `left == right` failed: the untouched "Same" sheet must not appear at all
  left: [Modified("Same"), Modified("Changed")]
 right: [Modified("Changed")]
```

Restored, re-ran green.

**`RenamedAndMoved`**: collapses into our `Renamed` — matching the
pre-suspension code's own choice for this exact case, not a new call. I
disclose the reasoning anyway since `SpreadsheetDiff`'s shape is frozen for
this handoff and there's no field to carry "and moved": emitting a second,
separate `Moved` entry for the same sheet would change the sheet count for
what is structurally one sheet's worth of change — worse than losing the
move fact while keeping sheet identity accurate. Fixture
`renamed_and_moved` has "Anchor" (name-matched, tab position changes —
independently confirms plain `Moved` still works) and a second sheet that
both renames and moves ("ToRename"→"RenamedSheet", position 1→0):

```
sheets: [Moved("Anchor"), Renamed { old_name: "ToRename", new_name: "RenamedSheet" }]
```

Exactly one entry for the renamed-and-moved sheet, confirmed by
`sheets-diff`'s own matcher source (`matcher.rs`'s `conservative_rename`:
exactly-one-unmatched-by-name pair, `old.index != new.index` →
`RenamedAndMoved`) before I built the fixture around it, not by trial and
error.

## §4 — cancellation, falsified for real

Fixture `large`: 51,000 cells in one sheet (just past the 50,000-cell
checkpoint, not deep past it). Cancelling the token *before* calling
`diff_xlsx` at all means the very first checkpoint must observe it:

```
uncancelled: 1.15s, Ok
cancelled:   ~1ms,  Err("... comparison was cancelled")
```

Falsified by reverting `build_options` to ignore `cancel` entirely:

```
thread 'xlsx::tests::cancellation_interrupts_a_comparison_that_crosses_the_checkpoint' panicked:
a cancelled comparison must not succeed: SpreadsheetDiff { sheets: [], cells: [], stats: ... }
```

With the token ignored, the "cancelled" run took the full ~1.3s and
returned `Ok` — the same shape as the uncancelled run, because it was one.
Restored, re-ran green.

## The two gates, fixed deliberately (§5)

**`cargo audit`**: exit 0, re-run against the real dependency now present
(not its absence). The 14 pre-existing warnings (`gtk-sys`, `glib`, `rand`,
`paste`, `proc-macro-error`, etc.) are all unrelated to
`sheets-diff`/`calamine`/`quick-xml`/`zip` — none of that chain appears
anywhere in the output. Confirmed CI's separate "Security Advisory Audit"
workflow green on this push (run 33872217902) independently of the local
run.

**`cargo xtask audit-deps`**: replaced the old `assert_package_absent`
pair (a gate that passed because the dependency didn't exist) with
`assert_immediate_dependents` for `sheets-diff` (→ `forskscope-core`
only), `calamine` (→ `sheets-diff` only), and `zip` (→ `calamine` only).
`quick-xml` needed more than a one-line change: adding this chain means
**two** `quick-xml` majors now coexist (0.39.4 via `wayland-scanner`,
already-reviewed; 0.41.0 via `calamine`), which makes `cargo tree -i
quick-xml` genuinely ambiguous by package name — the old check would have
hard-failed on that ambiguity alone, before ever evaluating whether either
path was reviewed. Generalized `assert_immediate_dependents` to resolve a
package to every version present (parsing cargo's own "specification is
ambiguous... one of the following" suggestions when that happens) and
check each version's immediate dependents independently. `.cargo/audit.toml`'s
comment, which said "the runtime XLSX parser path... has been removed," was
also stale — corrected to state which quick-xml version its ignore list
actually covers (0.39 only) and why 0.41 needs no entry there (`cargo
audit` doesn't flag it at all).

**RFC-078 P10** and **`docs/src/intermediate/file-types.md`** (handoff
§5 names `docs/src/users/file-types.md` — that path doesn't exist; the
real one is under `intermediate/`) both updated to state the restored
behavior. Historical dated evidence tables under
`docs/src/maintainers/release-evidence/0.167.0/` that reference P10's old
heading were deliberately left alone — they're frozen point-in-time
records of what a past run actually tested, not living spec.

## Disclosing scope beyond `xlsx.rs` — this had to happen, not a choice

Handoff §6 lists "any UI work" as out of scope. I want to be precise about
what that excluded and what it couldn't have, because two `forskscope-ui`
files needed real changes for restoration to be anything but dead code:

1. **`compare.rs`'s hard block.** Before this change, `load_and_diff`
   unconditionally refused any pair where either side was `.xlsx` —
   *before* ever calling anything in `xlsx.rs`. Restoring `diff_xlsx`
   alone would have left that refusal in place; nothing would ever have
   reached it. I restored the pre-suspension `&&`-gated wiring exactly
   (`git show 1156b86^:crates/forskscope-ui/src/state/compare.rs`): only
   a pair where *both* sides are `.xlsx` gets `derive_pair_text`'s output
   fed into the diff engine; a mixed pair falls through with the xlsx
   side's `diff_text()` staying empty, unchanged from before.
2. **`diff.rs`'s readonly-notice string** ("Spreadsheet comparison is
   temporarily disabled for security.") is what a user actually sees in
   the diff toolbar. Left as-is, it would have kept claiming the
   restored feature doesn't exist — the exact shape of defect this
   project's own audit (F92) exists to catch, just pointed the other
   direction. Changed to "Spreadsheet comparison is read-only." (matching
   the existing Binary-file precedent's phrasing), with `i18n.rs`'s
   Japanese translation updated to match, not left orphaned.

Neither is "UI work" in the sense of new interface — no spreadsheet view,
no new controls, nothing RFC-085's own Design section didn't already
describe as existing ("the pair projects to two text documents rendered
by the existing diff engine"). I read "any UI work" as ruling out building
something new, not un-wiring the restoration from the one path that was
disabled specifically to keep users out. Flagging this as a judgment call
rather than letting it pass silently as "just `xlsx.rs`."

## Two corrections to the handoff before they propagate

**§1's "byte-identical" claim about `load_placeholder`** — checked with a
real diff against `1156b86^`, not assumed. Everything else it named
(`derive_pair_text`, `SpreadsheetDiff`, `build_side_text`, etc.) genuinely
is byte-identical. `load_placeholder` is not: its warning field read
`LoadWarning::ExcelRenderedAsDerivedText` pre-suspension, and had been
renamed to `ExcelComparisonDisabled` during RFC-058 — a real, if small,
behavioral drift (the name became false the moment comparison is
disabled-vs-restored, and stayed a warning describing something that
isn't the reason `text` is `None`). Renamed it back; it's the only place
`load_placeholder`'s body differs from pre-suspension.

**§5's file path** — see above, `docs/src/users/file-types.md` doesn't
exist.

## Fixtures: real workbooks, not a new project dependency

All test fixtures under `src/tests/fixtures/xlsx/` are genuine `.xlsx`
files, built once with `rust_xlsxwriter` in a throwaway scratch project
(never added to any `Cargo.toml` here) and committed as binary data —
matching how `sheets-diff-rs`'s own test suite generates and commits its
fixtures. Two reasons this isn't a shortcut:

- `sheets_diff::WorkbookDiff`/`SheetDiff`/`SheetChange` are all
  `#[non_exhaustive]`, so this crate cannot construct one via struct-literal
  syntax regardless of intent — there is no way to hand-build a
  `WorkbookDiff` to drive `convert()` directly, real files are the only path.
- I did try adding `rust_xlsxwriter` as a `[dev-dependencies]` entry first.
  It shares `zip` 8.6.0 with `calamine`, which would have made
  `assert_immediate_dependents("zip", &["calamine "])` see two immediate
  dependents and fail — conflating a dev-only, never-shipped test-fixture
  tool with the real runtime parsing path `audit-deps` exists to police.
  Reverted; `forskscope-core`'s own dependency tree is untouched by fixture
  generation.

Fixture sizes: `large/{old,new}.xlsx` are ~235 KB each (51,000 cells,
single repeated value for compressibility) — deliberately just past the
50,000-cell checkpoint rather than deep past it. The rest are a few KB
each. ~530 KB total, committed.

## Verified live, not only by the test suite

Built the desktop binary, compared the `renamed_and_moved` fixture pair
for real (niri/wtype): the diff view showed the actual structural result —
`Sheet: Anchor (moved)` on both sides, `- ~ Sheet: ToRename` /
`+ ~ Sheet: RenamedSheet` — and the toolbar banner read "Spreadsheet
comparison is read-only." (the corrected string). Pressed Ctrl+S; the
target file's bytes were confirmed byte-for-byte unchanged (`md5sum`
before/after) — read-only genuinely held under a real save attempt, not
only in `save_capability`'s classification.

## Scope

Touched: `xlsx.rs` (the two functions + body), both `Cargo.toml`s +
`Cargo.lock`, `document.rs` (the `LoadWarning` correction),
`file_kind.rs` (doc-comment accuracy, no behavior change), `compare.rs` +
`compare/tests.rs`, `diff.rs` + `i18n.rs` (both disclosed above),
`xtask/src/main.rs` + `.cargo/audit.toml`, RFC-078 P10, `file-types.md`,
and `ROADMAP.md`'s top-of-file summary bullet (which stated, in present
tense, that XLSX comparison "fails closed" — now false; left the detailed
**F65** register entry itself untouched, since closing a register row with
a verdict has consistently been the architect's step in review, not mine
to pre-empt). Untouched: `build_side_text`, `SpreadsheetDiff`'s shape,
`FileKind::ExcelXlsx → EditabilityClass::ReadOnly`, `save_capability`'s
two block sites, merge/save for `.xlsx`, `keybindings.rs`,
`rfcs/accepted/085-*.md` (not moved to `done/` — that RFC's own text
says it moves there "when the work ships," which I read as the
architect's call on review, matching how RFC-060 stayed put through its
own handoff).

## Gates

`cargo fmt --check` (both the main workspace and `xtask`), `cargo clippy
--workspace --all-targets -- -D warnings`, `cargo test --workspace` (724
core / 116 ui-lib / 116 ui-bin / 200 ui-logic, all green — +7 xlsx tests
in core, +2 in ui), `cargo xtask css --check`, `version-sync`, `i18n`,
`rfc-sync`, `audit-deps`, `git diff --check`, `mdbook build docs` — all
green locally. `cargo audit` exit 0, confirmed both locally and via CI's
separate Security Advisory Audit workflow (run 33872217902, green). Main
CI run 33872217956 for `d492557` confirmed green.
