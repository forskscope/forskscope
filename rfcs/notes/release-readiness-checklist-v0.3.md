# Release Readiness Checklist v0.3

**Last updated:** v0.141.0 (2026-06-13)

Items marked `[x]` are confirmed complete. Items marked `[ ]` require GTK
or are explicitly deferred. Items marked `[~]` are partially complete.

## 1. Functional readiness

- [x] Two-file comparison opens from command-line arguments.
- [x] Text diff shows line-level changes.
- [x] Text diff shows inline character-level changes where enabled.
- [x] Binary/unsupported files show a safe fallback page.
- [x] Merge hunk copy works left-to-right.
- [x] Undo/redo works for merge actions.
- [x] Save requires explicit destination confirmation when risk exists.
- [x] Atomic write and backup are tested.
- [ ] Two-file comparison opens from file picker. *(requires GTK)*
- [ ] Directory comparison opens from file picker. *(requires GTK)*
- [ ] Directory comparison opens from command-line arguments. *(requires GTK)*
- [ ] Merge hunk copy works right-to-left. *(inverse direction, requires GTK verify)*
- [ ] Manual edits update dirty state. *(editor-adapter track, deferred)*

## 2. UX readiness

- [x] Keyboard shortcut reference exists. *(keybindings.rs, Ctrl+/)*
- [x] Main workflows can be completed without hidden gestures.
- [x] Dangerous batch operations require preview confirmation.
- [x] Error messages include cause and recovery action.
- [~] Long-running directory comparison shows progress. *(progress shown; cancellation UI deferred)*
- [ ] First-run page is understandable. *(no first-run page; deferred post-v1)*
- [ ] Empty states are helpful. *(Explorer shows current dir; "No comparison." for diff)*

## 3. Cross-platform readiness

- [ ] Linux Wayland smoke test passes. *(requires GTK/display server)*
- [ ] Linux X11 smoke test passes or fallback is documented. *(requires GTK)*
- [ ] Windows smoke test passes. *(requires Windows CI)*
- [ ] macOS smoke test passes. *(requires macOS CI)*
- [x] WebView missing/outdated diagnostics are actionable. *(troubleshooting.md, --diagnostics)*
- [ ] File dialog behavior is verified per platform. *(requires GTK)*
- [ ] External tool launch is verified per platform. *(requires GTK)*

## 4. Data safety readiness

- [x] Session files are versioned. *(VersionedEnvelope + SESSION_SCHEMA_VERSION=1)*
- [x] Session migration tests exist. *(persist_tests, session_tests)*
- [x] Corrupt session files fail safely. *(load_or_default() fallback)*
- [x] External file modification detection exists. *(check_external_state)*
- [x] Read-only file behavior is safe. *(can_save predicate, read-only notice)*
- [x] Backup restore procedure is documented. *(docs/src/users/merging.md)*

## 5. Documentation readiness

- [x] User guide exists. *(docs/src/users/)*
- [x] Quick start exists. *(docs/src/users/quick-start.md)*
- [x] Directory merge guide exists. *(docs/src/users/directory-compare.md)*
- [x] Safe save/backup guide exists. *(docs/src/users/merging.md)*
- [x] Troubleshooting guide exists. *(docs/src/users/troubleshooting.md)*
- [x] Known limitations are documented. *(docs/src/users/known-limitations.md)*

## 6. Release decision

v1.0 release candidate requires completing the 3 GTK smoke-test items
in RFC-041 (two-file compare end-to-end, directory compare, keyboard
navigation). Packaging (RFC-010) is separate and follows.
