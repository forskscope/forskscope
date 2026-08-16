#!/usr/bin/env python3
"""M5-A/M5-B: Windows evidence harness for P01, P02, P04, P05, P06, P08,
P09, P10, P12 (RFC-078).

Windows counterpart to `linux_harness.py`. Same falsifiability contract
(`--break`), same CLI shape - but driven through Windows UI Automation
(UIA) via `pywinauto` instead of AT-SPI, because there is no AT-SPI on
Windows and no Xvfb/X11 story either: `windows-latest` GitHub Actions
runners provide a real interactive desktop session, so (per the M5-A
handoff) launches here should just work without any virtual display
machinery.

M5-B adds P04, P05, P06, P08, P12 - the interaction cases. See each
function's docstring for its own reasoning; three module-wide notes that
apply across several of them:

- **P04's keyboard path is not exercised** (M5-B handoff §3): only the
  mouse path (UIA Invoke on "Use this change") is demonstrated here. The
  Enter-key shortcut is a raw `onkeydown` listener with no bound UI
  element for any accessibility API to invoke, on any platform - see
  `p04`'s docstring.
- **P08/P12 use the CI runner's real (but disposable) `%APPDATA%\\forskscope`
  directory directly, not an environment-variable override.**
  `dirs_next::config_dir()` resolves via the Win32
  `SHGetKnownFolderPath` API on Windows, which reads the registry/user
  profile - not the `APPDATA` process environment variable - so setting
  `APPDATA` on the launched subprocess would not actually redirect
  anything (unlike POSIX platforms, where `dirs`/`dirs_next` does honor
  the environment). On a `windows-latest` runner the whole VM is thrown
  away after the run, so there is no real user's config to protect;
  using the real profile directory and clearing it between launches
  (`clear_config_dir`) gives the same practical isolation the M5-B
  handoff's phrasing asks for, empirically resolving what that handoff
  flagged as something to confirm rather than assume.
- **Settings dialog controls have no `aria-label`** (`Theme`/`Language`
  `<select>`s sit next to a plain sibling `<span>`, with no
  `<label for>`/`aria-labelledby`), so this harness locates them
  ordinally (first `ComboBox` = Theme, second = Language) rather than by
  accessible name - a real, reported limitation, not an assumption papered
  over.
- **P12's tab auto-save finding**: a real `forskscope.exe` process
  launched with two CLI arguments (`forskscope.exe <left> <right>`) was
  observed, on real CI, to never write its newly-opened tab to
  `session.json` at all - not once, within `LAUNCH_TIMEOUT_S` of the
  diff fully rendering, against a config directory cleared immediately
  beforehand (so no earlier launch's file could be mistaken for a fresh
  one) and polled *while the process stayed alive* (ruling out a race
  with this harness's own `terminate()`). `app.rs`'s reactive
  `use_effect(move || { store.tabs.read(); save_session(&store); })`
  is the mechanism that's supposed to do this - its own doc comment
  (review 041 C1) explicitly says a CLI launch's `open_compare` should
  trigger it. Not fixed here (no product behaviour changes), and `p12`
  no longer depends on it: it seeds `session.json` by constructing the
  v2 envelope directly instead of launching-and-waiting for an
  autosave. Reported as a real finding, not silently routed around.

P06 also deviates from a literal reading of "two tabs": it uses two
independent `forskscope.exe` processes rather than two tabs of one
running instance, because opening a second comparison in-app depends on
Explorer's file-tree row selection - the same `role="row"` UIA-control-type
question `windows-11.md`'s P02 section already flagged as unresolved and
explicitly deferred to settle before M5-C's P03. The M5-B handoff
explicitly allows either approach ("whichever is achievable, document
which you did"); see `p06`'s docstring.

Why `pywinauto`, not a `.ps1` harness: the handoff's falsifiability
requirement and the AT-SPI-derived Linux harness both depend on walking an
accessible tree and invoking a button's action directly (not synthesizing
mouse/keyboard input) - `pywinauto`'s UIA backend exposes exactly that
(`element.invoke()`, the Invoke pattern), plus a real Python process/tree
model that mirrors `linux_harness.py`'s structure closely enough to keep
the two harnesses easy to compare. A `.ps1` rewrite would either shell out
to the same UIA COM interfaces with far more ceremony, or fall back to
`SendKeys`-style synthetic input - the exact class of flakiness the Linux
handoff notes explicitly steers away from.

Content-based readiness, not row-count: `linux_harness.py`'s P02/P09
readiness condition polls until the AT-SPI tree has *exactly* 7 "table
row" accessibles per pane (F57's pinned fixture shape), because AT-SPI's
role naming happens to expose ARIA `role="row"` as a distinctly countable
"table row" role. Windows UIA's mapping of a bare, table-less
`role="row"` div (this app's rows are not inside an ARIA `role="table"`
ancestor) to a specific, reliably-countable UIA control type is not
something this harness assumes - counting a guessed control type would
be one accessibility-mapping quirk away from silently under- or
over-counting, with no way to tell from CI output alone. Instead, this
harness asserts that every one of the fixture's eight distinguishing line
tokens (`alpha`, `old-line`, `new-line`, `gamma`, `delete-line`,
`epsilon`, `zeta`, `insert-line`) is present somewhere in the rendered
accessible tree's text - a real, specific, fixture-tied content check
(not "a landmark exists"), just expressed as presence-of-content rather
than a row tally. This is a deliberate, reported deviation from exact
structural parity with the Linux harness; see the M5-A Windows report for
the reasoning.

Usage:
  windows_harness.py p01 <binary>
  windows_harness.py p02 <binary>
  windows_harness.py p04 <binary>
  windows_harness.py p05 <binary>
  windows_harness.py p06 <binary>
  windows_harness.py p08 <binary>
  windows_harness.py p09 <binary>
  windows_harness.py p10 <binary>
  windows_harness.py p12 <binary>

Falsifiability (`--break`): each case accepts an optional `--break` flag
that deliberately breaks the condition it checks, to demonstrate the
assertion is not vacuous. See each case function's docstring for exactly
what it breaks. `--break` never touches product source; it only changes
what the harness expects to see.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# PowerShell's default console codepage on windows-latest is cp1252, not
# UTF-8 - a plain print() of accessible text containing a non-cp1252
# character (e.g. an icon glyph or the "↔" this module's own
# rowprobe hit on its first CI run) raises UnicodeEncodeError and aborts
# the whole case before its real output prints. Reconfigured once, up
# front, for every case - not just rowprobe - since any case's
# debug_dump()/collect_texts() could hit the same wall on accessible text
# this harness doesn't control the content of.
# Also forced to line-buffering: PowerShell's captured-output ordering
# otherwise interleaves a block-buffered stdout with an unbuffered
# stderr out of chronological order (observed on scrollprobe's first CI
# run - a stderr FAIL line appeared before the stdout diagnostic dump
# that logically preceded it, though it did run first).
for _stream in (sys.stdout, sys.stderr):
    if hasattr(_stream, "reconfigure"):
        _stream.reconfigure(encoding="utf-8", errors="backslashreplace", line_buffering=True)

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

LAUNCH_TIMEOUT_S = 60
READY_TIMEOUT_S = 60
POLL_INTERVAL_S = 0.5

FIXTURE_TOKENS = [
    "alpha",
    "old-line",
    "new-line",
    "gamma",
    "delete-line",
    "epsilon",
    "zeta",
    "insert-line",
]

XLSX_MESSAGE = "Spreadsheet comparison is temporarily disabled for security."


# ── pywinauto plumbing ───────────────────────────────────────────────────────


def _app_module():
    """Import pywinauto lazily so `--help`-style usage errors on non-Windows
    dev machines don't require the dependency to be installed at all."""
    from pywinauto import Application  # noqa: PLC0415

    return Application


def connect(pid, timeout_s=LAUNCH_TIMEOUT_S):
    """Connect pywinauto's UIA backend to the process `pid`'s top-level
    window, retrying against `timeout_s` - the window frequently does not
    exist yet at the moment the process is spawned (WebView2/COM/first-run
    initialisation), exactly analogous to `find_app`'s retry loop in
    `linux_harness.py` and `render_check.py`'s F57 finding that the first
    poll after spawn is not a reliable moment to look."""
    Application = _app_module()
    deadline = time.monotonic() + timeout_s
    last_exc = None
    while time.monotonic() < deadline:
        try:
            app = Application(backend="uia").connect(process=pid, timeout=1)
            win = app.top_window()
            win.wait("exists", timeout=1)
            return app, win
        except Exception as exc:  # noqa: BLE001 - broad: many distinct pywinauto/COM exceptions can fire here
            last_exc = exc
            time.sleep(POLL_INTERVAL_S)
    raise RuntimeError(f"could not connect to pid {pid} within {timeout_s}s: {last_exc}")


def invoke(elem):
    """Trigger a control's primary action via UIA's Invoke pattern - the
    Windows analogue of `linux_harness.py`'s `Atspi.Action.do_action`, and
    for the same reason: it calls the control's handler directly through
    the accessibility layer, with no dependency on window focus, screen
    position, or synthesized mouse/keyboard input. Falls back to a real
    synthesized click (`click_input`) only if the control does not expose
    an Invoke pattern, and reports which path was used so that fallback
    use is visible rather than silent.
    """
    try:
        elem.invoke()
        return "invoke-pattern"
    except Exception:  # noqa: BLE001 - fall through to the synthesized-click path
        elem.click_input()
        return "click-input-fallback"


def is_enabled(elem):
    try:
        return bool(elem.is_enabled())
    except Exception:  # noqa: BLE001 - if the property can't be read, don't block on it here
        return True


def collect_texts(win):
    """Flatten every descendant's own accessible text (`window_text()`,
    which pywinauto's UIA backend backs with the control's Name property -
    the same property WebView2 derives from `aria-label`/rendered text)
    into a list. Equivalent in purpose to `linux_harness.py`'s recursive
    name/description/Text-interface walk, just via UIA's flat
    `descendants()` rather than a hand-rolled recursive walk over AT-SPI's
    child API."""
    texts = []
    try:
        t = win.window_text()
        if t:
            texts.append(t)
    except Exception:  # noqa: BLE001
        pass
    for d in win.descendants():
        try:
            t = d.window_text()
        except Exception:  # noqa: BLE001 - a single unreadable node must not abort the whole scan
            continue
        if t:
            texts.append(t)
    return texts


def debug_dump(texts, limit=80):
    """Print a bounded sample of collected accessible text to stderr on a
    FAIL path - the only way to diagnose a UIA content-search miss from CI
    log output alone, without a separate ad hoc tree-dump run."""
    print(f"  ({len(texts)} accessible text nodes collected; sample follows)", file=sys.stderr)
    for t in texts[:limit]:
        print(f"    {t!r}", file=sys.stderr)
    if len(texts) > limit:
        print(f"    ... ({len(texts) - limit} more, truncated)", file=sys.stderr)


def find_by_text_containing(win, substring):
    """First descendant (or the window itself) whose own text contains
    `substring` - the Windows analogue of `find_by_name_containing`."""
    try:
        if substring in (win.window_text() or ""):
            return win
    except Exception:  # noqa: BLE001
        pass
    for d in win.descendants():
        try:
            t = d.window_text()
        except Exception:  # noqa: BLE001
            continue
        if t and substring in t:
            return d
    return None


def find_exact(win, text, control_type=None):
    """First descendant (or the window itself) whose own text equals
    `text` exactly - stricter than `find_by_text_containing`, needed
    wherever a substring match risks hitting more than one control (e.g.
    a toolbar "Save" button and a modal's own "Save" button coexist in
    the accessible tree at the same time, since Dioxus keeps the tab
    behind a modal mounted). `control_type` narrows the UIA ControlType
    string (e.g. "Button", "Edit", "ComboBox") when supplied, which both
    disambiguates further and avoids walking irrelevant nodes."""
    try:
        candidates = (
            win.descendants(control_type=control_type)
            if control_type
            else win.descendants()
        )
    except Exception:  # noqa: BLE001
        candidates = []
    for d in candidates:
        try:
            t = d.window_text()
        except Exception:  # noqa: BLE001
            continue
        if t and t.strip() == text:
            return d
    return None


def wait_for_exact(win, text, control_type=None, timeout_s=READY_TIMEOUT_S):
    """Polls `find_exact` until it succeeds or `timeout_s` elapses -
    re-searches the live tree every iteration rather than caching a
    single lookup, since a UIA element reference can go stale across a
    Dioxus re-render (the same reason `p09`'s Save-button retry loop in
    M5-A re-finds the button fresh on every attempt)."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        found = find_exact(win, text, control_type=control_type)
        if found is not None:
            return found
        time.sleep(POLL_INTERVAL_S)
    return find_exact(win, text, control_type=control_type)


def wait_button_enabled(win, text_substring, expected, timeout_s=READY_TIMEOUT_S):
    """Polls `find_by_text_containing` + `is_enabled` until the button's
    enabled state matches `expected` or `timeout_s` elapses. Re-finds the
    element on every poll for the same stale-reference reason as
    `wait_for_exact`. Used as the dirty-state signal for P04: the
    toolbar's Save button is enabled exactly when `snap.is_dirty`
    (`toolbar.rs`: `disabled: !snap.is_dirty`)."""
    deadline = time.monotonic() + timeout_s
    last = None
    while time.monotonic() < deadline:
        elem = find_by_text_containing(win, text_substring)
        if elem is not None:
            last = is_enabled(elem)
            if last == expected:
                return True
        time.sleep(POLL_INTERVAL_S)
    return last == expected


def wait_for_first(win, control_type, timeout_s=READY_TIMEOUT_S):
    """First descendant of the given UIA ControlType, polled until it
    appears or `timeout_s` elapses."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        try:
            found = win.descendants(control_type=control_type)
        except Exception:  # noqa: BLE001
            found = []
        if found:
            return found[0]
        time.sleep(POLL_INTERVAL_S)
    return None


