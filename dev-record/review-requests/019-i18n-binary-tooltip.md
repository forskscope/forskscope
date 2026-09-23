# Review Request: Localize Binary Tooltip

## Context

This closes the remaining i18n hardening note from CI/release gate alignment review: the binary-file badge tooltip in the directory tree was a hardcoded user-facing string outside the `t(...)` translation coverage gate.

## Requirements Change

- No release contract change.
- No new i18n enforcement model is introduced here.
- The existing `cargo xtask i18n` gate remains a translation-coverage check for strings passed through `t(...)`; it is not a complete hardcoded-string scanner.

## Implementation

- Threaded the active `Lang` into `TreeRow`.
- Wrapped the binary badge tooltip with `t(lang, ...)`.
- Added the Japanese translation for the new tooltip key.
- Updated both aligned and compact tree call sites to pass `lang`.

## Files To Inspect

- `crates/forskscope-ui/src/ui/view/dir_pane.rs`
- `crates/forskscope-ui/src/ui/view/explorer/tree.rs`
- `crates/forskscope-ui/src/ui/view/explorer/compact.rs`
- `crates/forskscope-ui/src/i18n.rs`

## Verification Observed

- `cargo fmt --check`
- `cargo xtask i18n`
  - `i18n audit passed: 203 UI keys are covered by Japanese translations.`
- `cargo check -p forskscope-ui`
- `git diff --check`

## Known Limits

- No live desktop UI/manual tooltip language check was run.
- This does not prove there are no other hardcoded user-facing strings; it fixes the known reviewed tooltip.
