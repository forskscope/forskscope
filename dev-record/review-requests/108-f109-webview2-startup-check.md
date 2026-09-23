# Review Request 108 — F109: tell Windows users when WebView2 is missing

Handoff: `dev-record/handoffs/036-f109-webview2-startup-check.md`
Commit: `f50a954` (pushed to `main`). CI run `35169964128` confirmed green.

## 1. The check

`crates/forskscope-ui/src/main.rs`, Windows-only (`#[cfg(windows)]`),
`check_webview2_or_exit()`: runs after `--diagnostics` and
`parse_startup_args`, before `dioxus_desktop::launch::launch`. Calls
`wry::webview_version()`, maps `Err` to a string, and hands it to
`webview2::decide`.

- **On failure:** prints the stderr line, shows an `rfd::MessageDialog`
  (level Error, title `ForskScope`, Yes/No, the exact text from the
  handoff), and exits **3** whichever button is pressed. Documented in
  `main.rs`'s module doc, alongside exit code 1.
- **Yes** opens `https://developer.microsoft.com/microsoft-edge/webview2/`
  via `std::process::Command::new("cmd").args(["/C", "start", "", url])`.
  **Why this and not `rundll32 url.dll,FileProtocolHandler`**: `Command`'s
  `args` array builds Windows' command line without going through a shell
  string, so there is no quoting to get wrong for `/` or `-` in the URL
  either way — the real reason to prefer `cmd /C start` is the empty
  `""` argument immediately after `start`, which is `start`'s own
  window-title parameter. Without it, `start` would treat the URL itself
  as a quoted title (its documented "first quoted argument is the title"
  behavior) and never launch it. No new dependency; `cmd.exe` is always
  present.

**The pure half** (handoff §5) is `webview2::decide(version: Result<String,
String>, lang: Lang) -> Decision` in the new
`crates/forskscope-ui/src/webview2.rs` — no OS calls, so it is
unit-tested without a window, a real install, or Windows itself.

## 2. Language: the side-effect-free case applies

Found `forskscope_core::persist::schema::settings::SettingsRepository::load()`
— confirmed by reading `repository.rs`: it calls
`read_to_string_or_missing` + the pure `load_settings` parser only, no
`atomic_write_envelope`, no backup, no migration commit. That is a
genuinely read-only path (unlike `session.rs`'s `resolve_and_commit`,
which durably rewrites migrated files — not used here on purpose).