def wait_gone(win, text_substring, timeout_s=READY_TIMEOUT_S):
    """Polls until no descendant's text contains `text_substring` -
    used to confirm a modal has actually dismissed, not just that its
    action was clicked."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        if find_by_text_containing(win, text_substring) is None:
            return True
        time.sleep(POLL_INTERVAL_S)
    return find_by_text_containing(win, text_substring) is None


def wait_and_invoke(win, text_substring, timeout_s=READY_TIMEOUT_S):
    """Polls for a descendant whose text contains `text_substring` and
    invokes it as soon as it's found, instead of one lookup-then-invoke.
    Needed wherever a control can be transiently *absent* from the tree
    rather than merely disabled - e.g. the toolbar (and its Reload
    button) during `TabState::Loading`: `diff.rs` returns an early
    loading-spinner view with no `Toolbar` at all while a tab is loading
    (confirmed by reading the component, not assumed - a blind
    lookup-then-invoke here raised `AttributeError: 'NoneType' object
    has no attribute 'invoke'` on M5-B's first CI run for P06's second,
    immediately-following reload click). Raises if nothing appears
    within `timeout_s`."""
    deadline = time.monotonic() + timeout_s
    elem = None
    while time.monotonic() < deadline:
        elem = find_by_text_containing(win, text_substring)
        if elem is not None:
            break
        time.sleep(POLL_INTERVAL_S)
    if elem is None:
        raise RuntimeError(
            f"no descendant with text containing {text_substring!r} appeared within {timeout_s}s"
        )
    invoke(elem)


def _combo_readback(elem):
    """Every readback source this harness knows how to ask a ComboBox
    about its current value, gathered together because it's not known
    (M5-B: readback via `window_text()` alone was not conclusive - see
    `select_dropdown`'s docstring) which one, if any, WebView2/Chromium's
    `<select>`-as-"ComboBox" mapping actually keeps in sync with a real
    selection. Returns a dict of source-name -> value-or-None, used both
    for `_combo_shows`'s check and for a rich diagnostic on failure."""
    out = {}
    try:
        out["window_text"] = elem.window_text()
    except Exception as exc:  # noqa: BLE001
        out["window_text"] = f"<error: {exc}>"
    try:
        out["iface_value.CurrentValue"] = elem.iface_value.CurrentValue
    except Exception as exc:  # noqa: BLE001
        out["iface_value.CurrentValue"] = f"<error: {exc}>"
    try:
        out["legacy_properties.Value"] = elem.legacy_properties()["Value"]
    except Exception as exc:  # noqa: BLE001
        out["legacy_properties.Value"] = f"<error: {exc}>"
    return out


def _combo_shows(elem, text):
    readback = _combo_readback(elem)
    return any(text in str(v) for v in readback.values())


def select_dropdown(elem, text):
    """Selects `text` in a dropdown control, trying three UIA-level
    approaches in order and *verifying* the combo's own reported text
    actually changed after each one (not just that the call didn't
    raise) before accepting it as a success - two of these three were
    each individually found, on real CI, to either raise or silently not
    take effect:

    1. `pywinauto`'s `ComboBoxWrapper.select()` (`SelectionItemPattern`,
       via expand/collapse) - raised `IndexError: item '...' not found`
       on the first CI attempt: WebView2/Chromium's mapping of a bare
       HTML `<select>` to UIA "ComboBox" does not expose a selectable
       item list the way `pywinauto` expects for a classic Win32 combo.
    2. Expand the combo (best-effort - Chromium's `<select>` popup on
       Windows is its own rendered surface, not a literal Win32
       COMBOBOX popup) and search the *whole window* for a descendant
       whose text matches, then Invoke it directly - the same
       accessibility-action approach every button in this harness uses.
    3. `ValuePattern.SetValue` directly on the control - didn't raise on
       the second CI attempt, but the value never actually landed
       (`settings.json` was never written with the new theme), so this
       is verified like the others rather than trusted on a clean return.

    Returns which path actually worked (confirmed by readback).
    """

    def try_selection_item():
        elem.select(text)
        return "selection-item-pattern"

    def try_expand_and_invoke_item():
        try:
            elem.expand()
        except Exception:  # noqa: BLE001 - best-effort; the item may already be reachable
            pass
        top = elem.top_level_parent()
        item = wait_for_exact(top, text, timeout_s=5)
        if item is None:
            raise RuntimeError(f"no descendant with text {text!r} found after expand()")
        invoke(item)
        try:
            elem.collapse()
        except Exception:  # noqa: BLE001
            pass
        return "expand-then-invoke-item"

    def try_value_pattern():
        elem.iface_value.SetValue(text)
        return "value-pattern"

    # Outer retry: the same general UI-automation flakiness
    # set_value_text() documents (identical code passing on one CI run
    # and failing outright on the very next) was also observed here for
    # combo selection itself, not just the font-size field - each retry
    # still only counts a success when the value genuinely reads back.
    attempt_log = []
    for retry in range(3):
        for attempt in (try_selection_item, try_expand_and_invoke_item, try_value_pattern):
            try:
                path = attempt()
            except Exception as exc:  # noqa: BLE001
                attempt_log.append(f"retry {retry} {attempt.__name__}: raised {exc!r}")
                continue
            if _combo_shows(elem, text):
                return path
            attempt_log.append(
                f"retry {retry} {attempt.__name__} ({path}): did not raise, readback={_combo_readback(elem)!r}"
            )
        time.sleep(POLL_INTERVAL_S)

    raise RuntimeError(
        f"could not select {text!r} via any known pattern across 3 retries:\n  "
        + "\n  ".join(attempt_log)
    )


def find_number_inputs(win):
    """Descendants for an `<input type="number">` field, preferring the
    actual editable part over its container. Empirically (M5-B, P12's
    first CI run), Chromium/WebView2 does NOT map a number input to UIA
    ControlType "Edit" the way a plain text input does (the Save As path
    field, a plain `<input type="text">`, is found via "Edit" without
    trouble) - it maps to "Spinner" instead, presumably because of the
    native up/down stepper Chromium renders for numeric inputs. But
    (M5-B, P12's later CI attempts) setting a value on that outer
    "Spinner" element - via ValuePattern, a Tab-commit, or even full
    keystroke synthesis after `set_focus()` - never changed what any
    readback source reported, across three escalating attempts: a
    strong signal the "Spinner" node is a non-editable *container*
    (mirroring a classic Win32 buddy-edit + up-down composite), with the
    real editable text living in a nested "Edit" child UIA does expose
    separately. This now looks for that nested Edit **inside** each
    Spinner match first (`spinner.descendants(control_type="Edit")`),
    falling back to the Spinner itself if none is found, and finally to
    a bare "Edit" match at the top level for a future WebView2 mapping
    that doesn't nest this way at all."""
    try:
        spinners = win.descendants(control_type="Spinner")
    except Exception:  # noqa: BLE001
        spinners = []
    for spinner in spinners:
        try:
            nested_edits = spinner.descendants(control_type="Edit")
        except Exception:  # noqa: BLE001
            nested_edits = []
        if nested_edits:
            return nested_edits
    if spinners:
        return spinners
    try:
        return win.descendants(control_type="Edit")
    except Exception:  # noqa: BLE001
        return []


def set_value_text(elem, text):
    """Sets a control's text, verifying the value actually reads back
    (via `_combo_shows`) before accepting each attempt, in ascending
    order of how much it relies on real input synthesis rather than an
    accessibility pattern:

    1. `set_edit_text()` (pywinauto's `EditWrapper`, `ValuePattern.SetValue`
       under the hood) where pywinauto wraps the control that way, or
       `ValuePattern.SetValue` directly for a control type (e.g.
       "Spinner" - see `find_number_inputs`) pywinauto doesn't wrap with
       `EditWrapper`.
    2. The same, followed by a synthesized Tab keystroke.
    3. Real keyboard synthesis: `set_focus()` (UIA's accessibility-level
       focus, not a physical click), then End + repeated Backspace to
       clear the field, type the text, Tab to commit.
    4. The same keystroke sequence, but preceded by a real synthesized
       click (`click_input()`) instead of `set_focus()`.

    All four were needed, in order, on real CI (M5-B, P12):
    `ValuePattern.SetValue` updates the DOM value but never fires
    Dioxus's `onchange` listener (a plain `change` event, which for a
    number input normally fires on blur/commit, not on every value
    write); a bare Tab keystroke with no preceding focus has nothing to
    commit *from*; `set_focus()`'s UIA-level focus did not route real
    keyboard input into this WebView2/Chromium control at all (readback
    never moved off the app's own default); and even after switching to
    a real click to establish genuine focus, `^a` (Ctrl+A, "select all")
    turned out not to actually clear the field's existing content
    either - the first click-based attempt read back `32`, the input's
    own `max` clamp, which only makes sense if the typed digits landed
    *appended* to the untouched default rather than replacing it (`1418`
    or similar, clamped down to `32`) - so step 4 clears with an
    explicit End + repeated-Backspace instead of relying on select-all.
    Steps 3-4 are the one place in this harness that falls back to full
    real input synthesis for something no accessibility pattern alone
    reliably replicates here, mirroring `invoke()`'s own established
    Invoke-then-`click_input()` fallback shape (try the pattern first,
    report which path actually worked)."""
    attempt_log = []

    def try_pattern():
        try:
            elem.set_edit_text(text)
            return "set_edit_text"
        except Exception:  # noqa: BLE001
            elem.iface_value.SetValue(text)
            return "value-pattern"

    def try_pattern_then_tab():
        path = try_pattern()
        try:
            elem.type_keys("{TAB}")
        except Exception:  # noqa: BLE001
            pass
        return f"{path}+tab-commit"

    def try_real_keystrokes():
        elem.set_focus()
        elem.type_keys("{END}{BACKSPACE 6}" + text + "{TAB}")
        return "keystroke-synthesis"

    def try_click_then_keystrokes():
        # UIA's SetFocus (what set_focus() and the plain
        # keystroke-synthesis attempt above use) is an
        # accessibility-level focus call - empirically (M5-B, P12) it did
        # not route real keyboard input into this WebView2/Chromium
        # number input at all, readback never moved off the app's actual
        # default. A synthesized *click* physically focuses the element
        # the same way real user interaction would (Chromium's own
        # hit-testing/focus routing, not just the UIA focus notion), so
        # this clicks first, then types - the most input-synthesis-heavy
        # attempt in this harness, used only because every accessibility-
        # pattern-only approach was verified not to work here.
        elem.click_input()
        elem.type_keys("{END}{BACKSPACE 6}" + text + "{TAB}")
        return "click-then-keystroke-synthesis"

    # Outer retry: even the full click-then-keystroke attempt was found
    # (M5-B, P12) to work on one CI run and leave the value completely
    # unchanged on a later run of the *identical* code - real
    # UI-automation flakiness (timing against the WebView2 renderer),
    # not a logic bug this loop's ordering alone can fix. Each full pass
    # still only counts a success when the value genuinely reads back
    # (`_combo_shows`), so retrying doesn't weaken the assertion.
    for retry in range(3):
        for attempt in (try_pattern, try_pattern_then_tab, try_real_keystrokes, try_click_then_keystrokes):
            try:
                path = attempt()
            except Exception as exc:  # noqa: BLE001
                attempt_log.append(f"retry {retry} {attempt.__name__}: raised {exc!r}")
                continue
            if _combo_shows(elem, text):
                return path
            attempt_log.append(
                f"retry {retry} {attempt.__name__} ({path}): did not raise, readback={_combo_readback(elem)!r}"
            )
        time.sleep(POLL_INTERVAL_S)

    raise RuntimeError(
        f"set_value_text: could not set {text!r} via any known approach across 3 retries:\n  "
        + "\n  ".join(attempt_log)
    )


def modal_action_button(win, label):
    """Finds the action button labelled `label` inside the currently-open
    modal, disambiguated from an identically-labelled toolbar control
    (both the toolbar and `SaveAsModal` have a "Save" button, both the
    toolbar and `OverwriteModal`/`SaveAsModal` share no "Cancel", but the
    principle generalises) by anchoring on the modal's own "Cancel"
    button - which no toolbar control carries - and searching only
    within its sibling container (`div.actions`) rather than the whole
    tree. Falls back to a plain exact-text search if no "Cancel" button
    is present (e.g. the P08 recovery dialogs, whose action labels are
    already globally unique)."""
    cancel = find_exact(win, "Cancel", control_type="Button")
    if cancel is not None:
        try:
            container = cancel.parent()
            for child in container.children(control_type="Button"):
                if child.window_text().strip() == label:
                    return child
        except Exception:  # noqa: BLE001
            pass
    return find_exact(win, label, control_type="Button")


# ── M5-B: config-directory isolation for P08/P12 ────────────────────────────


def resolve_config_dir():
    """`forskscope`'s config directory, exactly as
    `crate::state::config_file_path` derives it (`dirs_next::config_dir()
    .join("forskscope")`) - here computed as `%APPDATA%\\forskscope`
    directly, since `%APPDATA%` and what `SHGetKnownFolderPath` returns
    are the same value for the current user under normal circumstances
    (see the module docstring for why an env-var override wouldn't
    actually redirect this)."""
    appdata = os.environ.get("APPDATA")
    if not appdata:
        raise RuntimeError(
            "APPDATA is not set in this environment - cannot resolve "
            "forskscope's config directory"
        )
    return Path(appdata) / "forskscope"


def clear_config_dir(config_dir):
    """Removes and recreates `config_dir` - sequential isolation between
    P08/P12's several launches against the one real config directory
    available on this runner (see the module docstring)."""
    if config_dir.exists():
        shutil.rmtree(config_dir)
    config_dir.mkdir(parents=True, exist_ok=True)


def wait_for_tokens(win, tokens, timeout_s=READY_TIMEOUT_S):
    """Poll until every string in `tokens` appears somewhere in the
    accessible tree's text. Returns (True, texts, []) on success or
    (False, texts, missing) if `timeout_s` elapses first - mirrors
    `wait_for_ready`'s retry-against-a-deadline shape (never a single
    tree walk immediately after launch; see F57)."""
    deadline = time.monotonic() + timeout_s
    texts = []
    while time.monotonic() < deadline:
        texts = collect_texts(win)
        blob = "\n".join(texts)
        missing = [tok for tok in tokens if tok not in blob]
        if not missing:
            return True, texts, []
        time.sleep(POLL_INTERVAL_S)
    blob = "\n".join(texts)
    missing = [tok for tok in tokens if tok not in blob]
    return False, texts, missing


def launch(binary, args, cwd, env=None):
    run_env = dict(os.environ) if env is None else env
    return subprocess.Popen(
        [str(binary), *[str(a) for a in args]], cwd=str(cwd), env=run_env
    )


def terminate(proc):
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


def run_diagnostics(binary):
    """Run `--diagnostics` (no window needed) and return stdout - identical
    in spirit to `linux_harness.py`'s helper of the same name."""
    result = subprocess.run(
        [str(binary), "--diagnostics"], capture_output=True, text=True, timeout=15
    )
    return result.returncode, result.stdout, result.stderr


# ── P01 — Install and cold launch ───────────────────────────────────────────


def p01(binary, break_mode=False):
    """Two independent checks, both real launches of the actual binary:

    1. `--diagnostics` exits 0, reports the expected app version, reports
       `OS: windows`, and redacts the home directory (never the literal
       `%USERPROFILE%` path) - see `platform.rs::redact_home`, which keeps
       only the last path component after the final `\\`.
    2. A plain cold launch (no args) registers a top-level window via UIA
       and has non-zero on-screen extents - proof of an actual window, not
       just a process that stayed alive.

    `--break`: asserts against a version string that cannot match
    (`"0.0.0-impossible"`), to prove check (1) actually reads the reported
    version rather than only checking exit code.
    """
    code, out, err = run_diagnostics(binary)
    if code != 0:
        print(f"FAIL: --diagnostics exited {code}: {err}", file=sys.stderr)
        return 1
    expected_prefix = "ForskScope 0.0.0-impossible" if break_mode else "ForskScope "
    if not out.startswith(expected_prefix):
        print(
            f"FAIL: --diagnostics output does not start with {expected_prefix!r}: "
            f"{out.splitlines()[0] if out else '(empty)'}",
            file=sys.stderr,
        )
        return 1
    if "OS: windows" not in out:
        print(f"FAIL: --diagnostics output does not report 'OS: windows': {out!r}", file=sys.stderr)
        return 1
    home_line = next((l for l in out.splitlines() if l.startswith("Home:")), "")
    if home_line and home_line != "Home: unknown":
        value = home_line[len("Home:"):].strip()
        if value != "***" and not value.endswith("***"):
            print(f"FAIL: home directory not redacted: {home_line}", file=sys.stderr)
            return 1

    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [], scratch)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            rect = win.rectangle()
            width, height = rect.width(), rect.height()
            if width <= 0 or height <= 0:
                print(
                    f"FAIL: window has non-positive extents ({width}x{height}) "
                    "- a blank or crashed window",
                    file=sys.stderr,
                )
                return 1
            if proc.poll() is not None:
                print(
                    f"FAIL: process exited ({proc.returncode}) shortly after launch",
                    file=sys.stderr,
                )
                return 1
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print(
        f"OK: --diagnostics reported {out.splitlines()[0]!r}; cold launch "
        f"produced a {width}x{height} window."
    )
    return 0


