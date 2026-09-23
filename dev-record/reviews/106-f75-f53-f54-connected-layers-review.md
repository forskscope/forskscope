# Review 106 — Request 103: F75, F53, F54, connected layers

**Reviewer:** architect. **Date:** 2026-09-15. **Reviewed:** `b55acc6`.
**Verdict:** **Approved, with required follow-ups in handoff 034.** F75 and F53
are closed. **F54 is not closed**: the gate passes when a name appears only in
a comment, and I falsified that below. **F93 is reopened** because stale module
tables are back in two documents.

## 1. The call-graph table

I re-verified it, and the judgments hold. The four I would have been most
likely to challenge are all right:

- **`StatusRow` deleted, not wired.** Wiring it would have shipped an
  English-only aria-label over `dir_pane.rs`'s localized one. You called that
  "unwireable, not merely unwired", and that is the correct distinction.
- **`DeepCompareSummary` extended before wiring.** I checked equivalence by
  reading the code. Every count still runs over all entries, and the filter
  affects only `visible`. `DeepFilter::matches` treats *Different* as
  `status != Equal`, which is identical to the inline enum it replaced.
  Wiring the struct as it was would have collapsed four counts into one, and
  you did not do that.
- **`ProfileChoice`/`profile_presets` deleted under §3's rule**, even though
  §3 did not name them. The rule is about shape, not a list, so applying it
  was correct.
- **The four stragglers you found by re-running the check against the whole
  export list.** Nobody asked for that. It is the difference between cleaning
  up a list and establishing the invariant.

`theme_choices()` produces the same `<option>` values and labels byte for byte.

## 2. F53: closed

I reproduced your falsification independently. With `DIFF_FONT_SIZE_MAX` set
to `50`, `out_of_range_diff_font_size_clamps` fails with left `50`, right `32`.
With the value restored, it passes. Adding a separate constant instead of
narrowing `FONT_SIZE_MIN/MAX` was the right call.

**One user-visible consequence for the CHANGELOG, which I will write at the
cut:** a persisted `diff_font_size` of 33–50 now loads as 32, and 6–7 loads as
8. No dev-team action is needed.

## 3. F54: the gate counts comments as consumers (required)

**My falsification**, on a clean tree:

1. I planted `mod f54_probe { pub fn f54_probe_symbol() {} } pub use
   f54_probe::f54_probe_symbol;` in `crates/forskscope-ui-logic/src/lib.rs`.
   The gate **failed** and named the symbol, matching your result.
2. With that still in place, I added one line at the top of
   `crates/forskscope-ui/src/app.rs`:
   `// f54_probe_symbol (architect probe: comment-only mention)`.
   The gate **passed**: *"34 crate-root exports all have a consumer in
   forskscope-ui."*

Both files were then restored.

**Why this matters here specifically.** `contains_word`
(`xtask/src/ui_logic_connectivity.rs:176`) searches whole-file text. The
failure F54 exists to catch is a layer that is *talked about* but not
*called*. That talk is exactly what ends up in comments: a TODO, or a doc line
such as "`DeepFilter` is not used here yet". A comment stating the defect
would make the gate certify the opposite. This commit also shows how natural
such comments are: `settings_view.rs:16-37` and `lib.rs:28-35` explain the
deletions by naming the deleted symbols. Those comments are in `ui-logic`, so
they are not scanned, but the same habit in `forskscope-ui` would blind the
gate. String literals have the same problem. The doc comment discloses the
*overcount* from `ui-logic`'s own tests but not this one.

**Two related line-substring matches fail closed.** Neither is harmful today,
and the same fix covers both:

- `contains_glob_import` (`:168`) would fail the build on a comment that says
  `never use forskscope_ui_logic::*`.
- `extract_root_exports` (`:126`) would register a phantom export from a
  `//!` line in `lib.rs` containing `pub use X;`.

**Required change:**

- **One stripping pass,** applied to both the `forskscope-ui` sources and
  `lib.rs` before any search. It removes:
  - line comments, including `///` and `//!`;
  - block comments, including nested ones;
  - string literals, including raw `r#"…"#`.

  Char literals must not be confused with lifetimes: `'a` is not a literal.
  A hand-written scanner is fine. Do not add a parser dependency for this.
- **Unit tests** for each form, including a name that appears only in a
  comment and a lifetime next to a name.
- **Falsification:** my two-step probe above must **fail** after the change,
  and the real tree must still pass without an allowlist.

The known undercount for consumers that use field access only stays as you
documented it. Removing those names from the crate root is the right answer to
that, and it needs no change.

## 4. Stale documentation: F93, third time (required)

**`docs/src/maintainers/architecture.md`:**

- The heading at `:51` says `(13)`, but there are **11** leaf modules on disk.
- The rows at `:70` (`StatusRow`), `:71` (`compare::conflict_nav_view`) and
  `:74` (`compare::palette_view`) are stale.
- The row at `:80` lists helpers that no longer exist.

**`docs/src/maintainers/testing.md`, the `ui-logic` table (`:199-215`)**, is
older than this commit and worse:

- It still lists `command_bar`, `hunk_decorations`, `scroll_sync`, `summary`
  and `tab_state`. Those were deleted in `d69c83b`/`8f1af77`, and F93's own
  fix corrected only `architecture.md`.
- It also lists `conflict_nav_view` and `palette_view`, `StatusRow`
  constructors under `explore/status`, and removed tests under
  `settings/settings_view`.
- It is missing `compare/load_identity`, `compare/startup` and both
  `persistence_recovery` modules.

**Code comments:**

- `crates/forskscope-core/src/diff/options.rs:193-194` still names the deleted
  `ui-logic::settings_view::profile_presets()` as `all_presets`' "only
  bridge". `all_presets` now has **no non-test caller**, and the comment
  should say that. **Do not delete `all_presets`.** F25's decision stands and
  it is core surface.
- The parenthetical at `crates/forskscope-ui-logic/src/lib.rs:32` attributes
  "RFC-028's toolbar profile picker" to `conflict_nav_view`/`palette_view`.
  The profile picker was `ProfileChoice`/`profile_presets` in `settings_view`.

**Required mechanism, not only the edits.** F93 has now been fixed by hand
twice, and each fix missed a document. Add one `cargo xtask` check, wired into
`ci.yml` next to `ui-logic-connectivity`:

- The module names in `architecture.md`'s `ui-logic` table and in
  `testing.md`'s `ui-logic` table must each **equal** the set of leaf modules
  under `crates/forskscope-ui-logic/src`.
- The heading count must equal that set's size.
- No allowlist.

**Falsification:** remove one row, add a fictitious row, and change the
heading count. Each change must fail on its own. Show the output of all three.

You may keep the wording of the *Covers* column. The check concerns which
modules are listed, not what the rows say about them.

## 5. What could not be tested

Accepted. The argument is sound:

- `rsx!` is checked at compile time.
- `theme_choices` has byte-exact unit tests.
- `clamp_font_size` is tested at both bounds.
- I checked the Deep Compare wiring for equivalence by reading it (§1).

The click-through goes on the 0.171.0 pre-publish smoke on the owner's side:

- open Settings, switch theme, set the font size to 40 and confirm it becomes
  32;
- run Deep Compare and change the filter.

It does not block approval.

## 6. Register

- **F75:** closed (`b55acc6`).
- **F53:** closed (`b55acc6`).
- **F54:** stays open until handoff 034's gate fix lands.
- **F93:** reopened because the tables are stale again; it closes when 034's
  document check lands.
- **0.171.0:** the cut waits on handoff 034.
