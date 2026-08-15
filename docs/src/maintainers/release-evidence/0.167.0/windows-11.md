# Platform Evidence — `windows-11`

**Artifact filename:** `forskscope-v0.167.0-windows-x64.zip`
**SHA-256:** `bd7c1d9107754f7866639de7d09668fcd0c70ca5669f5cbee15ccdfeca293c1d`
**Source commit:** `cb6f5b6`
**Test date (UTC):** 2026-08-15
**Tester role:** implementer (M5-A), automated via CI; F45's prerequisite
sub-case remains owner-executed and outstanding (see below)
**Host OS and version:** Windows Server-based CI image — **resolved
runner image** `win25-vs2026`, image version `20260810.198.2`, kernel
`Microsoft Windows NT 10.0.26100.0` (recorded per review 057 §4.3's
rolling-label caveat: `windows-latest` itself is not reproducible, this
resolved image is)
**Architecture:** x86_64
**Display server / WebView runtime:** a real interactive Windows desktop
session (not a virtual framebuffer — GitHub's Windows runners provide
one directly, unlike the Linux row's Xvfb stand-in) + WebView2, per
`matrix-plan.md`'s stated CI method for this row
**Install source and prerequisites:** published GitHub Release asset,
downloaded and digest-verified by CI before every case run; no
undocumented runtime prerequisite was needed (contrast Linux's F59) —
`windows-latest` ships the VC++ redistributable and WebView2 already, so
this is not evidence that a bare Windows install needs nothing (see F45
below)

## Cases

| Case | Result | Evidence |
|---|---|---|
| P01 — Install and cold launch | **Pass** (CI) / **Not executed** (F45's prerequisite sub-case — manual only, see below) | CI run [`31853150147`](https://github.com/forskscope/forskscope/actions/runs/31853150147) |
| P02 — CLI file compare | **Pass** | CI run [`31853258477`](https://github.com/forskscope/forskscope/actions/runs/31853258477) |
| P04 — Merge, undo/redo, safe save | **Pass** (mouse path, CI) / **Not executed** (keyboard path — see below) | CI run [`31878228535`](https://github.com/forskscope/forskscope/actions/runs/31878228535) |
| P05 — External modification | **Pass** | CI run [`31878276750`](https://github.com/forskscope/forskscope/actions/runs/31878276750) |
| P06 — Async identity | **Pass** | CI run [`31879158929`](https://github.com/forskscope/forskscope/actions/runs/31879158929) |
| P08 — Persistence migration and recovery | **Pass** | CI run [`31878490123`](https://github.com/forskscope/forskscope/actions/runs/31878490123) |
| P09 — Mergetool | **Pass** | CI run [`31853496997`](https://github.com/forskscope/forskscope/actions/runs/31853496997) |
| P10 — Binary/XLSX fail-closed policy | **Pass** | CI run [`31853362804`](https://github.com/forskscope/forskscope/actions/runs/31853362804) |
| P12 — Session/settings restart | **Fail — real path not exercised (F61)**; theme/language/font restore itself passes | CI run [`31880416324`](https://github.com/forskscope/forskscope/actions/runs/31880416324) |

Harness: `packaging/evidence/windows_harness.py`, driven by
`.github/workflows/m5-evidence-windows.yml` (`workflow_dispatch`, one
case per run). Every case downloads and digest-verifies the **published**
artifact itself — nothing here was built from source.

This row and `windows-10.md` share the same CI host: `windows-latest`
resolves to one image family, and there is no separate Windows 11 vs.
Windows 10 CI runner available. Per `matrix-plan.md`'s own text for the
`windows-10` row ("a Server-based image ... stands in reasonably"), both
rows record the *same* CI run evidence below rather than duplicating it —
this is not a shortcut invented here, it is what the frozen plan already
specifies for CI method on both rows.

### P01

`--diagnostics` reported `ForskScope 0.167.0`, with `OS: windows` and a
redacted home path (`Home: C:\Users\***`); a plain cold launch produced
a real, non-blank 1044×788 window. No undocumented prerequisite was
needed to reach this — unlike Linux's F59, `windows-latest` already
carries what a real Windows 11 machine with WebView2 and the VC++
redistributable installed would have.

### F45 — the "prerequisites missing" sub-case is not executed here

Per the M5-A handoff §4 and `matrix-plan.md`: `windows-latest` ships the
VC++ redistributable and WebView2 already, so CI cannot exercise "these
are missing on a clean machine" — there is no way to uninstall them on a
hosted runner to observe the failure mode a real first-time user without
either dependency would see. This sub-case is recorded as **owner-executed
and outstanding**, not attempted here, not waived, and not silently
folded into the CI "Pass" above. The CI pass above establishes only that
launch succeeds *given* those prerequisites — a real, but narrower, claim
than "Windows 11 P01 is fully covered."

## M5-B — the interaction cases

P04, P05, P06, P08, P12: the second evidence-gathering slice (RFC-078),
covering in-app interaction rather than launch-only behavior. Same
harness, same digest-verified published artifact, same
`workflow_dispatch`/one-case-per-run shape as M5-A above. Every case's
`--break` demonstration is in the Falsifiability section below.

### P04 — Merge, undo/redo, safe save

Two-argument compare mode (`forskscope.exe <left> <right>`, not
mergetool): applied the fixture's first (Replace) hunk via "Use this
change" (UIA Invoke), watched the toolbar's Save button track dirty
state (disabled → enabled on apply, back to disabled on Undo, enabled
again on Redo — with "Use this change" correspondingly reappearing after
Undo), saved, and verified the saved file's exact resulting lines
(`alpha`, `old-line`, `gamma`, `epsilon`, `zeta`, `insert-line` — the
Replace hunk's left content in place of the right's), a `.bak` sibling
equal to the pre-save original right-fixture content, and no leftover
temp/sidecar file in the working directory.

**Keyboard path not executed.** RFC-078 P04 asks for "keyboard and
mouse". Per the M5-B handoff §3, this was settled before running the
case: `app.rs`'s Enter-key shortcut (`Key::Enter => apply_focused_hunk`)
is a raw `onkeydown` listener with no bound UI element — there is
nothing for UIA, or any accessibility API on any platform, to invoke for
it, unlike a button's Invoke pattern (which the mouse path above uses,
the same equivalent-path argument M5-A already used for Save). This is
**not** "the case's intent is served by the accessibility path alone" —
the handoff's §3 explicitly frames this as the narrower reading:
accessibility invocation demonstrates the handler mouse and keyboard
both ultimately call, while the *input paths themselves* go unverified
on CI. The keyboard portion is recorded here as **manual-outstanding**,
the same shape as F45's Windows sub-case above — not attempted, not
waived, and not silently folded into the "Pass" above, which establishes
the mouse path only.

### P05 — External modification

Three fresh launches, each applying the first hunk (so the ordinary
toolbar Save button — gated on dirty state — is actually clickable),
then modifying the right file's bytes externally before attempting a
write:

1. **Cancel**: Save is blocked by the "File changed on disk" modal; the
   file holds the externally-written bytes both before and after
   clicking Cancel.
2. **Overwrite**: the write succeeds: the resulting `.bak` sibling's
   bytes equal the *externally-modified* content that was just
   overwritten (`save_text` backs up the target's current on-disk bytes
   immediately before replacing them — confirmed by reading
   `crates/forskscope-core/src/save.rs`, not assumed), not the original
   pre-modification content.
3. **Save As**, to a different path: the original target is left with
   its externally-modified bytes untouched; the new path holds the
   app's content.

### P06 — Async identity

Two large (8,000-line, scattered-difference) synthetic file pairs opened
as two independent `forskscope.exe` processes; the first is terminated
mid-load, and the second is confirmed to display **its own** content —
its own distinguishing token present, the other process's token absent,
not merely "a window with some content exists." A further single-process
launch fires two reloads in succession (rewriting the compared files
between each), confirming the **second** reload's content is what's
ultimately displayed, not the first's stale result.

**Two processes, not two tabs of one instance** (deviation, reported):
opening a second comparison in-app while the first is still loading
would go through Explorer's file-tree row selection, which depends on
the exact UIA control type WebView2 maps a bare `role="row"` div to —
the same open question flagged as unresolved below (see "Known
dependency of M5-C") and explicitly deferred to settle before M5-C's
P03, not discover here. The M5-B handoff explicitly allows either
approach ("whichever is achievable, document which you did").

**The double-reload is not a literal overlapping race** (deviation,
reported): `diff.rs` renders no Toolbar at all while a tab's state is
`Loading` (a bare loading-spinner view instead — confirmed by reading
the component), and there is no keyboard shortcut for reload, so firing
a second reload while the first is genuinely still in flight is
structurally unreachable through this app's UI, on any platform — the
same shape of finding as P04's keyboard path. What was exercised
instead: firing the second reload the moment the UI allows another
interaction (as soon as the first reload's Ready state, and its
Toolbar, reappears), with a still-real, still-meaningful assertion about
which content ends up displayed. RFC-078's own framing already gives
deterministic tests the primary-proof role here ("this case confirms
runtime integration").

### P08 — Persistence migration and recovery

The highest-value case in this slice (handoff §4): F37's amendment
requires all three recovery-dialog choices exercised with the
**process's actual state** asserted, not just the dialog's
disappearance. Four sub-checks, run in sequence against this runner's
real `%APPDATA%\forskscope` directory (see below), each clearing and
reseeding that directory first so none sees another's state:

1. **Legacy migration.** The exact `settings-v0.json`/`session-v0.json`
   fixtures the project's own Rust tests use
   (`crates/forskscope-core/src/tests/fixtures/persistence/`) migrate to
   versioned v2 envelopes: every settings field checked against the v0
   fixture's actual values (theme, language, font size/family, context
   lines, active profile, ignore patterns, explorer/binary-comparison
   flags, all 5 profiles) matches exactly, and `.pre-v2.bak` backups for
   both files are byte-identical to the originals.
2. **Exit** (future-schema session fixture → "Exit"): **the OS process
   is confirmed gone**, not just the dialog — `psutil.pid_exists(pid)`
   polled until `False`, the specific mechanism the handoff calls out as
   the one that actually distinguishes a clean exit from a zombie a
   dialog-only check would miss.
3. **Continue with defaults** (future-schema session fixture): the
   dialog dismisses, the process keeps running, and `session.json`'s
   bytes are provably unchanged on disk (write-disabled).
4. **Reset and back up** (corrupt session fixture): the dialog
   dismisses, `session.json` is reset to a fresh versioned v2 envelope,
   and a `.reset.bak` sibling holds the original corrupt bytes exactly
   (`ensure_reset_backup`/`<name>.reset.bak`, confirmed by reading
   `crates/forskscope-core/src/persist/schema/repository.rs`, not
   assumed).

**Config-directory isolation, reasoned rather than assumed** (deviation,
reported): the handoff suggested overriding `%APPDATA%` for the launched
subprocess to redirect it to a scratch directory. `dirs_next::config_dir()`
resolves via the Win32 `SHGetKnownFolderPath` API on Windows, which reads
the registry/user profile, not the `APPDATA` process environment
variable — so that override would not actually redirect anything (unlike
POSIX platforms, where `dirs`/`dirs_next` does honor the environment).
Since a `windows-latest` runner's whole VM is thrown away after the run,
there is no real user's config to protect; this harness instead resolves
the runner's real (but disposable) `%APPDATA%\forskscope` directory
directly and clears it between each of the four sub-checks above,
achieving the same practical isolation the handoff's phrasing intended.

**A real, discovered behavior narrowed sub-check 1's tabs assertion**:
`session-v0.json`'s two tabs reference `/tmp/fixtures/left-a.txt` etc.,
paths that don't exist on this runner. A no-args launch runs
`restore_tabs`, which correctly drops both (neither path exists), and
`app.rs`'s `use_effect` persists that now-empty tab list immediately —
overwriting the migration commit's originally-correct tabs before this
harness ever reads them. This is not migration data loss (the
`.pre-v2.bak` byte-exact backup above proves the original content was
preserved); it is a real, subsequent launch cycle's auto-save correctly
reflecting that the referenced files are gone. The check now asserts the
live tabs are pruned to `[]` for this documented reason, not silently
wrong content.

### P12 — Session/settings restart

Changed theme (Dark → Light), language (English → Japanese), and diff
font size (14 → 18) via the Settings dialog, exited, relaunched with no
CLI arguments against the same real config directory, and confirmed all
three restored — plus a Japanese label (the header's Settings button,
reading "設定") rendering after restart as a practical-workflow check.
Tab restore was confirmed to happen only on a no-args relaunch (a
directly-seeded tab, see below, reappeared automatically), and *not*
when a relaunch supplies explicit CLI paths instead (the specified
compare opened; the previously-seeded tab's content did not appear
alongside or instead of it).

**Settings controls have no accessible name** (reported limitation, not
a defect): `settings/modal.rs`'s Theme/Language `<select>`s sit next to
a plain sibling `<span>`, with no `<label for>`/`aria-labelledby` — UIA
exposes no name beyond the generic ComboBox role, so this harness locates
them ordinally (first ComboBox = Theme, second = Language) rather than
by name. Reaching a genuine value change also needed more than a single
accessibility-pattern call: `select_dropdown()`/`set_value_text()` in
`windows_harness.py` each try progressively more input-synthesis-heavy
approaches (Invoke-pattern selection → click-then-real-keystrokes),
verifying an actual readback after each rather than trusting a clean
return — see their docstrings for the full sequence of what didn't work
and why, including real UI-automation flakiness where identical code
passed on one CI run and produced no change at all on the very next
(both `select_dropdown`/`set_value_text` retry their full attempt chain
up to three times for this reason).

**Tab-restore seeded by constructing `session.json` directly, not via a
CLI launch** (deviation, reported — and a genuine finding in its own
right, not a workaround for a harness bug): the natural approach — open
a compare with `forskscope.exe <left> <right>`, let `save_session`'s
reactive effect persist it, then relaunch with no arguments — was tried
first. **It does not work**: with the config directory cleared
immediately before the seed launch and the check polling `session.json`
for the full launch timeout *while the process stayed alive* (ruling out
a race against this harness's own process termination), the file was
never created at all, not once. This contradicts `app.rs`'s own doc
comment (review 041 C1), which explicitly says a CLI launch's
`open_compare` should trigger `save_session`. Not fixed here, per the
handoff's "no product behaviour changes" constraint — registered as a
real finding instead (see "Failures and issue links" below). P12's tab-
restore check no longer depends on this path: it writes the v2
`session.json` envelope directly (the same schema
`crates/forskscope-core/src/persist/schema/session.rs` and this
harness's own P08 fixtures already use), since `restore_tabs` doesn't
care how the file came to describe a tab — only that it does, and that
no explicit CLI paths were given at startup. This still exercises
exactly the restore mechanism P12 cares about, and separately confirms
the no-args-vs-explicit-args distinction the case is actually about.

**Recorded as Fail at the case level (review 064 §5.4), not Pass.** The
harness's own (worked-around) assertions all pass, but the case's real
path — a CLI-launched tab surviving to the next launch — is exactly what
F61 breaks, and it was never exercised here. A table reading "Pass" would
tell a reader this row is unaffected by F61, which is untrue; Linux and
macOS's P12 rows correctly read Fail for the same underlying defect,
reached without a workaround. The passing part (theme/language/font
restore) is real and independent of F61.

## Falsifiability

Every case's `--break` mode was run and confirmed to fail for the
expected reason, per handoff §7:

| Case | Break-mode run | Result |
|---|---|---|
| P01 | [`31853199727`](https://github.com/forskscope/forskscope/actions/runs/31853199727) | Fail — `--diagnostics output does not start with 'ForskScope 0.0.0-impossible': ForskScope 0.167.0` |
| P02 | [`31853350190`](https://github.com/forskscope/forskscope/actions/runs/31853350190) | Fail — `compare view never rendered these expected tokens within 60s: ['this-token-cannot-appear-in-real-output']` |
| P09 | [`31853592413`](https://github.com/forskscope/forskscope/actions/runs/31853592413) | Fail — content check correctly rejected a value real Save output can never produce |
| P10 | [`31853491433`](https://github.com/forskscope/forskscope/actions/runs/31853491433) | Fail — `could not find text containing 'this message cannot appear' within 60s` |
| P04 | [`31878264147`](https://github.com/forskscope/forskscope/actions/runs/31878264147) | Fail — saved content `['alpha', 'old-line', 'gamma', 'epsilon', 'zeta', 'insert-line']` != required (impossible) `['this exact content can never appear in real merge output']` |
| P05 | [`31878529324`](https://github.com/forskscope/forskscope/actions/runs/31878529324) | Fail — `.bak`'s content `'EXTERNALLY-MODIFIED-CONTENT\nsecond line\n'` != required (impossible) `'IMPOSSIBLE - this harness never externally writes this content'` |
| P06 | [`31878786557`](https://github.com/forskscope/forskscope/actions/runs/31878786557) | Fail — `process B never rendered 'TOKEN-AAAA': missing ['TOKEN-AAAA']` (process B correctly never shows process A's token) |
| P08 | [`31878790194`](https://github.com/forskscope/forskscope/actions/runs/31878790194) | Fail — "the process exited as expected, but --break requires it to still be running" — §6's specifically-flagged Exit risk, demonstrated: requiring the impossible (still running) correctly fails |
| P12 | [`31880463459`](https://github.com/forskscope/forskscope/actions/runs/31880463459) | Fail — `header button never showed 'this-label-can-never-appear' after restart` |

M5-A's four cases (P01/P02/P09/P10 above) each passed in normal mode and
failed correctly in break mode on the first dispatched run. **M5-B's
five did not** — P04 and P05 matched that pattern, but P06, P08, and
especially P12 needed real iteration against actual CI output (the same
"budget for real iteration" the M5-B handoff called for), documented in
full in `packaging/evidence/windows_harness.py`'s git history and
summarized in "Failures and issue links" below. This is expected, not a
regression in rigor: M5-A's cases were launch-only, M5-B's are real
in-app interaction, and — per the M5-B handoff's own framing — this is
"the harder slice."

**M5-A's four cases, specifically:** unlike the Linux harness's
development history (several iterations were needed to get P09's
button-invocation working reliably under a bare Xvfb display with no
window manager), every M5-A case here passed in normal mode and failed
correctly in break mode on the **first** dispatched run of each. Every
break-mode run above is a genuine, observed failure with the expected
message, not an assumption — this is not treated as evidence the checks
are weaker, only that a real interactive Windows desktop session (unlike
Linux's bare Xvfb) gave UIA's Invoke pattern a straightforward path with
no window-manager/focus complications to work around. One deliberate
difference from the Linux harness's readiness
condition: this harness waits for the fixture's distinguishing text
tokens to appear in the UIA tree, not an exact per-pane row count —
`windows_harness.py`'s module docstring has the full reasoning, including
that the exact UIA control type WebView2 maps `role="row"` to was never
empirically confirmed, since the content-based check already worked.

**Known dependency of M5-C, not a limitation of this slice (review 063
§5.3):** P02's content-presence check is sufficient for what P02 itself
needs — early satisfaction of "these tokens exist somewhere" costs
nothing when there's no wrong-answer risk from a partially-rendered tree.
It is not sufficient for **P03 (compare layout and scrolling)**, which
compares row alignment across the whole tree the way F34's Linux check
does — a partial row set there would compare a subset and risk a false
pass, the exact failure shape F34 exists to prevent. Establishing the
UIA control type WebView2 maps `role="row"` to (so an exact row count
becomes possible on Windows, matching Linux's AT-SPI approach) is
something to settle **before** P03 in M5-C, not discover during it.

## Failures and issue links

- **F45** — Windows P01's prerequisite sub-case ("VC++ redistributable /
  WebView2 missing") cannot be exercised on `windows-latest` because both
  are already present on the image. Recorded as owner-executed and
  outstanding, per the handoff.
- **P04's keyboard path** — not executed here, manual-outstanding, same
  shape as F45 above. See P04's section.
- **Candidate product defect, not fixed (reported, not registered as a
  ROADMAP finding here — that call belongs to the reviewer): a 2-arg
  CLI-launched compare's newly-opened tab is never persisted to
  `session.json`.** Found while building P12's tab-restore check: with
  the config directory cleared immediately beforehand and the check
  polling for the entire launch timeout *while the process stayed
  alive* (ruling out a race against this harness's own process
  termination), `session.json` was never created at all after launching
  `forskscope.exe <left> <right>` and waiting for the diff to fully
  render — not once, across repeated attempts. This directly contradicts
  `app.rs`'s own doc comment (review 041 C1): *"a CLI-mode launch
  (`forskscope left right`) never restores tabs, but still needs
  `session_write_disabled` set before its own `open_compare` triggers a
  `save_session`"* — the comment's own premise is that a CLI launch's
  `open_compare` **does** trigger `save_session`; that was not observed.
  Practical impact, if this is confirmed as a real defect rather than a
  test artifact: a user who opens `forskscope.exe left.txt right.txt`
  (the `git difftool`-compatible entry point) and then closes the app
  should — per every other tab-opening path's behavior — have that
  comparison offered again on the next argument-less launch; this
  evidence says that does not happen. P12's own tab-restore check was
  rewritten to construct `session.json` directly instead of depending on
  this path (see P12's section above), so this finding did not block
  evidence collection — but it was not independently re-verified beyond
  the CI runs already described, and a false-negative harness cause
  (however unlikely, having ruled out the two most obvious ones) cannot
  be stated as fully excluded without a lower-level reproduction outside
  this harness.
- Two further real, discovered (not invented) app *behaviors* shaped
  what a couple of checks could observe — neither a defect, both
  documented in their case's section above and not fixed (per the
  handoff's "no product behaviour changes" constraint, correctly — there
  was nothing to fix): `TabState::Loading` hides the Toolbar entirely
  (P06 — makes a literal overlapping double-reload structurally
  unreachable through this app's UI, on any platform), and
  `restore_tabs` + the reactive session auto-save prune tabs whose files
  no longer exist, immediately and durably (P08 — a migrated session's
  tabs referencing nonexistent paths cannot be observed surviving a real
  launch cycle, though the migration itself loses nothing, per the
  `.pre-v2.bak` byte-exact backup). Neither is proposed here as a new
  tracked ROADMAP finding — that judgment call belongs to the reviewer
  deciding whether either is worth tracking as its own item.
- Every other iteration documented in `windows_harness.py`'s git history
  (the M5-B fix commits) was a **harness** defect — a wrong assumption
  about how UIA exposes a given control, a race against the app's own
  async loading, a wrong expected value, real UI-automation flakiness —
  never a wrong result from the application itself.

## Waivers

None.