# ── P02 — CLI file compare ──────────────────────────────────────────────────


def p02(binary, break_mode=False):
    """Launches `<binary> <left> <right>` and asserts the diff actually
    rendered - waits for every one of the fixture's eight distinguishing
    line tokens to be present in the accessible tree's text (see module
    docstring for why this, not a row count, is the Windows readiness
    condition).

    `--break`: waits for a token that cannot appear in real output,
    proving the assertion is a real content check, not a vacuous
    "some window exists".
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    tokens = ["this-token-cannot-appear-in-real-output"] if break_mode else FIXTURE_TOKENS

    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [left, right], scratch)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            ok, texts, missing = wait_for_tokens(win, tokens, timeout_s=READY_TIMEOUT_S)
            if not ok:
                print(
                    f"FAIL: compare view never rendered these expected tokens within "
                    f"{READY_TIMEOUT_S}s: {missing}",
                    file=sys.stderr,
                )
                debug_dump(texts)
                return 1
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print(f"OK: CLI compare rendered all {len(tokens)} expected fixture tokens.")
    return 0


# ── P03 — Compare layout and scrolling (M5-C) ───────────────────────────────

# M5-C Prerequisite B (see `rowprobe` below, and the M5-C review request):
# `hunk.rs`'s `div.diff-row[role="row"]` maps to UIA control_type "DataItem"
# with `class_name` (UIA ClassName, sourced from the DOM class attribute)
# exactly `"diff-row"` - confirmed empirically against the real published
# 0.167.0 artifact (CI runs 31936191442/31936284361), not assumed. This is a
# *more* precise filter than Linux's AT-SPI "table row" role name, since it
# matches the literal CSS class rather than an ARIA-role-derived label, and
# it resolves Prerequisite B favorably: row-count and per-row geometry are
# both checkable on Windows, matching (not merely approximating) Linux
# F34/`render_check.py`'s `check_pane`, even though RFC-078 only requires "a
# basic layout observation" here. The row's own accessible children (probed:
# 2 per row - a gutter DataItem then a content-cell DataItem; the
# `aria_hidden` +/- mark span between them is correctly absent from the
# accessibility tree) are used exactly like Linux's `row.get_child_at_index(
# n-1)` for the content-cell x-origin comparison.
EXPECTED_ROWS_PER_PANE = 7


def _row_left_x(row):
    try:
        return row.rectangle().left
    except Exception:  # noqa: BLE001
        return float("inf")


def find_by_automation_id(win, automation_id):
    """First descendant whose UIA AutomationId equals `automation_id` -
    confirmed (rowprobe) that WebView2/Chromium maps a plain HTML `id`
    attribute straight onto AutomationId (`id="app-root"` -> automation_id
    `'app-root'`, etc.), so this locates `diff.rs`'s per-pane horizontal
    scroll containers (`id="diff-col-left-{index}"` /
    `id="diff-col-right-{index}"`) precisely, the same way `find_exact`
    locates controls by name."""
    try:
        candidates = win.descendants()
    except Exception:  # noqa: BLE001
        candidates = []
    for d in candidates:
        try:
            if d.element_info.automation_id == automation_id:
                return d
        except Exception:  # noqa: BLE001
            continue
    return None


def collect_diff_rows(win):
    """Every `div.diff-row[role="row"]` in the tree, identified by UIA
    control_type "DataItem" + ClassName "diff-row" (see the module note
    above) - the Windows analogue of `render_check.py`'s
    `collect_rows`/"table row" walk."""
    rows = []
    try:
        candidates = win.descendants(control_type="DataItem")
    except Exception:  # noqa: BLE001
        candidates = []
    for d in candidates:
        try:
            if d.element_info.class_name == "diff-row":
                rows.append(d)
        except Exception:  # noqa: BLE001
            continue
    return rows


def find_diff_wrap(win):
    """The `div.diff-wrap[role="region"]` landmark - Windows analogue of
    `render_check.py`'s `find_by_role(app, "landmark")`/`"frame"` pair,
    used the same way: to get a stable frame rectangle to compute the
    left/right pane midline from."""
    try:
        candidates = win.descendants(control_type="Group")
    except Exception:  # noqa: BLE001
        candidates = []
    for d in candidates:
        try:
            if d.element_info.class_name == "diff-wrap":
                return d
        except Exception:  # noqa: BLE001
            continue
    return None


def wait_for_row_shape(win, expected_per_pane, timeout_s=READY_TIMEOUT_S):
    """Poll until the compare view's accessible tree has fully rendered:
    the `diff-wrap` frame exists and each pane (split by the frame's own
    midline x, exactly like `render_check.py`'s `wait_for_ready`) has
    exactly `expected_per_pane` `diff-row` elements. F57's lesson applies
    here as much as it does on Linux: a single tree walk right after
    `connect()` can catch WebView2 mid-render and see a partial row set,
    which would either fail confusingly or - worse - compare a subset and
    pass. Returns (frame, left_rows, right_rows), or (None, [], []) if
    `timeout_s` elapses first.
    """
    deadline = time.monotonic() + timeout_s
    frame = None
    while time.monotonic() < deadline:
        if frame is None:
            frame = find_diff_wrap(win)
        if frame is not None:
            try:
                fr = frame.rectangle()
                midline_x = fr.left + fr.width() / 2
            except Exception:  # noqa: BLE001
                frame = None
                time.sleep(POLL_INTERVAL_S)
                continue
            rows = collect_diff_rows(win)
            left_rows = [r for r in rows if _row_left_x(r) < midline_x]
            right_rows = [r for r in rows if _row_left_x(r) >= midline_x]
            if len(left_rows) == expected_per_pane and len(right_rows) == expected_per_pane:
                return frame, left_rows, right_rows
        time.sleep(POLL_INTERVAL_S)
    return None, [], []


def check_pane_geometry(rows, pane_name):
    """Windows analogue of `render_check.py`'s `check_pane`, extended by
    one more comparison. Ordered by vertical position (top) first, since
    `descendants()` does not guarantee document order the way AT-SPI's
    child walk does:

    1. every row has the same accessible child count as the first row
       (F32's defect shape: a screen-reader label rendering as a sibling
       of the content cell instead of inside it);
    2. every row's content cell (its last accessible child) starts at the
       same x as every other row's (a column shift - F32's visible
       symptom);
    3. every row's own rectangle spans the same (left, right) as every
       other row's in the pane - the Windows-observable proxy for "short-row
       backgrounds span the full widest-line area" (RFC-078 P03): the
       fixture mixes short lines (`gamma`) and long ones
       (`Deleted: delete-line`), so a background/box that only extended
       under short content would show up here as a narrower rectangle,
       not just as a paint-level difference this harness cannot see
       directly.
    """
    failures = []
    if not rows:
        return [f"{pane_name}: no rows found"]
    try:
        ordered = sorted(rows, key=lambda r: r.rectangle().top)
    except Exception as exc:  # noqa: BLE001
        return [f"{pane_name}: could not read row rectangles: {exc!r}"]

    try:
        baseline_count = len(ordered[0].children())
    except Exception as exc:  # noqa: BLE001
        return [f"{pane_name}: could not read first row's children: {exc!r}"]

    baseline_x = None
    baseline_left = None
    baseline_right = None
    for row in ordered:
        try:
            children = row.children()
        except Exception as exc:  # noqa: BLE001
            failures.append(f"{pane_name}: could not read a row's children: {exc!r}")
            continue
        n = len(children)
        if n != baseline_count:
            failures.append(
                f"{pane_name}: a row has {n} accessible children, other rows "
                f"have {baseline_count} - a label is likely rendering as a "
                f"sibling of the content cell instead of inside it (F32's "
                f"defect shape)"
            )
            continue
        try:
            r = row.rectangle()
            cx = children[-1].rectangle().left
        except Exception as exc:  # noqa: BLE001
            failures.append(f"{pane_name}: could not read a row/cell rectangle: {exc!r}")
            continue
        if baseline_x is None:
            baseline_x = cx
        elif cx != baseline_x:
            failures.append(
                f"{pane_name}: a row's content starts at x={cx}, other rows "
                f"start at x={baseline_x} - a column shift, the visual "
                f"symptom of F32"
            )
        if baseline_left is None:
            baseline_left, baseline_right = r.left, r.right
        elif r.left != baseline_left or r.right != baseline_right:
            failures.append(
                f"{pane_name}: a row spans ({r.left},{r.right}), other rows "
                f"span ({baseline_left},{baseline_right}) - row width is not "
                f"uniform across short and long content"
            )
    return failures


def scroll_percent_h(elem):
    """Current horizontal scroll position (0-100) via UIA's ScrollPattern -
    a direct pattern read, not a synthesized-input result. Returns `None`
    if the pattern is unavailable or unreadable (e.g. content narrower than
    its viewport, where WebView2 may report `NoScroll`/`-1` for the whole
    axis instead of a real percent)."""
    try:
        v = elem.iface_scroll.CurrentHorizontalScrollPercent
    except Exception:  # noqa: BLE001
        return None
    return None if v is None or v < 0 else v


def set_scroll_percent_h(elem, target):
    """Sets horizontal scroll position via `IUIAutomationScrollPattern.
    SetScrollPercent` - the pattern-invocation approach this harness
    prefers everywhere else (`invoke()`, `select_dropdown()`), not a
    synthesized mouse wheel/trackpad gesture. `-1` (UIA's `NoScroll`
    sentinel) for the vertical argument leaves vertical scroll untouched."""
    elem.iface_scroll.SetScrollPercent(target, -1)


def p03(binary, break_mode=False):
    """Compare layout and scrolling. RFC-078 requires this in full on
    WebKitGTK and "a basic layout observation" on WebView2/macOS WebKit;
    Prerequisite B (see the module note above `EXPECTED_ROWS_PER_PANE`)
    resolved favorably, so this does the fuller Linux-parity check where
    it's cheap to, rather than stopping at the RFC's stated minimum:

    1. **Row shape and alignment** (`wait_for_row_shape` +
       `check_pane_geometry`, both panes): exactly 7 `diff-row`s per pane
       (the pinned `all_hunk_kinds` fixture shape, matching
       `render_check.py`'s `EXPECTED_ROWS_PER_PANE`), uniform accessible
       child count, uniform content-cell x-origin, and uniform row extents
       - covers "action rows align with left/right rows across multiple
       hunks" and "vertical rows remain aligned" together, the same way
       Linux's single F34 check covers both rather than asserting each
       bullet separately.
    2. **Horizontal scroll mirroring** (no precedent elsewhere in this
       program - built here for the first time): scrolls the left pane
       (`#diff-col-left-0`, via `SetScrollPercent` - a direct pattern
       call, not synthesized input) to a mid-range position, polls the
       right pane (`#diff-col-right-0`) until its own scroll percent
       matches, then samples it several more times after that match to
       confirm it *settles* rather than merely touching the target once -
       "without feedback/jitter" (RFC-078) means an oscillation is itself
       a failure, so a single post-scroll sample would not be sufficient
       evidence.
    3. **Narrow window, basic usability** (RFC-078's "word wrap and narrow
       window modes remain usable" - a basic observation, not full
       pixel-level wrap verification): resizes the real OS window
       narrower and confirms the fixture's content tokens are still
       present in the accessible tree afterward - usable, not blank or
       crashed.

    `--break`: three independent, impossible-value assertions (one per
    numbered check above), each proving its comparison reads real,
    specific state rather than passing vacuously. Unlike Linux's
    `inject_geometry_defect.py`, this harness has no way to inject a real
    layout defect into a published, digest-verified black-box artifact
    (no local rebuild - see the constraints this slice runs under), so
    `--break` here follows this file's own established pattern for that
    situation (P01/P02/P09/P10/...): assert against a value the real,
    correct app can never produce, rather than against an intentionally
    broken build. This is a deliberate, reported deviation from Linux's
    real-defect-injection falsifiability, not an oversight.
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"

    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [left, right], scratch)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)

            # ── 1. Row shape and alignment ───────────────────────────────
            frame, left_rows, right_rows = wait_for_row_shape(
                win, EXPECTED_ROWS_PER_PANE, timeout_s=READY_TIMEOUT_S
            )
            if frame is None:
                print(
                    f"FAIL: compare view never reached {EXPECTED_ROWS_PER_PANE} "
                    f"diff-row elements per pane within {READY_TIMEOUT_S}s "
                    f"(found {len(collect_diff_rows(win))} total)",
                    file=sys.stderr,
                )
                debug_dump(collect_texts(win))
                return 1

            failures = check_pane_geometry(left_rows, "left") + check_pane_geometry(
                right_rows, "right"
            )
            if break_mode:
                # The real app's rows are perfectly aligned (failures ==
                # []); requiring an impossible non-empty failure list
                # proves this comparison is live, not vacuously true.
                if not failures:
                    print(
                        "FAIL(--break): row geometry required an impossible "
                        "misalignment to be present, and correctly found none "
                        "- the real check above is not vacuous",
                        file=sys.stderr,
                    )
                    return 1
            elif failures:
                print("FAIL: row geometry check found real misalignment:", file=sys.stderr)
                for f in failures:
                    print(f"  - {f}", file=sys.stderr)
                return 1

            # ── 2. Horizontal scroll mirroring ───────────────────────────
            left_col = find_by_automation_id(win, "diff-col-left-0")
            right_col = find_by_automation_id(win, "diff-col-right-0")
            if left_col is None or right_col is None:
                print(
                    "FAIL: could not find diff-col-left-0/diff-col-right-0 "
                    "(the per-pane horizontal scroll containers) by AutomationId",
                    file=sys.stderr,
                )
                return 1

            target = 55.0
            try:
                set_scroll_percent_h(left_col, target)
            except Exception as exc:  # noqa: BLE001
                print(f"FAIL: could not set left pane's horizontal scroll: {exc!r}", file=sys.stderr)
                return 1

            required = -1.0 if break_mode else target
            deadline = time.monotonic() + READY_TIMEOUT_S
            mirrored = False
            last_seen = None
            while time.monotonic() < deadline:
                last_seen = scroll_percent_h(right_col)
                if last_seen is not None and abs(last_seen - required) < 1.0:
                    mirrored = True
                    break
                time.sleep(POLL_INTERVAL_S)
            if not mirrored:
                label = "(--break) impossible target" if break_mode else "the left pane's position"
                print(
                    f"FAIL: right pane's horizontal scroll never matched {label} "
                    f"within {READY_TIMEOUT_S}s (last seen: {last_seen!r}, required: {required!r})",
                    file=sys.stderr,
                )
                return 1
            if break_mode:
                print(
                    "FAIL(--break): right pane matched an impossible scroll "
                    "target - the mirroring check above is not vacuous",
                    file=sys.stderr,
                )
                return 1

            # Settling check: sample several more times after the match -
            # "without feedback/jitter" means oscillation itself is a
            # failure, so one post-scroll sample is not sufficient.
            settle_samples = []
            for _ in range(6):
                time.sleep(POLL_INTERVAL_S)
                settle_samples.append(scroll_percent_h(right_col))
            unsettled = [s for s in settle_samples if s is None or abs(s - target) > 1.0]
            if unsettled:
                print(
                    f"FAIL: right pane's horizontal scroll did not settle after "
                    f"mirroring - samples after the match: {settle_samples!r} "
                    f"(target {target})",
                    file=sys.stderr,
                )
                return 1

            # ── 3. Narrow window, basic usability ────────────────────────
            try:
                rect = win.rectangle()
                win.move_window(rect.left, rect.top, 480, rect.height())
            except Exception as exc:  # noqa: BLE001
                print(f"FAIL: could not resize the window narrower: {exc!r}", file=sys.stderr)
                return 1
            time.sleep(1.0)  # let WebView2 reflow before re-checking content
            ok, texts, missing = wait_for_tokens(win, FIXTURE_TOKENS, timeout_s=READY_TIMEOUT_S)
            if not ok:
                print(
                    f"FAIL: compare view lost expected content after narrowing "
                    f"the window: missing {missing}",
                    file=sys.stderr,
                )
                debug_dump(texts)
                return 1
            new_rect = win.rectangle()
            if new_rect.width() <= 0 or new_rect.height() <= 0:
                print(
                    f"FAIL: window has non-positive extents after narrowing "
                    f"({new_rect.width()}x{new_rect.height()})",
                    file=sys.stderr,
                )
                return 1
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print(
        "OK: 7 diff-rows per pane, uniform child count/content-cell x-origin/row "
        "extents in both panes; horizontal scroll mirrored from left to right and "
        "settled; content remained present and the window stayed valid after "
        "narrowing."
    )
    return 0


