# Review Request 074: F79 — an unreadable entry must never vanish

**Governing task.** `dev-record/handoffs/006-f79-unreadable-entries.md`
**Register.** F79 (fixed here). F76 explicitly out of scope (§6).
**Baseline.** `main` at `1a13085` (docs: F79 scoped for handoff 006 - two corrections)
**Commit.** `42a9ccb`

## The falsifications, first

### §8.1 — a per-entry `metadata()` failure produces `Unreadable`, not absence

```
thread 'tests::dir_unreadable_tests::an_unreadable_file_appears_as_unreadable_not_absent' panicked at crates/forskscope-core/src/tests/dir_unreadable_tests.rs:95:5:
assertion `left == right` failed: an unreadable file must appear as Unreadable, not be dropped from the result: []
  left: None
 right: Some(Unreadable)
```

Broken by restoring `Err(_) => continue` in `walk()`'s per-entry metadata match (removing the `mark_unreadable(map, rel)` call). With no right-side counterpart in this fixture, the entry is not merely misclassified — `scan.entries` comes back completely empty, exactly the "absent from the result" the handoff describes. Restored; the suite is green.

### A discovered subtlety while implementing §8.1 — the merge pass can silently reclassify

```
thread 'tests::dir_unreadable_tests::an_unreadable_left_file_is_not_reclassified_by_a_readable_right_counterpart' panicked at crates/forskscope-core/src/tests/dir_unreadable_tests.rs:143:5:
assertion `left == right` failed: a left-side Unreadable entry must not be silently reclassified just because the right side has a normal, readable counterpart: [RecEntry { rel_path: "blocked/file.txt", status: Changed, left_size: None, right_size: Some(13) }]
  left: Some(Changed)
 right: Some(Unreadable)
```

Not asked for by the handoff's numbered falsifications, but found while writing §8.1's test with a right-side counterpart present: `walk_and_merge`'s file branch ran a digest comparison against an already-`Unreadable` entry unconditionally. `file_digest_equal_with_cancel` fails to open the left path for the same reason `metadata()` already failed, and the pre-existing (F76-territory) `Err(_) => existing.status = RecStatus::Changed` fallback silently turned `Unreadable` back into a verdict — reproducing exactly the prohibited shortcut (§11: "do not map `Unreadable` to `Changed`"), just reached through a path I hadn't anticipated rather than written directly. Broken by removing the new `if existing.status == RecStatus::Unreadable { continue; }` guard at the top of that branch (same fix applied in `walk_and_merge_fast`, undemonstrated by a dedicated test there but structurally identical — see §5). Restored; the suite is green.

### §8.2 — an unreadable directory appears as `Unreadable`, its parent still lists

```
thread 'tests::dir_unreadable_tests::an_unreadable_directory_appears_as_unreadable_and_its_parent_still_lists' panicked at crates/forskscope-core/src/tests/dir_unreadable_tests.rs:186:5:
assertion `left == right` failed: an unreadable directory must appear as Unreadable at its own path: [RecEntry { rel_path: "parent/sibling.txt", status: LeftOnly, left_size: Some(10), right_size: None }]
  left: None
 right: Some(Unreadable)
```

Broken by restoring `let _ = walk(root, &path, map, token, _fast, make);` in `walk()`'s directory branch (discarding the `Result` instead of checking `.is_err()`). The sibling entry is present either way — confirming the parent's own loop was never the problem, only the missing report for the failed child. Restored; the suite is green.

### §8.3 — an unopenable root is distinguishable from an empty tree

```
thread 'tests::dir_unreadable_tests::an_unopenable_root_is_distinguishable_from_an_empty_tree' panicked at crates/forskscope-core/src/tests/dir_unreadable_tests.rs:244:5:
an unopenable right root must be flagged, not silently treated as empty
```

Broken by discarding `walk_and_merge`'s top-level `Result` in `recursive_diff_with_cancel` (`let _ = walk_and_merge(...)`, `right_root_unreadable` hardcoded `false`). With the flag gone, `scan.entries` still shows exactly what a genuinely empty right root would show — two confident `LeftOnly` entries, indistinguishable from the tree really being empty, which is the old defect verbatim. Restored; the suite is green.

## 1. The variant, and what it does not carry

`RecStatus::Unreadable`, no payload. `RecStatus` still derives `Copy` — confirmed by reading the derive line, unchanged by this commit (`#[derive(Debug, Clone, Copy, PartialEq, Eq)]`). No `String` or other heap payload was added anywhere on the variant.

## 2. The root signal — a new return type, not a synthetic entry

`recursive_diff`, `list_recursive_for_display`, and both `_with_cancel` variants now return `RecursiveScan { entries: Vec<RecEntry>, left_root_unreadable: bool, right_root_unreadable: bool }` instead of a bare `Vec<RecEntry>`. Chosen over:

- **A synthetic entry at the empty rel-path** — explicitly ruled out by the handoff (`explorer.rs` filters empty rel-paths; Deep Compare would render a nameless row).
- **`Result<Vec<RecEntry>, _>`** — would force choosing one side's failure over the other's, and would discard whatever entries the *readable* side did produce. `RecursiveScan` keeps both: a caller gets the partial `entries` (from whichever side opened) **and** an unambiguous flag per side.

The flags are computed from the same `Result<()>` the walk functions already returned and previously discarded (`let _ = walk(...)`) — `.is_err()` on that result, nothing new to detect.

## 3. The merge-guard fix (found during implementation, not asked for directly)

Both `walk_and_merge` and `walk_and_merge_fast`'s file branches now check `existing.status == RecStatus::Unreadable` before touching an already-present entry, and `continue` (leave it alone) if so — see the falsification above for why. Without this, an `Unreadable` entry set by the *left* pass could be silently overwritten by the *right* pass's per-file merge logic the moment the right side turned out to be readable, in two ways:

- `walk_and_merge`: a digest comparison against an unreadable left path fails and collapses to `Changed` via the pre-existing (F76-territory, untouched) I/O-error fallback.
- `walk_and_merge_fast`: the same slot unconditionally sets `existing.status = RecStatus::Computing`, implying a pending digest that would later hit the same failure.

