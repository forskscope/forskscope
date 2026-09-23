# Review Request 070: F74 — Explorer directory status

**Governing task.** `dev-record/handoffs/002-f74-explorer-directory-status.md`
**Register.** F74 (fixed here), F75 (deliberately untouched — see §4)
**Baseline.** `main` at `9f355c6`
**Commit.** `16c35f1`

## 1. The fix

`explorer.rs:251-258`'s digest block assigned `DigestState::Equal` to a
directory row purely because a same-named directory existed on the other
side — no digest, no recursion, no metadata, contents never examined. Two
directories with differing contents both showed `✓`.

Added `DigestState::NotCompared` — distinct from `Equal` (a false claim of
identity the old code made) and from `Unique` (present on only one side,
unchanged and still correct when the other side has no directory at all).
The digest-computation logic is now `dir_common_state(counterpart_is_dir:
bool) -> DigestState`, a plain function extracted out of the `use_effect`
closure specifically so it's unit-testable (§3).

**Two aggravating defects, both in scope per the task, both fixed in the
same commit:**

- **`filter.rs`'s hide-identical filter** matched `Some(DigestState::Equal)`
  on `DigestKey::Common` with no directory exemption. Since a directory can
  no longer be `Equal` at all, this alone would have silently stopped
  mattering — but the task asked for the exemption checked directly
  (`is_dir`), not inferred from the state disappearing, so a future state
  that ever misrepresented a directory as `Equal` again still couldn't earn
  hiding. `is_dir_row` is computed from `AlignedRow`'s own `RowData.is_dir`
  on either side, and `eq_ok` short-circuits to `true` whenever it's set,
  regardless of `hide_eq`.
- **`dir_pane.rs`'s status glyph had no accessible label at all** — only
  the adjacent `bin` badge carried a `title`. Every `DigestState` variant,
  not just the new one, now gets a label via `t(lang, …)`, extracted into a
  `status_label(status, lang)` function alongside a `status_glyph(status)`
  function (icon/class, no `lang` needed) — both plain, both directly
  testable, called from `TreeRow`'s render body rather than inlined in it.

## 2. Falsifiability — all three checks demonstrated failing

Per the task's explicit standard ("a test that passes against both the
fixed and the broken code proves nothing"), each of the three checks below
was confirmed to **fail** against a temporarily reintroduced pre-fix
version, then restored:

1. **`a_same_named_directory_on_both_sides_is_never_reported_equal`**
   (`explorer.rs`) — asserts `dir_common_state(true) == NotCompared`, not
   `Equal`. Broken by temporarily returning `Equal` from
   `dir_common_state`; failed with `left: Equal, right: NotCompared`;
   restored.
2. **`hide_identical_never_hides_a_directory_row_even_if_marked_equal`**
   (`filter.rs`) — a directory row explicitly marked `Equal` in the digest
   map, with `hide_eq: true`, must still appear in `apply_filter`'s output.
   Broken by temporarily dropping `&& !is_dir_row` from the guard; failed
   with `visible.len() == 0`, not `1`; restored.
3. **`every_digest_state_has_a_non_empty_label_in_every_language`**
   (`dir_pane.rs`) — every `DigestState` variant, in both `Lang::En` and
   `Lang::Ja`, yields a non-empty `status_label`. Broken by temporarily
   returning `String::new()` for `NotCompared`; failed with `"NotCompared
   has an empty accessible label for En"`; restored.

A fourth test (`a_directory_with_no_directory_counterpart_is_unique`)
pins the unchanged half of the branch (no directory on the other side →
still `Unique`) but isn't itself a falsifiability demonstration — nothing
about that path changed. A fifth
(`not_compared_has_its_own_distinct_glyph_and_label`) confirms
`NotCompared` doesn't silently share `Equal`'s glyph/class/label.

## 3. Both aggravating factors are done — stated plainly, per the task's own instruction