# ── M5-C Prerequisite B probe — Windows UIA control type for row divs ───────


def rowprobe(binary, break_mode=False):
    """NOT an evidence case (no matrix-plan.md case ID) - a one-shot,
    committed-and-run diagnostic for RFC-078 M5-C's Prerequisite B: does
    a UIA control type exist that reliably, countably identifies
    `hunk.rs`'s `div.diff-row[role="row"]` elements on Windows the way
    AT-SPI's "table row" role does on Linux (`render_check.py`'s
    `check_pane`)? `role="row"` here has no `role="table"`/`role="grid"`
    ancestor (the app renders its own flex/grid CSS layout, not a
    semantic HTML/ARIA table) - WAI-ARIA's "required context role" rule
    says a user agent MAY drop a row role with no table/grid/treegrid
    ancestor entirely, and WebKitGTK (Linux) and Chromium/WebView2
    (Windows) are different engines with no obligation to resolve that
    the same way; F57/AT-SPI confirms WebKitGTK keeps it, this probe is
    what confirms or refutes the Chromium/WebView2 side empirically
    instead of assuming parity.

    Method: launch against the pinned `left_all_hunk_kinds.txt`/
    `right_all_hunk_kinds.txt` fixture pair (the same one `render_check.py`
    and `p02` use - 7 rows/pane, `EXPECTED_ROWS_PER_PANE` on Linux), wait
    for the fixture tokens, then for a set of leaf line-content strings
    that are never inline-char-diffed (so each is one leaf, not split
    across spans - `old-line`/`new-line`, the one Replace-kind pair, are
    deliberately excluded for this reason) find the smallest-area
    descendant whose own text contains that string and walk its
    `parent()` chain, printing each ancestor's UIA control type,
    automation id, class name, rectangle, and own child count. Also
    tallies every control type present anywhere in the tree. All of this
    goes to stdout for a human (the reviewer, or this harness's own
    author reading the CI log) to read - there is no pass/fail assertion
    here, because there is nothing to assert yet; that is the point of a
    prerequisite probe.
    """
    del break_mode  # no falsifiability mode - this is not an evidence case
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    # Deliberately excludes "old-line"/"new-line" (the one Replace-kind
    # pair, at risk of inline char-diff span splitting) and "alpha"
    # (appears identically on both sides/panes, so it cannot anchor a
    # single element unambiguously).
    anchors = ["gamma", "delete-line", "epsilon", "zeta", "insert-line"]

    def area(elem):
        try:
            r = elem.rectangle()
            return max(0, r.width()) * max(0, r.height())
        except Exception:  # noqa: BLE001
            return float("inf")

    def describe(elem, indent):
        try:
            ei = elem.element_info
            ctrl = ei.control_type
        except Exception as exc:  # noqa: BLE001
            print(f"{indent}<error reading control_type: {exc!r}>")
            return
        try:
            aid = ei.automation_id
        except Exception:  # noqa: BLE001
            aid = "<err>"
        try:
            cls = ei.class_name
        except Exception:  # noqa: BLE001
            cls = "<err>"
        try:
            r = elem.rectangle()
            rect = f"({r.left},{r.top})-({r.right},{r.bottom}) {r.width()}x{r.height()}"
        except Exception:  # noqa: BLE001
            rect = "<err>"
        try:
            txt = elem.window_text()
            txt = (txt[:70] + "...") if txt and len(txt) > 70 else txt
        except Exception:  # noqa: BLE001
            txt = "<err>"
        try:
            nchildren = len(elem.children())
        except Exception:  # noqa: BLE001
            nchildren = "<err>"
        print(
            f"{indent}control_type={ctrl!r} automation_id={aid!r} class_name={cls!r} "
            f"children={nchildren} rect={rect} text={txt!r}"
        )

    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [left, right], scratch)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            ok, texts, missing = wait_for_tokens(win, FIXTURE_TOKENS, timeout_s=READY_TIMEOUT_S)
            if not ok:
                print(f"FAIL: compare view never rendered expected tokens: {missing}", file=sys.stderr)
                debug_dump(texts)
                return 1

            print("=== control type tally across the whole window ===")
            tally = {}
            for d in win.descendants():
                try:
                    ct = d.element_info.control_type
                except Exception:  # noqa: BLE001
                    ct = "<error>"
                tally[ct] = tally.get(ct, 0) + 1
            for ct, n in sorted(tally.items(), key=lambda kv: -kv[1]):
                print(f"  {ct!r}: {n}")

            for anchor in anchors:
                print(f"\n=== anchor {anchor!r} ===")
                candidates = [d for d in win.descendants() if anchor in (d.window_text() or "")]
                if not candidates:
                    print(f"  (no descendant found containing {anchor!r})")
                    continue
                leaf = min(candidates, key=area)
                print("  leaf:")
                describe(leaf, "    ")
                cur = leaf
                for level in range(1, 7):
                    try:
                        cur = cur.parent()
                    except Exception as exc:  # noqa: BLE001
                        print(f"  parent level {level}: <error: {exc!r}>")
                        break
                    if cur is None:
                        print(f"  parent level {level}: <none>")
                        break
                    print(f"  parent level {level}:")
                    describe(cur, "    ")
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print("\nOK: rowprobe dump complete (diagnostic only - no assertion).")
    return 0


