# Review Request: F61/F62 — Session Persistence and Its Silence

**Date:** 2026-08-16
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/f61-f62-session-persistence-handoff.md`
**Register items:** F61, F62
**Baseline:** `main` at `92fb9e3`
**Scope:** product work — F61's actual fix, not evidence-gathering. No M5 evidence files touched.

## 1. F61's established mechanism — before any fix

**Not a UI-wiring bug.** `app.rs`'s reactive `use_effect` on `store.tabs` was firing
correctly the whole time, for a CLI-opened tab exactly as for any other tab-list
change. The write beneath it was failing.

**Root cause: nothing in the app ever creates `~/.config/forskscope/` (or the
platform equivalent) before the first write into it.** `atomic_write_envelope`
(`crates/forskscope-core/src/persist/schema/repository.rs`) calls
`crate::save::atomic_replace`, which writes a temp file in the *same directory*
as its target, then renames it. If that directory doesn't exist yet — true for
any genuinely fresh config path, since nothing else in the shipped binary calls
`create_dir_all` on it — the temp-file write fails immediately with `NotFound`.
Every prior report of a `~/.config/forskscope/` directory that already existed
(the overwhelmingly common case: any machine that has ever launched ForskScope
once before) never saw this, which is exactly why it took three independent
platform harnesses plus a fourth manual reproduction to surface.

**How this was established, per the handoff's own prescribed order — F62 first,
as the diagnostic:**

1. Fixed F62 (below) so a save failure produces a real error toast instead of
   silence.
2. Wrote a regression test that renders the real `App()` component with a
   genuine CLI `Compare` startup request against an isolated, empty
   `XDG_CONFIG_HOME` (full detail in §4), and captured the `Store`'s toast
   after letting the effect run.
3. First result: **no toast at all**, and no file. This ruled out "the write
   is attempted and fails" and pointed at "the effect never runs" — until a
   further finding (§9) showed the harness itself needed more scheduling
   rounds to reach the effect; once it did:
4. **Toast: `Could not save session: write failed for
   '.../config/forskscope/.session.json.fsk-tmp': No such file or directory
   (os error 2)`.** F62's fix had done exactly what the handoff predicted —
   turned a guessing exercise into an observation.
5. Confirmed by toggling the single variable directly: pre-creating
   `config_home/forskscope/` before the same test run makes the save succeed
   silently (`toast: None`) and the file appear with the correct tab recorded.
   Removing the pre-creation reproduces the failure again. This is the
   complete, isolated proof of both the cause and the fix.

## 2. F62's chosen handling, and what a user sees on a persistence failure

`persist_session` (`state/session.rs`) and `persist_settings`
(`ui/view/settings.rs`) now return `Result<(), PersistenceIoError>` instead of
discarding it with `let _ =`. `PersistenceIoError` is re-exported from
`forskscope_core::persist::schema` (it wasn't before — `repository` is a
private module; added the re-export alongside the existing
`PersistenceCommitError` one, `persist/schema.rs`).

**Chosen handling: a `Notice::error` toast, uniformly, regardless of call
site.** `save_session`/`persist` (the `Store`-aware wrappers both functions
sit behind) call `store.notify(format!("Could not save {session,settings}:
{e}"))` on failure. This is deliberately **not** a different treatment for a
startup-triggered save versus a user-initiated one (`close_tab`, a Settings
change): either way the user's session or preference may not survive the next
launch, and the honest thing to show is the same clear error either way.
Considered and rejected: routing through RFC-076's recovery/dialog path
(reserved for *load*-time future/corrupt/unwritable states, a different
problem — a *save* failure here doesn't change what's safe to read next
launch) and a diagnostic-only log (invisible to the user is exactly what F62
exists to stop being true).

**RFC-076's write-disable guarantee is untouched** (handoff §6): the toast
path only wraps an *attempted* write's outcome; `save_session_if_allowed`'s
`write_disabled` early return (`Ok(())`, no attempt at all) is unchanged and
still covered by its own test
(`future_version_session_stays_byte_identical_through_a_disabled_save`,
updated only to assert the `Ok(())` return, not the disabled-write behavior
itself).

`Result` is itself `#[must_use]` in `std`, so no `#[must_use]` attribute was
added on top (clippy's `double_must_use` correctly flagged that redundancy
when tried) — a future reintroduction of `let _ = persist_session(...)` is a
compiler warning under this workspace's `-D warnings` gate, not silence.

## 3. The fix, at the level the cause sits at

`atomic_write_envelope` (`persist/schema/repository.rs`) now calls
`fs::create_dir_all` on the target's parent directory before calling
`atomic_replace`:

```rust
pub(super) fn atomic_write_envelope(path: &Path, json: &str) -> Result<(), PersistenceIoError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| PersistenceIoError(e.to_string()))?;
    }
    crate::save::atomic_replace(path, json.as_bytes())
        .map_err(|e| PersistenceIoError(e.to_string()))
}
```

This is the single call site both `SessionRepository::save` and
`SettingsRepository::save` go through — the fix covers both without
duplication. `ensure_pre_v2_backup`/`ensure_reset_backup` (the `.pre-v2.bak`/
`.reset.bak` writers) do **not** need the same fix: both only ever run against
a path that was just successfully read via `fs::read`/`fs::read_to_string` to
produce the value being migrated or reset, so that path's parent directory is
already known to exist by construction — no fresh-directory case reaches
them.

Not fixed in `crate::save::atomic_replace` itself (the lower-level, more
general primitive `save_text` also uses for document saves): a user's chosen
save target directory not existing is a different, more surprising situation
than a config file's own managed directory not existing yet, and
auto-creating directories there wasn't asked for and isn't this handoff's
scope (§4: "Any other persistence redesign... is not in scope").

## 4. The regression test through the real startup path

`crates/forskscope-ui/src/app/tests.rs` (new file),
`app::tests::a_cli_opened_tab_is_persisted_to_session_json`. `with_test_store`
(F36) genuinely cannot reach this: it renders a synthetic `root()` component
that only constructs and captures a `Store`, never `App()` itself, so
`app.rs`'s reactive effect never runs under it at all — confirmed, not
assumed, and noted in the test's own module docs per the handoff's explicit
instruction to say so rather than substitute something weaker.

Renders the real `App()` through a real `VirtualDom`: sets `STARTUP_REQUEST`
to a genuine `StartupRequest::Compare { left, right }` (the exact path
`forskscope <left> <right>` takes), points `XDG_CONFIG_HOME` at an isolated,
empty scratch directory (nothing pre-created beyond the directory itself —
the exact fresh-install condition F61 needs), drives Dioxus's own
`wait_for_work`/`render_immediate` scheduling loop until `session.json`
appears or a 10-second deadline passes, then asserts the file exists, parses
as a current-schema session, and names the right tab — and separately asserts
no error toast was left on the `Store` (a second, independent signal the
write actually succeeded, not merely that a stale file happened to exist).

**`#[ignore]`d, with the reason stated in code — a real finding about test
infrastructure, not a weaker test (handoff §3's explicit permission).**
Empirically narrowed while diagnosing F61 (`cargo test -p forskscope-ui --lib
-- --test-threads=N`, repeated at each `N`): passes reliably, 5/5, at
`N<=2`; fails reliably, 5/5, with a genuine 10-second timeout (the effect
never runs at all, not a slow pass) at every `N>=3` tried — 3, 4, 8, 16, and
this machine's ~32-thread default — regardless of which or how many *other*
tests run alongside it, including alone under `--test-threads=2` with zero
other tests selected. Added `state::DIOXUS_VDOM_TEST_LOCK`, shared with
`with_test_store`, specifically to rule out cross-test `VirtualDom`
interference as the cause; it did not fix the `N>=3` failure, so whatever
remains is something about `dioxus-core`/`tokio` under heavier concurrent
test-thread load that this investigation did not run down further. A
distinct, real bug *was* found and fixed in the same investigation along the
way: an earlier revision read the captured `Store`'s toast signal *after*
dropping the `VirtualDom` that owned it, which panics with
`Dropped(ValueDroppedError)` — that is a plain ordering bug in the test, now
fixed, and is not what the `--test-threads`-dependence is about.

Run on demand: `cargo test -p forskscope-ui --lib -- --ignored
--test-threads=2 a_cli_opened_tab_is_persisted_to_session_json`. Confirmed
passing, 3/3, at exactly that invocation as the last step before writing this
request.

## 5. Runtime confirmation that a CLI-opened tab now persists

The regression test above **is** this confirmation — the same observation
review 064 made manually (`forskscope <left> <right>`, isolated
`XDG_CONFIG_HOME`, session.json absent; close the tab, session.json appears),
now automated against the real startup path and passing.

A full GUI-launch manual re-verification (build the real binary, launch it
under a real display, terminate, inspect the file) was attempted and not
possible in this environment: launching a windowed GUI process requires a
permission this sandboxed session does not have. The Rust-level regression
test — which exercises the real `App()` component, the real repository code,
and the real startup-request path, just without an actually rendered window —
is offered as the evidence in its place; flagged as a limitation in §10.

## 6. §5's claim check — `README.md:96`

> **Session persistence** — open tabs are restored on next launch

**True on both paths now, not narrowed.** The fix sits at the repository
layer beneath *every* save call, not in a code path specific to CLI startup —
so the Explorer-driven path (already true; F16's audit covered it) and the
CLI-launch path (false before this fix, per F61) are both correct through the
same mechanism now. No wording change needed. Not independently re-verified
via a full GUI launch, for the same sandboxing reason as §5 — the regression
test is the evidence that the CLI path specifically is now correct.

## 7. Changed files

- `crates/forskscope-core/src/persist/schema.rs` — re-exports
  `PersistenceIoError` (needed so `forskscope-ui` can name it).
- `crates/forskscope-core/src/persist/schema/repository.rs` — F61's fix:
  `atomic_write_envelope` creates its target's parent directory before
  writing.
- `crates/forskscope-ui/src/state/session.rs` — F62: `persist_session`,
  `save_session_if_allowed` return `Result`; `save_session` shows an error
  toast on failure.
- `crates/forskscope-ui/src/state/session/tests.rs` — updated call sites for
  the new `Result` returns; the write-disabled test now asserts `Ok(())`.
- `crates/forskscope-ui/src/ui/view/settings.rs` — F62's settings mirror:
  `persist_settings` returns `Result`; `persist` shows an error toast on
  failure.
- `crates/forskscope-ui/src/ui/view/settings/tests.rs` — updated call sites.
- `crates/forskscope-ui/src/state.rs` — `DIOXUS_VDOM_TEST_LOCK`, shared by
  `with_test_store` and the new regression test, to serialize concurrent
  `VirtualDom` construction across the test suite.
- `crates/forskscope-ui/src/app.rs` — `#[cfg(test)] mod tests;` declaration,
  and a `#[cfg(test)]`-gated capture of `App()`'s own `Store` into a
  thread-local (mirrors `with_test_store`'s technique, applied to the real
  component) so the regression test can inspect it after rendering.
- `crates/forskscope-ui/src/app/tests.rs` (new) — the F61 regression test.
- `ROADMAP.md` — F61 and F62 marked resolved with the mechanism and fix
  recorded.

No M5 evidence file was touched, per the handoff's §6 constraint.

## 8. Differences from this handoff

None in substance. The one deviation from the letter of §7's structure: this
request answers §7's items 1–6 in the order the handoff itself presents them
(mechanism, F62, fix, test, runtime confirmation, claim check) rather than
re-deriving a different order — noted only because §7 lists them and this
follows that list directly.

## 9. Executed gates with observed output

- `cargo fmt --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean (also
  caught and required removing a redundant `#[must_use]` on functions
  already returning `Result`, `double_must_use` — fixed, not suppressed).
- `cargo test --workspace` — **1095 passed, 0 failed**, plus the one new
  `#[ignore]`d regression test (2 occurrences — `forskscope-ui`'s lib and bin
  targets both compile `app.rs`), run reliably 5/5 with default parallel
  threading.
- `cargo test -p forskscope-ui --lib -- --ignored --test-threads=2
  a_cli_opened_tab_is_persisted_to_session_json` — the ignored test itself,
  3/3 passing.
- `cargo xtask i18n` — clean (227 UI keys covered; no new UI-facing strings
  beyond the two toast messages, which are plain interpolated `String`s, not
  translation keys, matching this codebase's existing error-message
  convention).
- `mdbook build docs` — clean.
- `git diff --check` — clean.
- No dependency added, removed, or version-changed — `dioxus-desktop`
  included, `tokio`'s existing `["rt", "rt-multi-thread", "sync", "time"]`
  feature set was sufficient for the regression test (`Builder::
  new_current_thread()` only needs `"rt"`).

## 10. Unresolved issues and known limitations

- **The `--test-threads<=2` requirement (§4) is itself unresolved.** A real,
  reproducible test-infrastructure limitation, not explained further by this
  investigation. Worth its own look if this pattern (rendering the real
  `App()` in a test) becomes something future work wants to build on.
- **No full GUI-launch manual re-verification was possible in this sandbox**
  (§5/§6) — permission to launch a windowed process was not available.
  Recommend a real desktop smoke test (`forskscope <left> <right>`,
  terminate, relaunch with no args, confirm the tab restores) before or
  alongside cutting the new candidate this fix requires.
- **A new candidate is required** to re-run M5's P12 rows and clear F61 as a
  Gate D blocker (handoff §8) — not done here, out of this slice's scope.
- **F63 and F64** (macOS's accessibility size threshold; the three untracked-
  but-not-ignored generated paths) are explicitly out of scope per the
  handoff §4 and were not touched.

## 11. Requested review focus

1. **§2 and §3** — is a uniform error toast (no distinction between a
   startup-triggered save failure and a user-initiated one) the right call,
   or should a startup-time failure get different treatment (e.g. a
   dedicated startup notice, matching RFC-076's existing migration-notice
   pattern) since the user hasn't taken any action yet to associate the
   error with?
2. **§4's `#[ignore]`** — is documenting and ignoring this test-
   infrastructure limitation acceptable, or does the regression-test
   requirement need a different, more robust implementation before this
   slice is considered complete? (Options not attempted: a subprocess-based
   test that launches the actual binary rather than rendering `App()`
   in-process; investigating further into `dioxus-core`'s scheduler
   internals; reporting upstream.)
3. **§1's fix placement** — `atomic_write_envelope` versus a lower-level fix
   in `crate::save::atomic_replace` itself. Confirm the scoping reasoning
   (config-managed directories should auto-create; user-chosen document save
   targets should not) is the right line, not just a convenient one.
4. Whether §5/§6's sandboxed-environment limitation (no GUI-launch
   verification possible) is acceptable given the regression test's
   coverage, or whether this should block considering F61 resolved until a
   real desktop run confirms it.
