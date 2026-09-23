# Review Request 109 — F109 follow-ups: compile Windows code in CI, and prove the detection on real Windows

Handoff: `dev-record/handoffs/037-f109-windows-checks.md`
Commits: `0c21074` (part A), `151e98e` (parts B+C). CI runs `35171190383`
and `35171670335` confirmed green. `windows-check.yml` dispatched for
`151e98e`, run `35171995087`, confirmed green.

## A. Windows code now compiles on every push

`crates/forskscope-core/src/save.rs`: split the `tempfile::Builder`
construction into `unix_tempfile_builder()` (`#[cfg(unix)]`, does the
`mut` + `.permissions(0o666)` call) and a `#[cfg(not(unix))]` variant
returning the plain default — no `#[allow]` anywhere, matching the
handoff's explicit instruction. Both `atomic_replace` and
`persist_noclobber_with_hook` now call the shared helper. Behavior is
unchanged on every platform: unix still requests 0o666, everywhere else
still gets `tempfile`'s own default, exactly as before.

`ci.yml`'s existing Ubuntu job gained:
```
rustup target add x86_64-pc-windows-gnu
cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings
```
This is a type-check only (documented as such in the step's own
comment, naming what it does and does not cover) — nothing links, and
no MinGW toolchain was needed anywhere in the run.

**Running this for the first time against the full workspace surfaced
four more Windows-blind findings your own local check didn't hit**
(you ran `-p forskscope-ui` only, which never compiles
`forskscope-core`'s own test files): unused imports in
`dir_cancel_tests.rs`/`dir_unreadable_tests.rs`/`save_tests.rs` and one
dead-code function in `persist_v2_runtime_tests.rs`, all cases of a
helper or re-export used only inside `#[cfg(unix)]` test functions.
Fixed the same way — narrowed each import/function to `#[cfg(unix)]`
rather than suppressing the lint — since leaving them would have made
this handoff's own new gate fail on `main` immediately.

### Falsification, on a branch, not `main`

Branch `f109-windows-ci-falsification`, one commit adding
```rust
#[cfg(windows)]
const F109_FALSIFICATION_TYPE_ERROR: u32 = "this is not a u32";
```
to `main.rs`. Opened as PR #144 (a plain branch push doesn't trigger
`ci.yml`, which only listens to `push` on `main`/`master` — a PR does,
via `pull_request:`). Run `35171366915`:

```
✓ cargo clippy (workspace, all targets)              [native — passed]
✓ Add the Windows GNU target
X cargo clippy (workspace, all targets, Windows GNU target)
```
```
error[E0308]: mismatched types
  --> crates/forskscope-ui/src/main.rs:65:44
65 | const F109_FALSIFICATION_TYPE_ERROR: u32 = "this is not a u32";
   |                                      ---   ^^^^^^^^^^^^^^^^^^^ expected `u32`, found `&str`
error: could not compile `forskscope-ui` (bin "forskscope") due to 1 previous error
##[error]Process completed with exit code 101.
```
Every pre-existing Linux-only step (including the ordinary
`cargo clippy (workspace, all targets)` against the native target)
stayed green; only the new step failed. Closed PR #144 without merging
and deleted the branch, both locally and on `origin` (`gh pr close 144
--delete-branch`, confirmed by `git branch -a` and `git fetch --prune`
no longer listing it). No tag was pushed.

### Timing

From the clean, passing run (`35171190383`, part A alone):
`Add the Windows GNU target` — 2s. `cargo clippy (workspace, all
targets, Windows GNU target)` — 69s. **~71 seconds added to the job.**

## B. WebView2 detection proven on real Windows

Removed the `Confirm WebView2 detection (F109)` step from
`release.yml`'s `windows` job entirely (it only printed, asserted
nothing, and would have run for the first time at the 0.172.0 cut).

