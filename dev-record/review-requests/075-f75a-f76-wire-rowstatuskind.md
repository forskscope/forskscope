# Review Request 075: F75(a) + F76 — wire `RowStatusKind`, delete `DigestState`

**Governing task.** `dev-record/handoffs/007-f75-wire-rowstatuskind-and-f76.md`
**Register.** F75 (its wiring half — the other eight `ui-logic` modules untouched). F76 (both instances, folded in).
**Baseline.** `main` at `7fa9b43` (docs: correct RFC-080 Q4's ordering - F76 folds into F75, not before it)
**Commit.** `163a548`

## The falsifications, first

### §8.3 — `Unknown` maps to `NotCompared`, not `Computing`

```
thread 'explore::status::tests::unknown_maps_to_not_compared_not_computing' panicked at crates/forskscope-ui-logic/src/explore/status.rs:231:9:
assertion `left == right` failed
  left: Computing
 right: NotCompared
```

Broken by reverting `status.rs`'s `EqualityEvidence::Unknown => Self::NotCompared` to `=> Self::Computing`. Restored; the suite is green.

### §8.4 — a same-named directory pair still shows *not compared*, end to end

Re-ran the **same** `status.rs` falsification above (deliberately — this is a distinct test exercising it through the real `explorer.rs` classification path, not a repeat of §8.3's isolated one):

```
thread 'ui::view::explorer::tests::a_same_named_directory_pair_still_shows_not_compared_end_to_end' panicked at crates/forskscope-ui/src/ui/view/explorer.rs:653:9:
assertion `left == right` failed: F74: a same-named directory pair must render as not-compared, never a claimed verdict
  left: Computing
 right: NotCompared
```

Confirms F74's guarantee holds through the full pipeline (`classify_entry` → `RowStatusKind::from_evidence`), not just at `status.rs`'s own boundary. Restored; the suite is green.

### §8.1 — a type mismatch is not reported as present-on-one-side

```
thread 'ui::view::explorer::tests::a_directory_with_a_same_named_file_counterpart_is_a_type_mismatch_not_one_sided' panicked at crates/forskscope-ui/src/ui/view/explorer.rs:674:9:
assertion `left == right` failed: a directory whose counterpart is a same-named file must be a type mismatch, not falsely reported as present on one side only
  left: Final(LeftOnly)
 right: Final(TypeMismatch { left: Directory, right: File })
```

Broken by restoring the old collapse — a directory whose counterpart is a same-named file classified straight to `LeftOnly` (`Unique`'s pre-F76 equivalent), skipping the `cp.is_file()` branch entirely. Restored; the suite is green.

### §8.2 — a failed comparison is not reported as `Different`

```
thread 'ui::view::explorer::tests::a_failed_comparison_is_reported_as_error_not_different' panicked at crates/forskscope-ui/src/ui/view/explorer.rs:754:9:
a failed comparison must be reported as Error, not a fabricated verdict: Some(DigestDifferent)
```

Broken by restoring `Err(_) => Some(EqualityEvidence::DigestDifferent)` in `classify_digest_outcome`. The test drives this with a **real** `Err` from `file_digest_equal_with_cancel` called against a nonexistent path — not a hand-constructed error value — so this is the real call site's actual output. Restored; the suite is green.

### §8.5 — review 073's two falsifications, re-run against the converted code

**The real-path directory classification, at `classify_entry`:**

```
thread 'ui::view::explorer::tests::real_directories_with_the_same_name_and_differing_contents_are_not_compared' panicked at crates/forskscope-ui/src/ui/view/explorer.rs:620:9:
assertion `left == right` failed: a same-named directory whose contents differ must never be classified Equal - see 9f355c6's original defect
  left: Final(DigestEqual)
 right: Final(Unknown)
```

Broken by temporarily making the `cp.is_dir()` branch return `EqualityEvidence::DigestEqual` instead of `Unknown` — the exact shape of the original `9f355c6` defect, translated to the new vocabulary. Restored; the suite is green.

**`hide_eq`'s directory exemption, at `filter.rs`:**

```
thread 'ui::view::explorer::filter::tests::hide_identical_never_hides_a_directory_row_even_if_marked_equal' panicked at crates/forskscope-ui/src/ui/view/explorer/filter.rs:217:13:
assertion `left == right` failed: a directory row must stay visible under hide-identical, regardless of its recorded state
  left: 0
 right: 1
```

Broken by removing the `&& !is_dir_row` guard from `eq_ok`'s condition. Restored; the suite is green.

Both still bite after the refactor — neither of F74's checks went quiet.

## 1. §7a done first, standalone, before any wiring

Added `RowStatusKind::NotCompared` (glyph `–`, class `status-not-compared`, ARIA `"not compared"`) and corrected `EqualityEvidence::Unknown`'s mapping from `Computing` to `NotCompared`, in `status.rs` alone, before touching `explorer.rs` or `dir_pane.rs`. Its own test suite (`digest_equal_maps_to_equal` etc.) ran green against these fixes before any wiring existed to retrofit them to — per §11's explicit prohibition, and the reason §7a is ordered first in the handoff.

## 2. Storage shape (§7b) — a factual correction to the handoff's own framing

**Stored `EqualityEvidence` in `digest_map`, not `RowStatusKind`.**

The handoff frames this as a real trade-off: *"`DigestState` is `Copy`; `EqualityEvidence::Error{message}` is not... I have not weighed the `Copy` ripple and you will have."* I checked, and the premise doesn't hold: **`DigestState` was never `Copy`.** Its derive was `#[derive(Clone, PartialEq, Debug)]` — no `Copy` — and every existing call site (`tree.rs`, `compact.rs`) already used `.cloned()` on `digest_map.read().get(...)`, never relied on `Copy`. There is no ripple to weigh, because there was nothing to lose. Storing `EqualityEvidence` cost exactly the same as storing `DigestState` did: a `.clone()` at each read site, unchanged from before this commit.

Given that, the choice reduces to the handoff's own stated preference — richer evidence now, since "a message nobody kept cannot be shown later" and RFC-080 will want it — with no countervailing cost. I did not weigh a trade-off that turned out not to exist; I'm reporting that I checked and it doesn't, rather than silently proceeding on the corrected premise without saying so.

## 3. What `RowStatusKind` gained beyond `NotCompared`

Nothing beyond the one variant §7a specifies. No fourth field, no new method beyond what's needed to render `NotCompared` (its `glyph()`/`css_class()`/`aria_label()`/`needs_action()` arms). `needs_action()` returns `false` for `NotCompared`, matching `Computing`'s existing exemption — a directory pair with an unexamined verdict doesn't need user action any more than an in-flight comparison does.

## 4. `classify_entry`'s full shape (§7b)

```
is_dir=true,  cp.is_dir()   → Unknown         (not compared — §7a)
is_dir=true,  cp.is_file()  → TypeMismatch    (F76 instance 1)
is_dir=true,  cp absent     → LeftOnly        (genuinely one-sided, unchanged)
is_dir=false, cp.is_dir()   → TypeMismatch    (the mirror)
is_dir=false, cp.is_file()  → NeedsDigest     (unchanged path)
is_dir=false, cp absent     → LeftOnly        (unchanged)
```

The right-only branch (`Explorer()`'s second loop, entries present only on the right) now inserts `EqualityEvidence::RightOnly` where it inserted `DigestState::Unique` before — same meaning, renamed vocabulary.

## 5. F76 instance 2 — extracted for the same reason F77/F78 extracted their appliers

`classify_digest_outcome(outcome: Result<DigestOutcome>) -> Option<EqualityEvidence>` pulls the digest-result classification out of the `spawn` closure so a test can drive it without a Dioxus runtime — the same reasoning `apply_epoch_result` (F78) and `classify_entry` (F74) already established for this file. `None` means `Cancelled` (establish nothing); `Err` (from either the comparison itself or a join error on the blocking task panicking) becomes `EqualityEvidence::Error`, never a fabricated `DigestDifferent`.

§8.2's test feeds it a **real** error — `file_digest_equal_with_cancel` called against `/nonexistent/fsk-explorer-f76/{a,b}` — rather than constructing a `CoreError` by hand, so the test proves the real call site's actual output, not a synthetic stand-in matching review 072's standard (the real call site, not a helper the fix introduces built to pass).

## 6. Rendering (§7c)

`dir_pane.rs`'s `TreeRow` computes `RowStatusKind::from_evidence(&status)` and renders its `glyph()`/`css_class()` directly — both are language-independent and already unit-tested in `status.rs`. The accessible label is **not** `RowStatusKind::aria_label()` (fixed English; `ui-logic` has no i18n system) — `dir_pane.rs` keeps its own `Lang`-aware `status_kind_label(kind, lang)`, a small `match` over the seven kinds routed through `t(lang, …)`, replacing the deleted `status_label(&DigestState, Lang)`. `LeftOnly`/`RightOnly` share one label ("Only on this side"), matching the pre-existing behavior (the old `Unique` state never distinguished direction either — not a regression).

New i18n key: `"Comparison failed"` (JA: 比較に失敗しました), for `RowStatusKind::Error` — the one genuinely new user-facing state (F76's second instance previously had no UI representation at all; errors were silently `Different`).

**CSS**: `RowStatusKind::css_class()`'s fixed strings (`status-equal`, `status-different`, etc.) collide by name with Deep Compare's own bare `.status-equal`/`.status-error`-shaped classes in `12-view-directory-report.css` (a different view, different row shape). Used compound `.tree-status.status-*` selectors in `10-view-explorer.css` rather than bare ones, so the two views' rules don't depend on CSS file concatenation order. `status-error` uses `var(--err)`; the rest keep their pre-existing colors, renamed 1:1 from the old `st-*` classes (`st-equal`→`status-equal`, `st-diff`→`status-different`, `st-unique`→ split into `status-left-only`/`status-right-only`, `st-computing`→`status-computing`, `st-not-compared`→`status-not-compared`).

**Glyphs changed** (this is `RowStatusKind::glyph()` as specified, not a choice I made): `✓`→`=`, `⚠`→`≠`, `·`→`←`/`→`, `⟳`→`…`, `–` kept for not-compared. The handoff's §7c explicitly says to render using `RowStatusKind`'s own glyph/CSS/label, so this is the specified outcome, not an incidental side effect — flagging it as a visible change since it wasn't called out in the "no user-visible behaviour change" framing other handoffs in this series have used, and this one doesn't make that claim.

## 7. Scope discipline

- The other eight unwired `ui-logic` modules (`command_bar`, `conflict_nav_view`, `load_guard`, `palette_view`, `save_error`, `scroll_sync`, `summary`, `tab_state`) — untouched.
- F80 (Deep Compare's five unlabelled glyphs, `RecStatus`) — untouched, different enum, different view.
- No RFC-080 work — no tiers, no size cap, no tier-1 match state.
- Deep Compare — untouched; it renders `RecStatus`, not `DigestState`/`EqualityEvidence`.
- `ROADMAP.md` — not touched.

## 8. Acceptance criteria, checked directly

- `DigestState` does not exist: `grep -rn "DigestState" crates/forskscope-ui/src crates/forskscope-ui-logic/src` returns only historical prose in comments (three lines, all past-tense references to the deleted type), zero live code.
- `grep RowStatusKind crates/forskscope-ui/src` returns real call sites: `dir_pane.rs` (the render path and its tests) and `explorer.rs` (the end-to-end test).
- No `_ =>` arm was added on any status enum: the one `_ =>` in this diff's touched files is `explorer.rs`'s pre-existing `compare_action`, matching on `(Option<PickKind>, Option<PickKind>)` — unrelated, unchanged by this handoff.

## 9. Changed files

- `crates/forskscope-ui-logic/src/explore/status.rs` — `NotCompared` added, `Unknown` mapping corrected, tests extended.
- `crates/forskscope-ui/src/ui/view/explorer.rs` — `classify_entry` emits `EqualityEvidence`; `classify_digest_outcome` extracted; `dir_common_state`/`DigestState` gone; test module rewritten.
- `crates/forskscope-ui/src/ui/view/dir_pane.rs` — `DigestState`/`status_glyph`/`status_label` deleted; `TreeRow` renders through `RowStatusKind` + `status_kind_label`.
- `crates/forskscope-ui/src/ui/view/explorer/filter.rs` — `hide_eq` uses `EqualityEvidence::is_equal()`.
- `crates/forskscope-ui/src/ui/view/explorer/{tree,compact}.rs` — `digest_map`/prop types swapped, otherwise untouched pass-through.
- `crates/forskscope-ui/src/i18n.rs` — one new key.
- `crates/forskscope-ui/assets/css/10-view-explorer.css` (+ generated `main.css`) — status classes renamed and made compound-selector-scoped.

## 10. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (`forskscope-core` 697 unchanged, `forskscope-ui-logic` 257 (+1: `NotCompared`'s own distinctness test; `unknown_maps_to_computing` renamed/re-pointed, not net-new), `forskscope-ui` 72 (+3 net: two type-mismatch tests, the end-to-end not-compared test, and the failed-comparison test, minus the two deleted `dir_common_state` tests)), `cargo xtask css --check`, `cargo xtask i18n` (234 keys, +1), `git diff --check`. Pushed as `163a548`; CI green.

## 11. Requested review focus

1. **§2** — the `Copy`-ripple correction: is storing `EqualityEvidence` (not `RowStatusKind`) still your call given the premise changed, or does anything else favor the leaner storage now that the reason to avoid it doesn't actually exist?
2. **§6** — the glyph change (✓/⚠/·/⟳ → =/≠/←→/…) is real and visible. Is rendering `RowStatusKind`'s glyphs as specified the right call for this handoff, or should the Explorer have kept its old glyphs and only adopted the CSS-class/label wiring underneath them?
3. **§5** — is extracting `classify_digest_outcome` (mirroring `apply_epoch_result`'s precedent) the right granularity, or would you rather the digest-result classification stayed inline since it's a short match either way?