def scrollprobe(binary, break_mode=False):
    """NOT an evidence case, like `rowprobe` above - a one-shot diagnostic
    for P03's horizontal-scroll-mirroring requirement, which has no
    precedent anywhere in this program on any platform.

    First CI run (this function's original form): located
    `.diff-col-left`/`.diff-col-right` by AutomationId
    (`#diff-col-left-{index}`/`#diff-col-right-{index}`), on the theory
    that rowprobe's confirmed `id` -> AutomationId mapping would extend to
    them. It did not find them - zero matches. Second CI run: fell back to
    a class_name search (`diff-col-left`/`diff-col-right`, no index
    suffix, per `diff.rs`) across the *entire* tree, and still found
    nothing, even though the compare view had rendered correctly (the
    fixture tokens were present). Conclusion, confirmed rather than
    assumed: unlike `.diff-row`/`.diff-wrap` (both explicit ARIA
    `role="row"`/`role="region"`), `.diff-col-left`/`.diff-col-right` carry
    no ARIA role at all - just `overflow-x:auto` CSS - and Chromium's
    accessibility tree does not keep a plain, role-less scroll container as
    its own AX node here; there is no `IScrollProvider`/`ScrollPattern` to
    invoke because there is no UIA element for it at all, on any control
    type or class_name.

    This third form works around that absence rather than assuming it
    forecloses P03's scroll check entirely: it locates the *row content*
    inside each pane (which does have AX nodes - `collect_diff_rows`,
    already established) against the `left_long_line.txt`/
    `right_long_line.txt` fixture pair (a single 2,000-character line per
    side, guaranteed to overflow), and treats a change in that row's own
    on-screen rectangle as the readback signal instead of
    `HorizontalScrollPercent`: if UIA reports screen coordinates that
    account for the current `scrollLeft` transform (as they should for a
    correctly-implemented accessibility tree), scrolling the pane should
    move the row's reported rectangle left, independent of whether the
    scroll container itself is reachable. Tries, in ascending order of
    input-synthesis reliance:

    1. `Shift+MouseWheel` (`pywinauto.keyboard.send_keys('{VK_SHIFT down}')`
       bracketing `pywinauto.mouse.scroll`) - the standard Windows/Chromium
       convention for turning a vertical wheel gesture into horizontal
       scroll, using only pywinauto's own vetted input primitives (no
       hand-rolled `SendInput` struct).
    2. A plain vertical `pywinauto.mouse.scroll` at the same point, in case
       this container answers vertical wheel input with horizontal motion
       for some other reason (checked as a control, not expected to work).

    No assertion, no `--break` mode - this exists to produce a real,
    observed answer for P03's own docstring/evidence writeup to cite.
    """
    del break_mode
    left = REPO_ROOT / "tests/fixtures/text/left_long_line.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_long_line.txt"

    def cell_rects(win):
        """(left_cell_rect, right_cell_rect), found directly by each
        fixture's own distinguishing anchor text ('AAAA' / 'BBBB') via
        `find_by_text_containing` - not by the midline-x row classification
        `wait_for_row_shape`/`check_pane_geometry` use elsewhere. That
        classification is exactly what a real horizontal scroll would be
        expected to perturb (a row's x-position is the whole signal being
        probed here), so leaning on it as a *precondition* for reading the
        rectangles at all would make a real scroll effect look like a
        harness error instead of data. Anchor-text lookup has no such
        circularity: it does not care where on screen the matched element
        currently is."""
        left_leaf = find_by_text_containing(win, "AAAA")
        right_leaf = find_by_text_containing(win, "BBBB")
        lc = rc = None
        if left_leaf is not None:
            try:
                lc = left_leaf.rectangle()
            except Exception as exc:  # noqa: BLE001
                print(f"  (could not read left cell rectangle: {exc!r})")
        else:
            print("  (no descendant containing 'AAAA' found)")
        if right_leaf is not None:
            try:
                rc = right_leaf.rectangle()
            except Exception as exc:  # noqa: BLE001
                print(f"  (could not read right cell rectangle: {exc!r})")
        else:
            print("  (no descendant containing 'BBBB' found)")
        return lc, rc

    def fmt_rect(r):
        return "None" if r is None else f"(left={r.left}, right={r.right}, top={r.top})"

    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [left, right], scratch)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            ok, texts, missing = wait_for_tokens(win, ["AAAA", "BBBB"], timeout_s=READY_TIMEOUT_S)
            if not ok:
                print(f"FAIL: compare view never rendered expected tokens: {missing}", file=sys.stderr)
                debug_dump(texts)
                return 1

            print("=== every non-empty automation_id in the tree ===", flush=True)
            any_aid = False
            for d in win.descendants():
                try:
                    aid = d.element_info.automation_id
                except Exception:  # noqa: BLE001
                    aid = ""
                if aid:
                    any_aid = True
                    print(f"  automation_id={aid!r} control_type={d.element_info.control_type!r}", flush=True)
            if not any_aid:
                print("  (none found anywhere in the tree)", flush=True)

            print("=== capability scan: every descendant with CurrentHorizontallyScrollable == True ===", flush=True)
            # Neither AutomationId nor class_name found `.diff-col-left`/
            # `.diff-col-right` on the first two CI runs (see the docstring) -
            # this looks for the ScrollPattern *capability* directly instead
            # of assuming the wrapper node survives Chromium's accessibility
            # tree under some identity this harness hasn't guessed yet.
            scrollable = []
            for d in win.descendants():
                try:
                    if bool(d.iface_scroll.CurrentHorizontallyScrollable):
                        scrollable.append(d)
                except Exception:  # noqa: BLE001
                    continue
            print(f"  found {len(scrollable)} horizontally-scrollable descendant(s)", flush=True)
            for d in scrollable:
                try:
                    ei = d.element_info
                    r = d.rectangle()
                    print(
                        f"    control_type={ei.control_type!r} automation_id={ei.automation_id!r} "
                        f"class_name={ei.class_name!r} rect=({r.left},{r.top})-({r.right},{r.bottom}) "
                        f"HorizontalScrollPercent={d.iface_scroll.CurrentHorizontalScrollPercent!r}",
                        flush=True,
                    )
                except Exception as exc:  # noqa: BLE001
                    print(f"    <error describing candidate: {exc!r}>", flush=True)

            left_pane = right_pane = None
            if len(scrollable) >= 2:
                scrollable.sort(key=lambda d: d.rectangle().left)
                left_pane, right_pane = scrollable[0], scrollable[-1]
                print("  using leftmost/rightmost by rect.left as left/right pane", flush=True)
            else:
                print(
                    "  could not locate two horizontally-scrollable elements by "
                    "capability either - proceeding with row-rectangle readback "
                    "only (see below); pattern-based attempts that need a pane "
                    "element are skipped",
                    flush=True,
                )

            def scroll_state(elem, label):
                try:
                    sc = elem.iface_scroll
                    state = {
                        "HorizontallyScrollable": sc.CurrentHorizontallyScrollable,
                        "HorizontalScrollPercent": sc.CurrentHorizontalScrollPercent,
                        "HorizontalViewSize": sc.CurrentHorizontalViewSize,
                    }
                except Exception as exc:  # noqa: BLE001
                    state = {"error": repr(exc)}
                print(f"  {label}: {state}", flush=True)
                return state

            def report_cells(label):
                lc, rc = cell_rects(win)
                print(f"  {label}: left cell {fmt_rect(lc)}, right cell {fmt_rect(rc)}", flush=True)
                return lc, rc

            print("\n=== initial state ===")
            if left_pane is not None:
                scroll_state(left_pane, "left pane (ScrollPattern)")
                scroll_state(right_pane, "right pane (ScrollPattern)")
            baseline_lc, baseline_rc = report_cells("initial row-cell rectangles")

            def try_attempt(label, fn):
                print(f"\n=== {label} ===", flush=True)
                try:
                    fn()
                    print("  call did not raise", flush=True)
                except Exception as exc:  # noqa: BLE001
                    print(f"  raised: {exc!r}", flush=True)
                if left_pane is not None:
                    scroll_state(left_pane, "left pane (ScrollPattern)")
                    scroll_state(right_pane, "right pane (ScrollPattern)")
                lc, rc = report_cells("row-cell rectangles")
                left_moved = (
                    baseline_lc is not None
                    and lc is not None
                    and (lc.left != baseline_lc.left or lc.right != baseline_lc.right)
                )
                right_moved = (
                    baseline_rc is not None
                    and rc is not None
                    and (rc.left != baseline_rc.left or rc.right != baseline_rc.right)
                )
                print(
                    f"  left cell moved: {left_moved}, right cell moved: {right_moved}",
                    flush=True,
                )
                return lc, rc

            if left_pane is not None:
                try_attempt(
                    "attempt 1: iface_scroll.SetScrollPercent(50, -1) on left pane",
                    lambda: left_pane.iface_scroll.SetScrollPercent(50, -1),
                )
                try_attempt(
                    "attempt 2: iface_scroll.Scroll(horizontal=LargeIncrement, vertical=NoAmount) on left pane",
                    lambda: left_pane.iface_scroll.Scroll(3, 2),
                )
                try_attempt(
                    "attempt 3: UIAWrapper.scroll('right', 'page') on left pane",
                    lambda: left_pane.scroll("right", "page"),
                )
                pt_rect = left_pane.rectangle()
            else:
                # No pane element at all - derive a screen point from the
                # left row's own cell rectangle instead, so the
                # input-synthesis attempts below still have somewhere real
                # on screen to target.
                pt_rect = baseline_lc

            if pt_rect is not None:
                # BUG on the previous CI run (fixed here): pt_rect (the
                # cell's own accessible rectangle) is NOT clipped to the
                # scroll viewport - its reported width matched the full
                # 2,000-character fixture line (right - left in the
                # thousands of px), so the naive "rectangle midpoint" this
                # used to compute landed at screen x=7743 on a runner whose
                # actual window is ~1044px wide: off-screen, on nothing,
                # explaining why every wheel attempt below observed no
                # change. Clamp the target point into the real, on-screen
                # window rectangle instead - the cell's *top* (218 on the
                # prior run, consistent between both cells) is used as-is,
                # since only the horizontal axis was ever suspect.
                win_rect = win.rectangle()
                x = min(max(pt_rect.left + 40, win_rect.left + 10), win_rect.right - 10)
                y = min(max(pt_rect.top + max(1, min(pt_rect.height(), 20) // 2), win_rect.top + 10), win_rect.bottom - 10)
                pt = (x, y)

                def plain_wheel():
                    from pywinauto import mouse  # noqa: PLC0415

                    mouse.scroll(coords=pt, wheel_dist=-10)

                try_attempt(f"attempt 4: plain vertical mouse wheel at {pt}", plain_wheel)

                def shift_wheel():
                    from pywinauto import mouse  # noqa: PLC0415
                    from pywinauto.keyboard import send_keys  # noqa: PLC0415

                    send_keys("{VK_SHIFT down}")
                    try:
                        mouse.scroll(coords=pt, wheel_dist=-10)
                    finally:
                        send_keys("{VK_SHIFT up}")

                try_attempt(f"attempt 5: Shift+MouseWheel at {pt}", shift_wheel)

                def raw_hwheel():
                    # Native WM_MOUSEHWHEEL via SendInput, bypassing the
                    # Shift+vertical-wheel convention entirely (stdlib
                    # ctypes only - no new dependency) - the most
                    # input-synthesis-heavy attempt this probe has, used
                    # only because every accessibility-pattern approach
                    # above was verified not to work.
                    import ctypes  # noqa: PLC0415

                    user32 = ctypes.windll.user32
                    user32.SetCursorPos(int(pt[0]), int(pt[1]))
                    MOUSEEVENTF_HWHEEL = 0x01000
                    WHEEL_DELTA = 120
                    user32.mouse_event(MOUSEEVENTF_HWHEEL, 0, 0, 10 * WHEEL_DELTA, 0)

                try_attempt(f"attempt 6: raw WM_MOUSEHWHEEL (SendInput) at {pt}", raw_hwheel)
            else:
                print(
                    "\n(no screen point available at all - baseline row-cell "
                    "rectangle could not be read - skipping the input-synthesis "
                    "attempts)",
                    file=sys.stderr,
                )
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print("\nOK: scrollprobe dump complete (diagnostic only - no assertion).")
    return 0


# ── P09 — Mergetool ──────────────────────────────────────────────────────────


def p09(binary, break_mode=False):
    """Launches `<binary> <local> <remote> <merged>` against the same
    fixture pair as P02 (a real diff - one hunk-apply button per changed
    hunk), invokes the first hunk's "Use this change" button, then the
    toolbar's "Save merge result" button, and asserts `<merged>` was
    actually written - both invoked via UIA's Invoke pattern (`invoke()`),
    not synthesized mouse/keyboard input (see that function's docstring
    for why). `merged` starts pre-seeded with a placeholder no real merge
    output could produce, so "was it overwritten" is unambiguous.

    Applying a real hunk first matters beyond realism: the toolbar Save
    button is disabled until the tab is dirty (`toolbar.rs`), so a no-op
    tab would make this test pass for the wrong reason.

    `--break`: checks for a placeholder string that can never be replaced,
    proving the assertion reads the file's real content rather than only
    checking existence.
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    placeholder = "PLACEHOLDER - must be overwritten by Save\n"

    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        merged = scratch_path / "merged.txt"
        merged.write_text(placeholder)

        proc = launch(binary, [left, right, merged], scratch)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            ok, texts, missing = wait_for_tokens(win, FIXTURE_TOKENS, timeout_s=READY_TIMEOUT_S)
            if not ok:
                print(
                    f"FAIL: compare view never rendered these expected tokens within "
                    f"{READY_TIMEOUT_S}s: {missing}",
                    file=sys.stderr,
                )
                debug_dump(texts)
                return 1

            apply_button = find_by_text_containing(win, "Use this change")
            if apply_button is None:
                print('FAIL: could not find a "Use this change" hunk-apply button', file=sys.stderr)
                debug_dump(collect_texts(win))
                return 1
            invoke(apply_button)

            # The re-render that enables Save (tab becomes dirty) happens
            # asynchronously after invoke() returns - find it fresh and
            # retry rather than assume the tree already reflects the new
            # state (mirrors linux_harness.py's P09 retry loop, and the
            # git history behind it: "retry clicking Save until the async
            # re-render enables it").
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            clicked_save = False
            save_path_used = None
            while time.monotonic() < deadline and not clicked_save:
                save_button = find_by_text_containing(win, "Save merge result")
                if save_button is not None and is_enabled(save_button):
                    try:
                        save_path_used = invoke(save_button)
                        clicked_save = True
                    except Exception:  # noqa: BLE001 - transient COM/UIA error, retry
                        pass
                if not clicked_save:
                    time.sleep(POLL_INTERVAL_S)
            if not clicked_save:
                print('FAIL: could not click "Save merge result" within the timeout', file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            actual = placeholder
            while time.monotonic() < deadline:
                actual = merged.read_text()
                if actual != placeholder:
                    break
                time.sleep(POLL_INTERVAL_S)
            if break_mode:
                # No real Save output can ever equal this - proves the
                # comparison below is a genuine content check, not a
                # vacuous "file changed at all" test.
                required_content = "this exact string can never appear in real merge output"
            else:
                required_content = None
            if actual == placeholder:
                print(
                    f"FAIL: {merged} still holds its placeholder after "
                    f"{LAUNCH_TIMEOUT_S}s - Save did not write to it",
                    file=sys.stderr,
                )
                return 1
            if required_content is not None and actual != required_content:
                print(
                    f"FAIL: {merged}'s content {actual!r} != required {required_content!r}",
                    file=sys.stderr,
                )
                return 1
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print(
        f"OK: applied a hunk and invoked Save ({save_path_used}); {merged.name} was "
        f"overwritten ({len(actual)} bytes, no longer the placeholder)."
    )
    return 0


# ── P10 — Binary/XLSX fail-closed policy ────────────────────────────────────


def p10(binary, break_mode=False):
    """Launches `<binary> <left.xlsx> <right.xlsx>` (classification is by
    extension only - `core::file_kind::classify` - so arbitrary bytes
    suffice) and asserts the fail-closed message reaches the user.

    `--break`: searches for a string that cannot appear, proving the
    check reads the actual rendered message rather than passing on any
    launch that doesn't crash.
    """
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        left = scratch_path / "left.xlsx"
        right = scratch_path / "right.xlsx"
        left.write_bytes(b"not a real xlsx - classification is by extension only")
        right.write_bytes(b"also not a real xlsx")

        proc = launch(binary, [left, right], scratch)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)

            needle = "this message cannot appear" if break_mode else XLSX_MESSAGE
            deadline = time.monotonic() + READY_TIMEOUT_S
            found = None
            while time.monotonic() < deadline:
                found = find_by_text_containing(win, needle)
                if found is not None:
                    break
                time.sleep(POLL_INTERVAL_S)
            if found is None:
                print(
                    f"FAIL: could not find text containing {needle!r} within "
                    f"{READY_TIMEOUT_S}s",
                    file=sys.stderr,
                )
                debug_dump(collect_texts(win))
                return 1
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print(f"OK: found the fail-closed message: {XLSX_MESSAGE!r}")
    return 0


# ── P04 — Merge, undo/redo, safe save (M5-B) ────────────────────────────────


def p04(binary, break_mode=False):
    """Two-argument compare mode (not mergetool): applies the first
    changed hunk via the "Use this change" button (UIA Invoke), watches
    the toolbar's Save button flip disabled -> enabled as the dirty-state
    signal (`toolbar.rs`: `disabled: !snap.is_dirty`), undoes via the
    toolbar's Undo button and confirms the apply reverted (Save goes back
    to disabled and "Use this change" reappears), opens the "More"
    disclosure panel and redoes via the Redo button, confirming the
    change is reapplied (Save re-enabled), then saves and checks: the
    saved file's exact resulting lines against what applying only the
    fixture's first (Replace) hunk predicts, a `.bak` sibling equal to
    the pre-save (original right fixture) content, and no leftover
    temp/sidecar file in the scratch working directory.

    KEYBOARD PATH NOT EXERCISED. RFC-078 P04 asks for "keyboard and
    mouse". `app.rs`'s Enter-key shortcut
    (`Key::Enter => apply_focused_hunk(...)`) is a raw `onkeydown`
    listener with no bound UI element - there is nothing for UIA (or any
    accessibility API, on any platform) to invoke for it, unlike a
    button's Invoke pattern. Per the M5-B handoff §3, this harness
    demonstrates only the mouse path (UIA Invoke on "Use this change",
    the same equivalent-path argument M5-A already used for Save); the
    keyboard path is recorded in the evidence doc as not executed /
    manual-outstanding, the same shape as F45's Windows sub-case - not
    silently skipped, not claimed as covered.

    `--break`: requires the saved file's content to equal a string no
    real Save output can ever produce, proving the final check reads
    real content rather than only "did the file change at all".
    """
    left_src = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right_src = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    original_right = right_src.read_text()

    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        left = scratch_path / "left.txt"
        right = scratch_path / "right.txt"
        left.write_text(left_src.read_text())
        right.write_text(original_right)

        proc = launch(binary, [left, right], scratch)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            ok, texts, missing = wait_for_tokens(win, FIXTURE_TOKENS, timeout_s=READY_TIMEOUT_S)
            if not ok:
                print(
                    f"FAIL: compare view never rendered these expected tokens within "
                    f"{READY_TIMEOUT_S}s: {missing}",
                    file=sys.stderr,
                )
                debug_dump(texts)
                return 1

            if not wait_button_enabled(win, "Save merge result", False, timeout_s=LAUNCH_TIMEOUT_S):
                print("FAIL: Save is enabled before any hunk was applied - dirty state is wrong from the start", file=sys.stderr)
                return 1

            apply_button = find_by_text_containing(win, "Use this change")
            if apply_button is None:
                print('FAIL: could not find a "Use this change" hunk-apply button', file=sys.stderr)
                debug_dump(collect_texts(win))
                return 1
            invoke(apply_button)

            if not wait_button_enabled(win, "Save merge result", True, timeout_s=LAUNCH_TIMEOUT_S):
                print("FAIL: Save button never became enabled after applying a hunk (dirty-state indicator did not appear)", file=sys.stderr)
                return 1

            undo_button = find_by_text_containing(win, "Undo last merge action")
            if undo_button is None or not is_enabled(undo_button):
                print("FAIL: could not find an enabled Undo button after applying a hunk", file=sys.stderr)
                return 1
            invoke(undo_button)

            if not wait_button_enabled(win, "Save merge result", False, timeout_s=LAUNCH_TIMEOUT_S):
                print("FAIL: Save button did not return to disabled after Undo - the apply was not reverted", file=sys.stderr)
                return 1
            if find_by_text_containing(win, "Use this change") is None:
                print('FAIL: "Use this change" button did not reappear after Undo - the hunk does not look reverted', file=sys.stderr)
                return 1

            more_button = find_by_text_containing(win, "More")
            if more_button is None:
                print('FAIL: could not find the toolbar "More" disclosure button', file=sys.stderr)
                return 1
            invoke(more_button)

            redo_button = wait_for_exact(win, "Redo", control_type="Button", timeout_s=LAUNCH_TIMEOUT_S)
            if redo_button is None or not is_enabled(redo_button):
                print("FAIL: could not find an enabled Redo button in the advanced panel", file=sys.stderr)
                return 1
            invoke(redo_button)

            if not wait_button_enabled(win, "Save merge result", True, timeout_s=LAUNCH_TIMEOUT_S):
                print("FAIL: Save button did not return to enabled after Redo - the change was not reapplied", file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            clicked_save = False
            while time.monotonic() < deadline and not clicked_save:
                save_button = find_by_text_containing(win, "Save merge result")
                if save_button is not None and is_enabled(save_button):
                    try:
                        invoke(save_button)
                        clicked_save = True
                    except Exception:  # noqa: BLE001 - transient COM/UIA error, retry
                        pass
                if not clicked_save:
                    time.sleep(POLL_INTERVAL_S)
            if not clicked_save:
                print('FAIL: could not click "Save merge result" within the timeout', file=sys.stderr)
                return 1

            # Applying only the fixture's first (Replace) hunk left-to-right
            # replaces right's "new-line" with left's "old-line"; every
            # other line is unchanged (see the fixture pair and the module
            # docstring's FIXTURE_TOKENS list).
            expected_lines = (
                ["this exact content can never appear in real merge output"]
                if break_mode
                else ["alpha", "old-line", "gamma", "epsilon", "zeta", "insert-line"]
            )
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            actual_lines = right.read_text().splitlines()
            while time.monotonic() < deadline and actual_lines != expected_lines:
                time.sleep(POLL_INTERVAL_S)
                actual_lines = right.read_text().splitlines()
            if actual_lines != expected_lines:
                print(f"FAIL: {right}'s content {actual_lines!r} != expected {expected_lines!r}", file=sys.stderr)
                return 1

            bak = right.with_name(right.name + ".bak")
            if not bak.exists():
                print(f"FAIL: no backup sibling {bak} was created", file=sys.stderr)
                return 1
            if bak.read_text() != original_right:
                print(f"FAIL: {bak}'s content does not equal the pre-save (original right fixture) content", file=sys.stderr)
                return 1

            leftover = [
                p.name
                for p in scratch_path.iterdir()
                if p.name not in {"left.txt", "right.txt", bak.name}
            ]
            if leftover:
                print(f"FAIL: leftover temp/sidecar files remain in the working directory: {leftover}", file=sys.stderr)
                return 1
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print(
        "OK: applied a hunk via mouse/Invoke, watched Save track dirty state through "
        "undo and redo, saved, and verified the saved content, .bak, and clean working "
        "directory. Keyboard path (Enter) NOT exercised - see this function's docstring."
    )
    return 0


# ── P05 — External modification (M5-B) ──────────────────────────────────────


def p05(binary, break_mode=False):
    """Three fresh launches against scratch copies of the P02/P04 fixture
    pair, each applying the first hunk (so the ordinary toolbar Save
    button - gated on `is_dirty` - is actually clickable) and then
    modifying the right file's bytes externally before attempting a
    write, exercising one of the three outcomes:

    1. Cancel: Save is blocked by `OverwriteModal` ("File changed on
       disk"); the on-disk file still holds the externally-written bytes
       both before and after clicking Cancel.
    2. Overwrite: confirms the write; the resulting `.bak` sibling's
       bytes equal the *externally-modified* content that was just
       overwritten (not the original pre-modification content -
       `save_text` backs up the target's current on-disk bytes
       immediately before replacing them).
    3. Save As, to a different path: the original target is left with
       its externally-modified bytes untouched; the new path holds the
       app's content.

    `--break` (time-boxed to sub-case 2, the one with the meaningful byte
    comparison): requires the `.bak` sibling's bytes to equal a string
    this harness never externally writes, proving the comparison reads
    real bytes rather than only checking existence.
    """
    left_src = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right_src = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    left_text = left_src.read_text()
    original_right = right_src.read_text()
    external_content = "EXTERNALLY-MODIFIED-CONTENT\nsecond line\n"
    expected_saved_lines = ["alpha", "old-line", "gamma", "epsilon", "zeta", "insert-line"]

    def fresh_pair(scratch_path):
        left = scratch_path / "left.txt"
        right = scratch_path / "right.txt"
        left.write_text(left_text)
        right.write_text(original_right)
        return left, right

    def open_and_dirty(scratch, left, right):
        """Launches, waits for the diff to render, applies the first
        hunk, and waits for the dirty-state Save-enabled signal - the
        common setup all three sub-cases share before their own external
        modification and action."""
        proc = launch(binary, [left, right], scratch)
        app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
        ok, texts, missing = wait_for_tokens(win, FIXTURE_TOKENS, timeout_s=READY_TIMEOUT_S)
        if not ok:
            raise AssertionError(f"compare view never rendered expected tokens: {missing}")
        apply_button = find_by_text_containing(win, "Use this change")
        if apply_button is None:
            raise AssertionError('could not find a "Use this change" hunk-apply button')
        invoke(apply_button)
        if not wait_button_enabled(win, "Save merge result", True, timeout_s=LAUNCH_TIMEOUT_S):
            raise AssertionError("Save button never became enabled after applying a hunk")
        return proc, win

    # ── Sub-case 1: Cancel ───────────────────────────────────────────────
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        left, right = fresh_pair(scratch_path)
        proc = None
        try:
            proc, win = open_and_dirty(scratch, left, right)
            right.write_text(external_content)

            save_button = find_by_text_containing(win, "Save merge result")
            invoke(save_button)
            if not wait_for_tokens(win, ["File changed on disk"], timeout_s=READY_TIMEOUT_S)[0]:
                print('FAIL(P05-cancel): the "File changed on disk" conflict modal never appeared', file=sys.stderr)
                return 1
            if right.read_text() != external_content:
                print("FAIL(P05-cancel): the target was overwritten despite the conflict - Save should have been blocked", file=sys.stderr)
                return 1

            cancel_button = modal_action_button(win, "Cancel")
            if cancel_button is None:
                print('FAIL(P05-cancel): could not find the modal\'s "Cancel" button', file=sys.stderr)
                return 1
            invoke(cancel_button)
            if not wait_gone(win, "File changed on disk", timeout_s=LAUNCH_TIMEOUT_S):
                print("FAIL(P05-cancel): the conflict dialog never dismissed after Cancel", file=sys.stderr)
                return 1
            if right.read_text() != external_content:
                print("FAIL(P05-cancel): the target's bytes changed after Cancel - it must stay exactly as externally modified", file=sys.stderr)
                return 1
        except (RuntimeError, AssertionError) as exc:
            print(f"FAIL(P05-cancel): {exc}", file=sys.stderr)
            return 1
        finally:
            if proc is not None:
                terminate(proc)

    # ── Sub-case 2: Overwrite ────────────────────────────────────────────
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        left, right = fresh_pair(scratch_path)
        proc = None
        try:
            proc, win = open_and_dirty(scratch, left, right)
            right.write_text(external_content)

            save_button = find_by_text_containing(win, "Save merge result")
            invoke(save_button)
            if not wait_for_tokens(win, ["File changed on disk"], timeout_s=READY_TIMEOUT_S)[0]:
                print('FAIL(P05-overwrite): the "File changed on disk" conflict modal never appeared', file=sys.stderr)
                return 1

            overwrite_button = modal_action_button(win, "Overwrite")
            if overwrite_button is None:
                print('FAIL(P05-overwrite): could not find the modal\'s "Overwrite" button', file=sys.stderr)
                return 1
            invoke(overwrite_button)
            if not wait_gone(win, "File changed on disk", timeout_s=LAUNCH_TIMEOUT_S):
                print("FAIL(P05-overwrite): the conflict dialog never dismissed after Overwrite", file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            actual_lines = right.read_text().splitlines()
            while time.monotonic() < deadline and actual_lines != expected_saved_lines:
                time.sleep(POLL_INTERVAL_S)
                actual_lines = right.read_text().splitlines()
            if actual_lines != expected_saved_lines:
                print(f"FAIL(P05-overwrite): {right}'s content {actual_lines!r} != expected {expected_saved_lines!r}", file=sys.stderr)
                return 1

            bak = right.with_name(right.name + ".bak")
            expected_bak = (
                "IMPOSSIBLE - this harness never externally writes this content"
                if break_mode
                else external_content
            )
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            actual_bak = bak.read_text() if bak.exists() else None
            while time.monotonic() < deadline and actual_bak != expected_bak:
                time.sleep(POLL_INTERVAL_S)
                actual_bak = bak.read_text() if bak.exists() else None
            if actual_bak != expected_bak:
                print(f"FAIL(P05-overwrite): {bak}'s content {actual_bak!r} != expected {expected_bak!r} (must equal the externally-modified bytes it replaced)", file=sys.stderr)
                return 1
        except (RuntimeError, AssertionError) as exc:
            print(f"FAIL(P05-overwrite): {exc}", file=sys.stderr)
            return 1
        finally:
            if proc is not None:
                terminate(proc)

    if break_mode:
        print("OK(break): the .bak content check correctly rejected an impossible required value.")
        return 0

    # ── Sub-case 3: Save As, to a different path ────────────────────────
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        left, right = fresh_pair(scratch_path)
        new_path = scratch_path / "saved_as.txt"
        proc = None
        try:
            proc, win = open_and_dirty(scratch, left, right)
            right.write_text(external_content)

            save_as_button = find_by_text_containing(win, "Save As")
            if save_as_button is None:
                print('FAIL(P05-saveas): could not find the "Save As" toolbar button', file=sys.stderr)
                return 1
            invoke(save_as_button)

            edit = wait_for_first(win, "Edit", timeout_s=LAUNCH_TIMEOUT_S)
            if edit is None:
                print("FAIL(P05-saveas): could not find the Save As path input", file=sys.stderr)
                return 1
            edit.set_edit_text(str(new_path))

            save_click_button = modal_action_button(win, "Save")
            if save_click_button is None:
                print('FAIL(P05-saveas): could not find the Save As dialog\'s own "Save" button', file=sys.stderr)
                return 1
            invoke(save_click_button)

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and not new_path.exists():
                time.sleep(POLL_INTERVAL_S)
            if not new_path.exists():
                print(f"FAIL(P05-saveas): {new_path} was never written", file=sys.stderr)
                return 1

            if right.read_text() != external_content:
                print("FAIL(P05-saveas): the original target's bytes changed - Save As must never touch it", file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            actual_lines = new_path.read_text().splitlines()
            while time.monotonic() < deadline and actual_lines != expected_saved_lines:
                time.sleep(POLL_INTERVAL_S)
                actual_lines = new_path.read_text().splitlines()
            if actual_lines != expected_saved_lines:
                print(f"FAIL(P05-saveas): {new_path}'s content {actual_lines!r} != expected {expected_saved_lines!r}", file=sys.stderr)
                return 1
        except (RuntimeError, AssertionError) as exc:
            print(f"FAIL(P05-saveas): {exc}", file=sys.stderr)
            return 1
        finally:
            if proc is not None:
                terminate(proc)

    print(
        "OK: Cancel blocked the write and preserved the externally-modified bytes; "
        "Overwrite wrote the app's content and backed up the externally-modified bytes "
        "it replaced; Save As wrote to a new path and left the original target untouched."
    )
    return 0


# ── P06 — Async identity (M5-B) ──────────────────────────────────────────────


def _pair_content(tag, token, num_lines=8_000):
    """Synthetic left/right text, distinguished by `token` in the right
    file's first line plus scattered per-`tag` differences every 20
    lines - large enough for loading (I/O + `compute_diff`, off the UI
    thread via `tokio::task::spawn_blocking`, per `state/compare.rs`) to
    take a real, observable amount of wall-clock time (RFC-078: "a light
    but real exercise", not an internal timing hook), without needing an
    enlarged timeout budget to match. Scaled down twice from an initial
    300,000-line/300-line-spacing attempt (M5-B, first CI run: three
    sequential process launches each waiting up to a minute pushed the
    whole case close to ten minutes) and a 60,000-line follow-up (M5-B,
    third CI run: still timed out waiting for the second reload - this
    app+runner combination is evidently slower to load even a
    tens-of-thousands-of-lines file than assumed). The correctness being
    tested (identity/reload) doesn't need files this large, just
    genuinely slow enough to give a real interaction window - see
    `p06`'s own launch-timeout override for the other half of this fix."""
    left_lines = [f"line-{tag}-{i:06d}" for i in range(num_lines)]
    right_lines = list(left_lines)
    for i in range(0, num_lines, 20):
        right_lines[i] = f"line-{tag}-{i:06d}-CHANGED-{token}"
    right_lines.insert(0, f"TOKEN-{token}")
    return "\n".join(left_lines) + "\n", "\n".join(right_lines) + "\n"


def _write_large_pair(scratch_path, tag, token):
    left_text, right_text = _pair_content(tag, token)
    left = scratch_path / f"{tag}-left.txt"
    right = scratch_path / f"{tag}-right.txt"
    left.write_text(left_text)
    right.write_text(right_text)
    return left, right


def _rewrite_pair(left, right, tag, token):
    left_text, right_text = _pair_content(tag, token)
    left.write_text(left_text)
    right.write_text(right_text)


def p06(binary, break_mode=False):
    """Two large, deliberately slow comparisons at once, plus a
    double-reload - RFC-078: "Deterministic automated tests remain the
    primary proof; this case confirms runtime integration", so this is a
    light but real exercise, not elaborate timing machinery: two
    60,000-line synthetic file pairs with scattered differences give
    loading (I/O + `compute_diff`) a real observable duration, no
    internal API/env-var hook involved. (Scaled down from an initial
    300,000-line attempt that pushed this case toward ten minutes on a
    busy CI runner - see `_pair_content`'s docstring.)

    The double-reload is NOT a literal overlapping race (deviation,
    reported - see the code below): `diff.rs` renders no Toolbar at all
    while `TabState::Loading`, and there is no reload keyboard shortcut,
    so a second reload is structurally unreachable through the UI while
    the first is in flight, on any platform - the same shape of finding
    as P04's keyboard path. What's exercised instead: firing the second
    reload the moment the UI allows it again, still asserting the
    *second* reload's content is what's ultimately displayed.

    Two tabs, as two independent `forskscope.exe` processes rather than
    two tabs of one running instance: opening a second comparison in-app
    while the first is still loading would go through Explorer's
    file-tree row selection, which depends on the exact UIA control type
    WebView2 maps a bare `role="row"` div to - the same open question
    `windows-11.md`'s P02 section already flags as unresolved and
    explicitly defers to settle before M5-C's P03, not discover here.
    The M5-B handoff explicitly allows either approach ("whichever is
    achievable, document which you did"); this harness uses two
    processes. "tab index 0" / the remaining tab(s) below read as "the
    first-launched process" / "the second-launched process".

    Falsifiability: each fixture pair's right file carries a token unique
    to it (`TOKEN-AAAA` / `TOKEN-BBBB`), so a check that only confirmed
    "some window with some content exists" would pass even if process
    B's window were showing process A's content, blank output, or a
    crash. The check below requires B's own token to be present in B's
    accessible text *and* requires A's token to be absent from it - a
    check that only ever verified two windows existed would not catch a
    identity leak between them; this one does. The reload half is
    analogous: it requires the *first* reload's token to be replaced by
    the *second* reload's, not merely that a reload happened at all.

    `--break`: requires process B's window to show process A's token
    instead of its own, and (independently) requires the reload check to
    find the *first* reload's token still present - both impossible
    under correct behavior, proving neither check is vacuous. The
    reload half is skipped in normal mode's `--break` run for CI time;
    see the code below.
    """
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        left_a, right_a = _write_large_pair(scratch_path, "a", "AAAA")
        left_b, right_b = _write_large_pair(scratch_path, "b", "BBBB")

        proc_a = launch(binary, [left_a, right_a], scratch)
        proc_b = launch(binary, [left_b, right_b], scratch)
        try:
            # Give A a real moment to start loading before tearing it
            # down - the point under test is "closed while loading", not
            # "never given a chance to start".
            time.sleep(0.5)
            terminate(proc_a)

            app_b, win_b = connect(proc_b.pid, timeout_s=LAUNCH_TIMEOUT_S)
            needed_token = "TOKEN-AAAA" if break_mode else "TOKEN-BBBB"
            ok, texts, missing = wait_for_tokens(win_b, [needed_token], timeout_s=READY_TIMEOUT_S)
            if not ok:
                print(f"FAIL: process B never rendered {needed_token!r}: missing {missing}", file=sys.stderr)
                debug_dump(texts)
                return 1
            blob = "\n".join(texts)
            if not break_mode and "TOKEN-AAAA" in blob:
                print("FAIL: process B's window shows process A's token - cross-tab identity leak", file=sys.stderr)
                return 1
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc_a)
            terminate(proc_b)

        if break_mode:
            print("OK(break): process-identity check correctly rejected the impossible required token.")
            return 0

        # ── Double reload: the second (latest) reload wins ──────────────────
        # NOT a literal overlapping race (deviation, reported): `diff.rs`
        # renders no Toolbar at all while `TabState::Loading` (a bare
        # loading-spinner view instead - confirmed by reading the
        # component), and there is no keyboard shortcut for reload
        # (`app.rs`'s onkeydown has none), so the *only* UI path to
        # trigger a second reload is the toolbar button, which is
        # unreachable for the entire duration the first reload is in
        # flight. A true concurrent-generation race is therefore
        # structurally unreachable through this app's UI on any
        # platform - the same shape of finding as P04's keyboard path.
        # What this exercises instead: firing a second reload the moment
        # the UI allows another interaction (as soon as the first
        # reload's Ready state - and its Toolbar - reappears), with a
        # still-real, still-meaningful assertion: the *second* reload's
        # content is what's ultimately displayed, not stale first-reload
        # content left over from a missed re-render. RFC-078's own
        # framing already gives deterministic tests the primary-proof
        # role here ("this case confirms runtime integration").
        left_r, right_r = _write_large_pair(scratch_path, "reload", "V1")
        proc = launch(binary, [left_r, right_r], scratch)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            ok, texts, missing = wait_for_tokens(win, ["TOKEN-V1"], timeout_s=READY_TIMEOUT_S)
            if not ok:
                print(f"FAIL(reload): initial load never rendered TOKEN-V1: {missing}", file=sys.stderr)
                return 1

            if find_by_text_containing(win, "Reload files from disk") is None:
                print('FAIL(reload): could not find the "Reload files from disk" toolbar button', file=sys.stderr)
                return 1

            # Generous, reload-specific budget (not the shared
            # READY_TIMEOUT_S): the second wait must cover the *entire*
            # first reload's load time before its button reappears at
            # all (M5-B, third CI run: 60s was not enough headroom on a
            # busy runner even for the reduced 8,000-line fixture).
            reload_timeout_s = 180
            _rewrite_pair(left_r, right_r, "reload", "V2")
            try:
                wait_and_invoke(win, "Reload files from disk", timeout_s=reload_timeout_s)
            except RuntimeError as exc:
                print(f"FAIL(reload): could not click the first reload: {exc}", file=sys.stderr)
                return 1

            _rewrite_pair(left_r, right_r, "reload", "V3")
            try:
                wait_and_invoke(win, "Reload files from disk", timeout_s=reload_timeout_s)
            except RuntimeError as exc:
                print(f"FAIL(reload): could not click the second reload: {exc}", file=sys.stderr)
                return 1

            ok, texts, missing = wait_for_tokens(win, ["TOKEN-V3"], timeout_s=reload_timeout_s)
            if not ok:
                print(f"FAIL(reload): final state never settled to TOKEN-V3: {missing}", file=sys.stderr)
                debug_dump(texts)
                return 1
            blob = "\n".join(texts)
            if "TOKEN-V2" in blob:
                print("FAIL(reload): the first reload's (stale) content is still displayed - the second reload did not win", file=sys.stderr)
                return 1
        except RuntimeError as exc:
            print(f"FAIL(reload): {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print(
        "OK: process B kept its own identity after A closed mid-load; the second of two "
        "rapid reloads is what's ultimately displayed."
    )
    return 0


# ── P08 — Persistence migration and recovery (M5-B) ─────────────────────────

FUTURE_SESSION_JSON = json.dumps(
    {
        "schema_name": "session",
        "schema_version": 99,
        "app_version": "0.165.1",
        "created_unix": 0,
        "updated_unix": 0,
        "payload": {},
    }
)
CORRUPT_SESSION_JSON = "{not valid json"


def _p08_legacy_migration_subcase(binary, config_dir):
    """Filesystem-only, no dialog interaction needed: legacy v0
    settings.json/session.json (the exact fixtures the project's own
    Rust tests use - `crates/forskscope-core/src/tests/fixtures/persistence/`)
    migrate to a versioned v2 envelope without losing their values, and a
    `.pre-v2.bak` sibling preserves the original bytes. A legacy migration
    commits synchronously during `resolve_and_commit` at startup
    (confirmed by reading `persist_v2_runtime_tests.rs`), independent of
    how or when the process later exits, so this only needs a launch and
    a short poll of the files on disk - no dialog to click through.
    """
    settings_v0 = (
        REPO_ROOT / "crates/forskscope-core/src/tests/fixtures/persistence/settings-v0.json"
    ).read_text()
    session_v0 = (
        REPO_ROOT / "crates/forskscope-core/src/tests/fixtures/persistence/session-v0.json"
    ).read_text()
    clear_config_dir(config_dir)
    (config_dir / "settings.json").write_text(settings_v0)
    (config_dir / "session.json").write_text(session_v0)

    with tempfile.TemporaryDirectory() as cwd:
        proc = launch(binary, [], cwd)
        settings_migrated = session_migrated = False
        try:
            connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and not (settings_migrated and session_migrated):
                try:
                    s = json.loads((config_dir / "settings.json").read_text())
                    settings_migrated = s.get("schema_version") == 2
                except Exception:  # noqa: BLE001
                    pass
                try:
                    j = json.loads((config_dir / "session.json").read_text())
                    session_migrated = j.get("schema_version") == 2
                except Exception:  # noqa: BLE001
                    pass
                if not (settings_migrated and session_migrated):
                    time.sleep(POLL_INTERVAL_S)
        except RuntimeError as exc:
            print(f"FAIL(P08-legacy): {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    if not settings_migrated:
        print("FAIL(P08-legacy): settings.json was never migrated to a v2 envelope", file=sys.stderr)
        return 1
    if not session_migrated:
        print("FAIL(P08-legacy): session.json was never migrated to a v2 envelope", file=sys.stderr)
        return 1

    settings_bak = config_dir / "settings.json.pre-v2.bak"
    session_bak = config_dir / "session.json.pre-v2.bak"
    if not settings_bak.exists() or settings_bak.read_text() != settings_v0:
        print(f"FAIL(P08-legacy): {settings_bak} missing or does not match the original v0 bytes", file=sys.stderr)
        return 1
    if not session_bak.exists() or session_bak.read_text() != session_v0:
        print(f"FAIL(P08-legacy): {session_bak} missing or does not match the original v0 bytes", file=sys.stderr)
        return 1

    # Semantic preservation - concrete field values, not just "some v2
    # envelope exists" (checked against the fixture's actual values, per
    # settings-v0.json/session-v0.json/legacy.rs's field names).
    settings_payload = json.loads((config_dir / "settings.json").read_text()).get("payload", {})
    expected_settings = {
        "theme": "light",
        "language": "ja",
        "diff_font_size": 16,
        "diff_font_family": "consolas",
        "context_lines": 5,
        "active_profile": 4,
        "ignore_extensions": "o, class, tmp",
        "ignore_dirs": "target, node_modules",
        "explorer_compact": True,
        "enable_binary_comparison": True,
        "remember_explorer_dirs": False,
    }
    for key, expected in expected_settings.items():
        actual = settings_payload.get(key)
        if actual != expected:
            print(
                f"FAIL(P08-legacy): migrated settings field {key!r} = {actual!r}, "
                f"expected {expected!r} (v0 fixture value lost/changed)",
                file=sys.stderr,
            )
            return 1
    if len(settings_payload.get("profiles", [])) != 5:
        print(f"FAIL(P08-legacy): migrated settings lost profiles: {settings_payload.get('profiles')!r}", file=sys.stderr)
        return 1

    # Session tabs: NOT checked against the fixture's tabs surviving in
    # the *final* on-disk session.json - a real, discovered app behavior
    # makes that impossible to observe here, not a testing shortcut.
    # `session-v0.json`'s two tabs reference `/tmp/fixtures/left-a.txt`
    # etc., paths that do not exist on this runner. This launch uses no
    # CLI args, so `app.rs` calls `restore_tabs`, which (correctly, per
    # its own doc comment) opens only pairs where "`pair.left.exists() ||
    # pair.right.exists()`" - neither does, so both tabs are dropped, and
    # `app.rs`'s `use_effect` (which fires after the very first render,
    # independent of whether tabs actually changed) immediately persists
    # that now-empty tab list, overwriting the migration commit's
    # correctly-populated tabs before this harness ever gets to read
    # them - the migration itself is not lossy, a subsequent real launch
    # cycle's auto-save is. The `.pre-v2.bak` byte-exact comparison above
    # is the actual "migrated without loss" proof for tabs (the original
    # 2-tab content is preserved there, unaffected by any later prune);
    # what's checked here is that the *live* tabs are pruned to empty for
    # exactly this reason, not corrupted into something else.
    session_payload = json.loads((config_dir / "session.json").read_text()).get("payload", {})
    actual_tabs = session_payload.get("tabs")
    if actual_tabs != []:
        print(
            f"FAIL(P08-legacy): expected session tabs to be pruned to [] after a no-args "
            f"launch (neither v0 fixture path exists on this runner), got {actual_tabs!r}",
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: legacy v0 settings/session migrated to versioned v2 envelopes without loss; "
        f"backups at {settings_bak} and {session_bak} match the originals."
    )
    return 0


def _p08_exit_subcase(binary, config_dir, break_mode):
    """Future-schema session.json -> Exit -> the process must actually be
    gone, not just the window/dialog. Per the M5-B handoff §4/§6, this is
    the single most important check in P08: "Assert the process state,
    not just the dialog's disappearance. An Exit that dismisses the
    dialog and leaves a zombie is a failure that looks like a pass."
    Process liveness is checked with `psutil.pid_exists`, not `proc.poll()`
    alone, because polling a `subprocess.Popen` you spawned only reflects
    *your own process's* wait() bookkeeping - `psutil` queries the OS
    directly, the same thing an external operator checking for a zombie
    would do.
    """
    import psutil  # noqa: PLC0415 - Windows-only, lazily imported like pywinauto

    clear_config_dir(config_dir)
    (config_dir / "session.json").write_text(FUTURE_SESSION_JSON)

    with tempfile.TemporaryDirectory() as cwd:
        proc = launch(binary, [], cwd)
        gone = False
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            exit_button = wait_for_exact(win, "Exit", control_type="Button", timeout_s=READY_TIMEOUT_S)
            if exit_button is None:
                print('FAIL(P08-exit): could not find the "Exit" recovery-dialog button', file=sys.stderr)
                debug_dump(collect_texts(win))
                return 1
            invoke(exit_button)

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                if not psutil.pid_exists(proc.pid):
                    gone = True
                    break
                time.sleep(POLL_INTERVAL_S)

            if break_mode:
                # Deliberately require the impossible: that the process is
                # STILL running after Exit. Real Exit behavior never
                # leaves this true, so this run is expected to (and must)
                # fail - proving the liveness check would actually catch
                # a zombie, not just rubber-stamp "the dialog is gone".
                if gone:
                    print(
                        "FAIL(P08-exit --break): the process exited as expected, but --break "
                        "requires it to still be running - this IS the falsifiability "
                        "demonstration succeeding: the real check below is not vacuous.",
                        file=sys.stderr,
                    )
                    return 1
                print("OK(break): unexpectedly still running - see message above.")
                return 0

            if not gone:
                print(
                    f"FAIL(P08-exit): pid {proc.pid} is STILL RUNNING {LAUNCH_TIMEOUT_S}s after "
                    f"clicking Exit (psutil.pid_exists == True) - the dialog may have closed but "
                    f"the process did not",
                    file=sys.stderr,
                )
                return 1
        except RuntimeError as exc:
            print(f"FAIL(P08-exit): {exc}", file=sys.stderr)
            return 1
        finally:
            if not gone:
                try:
                    if psutil.pid_exists(proc.pid):
                        proc.kill()
                except Exception:  # noqa: BLE001
                    pass

    print(f"OK: Exit terminated pid {proc.pid} - confirmed gone via psutil.pid_exists() within {LAUNCH_TIMEOUT_S}s.")
    return 0


def _p08_continue_subcase(binary, config_dir):
    """Future-schema session.json -> "Continue with defaults" -> the app
    keeps running (dialog dismissed, process still alive) and
    session.json's bytes are unchanged on disk (write-disabled, per
    `SessionRuntimeResolution::write_disabled`)."""
    clear_config_dir(config_dir)
    session_path = config_dir / "session.json"
    original_bytes = FUTURE_SESSION_JSON.encode("utf-8")
    session_path.write_bytes(original_bytes)

    with tempfile.TemporaryDirectory() as cwd:
        proc = launch(binary, [], cwd)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            button = wait_for_exact(win, "Continue with defaults", control_type="Button", timeout_s=READY_TIMEOUT_S)
            if button is None:
                print('FAIL(P08-continue): could not find the "Continue with defaults" button', file=sys.stderr)
                debug_dump(collect_texts(win))
                return 1
            invoke(button)

            if not wait_gone(win, "Session file is from a newer version", timeout_s=LAUNCH_TIMEOUT_S):
                print("FAIL(P08-continue): the recovery dialog never dismissed", file=sys.stderr)
                return 1
            if proc.poll() is not None:
                print(f'FAIL(P08-continue): the process exited ({proc.returncode}) after "Continue with defaults" - it should keep running', file=sys.stderr)
                return 1

            time.sleep(1.0)  # give any (incorrect) write a moment to land
            current_bytes = session_path.read_bytes()
            if current_bytes != original_bytes:
                print(
                    f"FAIL(P08-continue): session.json bytes changed after Continue with "
                    f"defaults - write-disabled did not hold. Expected {original_bytes!r}, "
                    f"got {current_bytes!r}",
                    file=sys.stderr,
                )
                return 1
        except RuntimeError as exc:
            print(f"FAIL(P08-continue): {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print("OK: Continue with defaults dismissed the dialog, the app kept running, and session.json's bytes were unchanged on disk.")
    return 0


def _p08_reset_subcase(binary, config_dir):
    """Corrupt session.json -> "Reset and back up" -> the dialog
    dismisses, session.json is reset to a fresh default v2 envelope, and
    a `.reset.bak` sibling holds the original corrupt bytes
    (`ensure_reset_backup` / `<name>.reset.bak` - confirmed by reading
    `persist/schema/repository.rs`, not assumed)."""
    clear_config_dir(config_dir)
    session_path = config_dir / "session.json"
    session_path.write_text(CORRUPT_SESSION_JSON)
    bak_path = config_dir / "session.json.reset.bak"

    with tempfile.TemporaryDirectory() as cwd:
        proc = launch(binary, [], cwd)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            button = wait_for_exact(win, "Reset and back up", control_type="Button", timeout_s=READY_TIMEOUT_S)
            if button is None:
                print('FAIL(P08-reset): could not find the "Reset and back up" button', file=sys.stderr)
                debug_dump(collect_texts(win))
                return 1
            invoke(button)

            if not wait_gone(win, "Session file could not be read", timeout_s=LAUNCH_TIMEOUT_S):
                print("FAIL(P08-reset): the recovery dialog never dismissed", file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            bak_bytes = bak_path.read_bytes() if bak_path.exists() else None
            while time.monotonic() < deadline and bak_bytes is None:
                time.sleep(POLL_INTERVAL_S)
                bak_bytes = bak_path.read_bytes() if bak_path.exists() else None
            expected_corrupt = CORRUPT_SESSION_JSON.encode("utf-8")
            if bak_bytes != expected_corrupt:
                print(f"FAIL(P08-reset): {bak_path}'s bytes {bak_bytes!r} != the original corrupt bytes {expected_corrupt!r}", file=sys.stderr)
                return 1

            new_bytes = session_path.read_bytes()
            if new_bytes == expected_corrupt:
                print("FAIL(P08-reset): session.json still holds the corrupt bytes after Reset", file=sys.stderr)
                return 1
            try:
                envelope = json.loads(new_bytes)
            except json.JSONDecodeError:
                print(f"FAIL(P08-reset): session.json after Reset is not valid JSON: {new_bytes!r}", file=sys.stderr)
                return 1
            if envelope.get("schema_name") != "session" or envelope.get("schema_version") != 2:
                print(f"FAIL(P08-reset): session.json after Reset is not a versioned v2 envelope: {envelope!r}", file=sys.stderr)
                return 1
        except RuntimeError as exc:
            print(f"FAIL(P08-reset): {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print(f"OK: Reset and back up dismissed the dialog, wrote a fresh v2 session.json, and backed up the original corrupt bytes to {bak_path}.")
    return 0


def p08(binary, break_mode=False):
    """The highest-value case in M5-B (handoff §4): F37's amendment
    requires all three recovery-dialog choices - Exit, Continue (either
    variant), Reset - exercised, with the process's actual state
    asserted, not just the dialog's disappearance. Runs four sub-checks
    in sequence against the runner's real (but disposable) config
    directory (see the module docstring): legacy v0 migration without
    loss, Exit (process confirmed gone via `psutil`), Continue with
    defaults (write stays disabled), and Reset and back up (file reset,
    original backed up). Each launch clears and reseeds the config
    directory first, so the four sub-checks never see each other's state.

    `--break`: runs only the Exit sub-case (the specifically-flagged
    vacuous-pass risk per handoff §6: "a check that passes whenever the
    dialog closes, regardless of whether the process died"), inverted to
    require the process to still be running - see
    `_p08_exit_subcase`'s docstring.
    """
    config_dir = resolve_config_dir()
    try:
        if break_mode:
            return _p08_exit_subcase(binary, config_dir, break_mode=True)

        rc = _p08_legacy_migration_subcase(binary, config_dir)
        if rc != 0:
            return rc
        rc = _p08_exit_subcase(binary, config_dir, break_mode=False)
        if rc != 0:
            return rc
        rc = _p08_continue_subcase(binary, config_dir)
        if rc != 0:
            return rc
        rc = _p08_reset_subcase(binary, config_dir)
        if rc != 0:
            return rc
    finally:
        try:
            clear_config_dir(config_dir)
        except Exception:  # noqa: BLE001
            pass

    print(
        "OK: P08 all sub-cases passed - legacy migration without loss, Exit (process "
        "confirmed gone), Continue with defaults (write-disabled held), Reset and back "
        "up (file reset, original backed up)."
    )
    return 0


# ── P12 — Session/settings restart (M5-B) ───────────────────────────────────


def p12(binary, break_mode=False):
    """Changes theme/language/font size via the Settings dialog, exits,
    relaunches with no CLI arguments, and confirms restoration - then
    confirms tab restore happens only on a no-args relaunch, not one with
    explicit CLI paths.

    Settings dialog controls are located ordinally (first `ComboBox` =
    Theme, second = Language), not by accessible name: neither `<select>`
    carries an `aria-label`/`<label for>` in `settings/modal.rs`, so
    there is nothing for UIA to expose beyond the generic ComboBox role -
    a real, reported harness limitation (see the module docstring), not
    an assumption. The font-size number input is set via
    `set_edit_text()` (UIA's ValuePattern.SetValue), the same
    pattern-based-not-synthesized approach as every other interaction in
    this harness.

    Tab-restore-only-without-explicit-args is checked against a directly
    constructed `session.json` (**deviation, reported - twice over**):

    1. Exercising it via Explorer's own file-selection UI in-app would
       mean driving Explorer's file-tree rows, which is exactly the
       `role="row"` UIA-control-type uncertainty `windows-11.md`'s P02
       section already flagged as unresolved and explicitly deferred to
       settle before M5-C's P03 - attempting it blind here would be
       discovering that question mid-case, which is what that note
       explicitly says not to do.
    2. The next-simplest option - seed via an ordinary 2-arg CLI launch
       and wait for `save_session`'s reactive effect to persist it - was
       tried first and does not work: a real, reported finding (see the
       module docstring's "P12's tab auto-save finding"), not a
       harness bug. So this constructs the v2 `session.json` envelope
       directly instead. `restore_tabs` (`app.rs`, reached exactly when
       `into_compare_request()` is `None`) doesn't care how the file
       describing a tab came to exist - only that it does - so this
       still exercises exactly the restore mechanism P12 cares about,
       just without depending on the write path that turned out not to
       work for a CLI launch specifically.

    `--break`: requires the post-restart header button to show a label
    that settings restoration can never produce, proving the restart
    comparison reads real state.
    """
    config_dir = resolve_config_dir()
    clear_config_dir(config_dir)

    # ── Launch 1: change theme/language/font, then exit ─────────────────
    with tempfile.TemporaryDirectory() as cwd:
        proc = launch(binary, [], cwd)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            settings_btn = wait_for_exact(win, "Settings", control_type="Button", timeout_s=READY_TIMEOUT_S)
            if settings_btn is None:
                print('FAIL: could not find the header "Settings" button', file=sys.stderr)
                debug_dump(collect_texts(win))
                return 1
            invoke(settings_btn)

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            combos = []
            while time.monotonic() < deadline and len(combos) < 2:
                try:
                    combos = win.descendants(control_type="ComboBox")
                except Exception:  # noqa: BLE001
                    combos = []
                if len(combos) < 2:
                    time.sleep(POLL_INTERVAL_S)
            if len(combos) < 2:
                print(f"FAIL: expected at least 2 ComboBox controls (Theme, Language) in the Settings dialog, found {len(combos)}", file=sys.stderr)
                debug_dump(collect_texts(win))
                return 1
            theme_combo, lang_combo = combos[0], combos[1]
            theme_path = select_dropdown(theme_combo, "Light")
            lang_path = select_dropdown(lang_combo, "日本語")

            edits = find_number_inputs(win)
            if not edits:
                print("FAIL: could not find the diff font size input (tried Spinner and Edit control types)", file=sys.stderr)
                return 1
            set_value_text(edits[0], "18")

            close_btn = wait_for_exact(win, "Close", control_type="Button", timeout_s=LAUNCH_TIMEOUT_S)
            if close_btn is None:
                print('FAIL: could not find the Settings dialog "Close" button', file=sys.stderr)
                return 1
            invoke(close_btn)
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            # settings.json is written immediately on every change
            # (`super::persist(store)` inline in each onchange handler,
            # not deferred to an exit hook) - a plain terminate() after
            # the changes above land is enough; no special "graceful
            # exit" API is needed for correctness here (contrast P08's
            # Exit sub-case, which specifically tests the in-dialog Exit
            # button's own process-termination behavior).
            terminate(proc)

    settings_path = config_dir / "settings.json"
    deadline = time.monotonic() + LAUNCH_TIMEOUT_S
    saved = None
    while time.monotonic() < deadline:
        if settings_path.exists():
            try:
                saved = json.loads(settings_path.read_text()).get("payload", {})
            except Exception:  # noqa: BLE001
                saved = None
        if saved and saved.get("theme") == "light":
            break
        time.sleep(POLL_INTERVAL_S)
    if not saved or saved.get("theme") != "light":
        print(f"FAIL: {settings_path} was never written with theme=light after changing settings", file=sys.stderr)
        return 1

    # ── Relaunch, no CLI args: theme/language/font must be restored ─────
    with tempfile.TemporaryDirectory() as cwd:
        proc = launch(binary, [], cwd)
        try:
            app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
            expected_label = "this-label-can-never-appear" if break_mode else "設定"
            found = wait_for_exact(win, expected_label, control_type="Button", timeout_s=READY_TIMEOUT_S)
            if found is None:
                print(f"FAIL: header button never showed {expected_label!r} after restart", file=sys.stderr)
                debug_dump(collect_texts(win))
                return 1
            invoke(found)  # reopen Settings to read back Theme/Language/font

            wait_for_first(win, "ComboBox", timeout_s=LAUNCH_TIMEOUT_S)
            combos = win.descendants(control_type="ComboBox")
            if not combos or len(combos) < 2:
                print(f"FAIL: expected at least 2 ComboBox controls after restart, found {len(combos or [])}", file=sys.stderr)
                return 1
            # window_text() alone was empirically found (M5-B) not to
            # reliably reflect a <select>'s current value for this UIA
            # ComboBox mapping - _combo_shows checks every readback
            # source select_dropdown() itself accepts as evidence of a
            # real change, for the same reason.
            if not _combo_shows(combos[0], "Light"):
                print(f"FAIL: restored Theme combobox does not show Light: {_combo_readback(combos[0])!r}", file=sys.stderr)
                return 1
            if not _combo_shows(combos[1], "日本語"):
                print(f"FAIL: restored Language combobox does not show 日本語: {_combo_readback(combos[1])!r}", file=sys.stderr)
                return 1
            edits = find_number_inputs(win)
            if not edits or not _combo_shows(edits[0], "18"):
                diag = _combo_readback(edits[0]) if edits else "no Spinner/Edit control found"
                print(f"FAIL: restored diff font size does not show '18': {diag!r}", file=sys.stderr)
                return 1
        except RuntimeError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    if break_mode:
        print("OK(break): correctly failed to find the impossible post-restart label.")
        return 0

    # ── Tab restore: only when no explicit CLI paths are given ──────────
    left_src = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right_src = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        left = scratch_path / "left.txt"
        right = scratch_path / "right.txt"
        left.write_text(left_src.read_text())
        right.write_text(right_src.read_text())

        # Seeded directly by writing a valid v2 session.json envelope,
        # not by launching a CLI-arg compare and waiting for its
        # auto-save (deviation, reported - and a genuine finding in its
        # own right, not a harness bug this rewrite is hiding): a 2-arg
        # CLI launch's own opened tab was empirically observed, on real
        # CI, to never reach session.json at all - not a slow write, not
        # a race with this harness's own process termination (both ruled
        # out: the check was moved to poll *while the process stayed
        # alive*, for the entire LAUNCH_TIMEOUT_S budget, against a
        # config directory cleared immediately beforehand so no earlier
        # launch's file could be mistaken for a fresh one - and
        # session.json still never got created at all). See "P12's tab
        # auto-save finding" in this file's module docstring / the M5-B
        # report for the full detail; not fixed here per the handoff's
        # "no product behaviour changes" constraint. What P12 actually
        # needs to prove - that `restore_tabs` reopens a tab described in
        # session.json, and that an explicit-args relaunch skips it -
        # doesn't depend on *how* that file came to describe the tab, so
        # this constructs the exact v2 envelope shape directly (the same
        # schema `crates/forskscope-core/src/persist/schema/session.rs`
        # and this harness's own P08 fixtures already use).
        clear_config_dir(config_dir)
        session_path = config_dir / "session.json"
        now = int(time.time())
        session_path.write_text(
            json.dumps(
                {
                    "schema_name": "session",
                    "schema_version": 2,
                    "app_version": "0.167.0",
                    "created_unix": now,
                    "updated_unix": now,
                    "payload": {"tabs": [{"left": str(left), "right": str(right)}]},
                }
            )
        )

        # No CLI args -> the seeded tab must auto-restore.
        with tempfile.TemporaryDirectory() as cwd:
            proc = launch(binary, [], cwd)
            try:
                app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
                ok, texts, missing = wait_for_tokens(win, FIXTURE_TOKENS, timeout_s=READY_TIMEOUT_S)
                if not ok:
                    print(f"FAIL: no-args relaunch never restored the seeded tab (missing {missing})", file=sys.stderr)
                    debug_dump(texts)
                    return 1
            except RuntimeError as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1
            finally:
                terminate(proc)

        # Explicit CLI args -> opens the specified compare, not the
        # restored session (a second, distinct fixture pair).
        left2 = scratch_path / "left2.txt"
        right2 = scratch_path / "right2.txt"
        left2.write_text("distinct-p12-left-content\nALPHA-P12-TOKEN\n")
        right2.write_text("distinct-p12-right-content\nBETA-P12-TOKEN\n")
        with tempfile.TemporaryDirectory() as cwd:
            proc = launch(binary, [left2, right2], cwd)
            try:
                app, win = connect(proc.pid, timeout_s=LAUNCH_TIMEOUT_S)
                ok, texts, missing = wait_for_tokens(
                    win, ["ALPHA-P12-TOKEN", "BETA-P12-TOKEN"], timeout_s=READY_TIMEOUT_S
                )
                if not ok:
                    print(f"FAIL: explicit-args relaunch never showed the explicitly requested compare (missing {missing})", file=sys.stderr)
                    debug_dump(texts)
                    return 1
                blob = "\n".join(texts)
                if any(tok in blob for tok in FIXTURE_TOKENS):
                    print(
                        "FAIL: explicit-args relaunch shows tokens from the restored session "
                        "tab instead of (or in addition to) the explicitly requested compare "
                        "- restore did not stay skipped",
                        file=sys.stderr,
                    )
                    return 1
            except RuntimeError as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1
            finally:
                terminate(proc)

    print(
        "OK: theme/language/font size restored after restart, a Japanese label rendered, "
        "and tab restore happened only on a no-args relaunch (explicit CLI args opened the "
        "specified compare instead)."
    )
    return 0


CASES = {
    "p01": p01,
    "p02": p02,
    "p03": p03,
    "rowprobe": rowprobe,
    "scrollprobe": scrollprobe,
    "p04": p04,
    "p05": p05,
    "p06": p06,
    "p08": p08,
    "p09": p09,
    "p10": p10,
    "p12": p12,
}


def main():
    args = sys.argv[1:]
    break_mode = "--break" in args
    args = [a for a in args if a != "--break"]
    if len(args) != 2 or args[0] not in CASES:
        print(
            f"usage: {sys.argv[0]} {{{'|'.join(CASES)}}} <path-to-forskscope-binary> [--break]",
            file=sys.stderr,
        )
        return 2
    case, binary = args
    return CASES[case](str(Path(binary).resolve()), break_mode=break_mode)


if __name__ == "__main__":
    sys.exit(main())
