# Handoff 081 — 0.193.0: carry it with you

> **Updated 2026-10-17: RFC-091 is amended. Read its §2 (the end) and §2a
> before continuing.**
>
> 1. **Linux:** the system web view's cache and HSTS file are redirected with
>    the XDG variables, exactly as review 167 decided. That is now in the RFC.
> 2. **macOS:** `--private` and `--data-dir` **refuse to start**. They print one
>    plain message and exit with status 2, before any window. On macOS `--help`
>    omits them. **No macOS wording in the status bar, About or the docs.**
>    *Running portably* says once that the modes are for Windows and Linux, and
>    why.
> 3. **Windows:** measure where WebView2 keeps its cache. If anything escapes
>    the data location, **stop and tell me.** By the RFC, Windows would then get
>    the same refusal.
> 4. **The status-bar and About texts are unblocked.** They are the RFC's own
>    texts, with no platform exception anywhere. Where a mode runs, its promise
>    is whole.
>
> **Acceptance criterion 9 is new:** a resolver unit test with the platform as a
> parameter.

**From:** architect. **Date:** 2026-10-15. **Release:** `0.193.0`.
**Implements:** RFC-091 (`rfcs/accepted/091-portable-and-private-modes.md`),
accepted by the owner on 2026-10-15. **Register:** F45 (a).

**Start after handoffs 079 and 080 (0.192.0) are delivered.** All three touch
`xtask`, `release.yml` and the user guide.

**The RFC is the authority.** Where this document and the RFC differ, the RFC
wins. If implementation shows the RFC is wrong anywhere, for example a write to
home you cannot redirect or a linking flag that does not work, **stop and tell
me before changing course.** Do not change the design yourselves, and do not edit
any RFC.

---

## 1. The startup decision (RFC-091 §1)

- **One pure function, in `forskscope-ui-logic`:** given the arguments, the real
  program folder, and whether a `forskscope-data` folder exists there, it returns
  the mode (Private / Portable(path) / Installed) or an error.
  - `main.rs` calls it **before** any settings, session or web-view path is
    resolved, and passes the result to everything that writes.
  - Unit-test every row of §1's table, the `--private` + `--data-dir` exit 2,
    and a symbolic link resolved to the real program folder. Falsify each.
- **`config_file_path`, the manifest folder (`copy.rs`) and the web-view data
  directory** all derive from that one result. No other code calls
  `dirs_next::*` for these. Grep and say so.
- **Flags are parsed before the file arguments,** like `--version`. An unknown
  `--` flag is an error.

## 2. Portable and private (RFC-091 §2–§3)

- **`dioxus-desktop`'s `Config::with_data_directory`** takes `<data>/webview`
  in portable mode, and a fresh folder under the system temporary directory in
  private mode, removed on exit.
- **On Linux, measure first:** does WebKitGTK put **both** its data and its
  cache under that directory, or does the cache still go to
  `~/.cache/forskscope`? If it cannot be redirected through supported means,
  **stop and tell me.**
- **The status bar shows the mode** in every non-installed case, with the exact
  texts of RFC-091 §2 and §3 in English and Japanese.
- **About and *Copy diagnostics*** show the mode and the data location.
- **The read-only data location** (§2) is detected at startup by an actual write
  probe, not by permission bits. Then:
  - the app reads what it can;
  - it saves nothing;
  - it says so.

## 3. Windows: the hybrid CRT (RFC-091 §4)

- **`.cargo/config.toml`,** for `x86_64-pc-windows-msvc`:
  - `+crt-static`;
  - the linker arguments that swap the static UCRT for the dynamic one
    (`/NODEFAULTLIB:libucrt.lib` and `/DEFAULTLIB:ucrt.lib`, or the equivalent
    you verify).
  - Every Windows build uses it, the zip's and the Store's.
- **The release gate,** in `release.yml`'s Windows job:
  - list `forskscope.exe`'s imports (`dumpbin /imports`, or `llvm-objdump -p`);
  - fail on `VCRUNTIME*`, `MSVCP*`, or any DLL not on an explicit allow-list of
    Windows system DLLs. Build the allow-list from what the program imports
    after the change, and justify each entry in a comment.
  - **Falsify:** remove `+crt-static`, and the gate fails naming
    `VCRUNTIME140.dll`.
- **The cross-target clippy** (`x86_64-pc-windows-gnu`) is a check-only target.
  Confirm the config does not break it.
- **Report the size change** of `forskscope.exe`.

## 4. Documentation (RFC-091 §5)

- **`--help`** and `docs/src/intermediate/cli.md`: `--data-dir <path>` and
  `--private`, with their exact meaning. Private mode states *start with default
  settings, and save nothing*. It also says that the files you compare and save
  are still your files, written when you save.
- **`docs/src/users/installation.md`** gains **Running portably**:
  - the marker folder, `--data-dir`, `--private`;
  - what each writes and does not;
  - how to move existing settings (copy `settings.json`);
  - **a short, explicit macOS note** (the owner asked for it): *on macOS, use
    `--data-dir` or `--private`. A `forskscope-data` folder beside the app is not
    used, because macOS runs a downloaded app from a hidden, read-only copy.*
  - **The anchor must be `#running-portably`.** The landing page's Stage 2 will
    link to it.
- **The landing page is not yours.** Stage 2 of its wording returns to the owner
  when this ships.

## 5. Verification

**RFC-091's acceptance criteria 1 to 8, one by one, with evidence.** In
particular:

- **Criterion 3, the read-only home:**
  - run under `bwrap --ro-bind "$HOME" "$HOME"`, with invented files only and a
    portable folder outside home;
  - watch `$HOME` for write attempts (`inotifywait -m -r`, or `strace -f -e
    trace=%file` filtered to `$HOME`);
  - list every attempt and its source;
  - say for each one that is not ForskScope's own whether anything fails.
- **Criterion 5, private mode:** compare timestamps of every ForskScope file
  before and after, and confirm the temporary web-view folder is gone after
  exit.
- **Criterion 6, Windows:** show the import list from the CI build, before and
  after.
- **Screenshots** of the status bar in Portable, Portable (read-only) and
  Private, in the sandbox. Describe each one after looking at it.

## Scope

- **In:** §1 to §4; i18n (English and Japanese).
- **Out:**
  - A Settings control for the mode.
  - Importing or syncing settings.
  - The marker folder on macOS.
  - Bundling WebKitGTK or WebView2.
  - The landing page.
  - RFCs.
  - `CHANGELOG.md`, `ROADMAP.md`, the version.

## Gates

The standard set, including the Windows-target clippy with `--all-targets`,
`cargo fmt --manifest-path xtask/Cargo.toml --check`, every `cargo xtask` gate,
`pages-origin-check` and `pages-link-check` on an assembled site,
`mdbook build docs`, `actionlint`, the new Windows import gate (seen running in
CI on the Windows runner), and a green CI run with its ID.

Reply in `.git-exclude/review-request/156-carry-it-with-you.md`.
