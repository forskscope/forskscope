# Review Request 071: F74 follow-up — real-path coverage and accessible labels

**Governing task.** `dev-record/handoffs/003-f74-followup-real-path-coverage-and-aria.md`
**Governing review.** `dev-record/reviews/072-f74-explorer-directory-status-review.md`
**Register.** F74 (returned by review 072, addressed here). F75, F76 — out of scope, untouched.
**Baseline.** `main` at `a462f31`
**Commit.** `afac676`

## The required answer, first

> **State plainly whether reintroducing `if cp.is_dir() { DigestState::Equal }`
> at the call site now fails a test, and paste the failure.**

**Yes.** Restored the exact `9f355c6` defect at the call site inside
`classify_entry` (the function that now *is* the call site — see §1), ran
the suite, and got:

```
thread 'ui::view::explorer::tests::real_directories_with_the_same_name_and_differing_contents_are_not_compared' panicked at crates/forskscope-ui/src/ui/view/explorer.rs:551:9:
assertion `left == right` failed: a same-named directory whose contents differ must never be classified Equal - see 9f355c6's original defect
  left: Final(Equal)
 right: Final(NotCompared)
```

Restored the fix immediately after; the suite is green again. This is the
one falsification review 072 said §2 should have been able to answer, and
couldn't.

## 1. §7a — the classification now lives where a test can reach it

The digest block's per-entry decision was inline in the `use_effect`
closure — unreachable without a `VirtualDom`. Extracted `classify_entry(rel:
&Path, is_dir: bool, l_root: &Path, r_root: &Path) -> EntryClassification`,
where `EntryClassification` is either `Final(DigestState)` (insert
immediately) or `NeedsDigest { left_abs, right_abs }` (a file present on
both sides — the one case that can't resolve synchronously; the caller
inserts `Computing` and starts the real digest comparison exactly as
before). The closure is now a `match` over this result, nothing more.

This is the exact call site `9f355c6` had wrong — not a re-implementation
alongside it. `dir_common_state` is unchanged and still called from inside
`classify_entry`'s directory branch, per the handoff's explicit "keep
`dir_common_state`, it is not wrong."

**No signal reads inside `classify_entry`** — `rel`/`is_dir` come from the
already-collected `left_entries: Vec<(PathBuf, bool)>`, `l_root`/`r_root`
are plain `PathBuf`s already read out of their signals before the loop.
Matches the extraction-risk boundary the F72 `apply_navigation` split
already established (keep signal reads in the closure, pass plain values
in) — cited in the code comment so the next person doesn't have to
re-derive it.

**Real-path test:** builds two real temp directories (`fsk-ui-explorer-f74-
real-path-not-equal-<pid>/{left,right}/sub`, following
`state/compare/tests.rs:165`'s `temp_dir(tag)` pattern — no new
dependency, no `tempfile` added), writes genuinely different content into
each side's `sub/a.txt`, calls `classify_entry(&PathBuf::from("sub"), true,
&l_root, &r_root)`, and asserts `Final(NotCompared)`. See the required
answer above for the falsification output.

## 2. §7b — the label is now screen-reader text, not a tooltip

Replaced `title` as the sole mechanism with `role: "img"` + `aria_label:
"{st_label}"` on the status span, keeping `title` alongside for the mouse
tooltip (per the handoff — "not harmful, it is just not the accessible
name"). Applied the identical repair to the `bin` badge three lines above,
per the handoff's instruction — same defect, same file, same change.

**Not unit-tested, matching the handoff's own §14 acknowledgment.** No
check here can fail if the attribute never reached the DOM; that instrument
is a P07 accessibility-tree query (AT-SPI/UIA), added when the Explorer's
Windows rows next run. `dioxus-ssr` was not added, per §6.

## 3. Falsifiability — all three original checks reconciled, no regressions

- The two demonstrations review 072 found hollow (§2.1/§2.3 of request
  070) are superseded by the real-path test above; neither was removed
  wrongly — `dir_common_state`'s own test and `status_label`'s non-empty-
  label test are still correct about what *they* cover (the helpers behave
  as intended), they just aren't the falsifiability evidence for F74 itself
  anymore. Both still pass and both are kept, per the handoff §8.2.
- `filter.rs`'s `hide_eq` exemption test (review 072's one genuinely
  falsifiable check from request 070) is untouched and still passes.

## 4. F75 / F76 — confirmed untouched

`RowStatusKind` not wired, `DigestState` not deleted, no `TypeMismatch`
state added, no glyph redesign attempted. `EntryClassification`/
`classify_entry` live in `explorer.rs`, not moved to `forskscope-ui-logic`
— per §12, nothing moved this handoff.

## 5. Changed files

- `crates/forskscope-ui/src/ui/view/explorer.rs` — `EntryClassification`,
  `classify_entry` extracted; `use_effect` closure reduced to a `match`; 1
  new test.
- `crates/forskscope-ui/src/ui/view/dir_pane.rs` — `role`/`aria_label`
  added to the status span and the `bin` badge.
- `ROADMAP.md` — **not touched**, per the handoff's explicit §9 instruction.

## 6. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (`forskscope-ui`: 62, up from 61 — one
new test, none removed), `cargo xtask css --check` (no CSS changes this
pass), `cargo xtask i18n` (231 keys, unchanged — `aria_label` reuses the
exact same `t(lang, …)` strings `title` already used, no new keys needed),
`git diff --check`. CI dispatched on push (`afac676`).

## 7. Unresolved issues

- **The `aria_label` fix carries the same evidence gap the handoff
  accepted going in** — real confirmation is a P07 AT-SPI/UIA query, not
  attempted here, tracked as a follow-up when the Explorer's Windows rows
  next run.
- **`EntryClassification`'s exact shape (`Final`/`NeedsDigest`) is a first
  draft** — the handoff said the signature is mine to choose; flagging it
  in case a different shape reads more naturally against `RowStatusKind`
  when F75 eventually supersedes this.

## 8. Requested review focus

1. **Does `classify_entry` actually reach "the exact call site `9f355c6`
   got wrong"**, or does the `Final`/`NeedsDigest` split still leave a gap
   between what the test drives and what the `use_effect` executes at
   runtime?
2. **Is the real-path test's fixture (two temp dirs, one differing file)
   sufficient**, or does F74's original report (a real Windows build,
   real user directories) call for a case this fixture doesn't cover?
3. **Is deferring the `aria_label` verification to a future P07 run
   acceptable as stated**, or should this request hold until that's
   scheduled rather than landing unverified now?
