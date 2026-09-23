# Review 111 — Request 109: F109 follow-ups, Windows checks

**Reviewer:** architect. **Date:** 2026-09-17. **Reviewed:** `0c21074`, `151e98e`.
**Verdict:** **Approved. F109 is closed.**

## A. Windows compile check on every push

- **`save.rs`.** The `mut` binding now exists only in the `#[cfg(unix)]`
  helper, and behaviour is unchanged on both sides: Unix still requests
  `0o666`, and other platforms get the default. No `#[allow]`.
  - One naming nit, which you may fix whenever you next touch the file: the
    `#[cfg(not(unix))]` variant is also called `unix_tempfile_builder`, though
    it has nothing Unix-specific about it. A neutral name such as
    `temp_builder` reads better. It does not block approval.
- **The four test files.** You were right to fix them, and right to say so.
  My local check compiled only `-p forskscope-ui`, so it never built
  `forskscope-core`'s tests for Windows. **The gap was in my handoff, not
  scope creep in your work.** Each change narrows imports or one helper to
  `cfg(unix)`. Linux is `unix`, so no test is lost where tests actually run.
- **Falsification.** PR run `35171366915` shows the new step alone failing
  with `E0308` on a Windows-only type error, while the native clippy step
  passed. I confirmed that the PR is `CLOSED`, the branch is gone from
  `origin`, and no tag was pushed. Using a PR because `ci.yml` does not run on
  branch pushes was the right workaround.
- **Cost.** About 71 s per push. Acceptable for what it catches.

## B. `windows-check.yml`

**I read the workflow, not only the summary:**
- The binary's output is piped (`2>&1 | Out-String`), so PowerShell waits for
  it even though it is a GUI-subsystem program.
- Empty output fails first and on its own, so the stdout concern cannot pass
  silently.
- Both patterns are anchored with `(?m)^`.
- Both outputs are printed before any assertion.

**Run `35171995087`'s log confirms the evidence:**
```
WebView2 runtime: 152.0.4191.66
WebView2 runtime: not found (WebView2 error: WindowsError(Error { code: HRESULT(0x80070002), … }))
Both assertions passed.
```

That settles the open question: **pointing `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`
at an empty directory does simulate a missing runtime** on real Windows. The
owner's pre-publish check in review 109 §6 is therefore valid as written.

Removing the unasserted step from `release.yml`, and adding the checklist line
in `release.md`, are both right.

## C. The Japanese text

The falsification shows the new assertion failing, with identical left and
right values, once the `ja()` entry is removed. That is exactly the failure
the test's name promises to catch.

## Register

- **F109:** closed.
- **The 0.172.0 pre-publish list** keeps the owner's message-box check:
  1. Set `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` to an empty folder.
  2. Launch `forskscope.exe`.
  3. The dialog appears; No exits, and Yes opens Microsoft's page.
- **The 0.172.0 cut** now also requires a green `windows-check.yml` dispatch
  for the release commit, which the checklist in `release.md` now says.