The task named these as the two most likely to be dropped, because neither
was what the bug report was about. **Both are done:**

- **The `hide_eq` exemption is done** and demonstrated failing without it
  (§2.2).
- **The accessible labels are done for every variant**, not just the new
  one, and demonstrated failing without one (§2.3).

## 4. F75 — not touched, as instructed

`RowStatusKind` (`forskscope-ui-logic/src/explore/status.rs`) is not wired.
`DigestState` gained a variant, not a replacement. No sequencing objection
— the task's reasoning (a UI-wide type swap is a larger, deliberate,
post-Gate-D change; this fix will be superseded rather than duplicated by
it) reads as correct on its own terms, not just as an instruction to defer
to. Nothing found while doing this work changes that assessment.

## 5. What was not done, and why

- **No subtree recursion or digesting added** — `dir_common_state` takes a
  single `bool` and returns in O(1); the directory branch still does
  exactly the same `cp.is_dir()` check it always did, just labels the
  result honestly instead of claiming equality from it.
- **No real-desktop/GUI verification performed.** The task states plainly
  that `filter.rs` and the digest block are plain functions over a
  `HashMap`, these are unit tests, and "no RFC-078 harness work is needed
  for this handoff" — taken at face value; unit-test coverage is the bar
  this request meets, not a lesser substitute for one that was skipped.

## 6. Changed files

- `crates/forskscope-ui/src/ui/view/explorer.rs` — `DigestState::NotCompared`
  assignment, `dir_common_state` extracted, 2 new tests.
- `crates/forskscope-ui/src/ui/view/explorer/filter.rs` — `is_dir_row`
  exemption, 1 new test.
- `crates/forskscope-ui/src/ui/view/dir_pane.rs` — `DigestState::NotCompared`
  variant, `status_glyph`/`status_label` extracted, `title` added to the
  status span, 2 new tests.
- `crates/forskscope-ui/src/i18n.rs` — 4 new keys with Japanese translations
  (`Comparing…`, `Identical`, `Only on this side`, `Directory contents not
  compared — use Deep Compare`; `Different` reused an existing key).
- `crates/forskscope-ui/assets/css/10-view-explorer.css` (+ regenerated
  `main.css`) — `.st-not-compared` rule, deliberately not reusing
  `.st-equal`'s color.
- `ROADMAP.md` — F74 marked resolved pending this review.

## 7. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (all green, `forskscope-ui`'s count up
by 5), `cargo xtask css --check` (green after regenerating `main.css`),
`cargo xtask i18n` (231 keys, up from 227, all covered), `git diff --check`,
`mdbook build docs`. CI dispatched on push (`16c35f1`); will confirm green
before treating this as final if not already confirmed by the time this is
read.

## 8. Unresolved issues

- **No runtime/CI evidence, only unit tests** — matches the task's own
  stated scope, not an oversight, but flagging it explicitly since every
  other F-numbered fix this program has shipped recently carried real
  desktop or CI confirmation and this one deliberately doesn't.
- **The exact wording of the new labels** (`"Only on this side"`,
  `"Directory contents not compared — use Deep Compare"`, etc.) is a
  first draft, not run past anyone — worth a look if the phrasing matters
  to how RFC-009 §7 compliance reads in practice, not just that a label
  exists.

## 9. Requested review focus

1. **Is `NotCompared`'s glyph (`–`, en dash) and class (`st-not-compared`,
   `var(--muted)`) an appropriate visual/semantic choice**, or does it read
   as too close to `Unique`'s `·`/`var(--muted)` pairing in practice?
2. **Is the extraction level right** — `dir_common_state`,
   `status_glyph`/`status_label` as separate plain functions — or did this
   split more than the task's falsifiability requirement actually needed?
3. **Does anything here creep toward F75's territory** despite the
   intent not to, given `status_label`'s shape (`DigestState` + `Lang` →
   `String`) is structurally close to what `RowStatusKind::label()` was
   already going to provide?