Added `.github/workflows/windows-check.yml`: `workflow_dispatch` only,
no push trigger. Builds `forskscope.exe` in release mode on
`windows-latest` (same command `release.yml` uses), then in one step:
captures `--diagnostics`' output, **fails first and separately** if
that output is empty (the GUI-subsystem stdout-capture concern from
handoff §B.3), then re-runs it with
`WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` pointed at a fresh empty
directory, printing both outputs unconditionally before asserting
anything.

**Dispatched against the head of `main` (`151e98e`)**:
`gh workflow run windows-check.yml --ref main` → run `35171995087`,
**green**, 5m28s (`Build release binary` alone took ~4m59s of that).

**Both real outputs, quoted from the run's own log:**
```
--- diagnostics: WebView2 present (this runner's real install) ---
ForskScope 0.171.1
...
WebView2 runtime: 152.0.4191.66
```
```
--- diagnostics: WEBVIEW2_BROWSER_EXECUTABLE_FOLDER pointed at an empty directory ---
ForskScope 0.171.1
...
WebView2 runtime: not found (WebView2 error: WindowsError(Error { code: HRESULT(0x80070002), message: "The system cannot find the file specified." }))
Both assertions passed.
```
**This answers the open question**: `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`
pointed at an empty directory genuinely does make
`wry::webview_version()` fail on a real `windows-latest` runner that
has WebView2 installed — it is not a no-op, and both assertions in the
handoff hold as written (no need to drop the second one per §B.5).

**stdout capture**: confirmed, empirically, not assumed — the
`WebView2 runtime: 152.0.4191.66` line above came from the real
release (`windows_subsystem = "windows"`) binary's `println!`, reaching
the step log normally, because the runner invokes it with stdout
piped/captured rather than attached to an interactive console.

Added a line to `docs/src/maintainers/release.md`'s pre-release
checklist (item 14): dispatch `windows-check.yml` for the release
commit and confirm green, before every cut.

## C. The Japanese text is proven to actually be used

`webview2.rs`'s `absent_stops_with_a_non_empty_bilingual_message` now
also asserts `dialog_texts[0] != dialog_texts[1]` (En vs Ja).

**Falsified for real**: temporarily deleted the `ja()` match arm for
the dialog string in `i18n.rs`, ran the test:
```
thread '...' panicked at crates/forskscope-ui/src/webview2.rs:86:9:
assertion `left != right` failed: the Ja dialog text must differ from the En one — a missing i18n.rs entry would silently fall back to English
  left: "ForskScope needs the Microsoft Edge WebView2 Runtime, which is not installed on this computer. Open the download page?"
 right: "ForskScope needs the Microsoft Edge WebView2 Runtime, which is not installed on this computer. Open the download page?"
```
Confirmed the failure was real, restored the entry exactly, re-ran:
`2 passed; 0 failed`.

## Scope

Exactly the handoff's "In" list:
`crates/forskscope-core/src/save.rs` (plus the four test files the new
gate itself required — see §A's note; not naming them ahead of time
was the handoff author's own gap, not scope creep, since the alternative
was landing a gate that fails on `main` immediately), `ci.yml`,
`release.yml` (the removal only), `windows-check.yml` (new),
`docs/src/maintainers/release.md` (one line), `webview2.rs` (the test).
Untouched: `m5-evidence-windows.yml`, the harness, `CHANGELOG.md`,
`ROADMAP.md`, RFC-080, F107. No tag pushed, rehearsal or otherwise.

## Gates

- `cargo fmt --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` (run locally too) — clean.
- `cargo test --workspace` — clean (154 in `forskscope-ui`, includes the strengthened webview2 test).
- `cargo xtask i18n/ui-logic-connectivity/ui-logic-docs/rfc-sync/audit-deps/version-sync` — all clean.
- `actionlint` on all three workflow files (`ci.yml`, `release.yml`, `windows-check.yml`) — clean.
- `mdbook build docs` — clean.
- CI run `35171190383` (`0c21074`, part A) — green.
- CI run `35171670335` (`151e98e`, parts B+C) — green.
- `windows-check.yml` run `35171995087` (dispatched for `151e98e`) — green, both assertions passed, real quoted output above.
