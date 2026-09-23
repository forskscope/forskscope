# i18n Binary Tooltip Review

Request: `dev-record/review-requests/019-i18n-binary-tooltip.md`

Date: 2026-07-10

## Verdict

Accept with notes.

The known hardcoded binary-file badge tooltip is now routed through the existing `t(...)` i18n path, and the active `Lang` is threaded through all current `TreeRow` call sites. The existing i18n gate now covers this tooltip key and reports 203 Japanese-covered UI keys.

This resolves the specific i18n hardening note from `dev-record/reviews/008-ci-release-gate-alignment-rereview.md`. It does not change the broader i18n contract: `cargo xtask i18n` remains a coverage check for strings passed through `t(...)`, not a complete hardcoded-string scanner.

## Blocking findings

None.

## Non-blocking findings

None for this scoped tooltip fix.

## Requirement checks

- `crates/forskscope-ui/src/ui/view/dir_pane.rs:282` now uses `t(lang, "Binary file. Binary comparison is off — enable it in Settings → Advanced.")` for the binary badge tooltip.
- `crates/forskscope-ui/src/i18n.rs:214`-`crates/forskscope-ui/src/i18n.rs:216` adds the Japanese translation for the new tooltip key.
- `crates/forskscope-ui/src/ui/view/explorer/tree.rs:168` and `crates/forskscope-ui/src/ui/view/explorer/tree.rs:228` pass `lang` into aligned tree `TreeRow` calls.
- `crates/forskscope-ui/src/ui/view/explorer/compact.rs:69` and `crates/forskscope-ui/src/ui/view/explorer/compact.rs:124` pass `lang` into compact tree `TreeRow` calls.
- A call-site scan found the current `TreeRow` uses updated to include `lang`.

## Observed evidence

- `cargo fmt --check` passed.
- `cargo xtask i18n` passed with `i18n audit passed: 203 UI keys are covered by Japanese translations.`
- `cargo check -p forskscope-ui` passed.
- `git diff --check` passed.
- Generated `xtask/Cargo.lock` from the xtask run was removed after verification.

## Missing evidence

- No live desktop UI/manual tooltip language check was observed.
- No complete hardcoded user-facing string audit was performed.

## Recommended next action

Proceed with this tooltip localization fix. Keep any broader hardcoded-string scanning policy as a separate i18n process decision.
