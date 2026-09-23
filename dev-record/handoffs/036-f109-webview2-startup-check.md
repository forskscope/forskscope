# Handoff 036 — F109: tell Windows users when WebView2 is missing

**From:** architect. **Date:** 2026-09-17. **Release:** 0.172.0.
**Register:** F109 in `ROADMAP.md`. The owner approved scheduling it on
2026-09-17.

## Why

ForskScope renders through the WebView2 Runtime on Windows. When the runtime is
not installed, `dioxus-desktop` 0.7.9 reaches `let webview = webview.unwrap();`
in `src/webview.rs` and panics.

Release builds use `windows_subsystem = "windows"`, so there is no console and
the panic message goes nowhere. **The user double-clicks the app and nothing
useful happens.** I established this by reading the code, not by running it
without WebView2.

F45(b) records the dependency. The only mitigation today is a sentence in
`docs/src/users/installation.md`, which says the window "opens blank". That
probably does not match the real symptom.

## 1. The check

In `crates/forskscope-ui/src/main.rs`, **on Windows only** (`#[cfg(windows)]`):

1. **Placement.** Run the check after the `--diagnostics` branch and after
   `parse_startup_args`, so argument errors still print and exit as they do
   now. It must come **before** `dioxus_desktop::launch::launch`.
2. **The test.** Call `wry::webview_version()`. It is already available: `wry`
   is a direct dependency, and `dioxus-desktop` enables its `os-webview`
   feature. On Windows it calls `GetAvailableCoreWebView2BrowserVersionString`.
   Treat `Err` as "not installed".
3. **When it fails:**
   - Write one line to stderr: `forskscope: the Microsoft Edge WebView2
     Runtime is not installed (<error>)`.
   - Show a native message box with `rfd::MessageDialog`, which is already a
     dependency and needs no WebView. Use level Error, title `ForskScope`, and
     Yes/No buttons.
     - Text: *"ForskScope needs the Microsoft Edge WebView2 Runtime, which is
       not installed on this computer. Open the download page?"*
   - **Yes** opens `https://developer.microsoft.com/microsoft-edge/webview2/`,
     the same URL `installation.md` uses, in the default browser. Add **no
     new dependency** for that. Use a Windows shell call that has no
     quoting pitfalls with a URL containing `/` and `-`, and say which one
     you chose and why.
   - Whichever button is pressed, exit with code **3**. Codes 1 and 2 are
     taken: 1 is the startup-argument error, and 2 is the usage path of other
     tools. Document 3 in the `main.rs` module doc.
4. **Language.** Both strings (the message and the stderr line) go through the
   project's i18n with Japanese entries, so `cargo xtask i18n` covers them.
   - Use the **saved language** only if the persisted settings can be read
     **without side effects**: no migration writes, no recovery flow, no
     settings file created. If core has such a read-only path, use it.
   - If it doesn't, use English. The owner decided on 2026-09-17 that a first
     launch is English. Do **not** build a new settings reader for this.
   - Say which case applies.
5. **Keep the decision testable.** Put the part that doesn't touch the OS in
   a plain function, so it can be unit-tested: given the check's result,
   produce proceed or stop, with the message. `main.rs` only wires the OS
   calls to it.

**Out of scope:**
- A runtime that is installed but broken.
- WebView2 installing itself, or bundling a fixed-version runtime.
- A missing Visual C++ runtime (F45(a)). Windows cannot load the executable
  at all in that case, so no check in `main` can run.

## 2. `--diagnostics`

On Windows, add one line to the `--diagnostics` output:
- `WebView2 runtime: <version>` when it is present;
- `WebView2 runtime: not found (<error>)` when it is not.

Print it from `main.rs`, because `forskscope-core` has no `wry`. On other
platforms, print nothing new.

## 3. Documentation

`docs/src/users/installation.md` (Windows section) and
`docs/src/users/troubleshooting.md` (the WebView2 entry) must describe the new
behaviour: the message box, the link, and exit code 3. Remove "the window opens
blank" unless you can show that it happens. The Visual C++ sentence stays,
because that failure is unchanged.

## Verification

1. **Unit tests** for the decision function:
   - present leads to proceed;
   - absent leads to stop, with a non-empty message in both languages.

   Show that each test fails against a deliberately broken decision
   function, **not only** that it passes.
2. **Real Windows CI, the missing-runtime path.** CI's Windows runners
   (`windows-latest`) have WebView2 installed. The WebView2 loader honours
   `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`.
   - **Expected:** pointing it at an empty directory makes
     `GetAvailableCoreWebView2BrowserVersionString` fail.
   - **Confirm it:** in a CI step, run `forskscope --diagnostics` with and
     without that variable.
   - **Show both outputs:** the version without the variable, and `not found`
     with it.
   - **If the variable does not produce a failure,** say so and quote what
     happened. Do **not** add a test-only switch to the product to fake it.
3. **The message box.** If the existing Windows evidence harness (M5,
   UI Automation) can launch the app with that variable, find the dialog and
   press No at reasonable cost, do it. Show the dialog's text and exit code 3.
   If it can't, say so. I will then put a manual check on the 0.172.0
   pre-publish list for the owner. Do not guess the dialog works.
4. **No regression when WebView2 is present:**
   - the plain `render-check.yml` dispatch passes;
   - the Windows CI job still launches the app normally.

   Give the run IDs.

## Scope

- **In:**
  - `crates/forskscope-ui/src/main.rs`
  - a small module for the decision function, in `forskscope-ui` or
    `forskscope-ui-logic`. Mind the connectivity and doc-table checks if it
    goes in `ui-logic`: a new module needs a row in both tables.
  - `i18n.rs` entries
  - `installation.md` and `troubleshooting.md`
  - one CI step, in whichever existing Windows workflow fits
- **Out:**
  - dependencies: none added
  - bundling or installing WebView2
  - Visual C++ runtime linking (owner question open)
  - `CHANGELOG.md` and `ROADMAP.md`
  - RFC-080 and F107

**Rehearsal tags:** do not push a tag of the release form (`X.Y.Z`) for any
experiment. See review 108 §5.

## Gates

- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D
  warnings`.
- **Also clippy for the Windows target**, if you can run it at all. The new
  code is `cfg(windows)`, so Linux clippy never compiles it; CI's Windows
  build is the real check, so name that run.
- `cargo test --workspace`.
- `cargo xtask` with `i18n`, `ui-logic-connectivity`, `ui-logic-docs`,
  `rfc-sync`, `audit-deps` and `version-sync`.
- `mdbook build docs`.
- `actionlint`, for the workflow edit.
- A green CI run for the final commit, with its run ID.

Reply in `dev-record/review-requests/108-f109-webview2-startup-check.md`.