Only the first is covered by a dedicated falsifying test (§8's evidence above); the second received the identical fix by inspection (same shape, same reasoning) but not an independent falsification — flagging this rather than implying both are equally proven. If you want the second demonstrated too, I can add it; five minutes of work, held back only because the handoff's four required tests didn't ask for a fifth and the two code paths are structurally identical enough that I judged one falsification sufficient evidence of the *pattern*, not because I think it's untestable.

## 4. Every consumer

- **`explorer.rs`/`DigestState`** — untouched. Different enum, different crate module, F76's territory per the handoff's own correction (§6). Confirmed no references added.
- **`deep_compare.rs`** — new glyph (`⊘`) and CSS class (`.status-unreadable { color: var(--err); }`, `12-view-directory-report.css`), `role="img"` + `aria_label` + `title` via `t(lang, "Unreadable")` (new i18n key, JA: 読み取り不可) — **applied only to this one status**, not retrofitted onto the other five glyphs in the same match, which carry no accessible label today and are unchanged; retrofitting them is a separate, larger change this handoff didn't ask for. `can_cmp` now excludes `Unreadable`. Counted separately in the stats line (`· N unreadable`, shown only when N > 0, reusing the existing `.deep-progress` muted styling rather than inventing a new class for a single conditional span) — not folded into "different". Visible under the default `Different` filter and `All` filter via the existing `!= Equal` / `true` predicates, which already included it without any change (confirmed by inspection, not forced to add a test for a one-line comparison that cannot silently regress in a way existing coverage wouldn't already catch elsewhere).
- **`BatchCopyButtons`** — `to_right`/`to_left` now filter through `can_copy_left_to_right`/`can_copy_right_to_left` (the same functions `DeepRow`'s per-row buttons already used) instead of re-deriving the same status set inline. This was incidental duplication before this handoff; removing it is what makes "not in the batch manifest" actually provable by the existing `can_copy_*` tests rather than needing a third, parallel predicate tested separately. Two new assertions added to those tests: `!can_copy_left_to_right(RecStatus::Unreadable)`, `!can_copy_right_to_left(RecStatus::Unreadable)`.
- **`deep_filter.rs`** (`forskscope-ui-logic`, not currently wired into the live `deep_compare.rs` — it defines its own local `DeepFilter`/inline counts, a pre-existing state noted for context, not something this handoff changes) — `is_different` already excluded `Unreadable` by omission (a positive list: `Changed | LeftOnly | RightOnly`), no code change needed; added a direct test plus filter/summary tests using a one-off entry vec rather than extending the shared `entries()` fixture (which backs every exact-count assertion in the file already).
- **`patch/directory.rs`** — see §5.
- **`report/dir.rs`** — compiler-forced (exhaustive `match e.status`, not something I chose to touch), see §6.

## 5. `patch/directory.rs` — fails, does not silently omit

Argued in §6 of the handoff request: an unreadable root or entry now returns `Err(CoreError::Io { .. })` from `patch_from_directories`, through the `Result` it already returns for every other failure in this file.

**Reasoning.** A patch document claims to be a complete, trustworthy record of the difference between two trees, consumed non-interactively (patch tooling, not a live UI). Deep Compare has a reader surface — a glyph, a label, a count — to carry the caveat "this one couldn't be read" through to the person looking at it. A patch format has no equivalent per-entry channel here (`PatchFileChange` is `Modify`/`Add`/`Delete`/`BinaryNotice`; inventing a fifth variant for this is a design change to the patch model itself, well beyond this handoff's scope). Given no honest way to carry the caveat *in* the document, silently emitting an incomplete one is functionally the same omission the handoff exists to close, just moved one level up. Failing hard forces the caller (UI or CLI) to surface it through error-handling that already exists (RFC-017's severity/recovery-hint system), rather than trusting every future patch consumer to notice a caveat embedded in the payload.

Both the root case and the per-entry case are covered by dedicated tests in `patch_tests.rs` (`directory_patch_fails_rather_than_silently_omit_an_unreadable_entry`, `directory_patch_fails_rather_than_silently_treat_an_unreadable_root_as_empty`), `#[cfg(unix)]`-gated with the same root-detection skip pattern as `dir_unreadable_tests.rs`. Not part of §8's required falsification set (that set is scoped to the walk itself), so not claimed as such — these confirm the *consumer* behavior, run and green, no skip on this machine (confirmed non-root, `--nocapture`).

## 6. `report/dir.rs` — not in §5, touched because the compiler required it

`DirComparisonReport::from_entries` has an exhaustive `match e.status` with no catch-all — adding `Unreadable` produced a real `E0004` (not a warning I could route around). Per §14's own framing ("that is the tool doing its job, not a reason to add a catch-all arm"), I added a genuine arm: a new `unreadable: usize` field, counted like `symlinks`, surfaced conditionally in the Markdown table (`if self.unreadable > 0`, mirroring the existing `Symlinks` row) and unconditionally in the JSON summary object (`"unreadable": N`, additive — `schema_version` unchanged, existing tests use substring assertions and did not need updating). Flagging this as an addition beyond §5's literal file list, forced by the exhaustiveness check rather than chosen.

## 7. Scope discipline

- No RFC-080 work: no tiers, no size cap, no new status vocabulary beyond `Unreadable`.
- F74, F75, F77, F78 untouched.
- F76 untouched in both its instances (`explorer.rs`'s `DigestState`, and the pre-existing `Err(_) => Changed` I/O-error collapse inside `walk_and_merge`/`walk_and_merge_fast` — preserved exactly, confirmed unchanged by the `Err(_)` arm still reading identically in the diff).
- `ROADMAP.md` not touched.
- `forskscope-ui-logic` still depends only on `forskscope-core` — no new dependency introduced by the `deep_filter.rs` changes (there were none needed).

## 8. Your three required plain statements

**Was any `_ =>` arm added anywhere?** No. Grepped every exhaustive match touching `RecStatus` after the change (`report/dir.rs`, `patch/directory.rs`, `deep_compare.rs`'s `DeepRow` glyph match) — each has an explicit `RecStatus::Unreadable` arm. The two `_ =>` arms that exist anywhere near this diff (`deep_compare.rs`'s `size_label`, matching on `(Option<u64>, Option<u64>)`; `report/dir.rs`'s `ReportPathMode` match) are both on unrelated types, unchanged by this handoff.

**Does `RecStatus` still derive `Copy`?** Yes, unchanged, confirmed by reading the derive attribute directly.

**Did the permission tests actually run, on which platforms, and did any skip?** Ran on Linux (this development machine, `uid=1000`, not root) via `cargo test --workspace` and independently via `--nocapture` to confirm the skip branch's `eprintln!` never fired. All seven `#[cfg(unix)]`-gated tests (five in `dir_unreadable_tests.rs`, two in `patch_tests.rs`) executed their real assertions, not the skip path. **Not exercised on Windows or macOS** — no CI runner available to me for this session; the `#[cfg(unix)]` gates mean they compile out entirely there rather than silently no-op, which is the property the handoff asked for, but "compiles out" is not "ran green elsewhere" and I'm not claiming it is.

## 9. Required documentation updates

None, per §9. `ROADMAP.md` not touched.

## 10. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (`forskscope-core` 696, +7: 5 in `dir_unreadable_tests.rs`, 2 in `patch_tests.rs`; `forskscope-ui-logic` 256, +4 in `deep_filter.rs`; `forskscope-ui` 69, unchanged — new assertions added to existing tests, no new `#[test]` fns), `cargo xtask css --check`, `cargo xtask i18n` (233 keys, +2: `"unreadable"`/`"Unreadable"`, both translated), `git diff --check`. Pushed as `42a9ccb`; CI green.

## 11. Requested review focus

1. **§3** — is one falsification sufficient evidence for the `walk_and_merge_fast` guard given the structural identity to `walk_and_merge`'s (falsified) version, or do you want the fifth test?
2. **§5** — is "fail the whole patch" the right call for `patch/directory.rs`, or does an eventual `PatchFileChange` variant for this belong in this handoff after all, given it already has `Result` and I'm choosing not to extend the model? You said you don't have a strong prior here.
3. **§4** — is applying `role="img"`/`aria_label` only to the new `Unreadable` row (not retrofitting the other five glyphs) the right scope boundary, or should the existing rows have gotten the same treatment as part of this handoff since it touches the same match?