`main.rs`'s `saved_lang_no_side_effects()` uses it: `Current` or
`MigratedLegacy` (the migration isn't committed by reading it) gives the
saved `language`; `Missing`, `FutureVersion`, or `Corrupt` fall back to
English, matching the owner's 2026-09-17 first-launch decision. **This is
the "core has such a read-only path, use it" case**, not the English
fallback — the fallback only applies when no settings file has ever
been written, or it can't be read as a current/migrated value.

## 3. `--diagnostics`

Windows-only, added directly in `main.rs` after the existing platform
report (core still has no `wry` dependency):
```
WebView2 runtime: <version>          # present
WebView2 runtime: not found (<error>)  # absent
```

## 4. i18n

Both strings added as literal `t(lang, "...")` arguments (one in
`webview2::decide`, matching `cargo xtask i18n`'s literal-call-site
scan) with Japanese entries in `i18n.rs`'s `ja()`. `cargo xtask i18n`:
`i18n audit passed: 248 UI keys are covered by Japanese translations.`
(246 before this change — the two new keys are counted.)

## 5. Documentation

- `installation.md` (Windows section): describes the message box, the
  Yes/No choice, and exit code 3; kept "opens blank" removed for the
  WebView2 case (never verified it happens — the handoff's own framing).
  The Visual C++ sentence is unchanged and now explicitly contrasted:
  that failure happens **before** any of ForskScope's own code runs, so
  it looks different (a `VCRUNTIME140.dll` message, not this dialog).
- `troubleshooting.md`: rewrote the WebView2 entry's Symptom/Cause/Fix to
  describe the actual current behavior instead of a generic "an error
  dialog mentions WebView2".

## 6. Verification

### Unit tests, falsified for real

`webview2.rs`'s two tests (`present_proceeds`,
`absent_stops_with_a_non_empty_bilingual_message`) were run against a
temporarily broken `decide` that returned `Decision::Proceed`
unconditionally (an extra `Err(_error) if true => Decision::Proceed`
arm ahead of the real one):

```
test webview2::tests::present_proceeds ... ok
test webview2::tests::absent_stops_with_a_non_empty_bilingual_message ... FAILED

---- webview2::tests::absent_stops_with_a_non_empty_bilingual_message stdout ----
thread '...' panicked at crates/forskscope-ui/src/webview2.rs:78:38:
expected Stop when the runtime is absent
```

Confirmed the failure is real (not a vacuous pass), then restored the
file exactly and re-ran: `2 passed; 0 failed`.

### Real Windows CI, the missing-runtime path — not yet run

Added the step to `release.yml`'s `windows` job (right after `Build
release binary`, before `Package`): runs `forskscope.exe --diagnostics`
twice — once as-is, once with `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`
pointed at a freshly created empty directory — and prints both outputs.

**I have not obtained a real run of this step.** `release.yml` only
triggers on a pushed `X.Y.Z`-form tag, and per review 108 §5 I am not
creating one for a rehearsal without asking first. Checked the other two
Windows-capable workflows too: `store-submit.yml` requires an
already-*released* tag (`checkout ref: <tag>`) and only ever builds a
Store package, not a plain `--diagnostics` check; `m5-evidence-windows.yml`
is deliberately scoped to the *published* 0.167.1 artifact (RFC-078 §5),
which predates this feature and must never be built from source. None of
the three lets me exercise this new code on Windows without either a
real release or a rehearsal tag.

**Asking rather than guessing**: should I push a scratch (non-`X.Y.Z`)
tag or dispatch one of these differently to get a real run before
0.172.0, or is the actual 0.172.0 release tag the first time this step
runs for real? I did not act unilaterally either way.

### The message box — the M5 harness cannot drive it, in scope

`packaging/evidence/windows_harness.py` is architecturally bound to the
*published* 0.167.1 Windows artifact (downloaded and digest-verified,
never built from source — its own module doc states this is intentional
per RFC-078 §5) and its `case` input has a closed set of options with
none for this dialog. Driving the new message box for real would need a
new harness case plus a source-built binary in that workflow, which
means editing `windows_harness.py` and `m5-evidence-windows.yml` — both
outside this handoff's scope (`main.rs`, one module, `i18n.rs`, the two
docs files, one CI step). **I did not attempt it.** Per the handoff's own
fallback: I am saying plainly that it can't be done within scope rather
than guessing it works, so the architect can add the manual 0.172.0
pre-publish check.

### No regression when WebView2 is present

- CI run `35169964128` (this commit, `f50a954`) — the ordinary Linux
  `ci.yml` pipeline, all steps green, including `cargo test (workspace)`
  and both clippy passes.
- No `render-check.yml` dispatch was made for this change — it exercises
  only Explorer/diff-view rendering (Linux, AT-SPI), which this handoff's
  code never touches (the new code sits entirely before the window is
  created). Windows launch itself is unverified for the same reason §6.2
  above describes: no Windows CI run of the built binary happened yet.

## Scope

Exactly the handoff's "In" list: `crates/forskscope-ui/src/main.rs`,
the new `crates/forskscope-ui/src/webview2.rs` (in `forskscope-ui`, not
`ui-logic` — confirmed by `cargo xtask ui-logic-connectivity`/
`ui-logic-docs` both passing unchanged, since neither table needed a new
row), `i18n.rs`, `installation.md`, `troubleshooting.md`, and one step
in `release.yml`. No dependency added (`wry`/`rfd` were already direct
dependencies — confirmed by reading their `Cargo.toml` entries and the
installed `wry-0.53.5`/`rfd-0.17.2` source before writing any code).
Untouched: `CHANGELOG.md`, `ROADMAP.md`, RFC-080, F107.

## Gates

- `cargo fmt --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean. (The
  new module needed `#[cfg_attr(not(windows), allow(dead_code))]` on
  `Decision`/`decide`, since only `main.rs`'s `#[cfg(windows)]` path
  calls them outside of tests, and Linux `-D warnings` treats that as an
  error otherwise.)
- **Windows-target clippy**: not run — no dispatchable Windows CI job
  builds from source without a real/rehearsal tag (see §6.2). Flagging
  this rather than skipping it silently.
- `cargo test --workspace` — clean, `152 passed` in `forskscope-ui`
  (includes the two new `webview2` tests) plus all other crates
  unchanged.
- `cargo xtask i18n/ui-logic-connectivity/ui-logic-docs/rfc-sync/audit-deps/version-sync`
  — all clean (`i18n audit passed: 248 UI keys`).
- `mdbook build docs` — clean.
- `actionlint` (ran the cached binary at `.git-exclude/tmp/actionlint`,
  since it isn't on `PATH` in this sandbox) against every workflow file,
  including the edited `release.yml` — clean.
- CI run `35169964128` for `f50a954` confirmed green.
