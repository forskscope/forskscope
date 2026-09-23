# Handoff 034 — 0.171.0 follow-ups: stderr pipe, comment-blind gate, doc-table check

**From:** architect. **Date:** 2026-09-15. **Release:** 0.171.0. **This
blocks the cut.**

**Reasons and evidence:**
`dev-record/reviews/105-f102-render-check-startup-retry-review.md` (§2) and
`dev-record/reviews/106-f75-f53-f54-connected-layers-review.md` (§3, §4).
This file says what to do. The reviews say why.

Four parts, each in its own commit.

## A. F102: app stderr to a file (`packaging/render_check.py`)

1. **Pass a file handle as `stderr=`**, not `subprocess.PIPE`. Use one file
   per attempt, inside the existing `scratch` directory. Close each handle.
2. **On a crash before registration,** print the file's last 20 lines, as the
   script does today.
3. **On a `wait_for_ready` or geometry `FAIL`,** also print the last 20 lines.
4. **Do not use a reader thread,** and do not change `find_app` or
   `STARTUP_MAX_ATTEMPTS`.

**Falsifications.** Run each one against `9540254` first to show it failing,
then against the fix.

- **A1: a crash that leaves a child holding stderr.** Use a stub script:
  `sleep 120 &`, write a line to stderr, `exit 7`.
  - Before the fix, the `attempt 1/3` line appears only after the `sleep`
    ends.
  - After the fix, it appears right after `find_app`'s 30s.
  - Show `time` output for both.
- **A2: the success path under heavy stderr.** Run the real check with
  `G_MESSAGES_DEBUG=all` in the environment, and report how many bytes of
  stderr the app wrote.
  - **If it exceeds 64 KiB,** show 9540254 failing and the fix passing.
  - **If it does not,** say so and give the byte count. Do not manufacture a
    result.
- **A3: real CI.** Dispatch `render-check.yml` twice, once plain and once with
  `inject_geometry_defect=true`, and give the run IDs. The injected run must
  now include the app's stderr tail beneath the `FAIL`, and must still show no
  `attempt` lines.

## B. F54: the gate must not count comments or strings (`xtask/src/ui_logic_connectivity.rs`)

1. **Write one stripping function** and apply it to `forskscope-ui` sources
   and to `lib.rs` before `extract_root_exports`, `contains_glob_import` and
   `contains_word`. It must remove:
   - line comments, including `///` and `//!`;
   - block comments, including nested ones;
   - string literals, including raw `r#"…"#`.

   It must not treat a lifetime such as `'a` as a char literal. Write it by
   hand, with no new dependency.
2. **Add unit tests** for:
   - each comment form;
   - normal and raw strings;
   - a lifetime next to an identifier;
   - a name that appears only in a comment, which must count as unconsumed.
3. **Keep no allowlist.** The real tree must still pass.

**Falsification** (review 106 §3), run both steps before and after the fix:

1. Plant `mod f54_probe { pub fn f54_probe_symbol() {} } pub use
   f54_probe::f54_probe_symbol;` in `crates/forskscope-ui-logic/src/lib.rs`.
2. Add `// f54_probe_symbol` to `crates/forskscope-ui/src/app.rs`.

Against `b55acc6` the gate passes. After the fix it must fail and name the
symbol. Revert both files afterwards.

## C. F93: tie the module tables to disk (new `cargo xtask` command, wired in `ci.yml`)

1. **Build the set of `ui-logic` leaf modules on disk.** Walk
   `crates/forskscope-ui-logic/src` for `.rs` files, excluding `lib.rs`,
   parent module files (`compare.rs`, `explore.rs`, and so on) and tests.
   Today the set has 11 modules.
2. **`docs/src/maintainers/architecture.md`:** the module names in the
   `## \`ui-logic\` modules (N)` table must equal that set, and `N` must equal
   its size.
3. **`docs/src/maintainers/testing.md`:** the file names in the `ui-logic`
   table must equal the same set.
4. **Report every mismatch** in both directions, listing names that are
   missing and names that should not be there. Use no allowlist.

**Falsification:** make each of these changes separately and show that each
one fails on its own:

- delete one row;
- add a fictitious row;
- change `N`.

Revert each change afterwards.

## D. The document edits themselves

The details are in review 106 §4.

- **`architecture.md`:**
  - Fix the heading count.
  - Remove `StatusRow`, `compare::conflict_nav_view` and
    `compare::palette_view`.
  - Correct the `settings::settings_view` row so it names only what exists.
- **`testing.md`:** rebuild the `ui-logic` table from disk.
  - Remove `command_bar`, `hunk_decorations`, `scroll_sync`, `summary`,
    `tab_state`, `conflict_nav_view` and `palette_view`.
  - Add `compare/load_identity`, `compare/startup` and both
    `persistence_recovery` modules. Their *Covers* entries must describe
    their actual tests.
  - Correct the `explore/status` and `settings/settings_view` rows.
- **`crates/forskscope-core/src/diff/options.rs:193-194`:** `all_presets` now
  has no non-test caller, and the comment must say so. **Do not delete
  `all_presets`.**
- **`crates/forskscope-ui-logic/src/lib.rs:32`:** the RFC-028 toolbar profile
  picker was `ProfileChoice`/`profile_presets`, not
  `conflict_nav_view`/`palette_view`. Fix the parenthetical.

Land D together with C, so that C's first CI run is green against the real
tree.

## Scope

**Out of scope:**
- `find_app`
- `release.yml`
- `ROADMAP.md` and `CHANGELOG.md`, which the architect maintains
- RFC-080
- any change to `all_presets`
- `forskscope-ui` behaviour

## Gates

- `cargo fmt --check`, for the workspace and for `xtask`.
- `cargo clippy -- -D warnings`, for the workspace and for `xtask`.
- `cargo test --workspace`.
- `cargo test --manifest-path xtask/Cargo.toml`.
- `cargo xtask` with `css`, `version-sync`, `i18n`, `rfc-sync`, `audit-deps`,
  `ui-logic-connectivity` and the new check.
- `git diff --check`.
- `mdbook build docs`.
- `python3 -m py_compile packaging/render_check.py`.
- `actionlint`.
- A green CI run for the final commit, with its run ID.

Reply in `dev-record/review-requests/104-0171-review-follow-ups.md`.
