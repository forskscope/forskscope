# Review Request 076: F80 — Deep Compare's status glyphs need accessible labels

**Governing task.** `dev-record/handoffs/008-f80-deep-compare-status-labels.md`
**Register.** F80 (fixed here). Not a Gate D blocker.
**Baseline.** `main` at `960ec55` (docs: RFC-081 proposed - AUR publication automation; F81 opened)
**Commit.** `d393c39`

## The two falsifications

### §7.1 — every status yields a non-empty label

```
thread 'ui::view::deep_compare::tests::every_rec_status_has_a_non_empty_label_in_every_language' panicked at crates/forskscope-ui/src/ui/view/deep_compare.rs:738:17:
Symlink has an empty accessible label for En
```

Broken by temporarily returning `String::new()` for `RecStatus::Symlink` in `status_label`. Restored; the suite is green.

### §7.2 — no two statuses share a label

```
thread 'ui::view::deep_compare::tests::no_two_statuses_share_a_label' panicked at crates/forskscope-ui/src/ui/view/deep_compare.rs:765:9:
assertion `left == right` failed: every RecStatus must have a distinct label
  left: 6
 right: 7
```

Broken by temporarily pointing `RecStatus::Symlink` at the same key as `Changed` (`t(lang, "Different")`). Restored; the suite is green.

As instructed (§7), neither test claims a label reaches an actual screen reader — that's a P07 (AT-SPI/UIA) assertion, not a unit test, the same limit already recorded for F74.

## 1. `Symlink`'s wording, and why

`"Symlink not followed"` (JA: シンボリックリンク（未追跡）).

Core's own doc on `RecStatus::Symlink` says ForskScope "does not follow cross-root symlinks to avoid cycles; the entry is reported and left to the caller to act on." A label like "Symlink" or "Link" alone would be true but incomplete — a screen-reader user would have no way to know this status means *nothing was examined*, unlike every other status here, which all report an actual comparison outcome (or, for `Unreadable`, an actual failure to read). "Not followed" is the fact that explains why `can_cmp` excludes it and why no copy button appears: not a judgment about the target file, just a statement that ForskScope never looked at it. Added a dedicated test (`symlink_label_says_not_followed_not_a_verdict`) asserting the label contains "not followed", so this wording choice stays enforced rather than just described in a comment.

## 2. Nothing else changed

- **No glyph**: `(icon, cls) = match entry.status { ... }` is untouched, byte-for-byte.
- **No CSS class**: same match, same class strings; `cargo xtask css --check` passes with zero diff (I made no CSS edits at all).
- **No behaviour**: `can_cmp`, `can_copy_left_to_right`, `can_copy_right_to_left`, the summary counts, and the filter predicates are all untouched — confirmed by reading the diff, which touches only the new `status_label` function, the single `span` it feeds, and the three new tests.
- **One span, no per-status branch**: F79's `if entry.status == RecStatus::Unreadable { …labelled… } else { …bare… }` is gone, replaced by one `span` computing `label = status_label(entry.status, lang)` once and using it for both `aria_label` and `title` — matching §5's acceptance criterion literally.
- **Six of seven labels reused exactly**: `Different`, `Only on the left`, `Only on the right`, `Identical`, `Comparing…`, `Unreadable` — all pulled from the existing i18n table (added by the Explorer work, handoff 007), zero new near-duplicate strings. Only `"Symlink not followed"` is new.

## 3. The glyph divergence (§8) — my view

I would leave it, but not indefinitely. The two views' glyph sets (`RowStatusKind`'s `=`/`≠`/`←`/`→`/`…` vs. Deep Compare's `✓`/`⚠`/`←`/`→`/`⊙`) encode the same seven-ish concepts with different symbols, and unifying them now would mean picking a side inside a presentation-only accessibility fix — exactly the "resolving it badly, by copying one set into the other file" you flagged. If it's worth resolving, it reads to me as a `ui-logic` job: a single shared glyph/CSS/label table both views render through, the same shape `status.rs` already gives the Explorer, rather than either view importing the other's constants directly. That would also be the natural place to settle `LeftOnly`/`RightOnly` sharing a glyph with `Unreadable`'s distinctness precedent already established. Not proposing it as part of this handoff — just naming what I think the eventual fix looks like, since you asked.

## 4. Scope discipline

- `RecStatus` and its variants — untouched.
- No classification logic touched — `entry.status` itself is computed exactly as before.
- F75(b) (the other eight unwired `ui-logic` modules), RFC-080 — untouched.
- `ROADMAP.md` — not touched (not required per §9, and not edited).

## 5. Changed files

- `crates/forskscope-ui/src/ui/view/deep_compare.rs` — `status_label` added; `DeepRow` collapsed to one span; three new tests.
- `crates/forskscope-ui/src/i18n.rs` — one new key (`"Symlink not followed"`).

## 6. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (`forskscope-ui` 76, +3), `cargo xtask css --check` (no diff — confirms no CSS touched), `cargo xtask i18n` (236 keys, +1, translated), `git diff --check`. Pushed as `d393c39`; CI green.
