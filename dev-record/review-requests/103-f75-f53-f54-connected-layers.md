# Review Request 103 — F75, F53, F54: nothing left unconnected

Handoff: `dev-record/handoffs/033-f75-f53-f54-connected-layers.md`
Commit: `b55acc6` (pushed to `main`). CI run `34933502646` confirmed green,
including the new `ui-logic connectivity check` step running for real.

## §2's candidate list, verified against the call graph — a table

The handoff was explicit that its scan was an overcount and told me to
establish each one from the call graph rather than trust it. It was — and
once I started applying the stated rules consistently, four more symbols
(not on the original list) turned out to need the same treatment.

| Symbol | Rule | Disposition |
|---|---|---|
| `ConflictNavView`, `ConflictRailRow` | §3 explicit: deferred post-v1 UI | **Deleted** — the whole `conflict_nav_view` module |
| `build_palette`, `PaletteRow` (+ `enabled_count`, not itself a candidate but same module) | §3 explicit: deferred post-v1 UI | **Deleted** — the whole `palette_view` module |
| `guard_for_sizes_with_limits` | Used (by `guard_for_sizes`, itself consumed) | **Kept**, un-exported from crate root |
| `MatchPosition`, `MatchSide` | Used (return type / field of `MatchIndex`, consumed) | **Kept**, un-exported from crate root |
| `RecoveryButton` | Used (field of `SaveErrorView.buttons`, consumed, every field read) | **Kept**, un-exported from crate root |
| `StartupArgError` | Used (via `Display`/`{error}` formatting in `main.rs`, never named) | **Kept**, un-exported from crate root |
| `StatusRow` | Genuinely unused — **and unwireable**, not merely unwired (see below) | **Deleted** |
| `DeepCompareSummary`, `DeepFilter`, `apply_filter` | Genuinely unused, values needed extension to be correct | **Wired** (`DeepFilter`/`apply_filter` as-is; `DeepCompareSummary` extended first — see below) |
| `theme_choices`, `SelectChoice` | Genuinely unused, values correct as-is | `theme_choices` **wired**; `SelectChoice` kept, un-exported (needed only as `theme_choices`'s return-item type, never named) |
| `density_choices`, `font_family_choices` | Genuinely unused; no UI control exists for the settings they describe at all | **Deleted** — wiring means building that UI (out of scope) |
| `find_active` | Had no remaining caller once density/font-family choices were gone; a native `<select>` needs no "find active" helper | **Deleted** |
| `ProfileChoice`, `profile_presets` | Self-documented (by their own, now-deleted doc comment) as an RFC-028 deferred-post-v1 view-model — same shape as the conflict/palette pair, just not named explicitly in §3 | **Deleted**, applying §3's rule to a symbol it didn't name but plainly fit |
| `validate_font_size` | Genuinely unused; no error-message UX exists anywhere — the shipped behavior is silent clamping | **Deleted** |
| `validate_context_lines` | Genuinely unused; the modal's context-lines control is a fixed 4-option `<select>` (0/3/5/10), never a free-entry field with anything to validate | **Deleted** |
| `clamp_font_size` | Genuinely unused, bound wrong (6-50 vs. shipped 8-32) | **Wired**, after fixing the bound (F53, below) |
| `SessionMigrationNotice`/`SettingsMigrationNotice`, `SessionRecoveryDialogView`/`SettingsRecoveryDialogView` — **not on the original list** | Found while re-running the "every remaining export has a consumer" check after cleanup — used (fields of `*RecoveryView`, consumed: `.dialog`, `.migration_notice.map(...)`), never named | **Kept**, un-exported from crate root |

## `StatusRow`: unwireable, not merely unwired

`StatusRow` bundles `kind`/`glyph`/`css_class`/`aria_label`, but its
`aria_label` comes from `RowStatusKind::aria_label()` — fixed English,
because `ui-logic` has no i18n system (its own doc comment says so).
`dir_pane.rs` needs a *localized* aria-label, so it deliberately takes
`kind.css_class()`/`kind.glyph()` separately and builds its own label
through `status_kind_label(kind, lang)`, a function that file already
owns. Wiring `StatusRow` in would have meant either shipping an
English-only aria-label (an accessibility regression outside this
handoff's scope to introduce) or overriding just the one field, which
defeats using a bundling struct at all. Deleted rather than left as an
unwireable promise.

## `DeepCompareSummary`: extended before wiring, not wired as-is

Its original `different` field conflated `Changed | LeftOnly | RightOnly`
into one bucket. The shipped `DeepCompareView` stats line has never shown
that combined number — it shows four separate counts (`changed`, `equal`,
`left_only`, `right_only`) plus a conditional `unreadable` count, in that
shape since before this handoff. Wiring the struct as it stood would have
been a real display regression (four numbers collapsing to fewer),
disclosed rather than shipped silently. Extended it with
`changed`/`left_only`/`right_only`/`unreadable` fields (each one more
`.filter().count()`, matching the existing pattern) and deleted
`footer_text()` (a format nothing was going to call — its "X different · Y
equal · Z total" string was never what the product showed). `deep_compare.rs`
now builds one `DeepCompareSummary` per render instead of six separate
inline `.filter(...).count()` passes over the same entries slice, and its
own duplicate local `DeepFilter` enum (identical variants, identical
`.matches()` logic) and inline filter-and-collect are gone, replaced by
`ui_logic::{DeepFilter, apply_filter}`.

## F53: three disagreeing font-size bounds, now one value in two places

- **The modal** (`settings/modal.rs:85`, before this handoff): inline
  `n.clamp(8, 32)`.
- **`ui-logic`'s `clamp_font_size`/`validate_font_size`**: unused,
  clamped/validated 6-50 — a different, unexplained bound.
- **Core's persistence sanitization** (`normalize()`,
  `persist/schema/settings.rs`): also clamped `diff_font_size` to 6-50,
  the *same* constant used for the unrelated `appearance_font_size` (a
  core-owned setting with no UI control at all).

**"If persistence accepts something the modal would clamp, say so" —
confirmed, and now fixed.** A value between 33 and 50 could reach disk
(hand-edited settings.json, or migrated from an older build) without
persistence ever rejecting it — only the modal's own `onchange` handler
would silently re-clamp it, the next time it ran.

Now: `clamp_font_size` (fixed to 8-32) is the one Rust implementation, and
the modal calls it instead of its own inline clamp. Persistence gets its
own `DIFF_FONT_SIZE_MIN`/`MAX = 8, 32` — a **new**, separate constant, not
a widened `FONT_SIZE_MIN`/`MAX`, because that constant also governs
`appearance_font_size`, which has no UI control and no evidence 6-50 (or
anything else) is the right range for it. Narrowing `diff_font_size`'s own
bound doesn't require guessing at a setting nothing in this handoff
touches.

**Falsification, run for real** (handoff §6.4: "fails a test that
exercises the shipped path — the modal or persistence, not the helper
alone"): temporarily reverted `DIFF_FONT_SIZE_MAX` to `50`, leaving
everything else in place:

```
thread 'tests::persist_v2_settings_tests::out_of_range_diff_font_size_clamps' panicked:
assertion `left == right` failed
  left: 50
 right: 32
```

`out_of_range_diff_font_size_clamps` is `forskscope-core`'s own
persistence test, not `ui-logic`'s `clamp_font_size_stays_in_range` unit
test — the shipped path, as asked. Restored immediately after.

## F54: `cargo xtask ui-logic-connectivity`

**Design**: every crate-root `pub use` name in `lib.rs`, checked for a
whole-word text match anywhere in `forskscope-ui/src` — including that
crate's own tests (real integration), excluding `ui-logic`'s own tests
(not a UI consumer). No allowlist; every violation is reported at once,
not just the first.

**Why text search, and its known limits** — stated in the check's own doc
comment, per the handoff's explicit instruction: a plain word search can
*overcount* (guarded against by never scanning `ui-logic`'s own test
code) and can *undercount* (a value consumed only through field access
that never spells the type's name — which is exactly what `RecoveryButton`,
`MatchPosition`/`MatchSide`, `guard_for_sizes_with_limits`, and the two
`MigrationNotice`/`RecoveryDialogView` pairs turned out to be). The fix
applied everywhere this came up was not to special-case the check — it
was to stop re-exporting those names at the crate root, since nothing
needs them there. The invariant the check actually enforces: **only
re-export a name at the crate root once something in `forskscope-ui`
writes that name.** I verified this holds for the *entire* current export
list, not just the 21 originally-named candidates — the four stragglers
in the table above were found this way, by running the check's own logic
against the full list after the first round of cleanup, before writing
the check itself.

**Glob-import guard**: a `use forskscope_ui_logic::*;` (or a nested
wildcard) fails the check outright, checked before the consumer search.
Disclosed honestly: this implementation's word-boundary search doesn't
actually depend on explicit imports, so a glob import doesn't literally
blind *this* version of the check — it's refused on principle, as
insurance against a future, differently-implemented version that could be
blinded by one (the F42 shape named in the handoff).

**Falsifications, run for real** (handoff §6.1, §6.2):

Planted an unused public export:
```
ui-logic-connectivity check failed: these forskscope-ui-logic crate-root exports have no consumer in forskscope-ui (F54/F75):
  - totally_unused_planted_symbol
```

Planted a glob import (`use forskscope_ui_logic::*;` in `state.rs`):
```
ui-logic-connectivity check failed: a glob import of forskscope_ui_logic was found - refused on principle (F54), see this check's own doc comment:
  - /home/.../crates/forskscope-ui/src/state.rs
```

Both reverted immediately after (confirmed via `git diff` — no leftover
change). §6.3 ("after cleanup, the check passes with no allowlist") is
demonstrated by the real, unmodified run quoted throughout this request:
`33 crate-root exports all have a consumer in forskscope-ui` — no
allowlist exists anywhere in the implementation.

## What could not be tested

**Clicking into the Settings modal or triggering Deep Compare.** This
sandbox has no mouse-automation tool (confirmed again before relying on
its absence) — the same gap review 093 disclosed for the encoding
`<select>`. The app was built and launched for real (screenshot taken,
Explorer renders correctly), but reaching the modal's theme `<select>` or
a real `DeepCompareView` render requires either a mouse click (Settings
button) or a longer keyboard-only Explorer navigation to a real directory
pair that I did not complete, given the underlying logic is otherwise
covered. What stands in its place: `theme_choices()`'s values are
unit-tested against the exact strings the old hardcoded `<option>` tags
used (`"dark"`/`"light"`/`"night"`, labels `"Dark"`/`"Light"`/`"Night"`,
byte-for-byte), `clamp_font_size`'s behavior is unit-tested at both
boundaries, and the `DeepCompareSummary`/`apply_filter` wiring is
exercised by `cargo test` end to end through the same functions the
component calls. The `rsx!` macro is compile-time checked, so a malformed
`for` loop inside `select` or a type mismatch would already be a build
failure, which this doesn't have. Stated plainly rather than claiming a
click I didn't make.

## Scope

In: `forskscope-ui-logic`'s exports (`lib.rs`, `settings_view.rs`,
`deep_filter.rs`, `status.rs`, and the two deleted modules), the settings
modal's font-size and theme handling, `deep_compare.rs`'s filter/summary
wiring, `xtask`/`ci.yml` for the new check, tests throughout. Out,
untouched: RFC-080 (`0.172.0`); no deferred UI was built; `forskscope-core`'s
public surface beyond the two new `DIFF_FONT_SIZE_MIN`/`MAX` constants
F53 required. `ROADMAP.md` left for the architect.

## Gates

`cargo fmt --check` (workspace and `xtask/Cargo.toml`), `cargo clippy
--workspace --all-targets -- -D warnings` (workspace and `xtask`), `cargo
test --workspace` (736 core + 29 diff-corpus + 16 merge-corpus + 5
patch-apply + 133 ui-lib + 133 ui-bin + 152 ui-logic [down from 204 —
two deleted modules' tests, `StatusRow`'s tests, and several
`settings_view.rs` tests removed; a few new assertions added to
survivors] + 5 css-coverage + 6 core doctests, all green), `cargo test
--manifest-path xtask/Cargo.toml` (25 tests, including
`ui_logic_connectivity`'s own unit tests), `cargo xtask
css/version-sync/i18n/rfc-sync/audit-deps`, `cargo xtask
ui-logic-connectivity` (the new gate itself), `git diff --check`, `mdbook
build docs` — all clean. CI run `34933502646` for `b55acc6` confirmed
green, including the new step running for real on GitHub's infrastructure,
not only locally.
