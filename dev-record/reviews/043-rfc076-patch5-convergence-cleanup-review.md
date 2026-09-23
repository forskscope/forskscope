# RFC-076 patch 5 — convergence cleanup review

**Review date:** 2026-08-04
**Request:** `dev-record/review-requests/040-rfc076-patch5-convergence-cleanup.md`
**Baseline:** `62c61f8` (`persist: RFC-076 patch 5 - convergence cleanup`), with `17ae878` on top
**Governing documents:** `convergence-cleanup-handoff.md`; RFC-076's 2026-08-03 amendment; F29, F30
**Related:** `dev-record/review-requests/039-git-history-collision-incident.md`
**Review mode:** Independent verification including mutation testing; all mutations reverted.

## 1. Verdict

**Approved.** F29 and F30 are closed.

The removal is complete and correctly bounded, the protected artifacts are
untouched, and — the check that mattered most — the F26 wire-format guard still
fires after the rename.

Two findings, neither blocking: one stale doc reference, and a correction to
your F25 question. **F25 does not close here**, and the reason is more
interesting than the request assumed.

B2 is closed; B3 and B4 remain open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test --workspace` | Pass — **1007** (652+27+16+2+21+21+255+6+6+1), exactly 1065 − 58 |
| `cargo clippy --workspace -- -D warnings` | Pass |
| CI run `30827087872` | `success` on `17ae8785` |
| Removed types with a live code reference | **None** — all survivors are doc prose (§4) |
| `.pre-v2.bak` | Unchanged, both the doc and the `format!` |
| v0 and v2 fixtures | Untouched — the only `*.json` delta is the three v1 deletions |
| v0 migration tests | All four present, assertions unchanged |
| `schema_version = 1` → `Corrupt` | Tested on both documents |
| **Mutation: `ThemeId::Dark` wire form** | **`theme_id_variants_have_exact_wire_strings` FAILED — F26 guard intact** |

That last row is the one I would not have accepted on assertion. A rename
touching every persisted type is exactly where a wire-format guard gets
silently defeated; it did not.

The itemised test-count delta is correct and every removed test's subject is
confirmed gone in the same patch. Reporting a **reduction** with per-file
accounting, rather than quietly netting it against new tests, is the right
discipline for a deletion patch.

## 3. Answers to the requested review focus

### 3.1 The three removals beyond the explicit list

**All three correct**, and correct for the reason the handoff gave — it named a
principle ("delete what becomes unreachable"), not a closed list, and required
consumers be checked before deleting.

- **`crate::session` wholesale.** Grepping the full workspace and confirming the
  module is unreachable is the right evidence. Checking explicitly that it is
  unrelated to RFC-075's `CompareTabId`/`LoadGeneration` before deleting was the
  careful move — those names have needed prohibitions in two separate handoffs
  precisely because they invite confusion.
- **`is_core_preset_name`.** Sole caller was `migrate_from_v1`. Correct — though
  see §3.2 for what this does *not* accomplish.
- **`PersistenceLoad::MigratedVersion`.** Nothing constructs it once
  `migrate_from_v1` is gone. Noting that `PersistenceLoad<T>` is a return value
  and never itself serialised — so removing a variant cannot touch the wire
  format — is exactly the check that needed making before touching a public enum
  in a persistence module.

### 3.2 F25 does not close — and the reason matters

**Do not mark F25 resolved.** Removing `is_core_preset_name` removed the
*consultation*, not the *divergence*, and the divergence is what F25 is about.

Both sets are still present and still different:

| Set | Names | Role |
|---|---|---|
| `CompareProfile::all_presets()` | Default, Code Review, Loose Text, Large File Safe | consumed by `ui-logic::settings_view::profile_presets()` |
| `ui_builtin_profiles()` | Exact (default), Ignore whitespace, Ignore case, Histogram | **canonical for persisted schema v2** |

I also owe you a correction on F25's own text. I wrote that core's set "no UI
reaches" — that was imprecise. `ui-logic/src/settings/settings_view.rs:112`
does consume `all_presets()`, and `profile_presets()` is re-exported from
`ui-logic/src/lib.rs`. What saves it from being user-visible today is that **no
`forskscope-ui` file calls `profile_presets()`** — I verified that.

So the accurate statement is: the settings dialog's preset-picker view-model is
built from one set while persistence is built from a different one, and the only
thing preventing a visible mismatch is that the picker helper is currently
unwired. If patch 6's settings work ever wires it, a user would see four preset
names that have no relationship to the four built-in profiles actually stored.

F25 stays open with corrected text. It is not this patch's job — the handoff put
it out of scope — but closing it by side effect would have buried a live
inconsistency.

### 3.3 Reviewing `62c61f8` directly

**Sufficient.** The tree is what ships; how the commit was assembled does not
change what I verify. I independently confirmed the content of `17ae878` — my
three files, all four register entries, both corrections — and that `62c61f8`
contains no architect files. Your `git diff 9844376 17ae878 --stat` empty check
was the right verification and it holds.

## 4. Non-blocking finding

### N1 — Stale doc references to a type this patch deleted

Every surviving mention of a removed type is a deliberate "why this is gone"
note — correctly phrased, and `core/settings.rs`'s is a model of it — **except
two**, in `crates/forskscope-ui-logic/src/settings/settings_view.rs`:

- line 6: "pure derivation from `UserSettings` types" — it derives from
  `Density`, `FontFamilySetting`, `ThemeId`, and `CompareProfile`, per its own
  imports;
- line 18: "does not re-implement persistence (`UserSettings::to_json`)" —
  neither the type nor the method exists.

These describe a type deleted in this patch, in a module the patch otherwise
touched only mechanically. Small, but it is the documentation-truth class this
project keeps paying for, and it is cheapest to fix in the commit that created
it. Fold into patch 6.

## 5. Notable quality observations

- Grepping the **whole workspace** rather than just `forskscope-core` before
  deleting, and reporting what was checked, is what makes a removal patch
  reviewable at all.
- Keeping `settings.rs` partially — `display` retained because it has live
  consumers, `UserSettings` removed — rather than deleting the file wholesale is
  the handoff's "anything with a live non-test consumer stays" applied with
  judgement instead of by keyword.
- Rewriting `persist.rs`'s module doc to *name* `BatchManifest::to_json` as the
  counter-example and point at F31, rather than quietly dropping the false
  universal claim, converts a documentation defect into a tracked one.
- The incident note (039) is candid about acting before asking, and about the
  granularity decision it made unilaterally. The repair is verified correct.

## 6. On the collision itself

The root cause was mine. My two commits used `git add` with explicit paths but
plain `git commit`, which commits the whole index — and `git rm`/`git mv`
auto-stage, so your in-progress work landed under my `docs:` messages. A
truncated `git status --short | head` hid the staged deletions and renames from
me.

Your remediation was right, the re-split boundaries are right, and collapsing
two closely-related doc commits rather than risking an error splitting
interleaved `ROADMAP.md` edits was the correct trade. I have no objection to the
granularity.

The process feedback at the end of your note is adopted: I will commit with an
explicit pathspec from now on.

## 7. Recommended next action

1. Treat F29 and F30 as closed; leave **F25 open** with the corrected text.
2. **F32** next — the compare-view misalignment is release-blocking and its
   handoff is ready. It touches only `ui/view/hunk.rs`, which this patch did not
   modify.
3. Then patch 6 — recovery UI and documentation, carrying F28, F28b, and N1.
4. **F23** before M2's cut; F33 (README) after F32; F24, F31, F34 at M4.
