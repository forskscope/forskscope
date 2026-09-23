# Review Request 097 — Handoff 027: F99 recovery errors, PathBar labels

Handoff: `dev-record/handoffs/027-f99-recovery-errors-and-pathbar-labels.md`
Commit: `61e30dd` (pushed to `main`). CI run `34166837381`: green.

## §3's falsifications, run for real

### C10 — reverting either recovery handler fails the end-to-end test

Falsified by reverting `settings_recovery_action`'s `Err` arm back to
`store.notify(e.to_string())`:

```
thread 'ui::overlay::modals::recovery::tests::settings_reset_conflict_notifies_the_mapped_message_not_the_raw_one'
panicked at crates/forskscope-ui/src/ui/overlay/modals/recovery.rs:360:13:
assertion `left != right` failed: the raw PersistenceCommitError text must
not reach the user
  left: "file changed on disk since it was read; refusing to overwrite"
 right: "file changed on disk since it was read; refusing to overwrite"
```

Same for `session_recovery_action`'s arm:

```
thread 'ui::overlay::modals::recovery::tests::session_reset_conflict_notifies_the_mapped_message_not_the_raw_one'
panicked at crates/forskscope-ui/src/ui/overlay/modals/recovery.rs:405:13:
assertion `left != right` failed: the raw PersistenceCommitError text must
not reach the user
  left: "file changed on disk since it was read; refusing to overwrite"
 right: "file changed on disk since it was read; refusing to overwrite"
```

Both tests drive the real dispatch function end to end through
`with_test_store` against a **genuine** `PersistenceCommitError::Conflict`
— `ConfigRootOverrideGuard` points `config_file_path` at a temp
directory, the test writes different bytes on disk than the resolution's
`raw_bytes` claims were read, and `reset_settings_with_backup`'s real
`verify_unchanged` call produces the error, not a hand-built one. Both
restored and green afterward.

### C3 — removing an `aria_label` fails the rendered-attribute test

Falsified by removing `aria_label: t(lang, "Home directory")` from
`PathBar`'s Home button:

```
thread 'ui::view::dir_pane::tests::path_bar_buttons_carry_the_expected_aria_label'
panicked at crates/forskscope-ui/src/ui/view/dir_pane.rs:688:13:
expected an aria-label "Home directory" among the rendered attributes
["Back", "Forward", "Go up one directory", "Open folder…"]
```

Restored and green.

## Design decisions disclosed

**`AppError::new`, not `AppError::from_core` as the handoff suggested.**
`PersistenceCommitError` is deliberately not `CoreError` — its own doc
comment says so, and its two variants (`Conflict`, `Io(String)`) don't
map onto `CoreError`'s IO/document-shaped variants, so `from_core` does
not type-check against it. `AppError::new(kind, technical_detail)` is
the sibling constructor whose own doc comment describes exactly this
case ("when the kind is known directly ... from application-layer code
that doesn't go through `CoreError`"). I mapped `Conflict` →
`AppErrorKind::SaveConflict` (closest existing kind: the file changed on
disk since it was read) and `Io` → `FileWriteFailed`. Only
`message.short` is surfaced, not `.detail` — `UserMessage`'s own doc
says `short` fits a toast and `detail` fits a dialog body, and
`store.notify` only ever shows a toast at this call site; there's no
dialog surface to put `detail` in without inventing one, which the
handoff didn't ask for and I didn't build.

**Only one of the two stale comments actually needed fixing.**
Re-reading `overlay/modals/file.rs:268` before touching it: it already
says `handle_result`'s **old** `Err(e) => store.notify(e.to_string())`
arm **set** [a precedent] — accurately past tense, correctly describing
history rather than claiming the pattern still exists. `diff_actions.rs:321`
was the one that actually said "is the established precedent" (present
tense) about an arm that no longer has that shape — that's the one I
rewrote, to name the migration explicitly rather than silently drop the
claim. Flagging this because the handoff named both lines; I checked
before editing rather than mechanically changing text that was already
correct.

**Checked whether Home/Open-folder have a keyboard route at all: they
don't.** Grepped `keyboard.rs`, `app.rs`, and the Explorer view tree for
any binding reaching `home_dir()`/`pick_folder` outside the button
click — nothing. Per the handoff, reporting this rather than fixing it:
adding a keyboard shortcut is new functionality, not part of a five-line
accessibility-attribute fix. The help modal's existing gaps stand as
found: Up and Back/Forward are documented (`overlay/keybindings.rs:42-43`),
Home and the folder picker are not, for either input method.

**How the C3 test works, and its disclosed limit.** No `dioxus-ssr`
dependency exists in this workspace, so the test renders `PathBar` via
a bare `VirtualDom` and calls `rebuild_to_vec()`, then scans the
resulting `Mutations` for `Mutation::SetAttribute { name: "aria-label",
value: AttributeValue::Text(s), .. }` entries — both types come from
`dioxus_core`, already a direct dependency, so no new one was added.
This proves the attribute is present on the rendered element with the
expected translated string. It does **not** prove the attribute reaches
a real screen reader's accessibility tree — that remains an AT-SPI/UIA
assertion, the same limit F74 recorded and review 093 disclosed for the
untestable encoding `<select>`. Stating this plainly rather than
implying more coverage than the test gives.

## Scope

Touched exactly what §4 named: `overlay/modals/recovery.rs`,
`ui/view/dir_pane.rs`'s `PathBar`, the one stale comment that needed it
(`diff_actions.rs:321`), and tests. Did not touch C1's first-run
persistence, C6's plain-language settings tier, or C7's narrow-layout
trust marker — all three recorded in F99 as deliberately unscheduled
polish, not built here.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (736 core / 128 ui-lib / 128 ui-bin /
204 ui-logic, all green — +5 ui-lib tests, +5 ui-bin tests over the
pre-handoff baseline: 2 pure `recovery_failure_message` mapping tests, 2
end-to-end recovery tests, 1 PathBar aria-label test), `cargo xtask css
--check`, `version-sync`, `i18n` (**244 keys, unchanged** — no new
strings, exactly as the handoff predicted since `aria_label` reuses each
button's existing `title` translation call), `rfc-sync`, `audit-deps`,
`git diff --check`, `mdbook build docs` — all green. `cargo audit`: exit
0, the same 14 pre-existing warnings as recent reviews (no dependency
added).
