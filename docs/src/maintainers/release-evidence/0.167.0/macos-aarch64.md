# Platform Evidence — `macos-aarch64`

**Artifact filename:** `forskscope-v0.167.0-macos-aarch64.dmg`
**SHA-256:** `2d66f125f0325adfef36cdf9bbb643a8deed50112db77707f7e6c9970ca25099`
**Source commit:** `cb6f5b6`
**Test date (UTC):** 2026-08-15
**Tester role:** implementer (M5-A), automated via CI (`macos-latest`) — no
manual macOS host exists in the owner's stated execution model
(`matrix-plan.md` §1)
**Host OS and version:** macOS 26.5.2 — **resolved runner image**
`macos26`, image version `20260728.0273.1` (recorded per review 057
§4.3's rolling-label caveat: `macos-latest` itself is not reproducible,
this resolved image is; `Darwin ... RELEASE_ARM64_VMAPPLE arm64` confirms
the runner is aarch64, matching this row's architecture)
**Architecture:** aarch64
**Display server / WebView runtime:** real macOS window server (the
runner is a full GUI-session VM, not headless) + WKWebView
**Install source and prerequisites:** published GitHub Release asset,
downloaded and digest-verified by CI before every case run; `.dmg`
mounted with `hdiutil attach`, `ForskScope.app` copied out, its
`Contents/MacOS/forskscope` binary launched directly

## Cases

| Case | Result | Evidence |
|---|---|---|
| P01 — Install and cold launch | **Pass** | CI run [`31853094639`](https://github.com/forskscope/forskscope/actions/runs/31853094639) |
| P02 — CLI file compare | **Pass** | CI run [`31853203173`](https://github.com/forskscope/forskscope/actions/runs/31853203173) |
| P09 — Mergetool | **Pass** | CI run [`31853204009`](https://github.com/forskscope/forskscope/actions/runs/31853204009) |
| P10 — Binary/XLSX fail-closed policy | **Pass** | CI run [`31853205096`](https://github.com/forskscope/forskscope/actions/runs/31853205096) |
| P04 — Merge, undo/redo, safe save | **Pass** (mouse path only — see §3 resolution below) | CI run [`31879443866`](https://github.com/forskscope/forskscope/actions/runs/31879443866) |
| P05 — External modification | **Pass** | CI run [`31879848097`](https://github.com/forskscope/forskscope/actions/runs/31879848097) |
| P06 — Async identity | **Pass** (sequential two-launch variant — see case notes) | CI run [`31883446824`](https://github.com/forskscope/forskscope/actions/runs/31883446824) |
| P08 — Persistence: Exit | **Pass** | CI run [`31879559456`](https://github.com/forskscope/forskscope/actions/runs/31879559456) |
| P08 — Persistence: Continue with defaults | **Pass** | CI run [`31879563832`](https://github.com/forskscope/forskscope/actions/runs/31879563832) |
| P08 — Persistence: Reset and back up | **Pass** | CI run [`31879567945`](https://github.com/forskscope/forskscope/actions/runs/31879567945) |
| P08 — Persistence: legacy migration (filesystem) | **Pass** | CI run [`31879767594`](https://github.com/forskscope/forskscope/actions/runs/31879767594) |
| P12 — Session/settings restart | **Fail** (real product defect found — session not persisted; see below) | CI run [`31881394791`](https://github.com/forskscope/forskscope/actions/runs/31881394791) |

Harness: `packaging/evidence/macos_harness.py` + `macos_ui.applescript`,
driven by `.github/workflows/m5-evidence-macos.yml` (`workflow_dispatch`,
one case per run). Every case downloads and digest-verifies the
**published** artifact itself — nothing here was built from source.

### P01

`--diagnostics` exited 0 and reported `ForskScope 0.167.0`; the harness's
own assertions (not just a printed summary line) confirmed the `OS:` line
contains `macos` and, when a `Home:` line is present, that it is redacted
to `***` rather than a literal path — the harness's stdout only echoes a
one-line summary of these checks, not the full diagnostic report text
verbatim, so this record states what was *asserted and passed*, not a
literal captured string. A plain cold launch (no args) produced a real,
non-blank 1024×677 window, confirmed via macOS's Accessibility API
(System Events).

### P02

`forskscope <left> <right>` rendered the F34 fixture pair
(`left_all_hunk_kinds.txt`/`right_all_hunk_kinds.txt`) as 14 `AXRow`
accessible elements — `hunk.rs`'s `RowLeft`/`RowRight` each independently
carry `role="row"`, so the pinned 7-rows-per-pane fixture produces 14
total once both panes render. This asserts rendered content, not merely a
zero exit code.

### P09

Applied the fixture's first hunk via the "Use this change" button
(clicked through System Events' `AXPress`, addressed by its accessible
description, not synthetic keyboard/mouse input), then clicked the
toolbar's "Save merge result" button the same way. `<merged>`, pre-seeded
with an unmistakable placeholder, was confirmed overwritten with real
content (46 bytes) afterward.

### P10

`forskscope <left.xlsx> <right.xlsx>` (arbitrary bytes; classification is
by extension only, `core::file_kind::classify`) produced the fail-closed
message **"Spreadsheet comparison is temporarily disabled for security."**,
found by walking the accessible tree for that exact text — the message
reaching the user was asserted directly, not merely "no crash occurred."

## M5-B — the interaction cases (P04, P05, P06, P08, P12)

The five cases below all need real in-app interaction, not just launch
and CLI-argument checks — the harder slice, per the M5-B handoff. Same
artifact, same digests, same harness family (`macos_harness.py` +
`macos_ui.applescript`, extended with new commands and case functions,
not restructured).

### §3 resolution — P04's "keyboard and mouse"

Per the M5-B handoff §3, this was settled *before* execution, not during:
**the mouse path is CI-verifiable and was executed; the keyboard path is
structurally not CI-verifiable on any platform and was not attempted.**

The hunk's "Use this change" button (`aria_label: "Use this change (apply
left to right)"` in `hunk.rs`) has a real `onclick` handler. Invoking it
via `AXPress` (System Events' `perform action "AXPress"`, or the
equivalent `click` verb once the accessible element is found) calls the
exact same handler a real mouse click would — the same equivalent-path
argument M5-A already used for Save. The keyboard shortcut
(`Key::Enter => apply_focused_hunk(...)` in `app.rs`'s global
`onkeydown`) is a raw keydown listener with no accessible action to
invoke — there is no accessibility mechanism to exercise it, on any
platform, not just this one. Recorded as **manual-outstanding**, not
silently skipped and not claimed covered — see P04's own report below.

### P04

`forskscope <left> <right>` (plain 2-arg compare, F34's fixture pair —
*not* mergetool mode this time, so Save writes to `<right>` per
`compare.rs`'s `SaveDestination::RightInput`). Applied the first hunk via
the "Use this change" button's `AXPress`; the dirty marker ("unsaved",
`statusbar.rs`) appeared. Clicked Undo (toolbar) — the marker
disappeared. Opened the advanced panel ("More ▼") and clicked Redo — the
marker reappeared. Clicked Save — the marker disappeared again, `<right>`
was confirmed overwritten with real content (differs from its pre-edit
bytes), a `.bak` sibling existed whose bytes equaled the file's *pre-save*
content (the original `right` fixture, matching `save.rs`'s
`SiblingBak` policy), and no leftover `.right.txt.fsk-tmp` sidecar
remained after the successful save. **Keyboard-Enter apply path: NOT
executed** — see §3 resolution above.

### P05

Three fresh launches, each opening F34's fixture pair and applying the
first hunk (mouse/`AXPress`, same technique as P04) to make the tab dirty
before Save's toolbar button becomes clickable:

1. **Cancel.** While the app had `<right>` open, it was overwritten
   *outside the app* with different bytes. Clicking Save triggered
   `OverwriteModal` ("File changed on disk") — the file still held the
   externally-written bytes (Save did not silently win the race).
   Clicking Cancel closed the dialog; the file was still untouched.
2. **Overwrite.** Same setup; this time clicking "Overwrite" let the
   write proceed. The `.bak` sibling's bytes were confirmed to equal the
   bytes that were *just overwritten* — the externally-modified content,
   not the original pre-edit fixture content. This is the detail the
   handoff called out as the one that makes the check meaningful, not
   just "a `.bak` exists".
3. **Save As.** Same setup; this time "Save As" to a different path
   (`type_into`'s click-to-focus-then-keystroke on the plain `<input
   type="text">` path field — `set_value`'s direct `AXValue` write was
   tried first and also worked in this run, since a plain text field
   behaves differently from the font-size spinner that confirmed
   `set_value` a no-op; see the M5-B technique notes below). The original
   target file was confirmed still holding the externally-modified bytes
   (untouched by Save As), and the new path held the app's own merge
   output.

### P06

RFC-078 describes this case as opening a second compare in-app while the
first is still loading, in one process. That is **not what this run
does** — see the case's own extensive docstring in `macos_harness.py` for
the full account of why, but summarized: three shapes were tried, in
order, against real CI:

1. **In-app** (RFC-078's literal description): switch to the Explorer tab
   while tab 0 loads, pick a second pair's files there, open it as tab 1,
   close tab 0 mid-load. Hung 45s+ in accessibility queries against
   Explorer whenever another tab's load was in progress.
2. **Two concurrent processes** (the handoff's own pre-approved
   fallback). Still unreliable even after ruling out process-identity
   ambiguity outright via unambiguous PID-based addressing.
3. **What actually ran**: two launches that never overlap. Process A
   launches, gets a short real load window, and is terminated before
   process B ever starts; process B is then interacted with alone, using
   the same single-process pattern every other passing case in this file
   already relies on.

The real root cause of why (1) and (2) failed was found only after
retreating to (3): a generated diff pair's content simply never reached
the accessibility tree once the file crossed a size threshold between 30
and 100 lines (binary-searched in isolation, with nothing else running:
10 and 30 lines render correctly; 100 and 400 do not, consistently) — not
a process-concurrency problem at all. (1) and (2) were most likely
failing for this same reason. Revisiting the in-app design with correctly
-sized fixtures is a reasonable follow-up not attempted in this slice.

**What this run actually verifies**: process A exited cleanly
(`terminate()`) after a short real load window; process B, launched only
afterward with a distinct 30-line synthetic pair, showed its own correct
sentinel content (not blank, not crashed) — the falsifiable content check
the handoff's §6 requires (a bare "the process is still running" check
would be vacuous). Reloading twice in quick succession (rewriting the
right-hand file with a different sentinel between clicks) resulted in the
*second* reload's content being displayed, not the first's — `reload_tab`'s
`LoadToken` machinery exercised for real. **This is materially weaker
than RFC-078's description**: it cannot exercise concurrent-process
coexistence, let alone in-process async-task-identity confusion. Recorded
here plainly, not hidden behind the original case description. Per
review 056 §5.3's standing rule referenced in the handoff, this reduced
scope should be treated as informative context for any future P06 spot-
check on this platform, not a clean pass on the original case.

### P08

The highest-value case in this slice per the handoff §4 — all three
recovery-dialog choices, each on its own fresh, isolated `$HOME`
(`dirs_next::config_dir()` = `$HOME/Library/Application Support` on
macOS, confirmed against `dirs-next`'s own source before relying on it).

- **Exit** (future-schema session fixture, `Incompatible` outcome,
  actions Exit/Continue with defaults). Clicking Exit
  (`dioxus_desktop::window().close()`) — the process was confirmed to
  actually terminate: `proc.poll()` (the direct, non-racy child-reap
  check, reading this exact child's own wait status) returned a real
  return code, corroborated by `os.kill(pid, 0)` reporting no such
  process. This is the single most important assertion in this slice per
  the handoff — "an Exit that dismisses the dialog and leaves a zombie is
  a failure that looks like a pass" — and it was checked directly, not
  inferred from the dialog closing.
- **Continue with defaults** (same future-schema fixture). The app kept
  running, the dialog dismissed, and `session.json`'s bytes were
  confirmed byte-for-byte unchanged afterward — the write-disabled
  behaviour `session_resolve_future_version_disables_writes_and_preserves_bytes`
  already covers at the unit level, exercised here through the real
  dialog and a real subsequent wait.
- **Reset and back up** (corrupt session fixture — literal invalid JSON,
  `CorruptPreserved` outcome, actions Continue with defaults/Reset — no
  Exit offered, matching `corrupt_shows_continue_and_reset_but_not_exit`).
  The dialog dismissed, `session.json` was reset away from the corrupt
  bytes, and a `session.json.reset.bak` sibling was confirmed to hold the
  *original corrupt bytes* exactly (`ensure_reset_backup` in
  `persist/schema/repository.rs`).
- **Legacy migration (filesystem-only)**: the shipping v0 settings/session
  fixtures (`crates/forskscope-core/src/tests/fixtures/persistence/
  settings-v0.json`/`session-v0.json`, already sanitized `/tmp/fixtures/…`
  paths) migrated without loss — theme, language, font size, and both tab
  pairs were all confirmed present in the migrated payload — and both
  files got a versioned envelope plus a `.pre-v2.bak` matching the
  original v0 bytes exactly.

### P12

**Result: Fail — a real product defect was found (registered here, not
fixed; see the dedicated section below).** Settings (Theme → Night,
Language → 日本語, both via `select_popup_item`'s `AXPress`-on-the-
opened-dropdown's-`AXMenuItem` — see the technique notes below) *did*
survive a normal close (the window's own close button) and restart: a
Japanese label ("設定") rendered post-restart, and re-opening Settings
confirmed both values read back correctly. **Tab restoration did not
happen** — a tab opened via CLI at the first launch was not restored on
the no-args relaunch, because `session.json` was never written for it in
the first place (the product defect below). Font size was **not**
changed via the UI in this run: every accessibility technique tried
against the font-size spinner (direct `AXValue` write, `AXIncrement`,
keystroke after `set focused`, keystroke after a UI-element click,
keystroke after a real `click at {x,y}` with the process made frontmost)
was a confirmed no-op on readback — recorded as manual-outstanding,
mirroring F45's shape, not silently skipped.

The explicit-CLI-args-does-not-restore-old-tabs half of P12 (relaunching
with `<left> <right>` must show only the new pair, not the session's old
tab) was implemented and is exercised by the same case, but was not
separately re-verified once the tab-restore defect was found, since there
was nothing to restore in this run's session file to begin with — this is
flagged as an open item, not asserted as passing.

## M5-B — accessibility technique findings worth recording

Several real dead ends and their eventual fixes, useful for any future
case built on this harness:

- **Buttons with no `aria_label`** (Redo, More ▼, Save As, Exit, Continue
  with defaults, Continue without saving, Reset and back up, Cancel,
  Overwrite) *are* findable via `click_button`'s existing description
  -then-title fallback — WebKit does expose an accessible name derived
  from a button's inner text when no `aria-label` is set. The one
  apparent counter-example (the header's "Settings" button returning
  `NOT_FOUND` on a probe fired immediately after the window appeared)
  turned out to be the accessibility tree lagging the DOM right after
  initial mount, not a real gap — a second probe ~0.4s later in the same
  run found it fine, and `click_wait`'s poll loop already tolerates this.
- **`<select>` elements are `AXPopUpButton`, not `AXComboBox`**, but a
  direct `AXValue` write (`set value of e to "Night"`) and
  `perform action "AXIncrement"` are both confirmed silent no-ops against
  them (readback never changes). What works: click the popup (`AXPress`,
  opens its dropdown), then find the opened dropdown's option as a plain
  `AXMenuItem` element via an ordinary tree walk (title match) and invoke
  `perform action "AXPress"` on *that* — not the generic `click` verb,
  which structurally succeeds but does not change the underlying value
  for this control. The dropdown's options are not reachable via the
  standard `menu item X of menu 1 of popUpButton` reference syntax at all
  (`Can't get menu 1 of pop up button N ... Invalid index`) — this
  control is not a true native `NSPopUpButton`, only styled to look like
  one.
- **Text fields**: a plain `<input type="text">` (Save As's path field)
  accepted a direct `AXValue` write in the run that exercised it. The
  font-size `<input type="number">` spinner never did, under any
  technique tried (see P12 above) — text vs. number inputs appear to
  behave differently for this purpose, not yet fully explained.
- **`entire contents of w` and per-property queries can wedge or fail
  outright** against certain views/element combinations for reasons never
  fully root-caused (`dump_roles`, an internal-only debugging aid, was
  abandoned after exhausting several hypotheses — see
  `macos_ui.applescript`'s own comments for the investigation). The
  commands actually used by every real case (`count_rows`, `find_text`,
  `click_button`, `click_row`, `click_row_side`, `click_any`) all remained
  reliable throughout.
- **Two same-named processes**: addressing a specific one by PID
  (`"pid:<n>"`, `first process whose unix id is n`) is supported and
  works correctly, but did not turn out to be the fix for P06's
  concurrent-process attempt — that was the fixture-size issue above, not
  process-identity ambiguity. Kept as infrastructure since it's a real,
  useful capability regardless.

## M5-B — product defect found (registered, not fixed)

**Session state opened via CLI startup is never persisted if the tab is
never closed before the app quits — the reactive tabs-changed
`use_effect` meant to auto-save it does not actually write, while direct
(non-reactive) save call sites do.**

Found while developing P12, isolated via a dedicated diagnostic
(`recon_session_save`, not a scored case) that strips away every other
variable:

1. `forskscope <left> <right>` opens a tab. `session.json` is confirmed
   **absent** (not even created) 1.5s after the tab fully renders, even
   though `app.rs` registers:
   ```rust
   // Persist the session whenever the tab list changes.
   use_effect(move || {
       let _tabs = store.tabs.read();
       save_session(&store);
   });
   ```
2. Clicking the tab's own Close button — `close_tab` in `state/session.rs`,
   which calls `save_session(&store)` **directly**, not through the
   reactive effect — and `session.json` appears correctly right
   afterward, in the same process, same environment.

Two earlier hypotheses were investigated and ruled out before finding
this:

- *Missing config directory* (`dirs_next::config_dir()/forskscope` never
  created by anything in the shipped binary — confirmed true by grepping
  the whole crate, and a real gap on its own: neither `persist_session`
  (`state/session.rs`) nor `persist_settings` (`ui/view/settings.rs`)
  check the `Result` of `repo.save(...)`, both discard it with `let _ =`,
  so any write failure — directory-related or otherwise — is silent by
  construction). Pre-creating the directory did **not** fix the missing-
  session-file symptom on its own, ruling this out as the sole cause,
  though the silent-discard pattern remains real and worth fixing
  independently of this specific defect.
- *App Sandbox entitlements* — ruled out via `codesign -d --entitlements`,
  which came back empty (unsigned, no sandbox container restricting
  writes).

**Practical impact**: any user who runs `forskscope <left> <right>` from
the command line and quits without otherwise touching the tab list (the
single most ordinary CLI/`git difftool`-style workflow this application
exists for) loses that session silently — nothing is written to disk, no
error is shown, and the next launch has nothing to restore. Settings
changes made via the Settings dialog are unaffected (they persist through
a separate, synchronous, non-reactive call path) — this is specific to
session/tab-list persistence.

Reported here precisely, per the handoff's instruction, and **not fixed**
in this pass.

## Accessibility approach — a deviation from the Linux harness worth noting

The Linux harness (`linux_harness.py`) drives AT-SPI's `Action.do_action`
via `pyobjc`-equivalent Python GTK bindings (`gi.repository.Atspi`), which
are pre-installed via `apt`. macOS has no equivalent pre-installed Python
binding for the Accessibility API (`pyobjc` is not on the `macos-latest`
image and would need a `pip install` step — a new tooling dependency to
manage, and still needs its own accessibility grant separate from
whatever System Events already has). This harness instead drives macOS's
built-in `osascript`/AppleScript against System Events, invoking the same
kind of direct accessible action (`click`/`AXPress`) that AT-SPI's
`do_action` provides on Linux — see `macos_ui.applescript`'s header
comment for the full reasoning. This worked without any special CI setup:
`macos-latest`'s System Events UI scripting was already usable with no
permission wall encountered on any of the eight runs in this pass (see
"Falsifiability" below) — worth flagging as **not fully proven robust**
for a wider ongoing use, since it was observed to work on this specific
resolved image (`macos26`/`20260728.0273.1`) only.

## Falsifiability

Every case's `--break` mode was run and confirmed to fail for the
expected reason, per handoff §7:

| Case | Break-mode run | Result |
|---|---|---|
| P01 | [`31853183079`](https://github.com/forskscope/forskscope/actions/runs/31853183079) | Fail — `--diagnostics output does not start with 'ForskScope 0.0.0-impossible': ForskScope 0.167.0` |
| P02 | [`31853264441`](https://github.com/forskscope/forskscope/actions/runs/31853264441) | Fail — `compare view showed 14 AXRow elements, expected 99, within 45s` |
| P09 | [`31853265706`](https://github.com/forskscope/forskscope/actions/runs/31853265706) | Fail — real merge content (`alpha\nold-line\ngamma\nepsilon\nzeta\ninsert-line\n`) correctly rejected against a value real Save output can never produce |
| P10 | [`31853267021`](https://github.com/forskscope/forskscope/actions/runs/31853267021) | Fail — `could not find text containing 'this message cannot appear' within 45s` |
| P04 | [`31880080196`](https://github.com/forskscope/forskscope/actions/runs/31880080196) | Fail — `saved content b'alpha\nold-line\ngamma\nepsilon\nzeta\ninsert-line\n' != impossible expected b'this exact string can never appear in real merge output'` |
| P05 | [`31880082912`](https://github.com/forskscope/forskscope/actions/runs/31880082912) | Fail — `.bak content b'EXTERNALLY MODIFIED WHILE APP WAS OPEN\nsecond line\n' != impossible expected b'this exact string can never be the pre-overwrite content'` |
| P06 | [`31883530193`](https://github.com/forskscope/forskscope/actions/runs/31883530193) | Fail — `expected final sentinel 'ASYNC-IDENTITY-SENTINEL-PAIR-B-RELOAD-V1' never appeared` (the impossible expectation — the stale reload's content — was correctly not satisfied; real behaviour shows the second reload's content) |
| P08 — Exit | [`31880085680`](https://github.com/forskscope/forskscope/actions/runs/31880085680) | Fail (expected) — `process (pid 27125) exited with returncode 0 within 20s of clicking Exit - the real behaviour --break's impossible expectation ('still running') was checked against` — the exact falsifiability demonstration handoff §6 requires for Exit specifically |
| P08 — Continue with defaults | [`31880089230`](https://github.com/forskscope/forskscope/actions/runs/31880089230) | Fail (expected) — `session.json is unchanged, matching write-disabled behaviour - --break's impossible expectation ('bytes changed') was correctly not satisfied` |
| P08 — Reset and back up | [`31880092086`](https://github.com/forskscope/forskscope/actions/runs/31880092086) | Fail — `backup content b'{not valid json' != impossible expected b'this exact string can never be the original corrupt bytes'` |
| P08 — legacy migration | [`31880096494`](https://github.com/forskscope/forskscope/actions/runs/31880096494) | Fail — `migrated settings payload language 'ja' != expected 'impossible-language'` |
| P12 | [`31883545108`](https://github.com/forskscope/forskscope/actions/runs/31883545108) | Fail — `restored Theme 'VALUE: Night' != expected 'VALUE: Light'` |

## F46 — Gatekeeper: Blocked, not Pass, not Waived

Per `matrix-plan.md` §3 and the M5-A handoff, out of scope for this slice
to resolve and not attempted here. `gh release download` and CI's
checkout/download path never apply the quarantine extended attribute a
real browser/`curl`-to-Finder download would, so Gatekeeper never engages
regardless of what this workflow does — the `.dmg` mounted and the
binary launched cleanly in every run above, which is expected and
uninformative about signing/notarization posture, not evidence Gatekeeper
was satisfied. **No manual macOS host exists in the current execution
model, so F46 has no resolution path under current resourcing** — recorded
here as an explicit, unresolved Gate D input, exactly as `matrix-plan.md`
directs.

## Failures and issue links

None found. All four cases pass functionally on `macos-latest`
(`macos26`/`20260728.0273.1`, aarch64) — no P01/P02/P09/P10 product
defect was observed in this pass. F46 (Gatekeeper) remains open per
above, and is not a P01-vs-Pass/Fail case result — it is a separate,
structurally-unverifiable posture question.

## M5-B — Failures and issue links

One real product defect found, registered, not fixed: session state
opened via CLI startup is never persisted if the tab is never closed
before the app quits — see "M5-B — product defect found" above for the
full writeup. P12 is recorded as **Fail** for this reason, distinct from
a harness bug; Theme/Language restoration (the working half of P12) was
confirmed passing within the same run. P06 runs as a materially reduced-
scope variant of RFC-078's description (sequential, non-overlapping
launches instead of concurrent ones) — see P06's own section above for
why and what this does and doesn't verify; recorded as **Pass** for what
it actually checks, not for RFC-078's original description. P04's
keyboard-Enter apply path and P12's font-size UI change are both
recorded as manual-outstanding (structurally unreachable via
accessibility on any platform, for the former; five independent
accessibility techniques all confirmed no-ops, for the latter) — neither
silently skipped nor claimed covered.

## M5-B — Waivers

None.

## Waivers

None.
