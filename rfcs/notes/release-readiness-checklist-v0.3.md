# Release Readiness Checklist v0.3

## 1. Functional readiness

- [ ] Two-file comparison opens from file picker.
- [ ] Two-file comparison opens from command-line arguments.
- [ ] Directory comparison opens from file picker.
- [ ] Directory comparison opens from command-line arguments.
- [ ] Text diff shows line-level changes.
- [ ] Text diff shows inline character-level changes where enabled.
- [ ] Binary/unsupported files show a safe fallback page.
- [ ] Merge hunk copy works left-to-right.
- [ ] Merge hunk copy works right-to-left.
- [ ] Manual edits update dirty state.
- [ ] Undo/redo works for merge actions.
- [ ] Save requires explicit destination confirmation when risk exists.
- [ ] Atomic write and backup are tested.

## 2. UX readiness

- [ ] First-run page is understandable.
- [ ] Empty states are helpful.
- [ ] Error messages include cause and recovery action.
- [ ] Keyboard shortcut reference exists.
- [ ] Main workflows can be completed without hidden gestures.
- [ ] Long-running directory comparison shows progress and cancellation.
- [ ] Dangerous batch operations require preview confirmation.

## 3. Cross-platform readiness

- [ ] Linux Wayland smoke test passes.
- [ ] Linux X11 smoke test passes or fallback is documented.
- [ ] Windows smoke test passes.
- [ ] macOS smoke test passes.
- [ ] WebView missing/outdated diagnostics are actionable.
- [ ] File dialog behavior is verified per platform.
- [ ] External tool launch is verified per platform.

## 4. Data safety readiness

- [ ] Session files are versioned.
- [ ] Session migration tests exist.
- [ ] Corrupt session files fail safely.
- [ ] Backup restore procedure is documented.
- [ ] External file modification detection exists.
- [ ] Read-only file behavior is safe.

## 5. Documentation readiness

- [ ] User guide exists.
- [ ] Quick start exists.
- [ ] Directory merge guide exists.
- [ ] Safe save/backup guide exists.
- [ ] Troubleshooting guide exists.
- [ ] Known limitations are documented.

## 6. Release decision

A v3 preview release may be published only when all blocking items above are complete or explicitly waived by project owner approval.
