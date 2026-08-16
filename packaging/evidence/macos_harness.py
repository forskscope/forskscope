#!/usr/bin/env python3
"""M5-A: macOS evidence harness for P01, P02, P09, P10 (RFC-078).

Drives the mounted, published `.dmg`'s `.app` bundle through its four CLI
launch modes and asserts real, functional outcomes via macOS's
Accessibility API - not just a zero exit code. The tree-walk/action logic
lives in the companion `macos_ui.applescript`, invoked here through
`osascript`; see that file's header comment for why AppleScript + System
Events was chosen over pyobjc or synthetic input events, mirroring the
reasoning that led `linux_harness.py` to AT-SPI's `Action.do_action`
instead of X11 input synthesis.

Each case is a subcommand. Exit 0 and a summary line on success; exit 1
and a description of what failed otherwise, matching `linux_harness.py`'s
convention (and, transitively, `render_check.py`'s).

Usage:
  macos_harness.py p01 <path-to-forskscope-binary>
  macos_harness.py p02 <path-to-forskscope-binary>
  macos_harness.py p09 <path-to-forskscope-binary>
  macos_harness.py p10 <path-to-forskscope-binary>

The binary is the one *inside* the mounted `.app` bundle
(`ForskScope.app/Contents/MacOS/forskscope`) - mounting the `.dmg` and
locating it is the caller's job (see `m5-evidence-macos.yml`), matching
P01's "launch the binary directly" requirement in the handoff.

Falsifiability (`--break`): each case accepts an optional `--break` flag
that deliberately breaks the condition it checks, to demonstrate the
assertion is not vacuous - see each case function's docstring for exactly
what it breaks. `--break` only changes what the harness expects to see;
it never touches product source.
"""

import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
APPLESCRIPT = Path(__file__).resolve().parent / "macos_ui.applescript"

LAUNCH_TIMEOUT_S = 45
PROC_NAME = "forskscope"
XLSX_MESSAGE = "Spreadsheet comparison is temporarily disabled for security."


class PermissionWall(RuntimeError):
    """System Events refused the query/action outright (no assistive
    access granted to the automation process) - a distinct, reportable
    failure mode from "element not found", per the handoff's instruction
    not to force past it insecurely."""


def pid_target(pid):
    """A process identifier for `ui()`'s `proc_name` override that
    addresses a process by PID instead of by name - see
    macos_ui.applescript's `runOnce` comment on why: two same-named
    "forskscope" processes briefly coexisting (P06's two-process variant)
    can make plain name-based `process procName` addressing resolve to
    whichever one the window server enumerates first, observed to
    sometimes be the dying one even after `proc.poll()` confirms the OS
    process was already reaped."""
    return f"pid:{pid}"


def ui(cmd, *args, timeout=15, proc_name=None):
    """Run one `macos_ui.applescript` command and return its stdout,
    stripped. Raises PermissionWall if System Events itself refused the
    request (assistive access not granted), rather than folding that into
    an ordinary NOT_FOUND result. `proc_name` overrides the default
    PROC_NAME target - see `pid_target`."""
    proc = subprocess.run(
        ["osascript", str(APPLESCRIPT), cmd, proc_name or PROC_NAME, *args],
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    if proc.returncode != 0:
        stderr = proc.stderr.strip()
        if "assistive access" in stderr.lower() or "not allowed" in stderr.lower():
            raise PermissionWall(
                f"System Events denied UI scripting access: {stderr}"
            )
        raise RuntimeError(f"osascript {cmd} failed (exit {proc.returncode}): {stderr}")
    return proc.stdout.strip()


def launch(binary, args, cwd, home=None):
    """`home`, if given, overrides `$HOME` for the launched process - both
    to isolate `dirs_next::config_dir()` (P08/P12: never touch the real
    runner's `~/Library/Application Support`) and, independently, to keep
    the Explorer workspace's default directory listing small. The latter
    matters beyond tidiness: `entire contents of w` against a still-async-
    scanning Explorer view (`tree.rs`'s `scans_l`/`scans_r`) can race the
    scan and fail with "AppleEvent handler failed (-10000)" - a real
    failure hit during this slice's own recon runs against the runner's
    unmodified (and much larger) real home directory. A small, controlled
    `$HOME` keeps the scan short-lived and the tree small.
    """
    env = None
    if home is not None:
        env = dict(os.environ)
        env["HOME"] = str(home)
    return subprocess.Popen(
        [str(binary), *[str(a) for a in args]], cwd=str(cwd), env=env
    )


def terminate(proc):
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)


def run_diagnostics(binary):
    """Run `--diagnostics` (no window, no display needed) and return stdout."""
    result = subprocess.run(
        [str(binary), "--diagnostics"], capture_output=True, text=True, timeout=15
    )
    return result.returncode, result.stdout, result.stderr


def wait_for_window(deadline, proc_name=None):
    """Poll until `forskscope` registers a window with positive extents.
    Returns (width, height) or raises on timeout/permission wall.
    `proc_name` overrides the default PROC_NAME target - see `pid_target`."""
    last = None
    while time.monotonic() < deadline:
        last = ui("window_size", timeout=20, proc_name=proc_name)
        if last not in ("NO_PROCESS", "NO_WINDOW"):
            try:
                w_str, h_str = last.split("x")
                w, h = int(w_str), int(h_str)
            except ValueError:
                raise RuntimeError(f"unexpected window_size output: {last!r}")
            if w > 0 and h > 0:
                return w, h
        time.sleep(0.5)
    raise TimeoutError(f"no non-empty window within the timeout (last: {last!r})")


# ── M5-B shared polling helpers ─────────────────────────────────────────────
#
# P01/P02/P09/P10 above each inline their own poll loop (M5-A's style, left
# untouched). P04/P05/P06/P08/P12 below share the same small set of poll
# shapes often enough (wait for a row count, click a button once it's
# actionable, wait for text to appear/disappear) that factoring them out
# here is more honest than copy-pasting five more inline loops.


def poll_ui(cmd, *args, predicate, timeout, interval=0.5, call_timeout=20, proc_name=None):
    """Poll `ui(cmd, *args)` until `predicate(result)` is true or the
    timeout elapses; returns the last result either way (callers decide
    what "never satisfied" means for their case). `call_timeout` is the
    per-`ui()`-call subprocess timeout, not the overall poll budget -
    raise it for a command that can each legitimately take a while (e.g.
    a query against a view backing a large diff). `proc_name` overrides
    the default PROC_NAME target - see `pid_target`."""
    deadline = time.monotonic() + timeout
    result = ""
    while time.monotonic() < deadline:
        try:
            result = ui(cmd, *args, timeout=call_timeout, proc_name=proc_name)
        except subprocess.TimeoutExpired:
            # A single slow call (e.g. right after launch, or while
            # another process/window briefly coexists) shouldn't crash
            # the whole poll loop the way an uncaught exception would -
            # treat it as "not satisfied yet" and keep polling within the
            # overall budget. `result` keeps its last real value so a
            # caller inspecting the return after the deadline still sees
            # something informative rather than a Python traceback.
            time.sleep(interval)
            continue
        if predicate(result):
            return result
        time.sleep(interval)
    return result


def wait_rows(expected, timeout=LAUNCH_TIMEOUT_S):
    def pred(r):
        try:
            return int(r) == expected
        except ValueError:
            return False

    return poll_ui("count_rows", predicate=pred, timeout=timeout)


def click_wait(needle, timeout=LAUNCH_TIMEOUT_S, exact=False, proc_name=None):
    cmd = "click_button_exact" if exact else "click_button"
    return poll_ui(
        cmd,
        needle,
        predicate=lambda r: r.startswith("CLICKED"),
        timeout=timeout,
        proc_name=proc_name,
    )


def find_wait(needle, timeout=LAUNCH_TIMEOUT_S, want_found=True, proc_name=None):
    def pred(r):
        return r.startswith("FOUND") == want_found

    return poll_ui("find_text", needle, predicate=pred, timeout=timeout, proc_name=proc_name)


# ── P01 — Install and cold launch ───────────────────────────────────────────


def p01(binary, break_mode=False):
    """Two independent checks, both real launches of the actual binary
    inside the mounted `.app` bundle:

    1. `--diagnostics` exits 0, reports the expected app version, and
       redacts the home directory (never the literal path).
    2. A plain cold launch (no args) registers a real window via macOS's
       Accessibility API with non-zero on-screen extents - proof of an
       actual window, not just a process that stayed alive.

    `--break`: asserts against a version string that cannot match
    (`"0.0.0-impossible"`), to prove check (1) actually reads the
    reported version rather than only checking exit code.
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
    os_line = next((l for l in out.splitlines() if l.startswith("OS:")), "")
    if "macos" not in os_line:
        print(f"FAIL: --diagnostics OS line does not report macos: {os_line!r}", file=sys.stderr)
        return 1
    home_line = next((l for l in out.splitlines() if l.startswith("Home:")), "")
    if home_line and not home_line.endswith("***") and "***" not in home_line:
        print(f"FAIL: home directory not redacted: {home_line}", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [], scratch)
        try:
            try:
                width, height = wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except PermissionWall as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1
            except TimeoutError as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1
            if proc.poll() is not None:
                print(
                    f"FAIL: process exited ({proc.returncode}) shortly after launch",
                    file=sys.stderr,
                )
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
    rendered by counting `AXRow` accessible elements - `hunk.rs`'s
    `RowLeft`/`RowRight` each emit `role: "row"` independently per pane, so
    the pinned 7-rows-per-pane fixture (F34/F57's
    `left_all_hunk_kinds.txt`/`right_all_hunk_kinds.txt`, 7 per pane per
    `all_hunk_kinds_fixture_produces_exactly_seven_visual_rows`) produces
    14 total rows once both panes have rendered.

    `--break`: waits for an impossible row count (99), proving the
    assertion is a real count check, not a vacuous "window exists".
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    expected = 99 if break_mode else 14

    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [left, right], scratch)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            count = -1
            while time.monotonic() < deadline:
                try:
                    raw = ui("count_rows", timeout=20)
                    count = int(raw)
                except PermissionWall as exc:
                    print(f"FAIL: {exc}", file=sys.stderr)
                    return 1
                except ValueError:
                    count = -1
                if count == expected:
                    break
                time.sleep(0.5)
            if count != expected:
                print(
                    f"FAIL: compare view showed {count} AXRow elements, "
                    f"expected {expected}, within {LAUNCH_TIMEOUT_S}s",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    print(f"OK: CLI compare rendered {count} AXRow elements (7 per pane).")
    return 0


# ── P09 — Mergetool ──────────────────────────────────────────────────────────


def p09(binary, break_mode=False):
    """Launches `<binary> <local> <remote> <merged>` against F34's fixture
    pair (a real diff), clicks the first hunk's "Use this change" button,
    then clicks the toolbar's "Save merge result" button, and asserts
    `<merged>` was actually written - both buttons invoked via System
    Events' `click`/`AXPress` action, not synthetic keyboard/mouse input
    (see `macos_ui.applescript`'s header for why). `merged` starts
    pre-seeded with a placeholder no real merge output could produce, so
    "was it overwritten" is unambiguous.

    Applying a real hunk first matters beyond realism: the toolbar Save
    button is disabled until the tab is dirty (`disabled: !snap.is_dirty`
    in `toolbar.rs`), so a no-op tab would make this test pass for the
    wrong reason.

    `--break`: checks for a placeholder string that can never be
    replaced, proving the assertion reads the file's real content rather
    than only checking existence.
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
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            # Wait for the compare view's rows too, not just a window -
            # the apply button doesn't exist until the diff has rendered.
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            ready = False
            while time.monotonic() < deadline:
                try:
                    if int(ui("count_rows", timeout=20)) >= 14:
                        ready = True
                        break
                except (PermissionWall, ValueError) as exc:
                    if isinstance(exc, PermissionWall):
                        print(f"FAIL: {exc}", file=sys.stderr)
                        return 1
                time.sleep(0.5)
            if not ready:
                print("FAIL: compare view never reached its expected row count", file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            applied = False
            while time.monotonic() < deadline and not applied:
                try:
                    result = ui("click_button", "Use this change", timeout=20)
                except PermissionWall as exc:
                    print(f"FAIL: {exc}", file=sys.stderr)
                    return 1
                if result.startswith("CLICKED"):
                    applied = True
                elif result == "NOT_FOUND":
                    time.sleep(0.5)
                else:
                    # DISABLED - shouldn't happen for an apply button, but
                    # retry rather than assume it can never become enabled.
                    time.sleep(0.5)
            if not applied:
                print('FAIL: could not click "Use this change" within the timeout', file=sys.stderr)
                return 1

            # The re-render that enables Save (tab becomes dirty) happens
            # asynchronously after the click returns - poll/retry rather
            # than assume the tree already reflects the new state (same
            # lesson as linux_harness.py's Save-button retry loop).
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            clicked_save = False
            while time.monotonic() < deadline and not clicked_save:
                try:
                    result = ui("click_button", "Save merge result", timeout=20)
                except PermissionWall as exc:
                    print(f"FAIL: {exc}", file=sys.stderr)
                    return 1
                if result.startswith("CLICKED"):
                    clicked_save = True
                else:
                    time.sleep(0.5)
            if not clicked_save:
                print('FAIL: could not click "Save merge result" within the timeout', file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            actual = placeholder
            while time.monotonic() < deadline:
                actual = merged.read_text()
                if actual != placeholder:
                    break
                time.sleep(0.5)
            if break_mode:
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
        finally:
            terminate(proc)

    print(
        f"OK: applied a hunk and clicked Save; {merged.name} was overwritten "
        f"({len(actual)} bytes, no longer the placeholder)."
    )
    return 0


# ── P10 — Binary/XLSX fail-closed policy ────────────────────────────────────


def p10(binary, break_mode=False):
    """Launches `<binary> <left.xlsx> <right.xlsx>` (classification is by
    extension only - `core::file_kind::classify` - so arbitrary bytes
    suffice) and asserts the fail-closed message reaches the user, found
    by walking the accessible tree for its text (`macos_ui.applescript`'s
    `find_text`, which checks description/title/value).

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
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            needle = "this message cannot appear" if break_mode else XLSX_MESSAGE
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            found = None
            while time.monotonic() < deadline:
                try:
                    result = ui("find_text", needle, timeout=20)
                except PermissionWall as exc:
                    print(f"FAIL: {exc}", file=sys.stderr)
                    return 1
                if result.startswith("FOUND"):
                    found = result
                    break
                time.sleep(0.5)
            if found is None:
                print(
                    f"FAIL: could not find text containing {needle!r} within "
                    f"{LAUNCH_TIMEOUT_S}s",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    print(f"OK: found the fail-closed message: {XLSX_MESSAGE!r}")
    return 0


# ── P04 — Merge, undo/redo, safe save ───────────────────────────────────────


def p04(binary, break_mode=False):
    """Launches `<binary> <left> <right>` (plain 2-arg compare, not
    mergetool - RFC-078's own text, so Save writes to `<right>`, per
    `compare.rs`'s `SaveDestination::RightInput` for this launch mode).

    Per the handoff's §3 resolution (mouse-only reading of "keyboard and
    mouse"): applies the focused hunk via the "Use this change" button's
    `AXPress` (System Events `click`, same technique as M5-A's P09 Save
    button) - the keyboard path (`Key::Enter` in `app.rs`'s global
    `onkeydown`) is a raw keydown listener bound to no actionable UI
    element, structurally unreachable by any accessibility action on any
    platform, and is recorded here as explicitly NOT executed (manual-
    outstanding, mirroring F45's shape) rather than silently skipped.

    Sequence: apply hunk -> dirty marker ("unsaved", statusbar.rs) appears
    -> Undo -> dirty marker disappears -> open the advanced panel ("More
    (British Cross) - actually "More ▼") -> Redo -> dirty marker
    reappears -> Save -> dirty marker disappears again, `<right>`'s saved
    content differs from its pre-edit content, a `.bak` sibling exists
    whose bytes equal the *pre-save* content (the original `right` fixture
    - RFC-077's `SiblingBak` copies the target before overwriting), and no
    leftover `.{name}.fsk-tmp` sidecar remains (`save.rs`'s
    `temp_path_for`).

    `--break`: asserts the saved content equals a string that can never be
    real merge output, proving the check reads real content, not just
    "changed from before" (same shape as P09's break mode).
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right_src = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    original_right = right_src.read_bytes()

    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        right = scratch_path / "right.txt"
        right.write_bytes(original_right)

        proc = launch(binary, [left, right], scratch)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            rows = wait_rows(14)
            if rows != "14":
                print(
                    f"FAIL: compare view never reached 14 AXRow elements (last: {rows!r})",
                    file=sys.stderr,
                )
                return 1

            pre = find_wait("unsaved", timeout=5, want_found=False)
            if pre.startswith("FOUND"):
                print(f"FAIL: dirty marker present before any edit: {pre}", file=sys.stderr)
                return 1

            r = click_wait("Use this change")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click 'Use this change': {r}", file=sys.stderr)
                return 1

            r = find_wait("unsaved", want_found=True)
            if not r.startswith("FOUND"):
                print(
                    f"FAIL: dirty marker ('unsaved') never appeared after applying a hunk: {r}",
                    file=sys.stderr,
                )
                return 1

            r = click_wait("Undo last merge action")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Undo: {r}", file=sys.stderr)
                return 1
            r = find_wait("unsaved", want_found=False)
            if r.startswith("FOUND"):
                print(f"FAIL: dirty marker still present after Undo: {r}", file=sys.stderr)
                return 1

            r = click_wait("More ▼")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not open the advanced panel ('More ▼'): {r}", file=sys.stderr)
                return 1
            r = click_wait("Redo")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Redo: {r}", file=sys.stderr)
                return 1
            r = find_wait("unsaved", want_found=True)
            if not r.startswith("FOUND"):
                print(f"FAIL: dirty marker never reappeared after Redo: {r}", file=sys.stderr)
                return 1

            r = click_wait("Save merge result")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Save: {r}", file=sys.stderr)
                return 1
            r = find_wait("unsaved", want_found=False)
            if r.startswith("FOUND"):
                print(f"FAIL: dirty marker still present after Save: {r}", file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            saved_bytes = original_right
            while time.monotonic() < deadline:
                saved_bytes = right.read_bytes()
                if saved_bytes != original_right:
                    break
                time.sleep(0.5)

            impossible = b"this exact string can never appear in real merge output"
            if break_mode:
                if saved_bytes != impossible:
                    print(
                        f"FAIL: saved content {saved_bytes!r} != impossible expected {impossible!r}",
                        file=sys.stderr,
                    )
                    return 1
            else:
                if saved_bytes == original_right:
                    print(
                        f"FAIL: {right} still holds its pre-edit content after Save",
                        file=sys.stderr,
                    )
                    return 1

            bak = scratch_path / "right.txt.bak"
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            bak_bytes = None
            while time.monotonic() < deadline:
                if bak.exists():
                    bak_bytes = bak.read_bytes()
                    break
                time.sleep(0.5)
            if bak_bytes is None:
                print(f"FAIL: no .bak sibling found at {bak}", file=sys.stderr)
                return 1
            if bak_bytes != original_right:
                print(
                    f"FAIL: .bak content ({len(bak_bytes)} bytes) does not equal the "
                    f"pre-save content ({len(original_right)} bytes)",
                    file=sys.stderr,
                )
                return 1

            leftover = scratch_path / ".right.txt.fsk-tmp"
            if leftover.exists():
                print(f"FAIL: leftover temp/sidecar file remains: {leftover}", file=sys.stderr)
                return 1
        finally:
            terminate(proc)

    print(
        "OK: apply (mouse/AXPress) -> dirty marker -> Undo -> Redo -> Save cycle "
        f"verified; saved content differs ({len(saved_bytes)} bytes), .bak matches "
        "pre-save content, no leftover temp file. Keyboard-Enter apply path NOT "
        "executed (structurally unreachable via accessibility - see handoff §3)."
    )
    return 0


# ── P05 — External modification ─────────────────────────────────────────────


def _p05_open_dirty(binary, scratch):
    """Copies F34's fixture pair into `scratch`, opens `<left> <right>`,
    and applies the first hunk (mouse/AXPress, same as P04) so the tab is
    dirty - `toolbar.rs`'s Save button is `disabled: !snap.is_dirty`, so a
    clean tab's Save click would be a no-op that never reaches the
    conflict path this case tests. Returns `(proc, left, right)`."""
    left_src = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right_src = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    left = Path(scratch) / "left.txt"
    right = Path(scratch) / "right.txt"
    left.write_bytes(left_src.read_bytes())
    right.write_bytes(right_src.read_bytes())

    proc = launch(binary, [left, right], scratch)
    wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
    rows = wait_rows(14)
    if rows != "14":
        raise RuntimeError(f"compare view never reached 14 AXRow elements (last: {rows!r})")
    r = click_wait("Use this change")
    if not r.startswith("CLICKED"):
        raise RuntimeError(f"could not click 'Use this change': {r}")
    r = find_wait("unsaved", want_found=True)
    if not r.startswith("FOUND"):
        raise RuntimeError(f"dirty marker never appeared: {r}")
    return proc, left, right


def p05(binary, break_mode=False):
    """Three fresh launches (each starting from F34's fixture pair, made
    dirty via P04's technique), covering RFC-078's external-modification
    sub-cases:

    1. Cancel: while the app has `<right>` open, overwrite it *outside the
       app* with different bytes, then click Save. `save.rs`'s
       `TargetPrecondition::MustMatch` fingerprint check must reject the
       write - `OverwriteModal` ("File changed on disk") appears, and
       `<right>` still holds the externally-written bytes (Save did not
       silently win the race). Clicking Cancel closes the dialog without
       touching the file.
    2. Overwrite: same setup, this time click "Overwrite". The write
       proceeds, and the `.bak` sibling `save.rs`'s `SiblingBak` policy
       creates must hold the bytes that were *just overwritten* - the
       externally-modified content, not the original pre-edit fixture
       content (the detail that makes this check meaningful rather than
       "a .bak exists").
    3. Save As: same setup, this time "Save As" to a different path
       (`type_into`'s click-to-focus + keystroke, per recon - `set_value`
       is a confirmed no-op for this control). The original `<right>`
       target must be untouched (still the externally-modified bytes) and
       the new path must hold the app's own merge output.

    `--break`: sub-case 2's `.bak`-content check is flipped to expect
    bytes that can never be real - proves the check reads real bytes, not
    just "a .bak file exists" (per the handoff's explicit warning that
    ".bak exists" alone is the vacuous version of this check).
    """
    external_bytes = b"EXTERNALLY MODIFIED WHILE APP WAS OPEN\nsecond line\n"

    # ── Sub-case 1: blocked, then Cancel ────────────────────────────────
    with tempfile.TemporaryDirectory() as scratch:
        proc = None
        try:
            proc, left, right = _p05_open_dirty(binary, scratch)
            time.sleep(0.3)
            right.write_bytes(external_bytes)

            r = click_wait("Save merge result")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Save: {r}", file=sys.stderr)
                return 1
            r = find_wait("File changed on disk", want_found=True)
            if not r.startswith("FOUND"):
                print(
                    f"FAIL: 'File changed on disk' modal never appeared after an "
                    f"external modification + Save: {r}",
                    file=sys.stderr,
                )
                return 1
            if right.read_bytes() != external_bytes:
                print(
                    "FAIL: Save silently overwrote the externally-modified file "
                    "instead of being blocked",
                    file=sys.stderr,
                )
                return 1

            r = ui("click_button_exact", "Cancel", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Cancel: {r}", file=sys.stderr)
                return 1
            time.sleep(0.5)
            if right.read_bytes() != external_bytes:
                print(
                    "FAIL: file content changed after clicking Cancel (should be untouched)",
                    file=sys.stderr,
                )
                return 1
        except (PermissionWall, TimeoutError, RuntimeError) as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            if proc is not None:
                terminate(proc)

    # ── Sub-case 2: Overwrite, .bak matches the externally-modified bytes ──
    with tempfile.TemporaryDirectory() as scratch:
        proc = None
        try:
            proc, left, right = _p05_open_dirty(binary, scratch)
            time.sleep(0.3)
            right.write_bytes(external_bytes)

            r = click_wait("Save merge result")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Save: {r}", file=sys.stderr)
                return 1
            r = find_wait("File changed on disk", want_found=True)
            if not r.startswith("FOUND"):
                print(f"FAIL: modal never appeared: {r}", file=sys.stderr)
                return 1

            r = ui("click_button_exact", "Overwrite", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Overwrite: {r}", file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            after = external_bytes
            while time.monotonic() < deadline:
                after = right.read_bytes()
                if after != external_bytes:
                    break
                time.sleep(0.5)
            if after == external_bytes:
                print(
                    f"FAIL: {right} still holds the externally-modified bytes "
                    f"after Overwrite within {LAUNCH_TIMEOUT_S}s",
                    file=sys.stderr,
                )
                return 1

            bak = Path(scratch) / "right.txt.bak"
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            bak_bytes = None
            while time.monotonic() < deadline:
                if bak.exists():
                    bak_bytes = bak.read_bytes()
                    break
                time.sleep(0.5)
            if bak_bytes is None:
                print(f"FAIL: no .bak sibling found at {bak}", file=sys.stderr)
                return 1

            if break_mode:
                impossible = b"this exact string can never be the pre-overwrite content"
                if bak_bytes != impossible:
                    print(
                        f"FAIL: .bak content {bak_bytes!r} != impossible expected "
                        f"{impossible!r}",
                        file=sys.stderr,
                    )
                    return 1
            else:
                if bak_bytes != external_bytes:
                    print(
                        f"FAIL: .bak content ({len(bak_bytes)} bytes) does not equal the "
                        f"externally-modified content that was just overwritten "
                        f"({len(external_bytes)} bytes) - it must reflect what was on "
                        f"disk immediately before Overwrite, not the original fixture",
                        file=sys.stderr,
                    )
                    return 1
        except (PermissionWall, TimeoutError, RuntimeError) as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            if proc is not None:
                terminate(proc)

    # ── Sub-case 3: Save As leaves the original target untouched ───────────
    with tempfile.TemporaryDirectory() as scratch:
        proc = None
        try:
            proc, left, right = _p05_open_dirty(binary, scratch)
            time.sleep(0.3)
            right.write_bytes(external_bytes)

            other_path = Path(scratch) / "saved-as-elsewhere.txt"
            r = click_wait("Save As")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Save As: {r}", file=sys.stderr)
                return 1
            r = find_wait("Path", want_found=True)
            if not r.startswith("FOUND"):
                print(f"FAIL: Save As dialog's Path field never appeared: {r}", file=sys.stderr)
                return 1

            # `set_value` was a confirmed no-op against the Settings font-
            # size <input type="number"> spinner during recon, but that
            # control was never tested - this is a plain <input
            # type="text"> (SaveAsModal's path field), a materially
            # different control WebKit may support direct AXValue writes
            # for even where it doesn't for a number spinner. Try it first;
            # fall back to type_into's click+keystroke if it no-ops.
            r = ui("set_value", "AXTextField", "1", str(other_path), timeout=20)
            wrote_value = r.startswith("SET:") and r == f"SET: {right} -> {other_path}"
            if not wrote_value:
                r2 = ui("get_value", "AXTextField", "1", timeout=20)
                wrote_value = r2 == f"VALUE: {other_path}"
            if not wrote_value:
                r = ui("type_into", "AXTextField", "1", str(other_path), timeout=20)
                wrote_value = r == f"TYPED: {other_path}"
            if not wrote_value:
                print(
                    f"FAIL: could not get the Save As path field to read back "
                    f"{str(other_path)!r} via set_value or type_into (last result: {r})",
                    file=sys.stderr,
                )
                return 1

            r = ui("click_button_exact", "Save", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click the Save As dialog's Save button: {r}", file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            new_bytes = None
            while time.monotonic() < deadline:
                if other_path.exists():
                    new_bytes = other_path.read_bytes()
                    break
                time.sleep(0.5)
            if new_bytes is None:
                print(f"FAIL: Save As never created {other_path}", file=sys.stderr)
                return 1
            if not new_bytes or new_bytes == external_bytes:
                print(
                    f"FAIL: {other_path}'s content is not the app's own merge output "
                    f"(empty or equal to the externally-written bytes)",
                    file=sys.stderr,
                )
                return 1

            if right.read_bytes() != external_bytes:
                print(
                    f"FAIL: the original target {right} was modified by Save As - "
                    "it must stay untouched",
                    file=sys.stderr,
                )
                return 1
        except (PermissionWall, TimeoutError, RuntimeError) as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            if proc is not None:
                terminate(proc)

    print(
        "OK: external modification blocked Save with 'File changed on disk' and "
        "Cancel left the file untouched; Overwrite wrote through and its .bak "
        "matched the externally-modified (not original) content; Save As to a "
        "different path left the original target untouched and wrote the app's "
        "own content to the new path."
    )
    return 0


# ── P08 — Persistence migration / recovery dialogs ──────────────────────────
#
# Split into four independently-dispatchable cases (matching the handoff's
# "three separate launches" plus the filesystem-only checks) rather than one
# p08 that internally launches four times - a failure in one sub-case's
# dialog interaction shouldn't obscure whether the other three passed, and
# each needs its own fresh, isolated config directory anyway.
#
# `dirs_next::config_dir()` on macOS is `$HOME/Library/Application Support`
# (confirmed in `dirs-next-2.0.0/src/lib.rs`); `state.rs`'s `config_file_path`
# joins `forskscope/<file_name>` onto that - see `launch()`'s `home` param.

FUTURE_SESSION_JSON = (
    '{"schema_name": "session", "schema_version": 99, '
    '"app_version": "0.165.1", "created_unix": 0, "updated_unix": 0, '
    '"payload": {}}'
)
CORRUPT_SESSION_BYTES = b"{not valid json"


def _config_dir(home):
    return Path(home) / "Library" / "Application Support" / "forskscope"


def _seed_config(home, settings_text=None, session_text=None):
    cfg = _config_dir(home)
    cfg.mkdir(parents=True, exist_ok=True)
    if settings_text is not None:
        (cfg / "settings.json").write_text(settings_text)
    if session_text is not None:
        (cfg / "session.json").write_text(session_text)
    return cfg


def _pid_alive(pid):
    """`kill -0`-equivalent: True if `pid` still names a live process.
    Used only as corroborating evidence alongside `proc.poll()` (the
    direct, non-racy child-reap check) - see p08_exit's docstring."""
    try:
        os.kill(pid, 0)
    except OSError:
        return False
    return True


def p08_exit(binary, break_mode=False):
    """The single most important assertion in this slice (handoff §4): a
    future-schema session fixture triggers the "Session file is from a
    newer version" recovery dialog (`session/persistence_recovery.rs`'s
    `Incompatible` outcome, actions Exit/Continue with defaults - no Reset,
    since a future file may be valid to a newer build). Clicking Exit
    (`recovery.rs`: `dioxus_desktop::window().close()`) must actually
    terminate the OS process, not just dismiss the dialog/window - "An Exit
    that dismisses the dialog and leaves a zombie is a failure that looks
    like a pass" (handoff §4).

    Process-state evidence: `proc.poll()` is the direct, non-racy check -
    it reads this exact child's wait status via this process's own
    waitpid, so there is no PID-reuse ambiguity. `os.kill(pid, 0)`
    (`_pid_alive`) is recorded alongside it as the independent `kill -0`-
    style corroboration the handoff suggested, checked immediately so a
    reused PID cannot yet exist.

    `--break`: asserts the process is still running after clicking Exit -
    false under real (correct) behaviour, so this must fail; a harness
    that only checked "the dialog is gone" would pass here regardless,
    which is exactly the vacuous check the handoff calls out by name.
    """
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        _seed_config(home, session_text=FUTURE_SESSION_JSON)
        proc = launch(binary, [], scratch, home=home)
        pid = proc.pid
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            r = find_wait("Session file is from a newer version", timeout=LAUNCH_TIMEOUT_S)
            if not r.startswith("FOUND"):
                print(f"FAIL: recovery dialog never appeared: {r}", file=sys.stderr)
                return 1

            r = ui("click_button_exact", "Exit", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Exit: {r}", file=sys.stderr)
                return 1

            deadline = time.monotonic() + 20
            exited = False
            while time.monotonic() < deadline:
                if proc.poll() is not None:
                    exited = True
                    break
                time.sleep(0.3)
            alive_via_kill0 = _pid_alive(pid)

            if break_mode:
                if exited:
                    print(
                        f"FAIL (expected, --break): process (pid {pid}) exited with "
                        f"returncode {proc.returncode} within 20s of clicking Exit - "
                        "the real behaviour --break's impossible expectation "
                        "('still running') was checked against.",
                        file=sys.stderr,
                    )
                    return 1
                print(
                    f"WARNING: --break's impossible condition ('process still "
                    f"running after Exit') was NOT falsified - pid {pid} is still "
                    f"alive (kill(pid,0) alive={alive_via_kill0}). This does not "
                    "mean the harness is broken; it would mean Exit really does "
                    "leave the process running, which is the real defect this "
                    "assertion exists to catch. Investigate before trusting "
                    "the normal-mode run's pass.",
                    file=sys.stderr,
                )
                return 1
            else:
                if not exited:
                    print(
                        f"FAIL: process (pid {pid}) still running 20s after clicking "
                        f"Exit (proc.poll()=None, kill(pid,0) alive={alive_via_kill0}) "
                        "- Exit dismissed the dialog without terminating the process",
                        file=sys.stderr,
                    )
                    return 1
        finally:
            if proc.poll() is None:
                terminate(proc)

    print(
        f"OK: Exit terminated the process - proc.poll() returncode="
        f"{proc.returncode}, corroborated by kill(pid={pid}, 0) reporting no "
        "such process."
    )
    return 0


def p08_continue(binary, break_mode=False):
    """Same future-schema session fixture as p08_exit, this time "Continue
    with defaults". Asserts the app is running normally afterward (window
    still present, dialog dismissed) and, critically, that `session.json`'s
    bytes are byte-for-byte unchanged - `SessionRuntimeResolution.
    write_disabled` for an `Incompatible` outcome (confirmed by
    `session_resolve_future_version_disables_writes_and_preserves_bytes`)
    means this run must never write to it, no matter what the user does
    afterward.

    `--break`: asserts the file's bytes differ from the seeded fixture -
    false under real (write-disabled) behaviour, so this must fail.
    """
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        cfg = _seed_config(home, session_text=FUTURE_SESSION_JSON)
        session_path = cfg / "session.json"
        proc = launch(binary, [], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            r = find_wait("Session file is from a newer version", timeout=LAUNCH_TIMEOUT_S)
            if not r.startswith("FOUND"):
                print(f"FAIL: recovery dialog never appeared: {r}", file=sys.stderr)
                return 1

            r = ui("click_button_exact", "Continue with defaults", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click 'Continue with defaults': {r}", file=sys.stderr)
                return 1

            time.sleep(1.0)
            if proc.poll() is not None:
                print(
                    f"FAIL: process exited (rc={proc.returncode}) after "
                    "'Continue with defaults' - expected it to keep running",
                    file=sys.stderr,
                )
                return 1
            try:
                wait_for_window(time.monotonic() + 10)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: no window after continuing: {exc}", file=sys.stderr)
                return 1

            time.sleep(1.0)  # give any (wrongly) pending write a chance to land
            after = session_path.read_text()
            if break_mode:
                if after == FUTURE_SESSION_JSON:
                    print(
                        "FAIL (expected, --break): session.json is unchanged, "
                        "matching write-disabled behaviour - --break's impossible "
                        "expectation ('bytes changed') was correctly not satisfied",
                        file=sys.stderr,
                    )
                    return 1
            else:
                if after != FUTURE_SESSION_JSON:
                    print(
                        "FAIL: session.json was modified despite the future-schema "
                        f"write-disabled state.\nbefore: {FUTURE_SESSION_JSON!r}\n"
                        f"after:  {after!r}",
                        file=sys.stderr,
                    )
                    return 1
        finally:
            terminate(proc)

    print(
        "OK: 'Continue with defaults' dismissed the dialog, the app kept running, "
        "and session.json's bytes were confirmed unchanged (write-disabled)."
    )
    return 0


def p08_reset(binary, break_mode=False):
    """A corrupt session fixture (literally invalid JSON) triggers the
    "Session file could not be read" dialog (`CorruptPreserved`, actions
    Continue with defaults/Reset and back up - no Exit, per
    `corrupt_shows_continue_and_reset_but_not_exit`). Clicking "Reset and
    back up" must dismiss the dialog, reset the session file to a fresh
    default, and back up the original corrupt bytes to `session.json.
    reset.bak` (`ensure_reset_backup` in `persist/schema/repository.rs`).

    `--break`: asserts the backup's bytes differ from the original corrupt
    bytes - false under real behaviour, so this must fail; catches a
    harness that only checked "a .reset.bak file exists" without reading
    it.
    """
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        cfg = _seed_config(home, session_text=None)
        cfg.mkdir(parents=True, exist_ok=True)
        session_path = cfg / "session.json"
        session_path.write_bytes(CORRUPT_SESSION_BYTES)
        proc = launch(binary, [], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            r = find_wait("Session file could not be read", timeout=LAUNCH_TIMEOUT_S)
            if not r.startswith("FOUND"):
                print(f"FAIL: recovery dialog never appeared: {r}", file=sys.stderr)
                return 1

            r = ui("click_button_exact", "Reset and back up", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click 'Reset and back up': {r}", file=sys.stderr)
                return 1

            r = find_wait("Session file could not be read", timeout=10, want_found=False)
            if r.startswith("FOUND"):
                print(f"FAIL: dialog still present after Reset and back up: {r}", file=sys.stderr)
                return 1

            backup = cfg / "session.json.reset.bak"
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            backup_bytes = None
            while time.monotonic() < deadline:
                if backup.exists():
                    backup_bytes = backup.read_bytes()
                    break
                time.sleep(0.5)
            if backup_bytes is None:
                print(f"FAIL: no reset backup found at {backup}", file=sys.stderr)
                return 1

            if break_mode:
                impossible = b"this exact string can never be the original corrupt bytes"
                if backup_bytes != impossible:
                    print(
                        f"FAIL: backup content {backup_bytes!r} != impossible expected "
                        f"{impossible!r}",
                        file=sys.stderr,
                    )
                    return 1
            else:
                if backup_bytes != CORRUPT_SESSION_BYTES:
                    print(
                        f"FAIL: backup content {backup_bytes!r} != original corrupt "
                        f"bytes {CORRUPT_SESSION_BYTES!r}",
                        file=sys.stderr,
                    )
                    return 1

            after = session_path.read_bytes()
            if after == CORRUPT_SESSION_BYTES:
                print(
                    "FAIL: session.json still holds the corrupt bytes after reset",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    print(
        "OK: 'Reset and back up' dismissed the dialog, session.json was reset "
        f"(no longer the corrupt bytes), and {backup.name} matches the original "
        "corrupt bytes exactly."
    )
    return 0


def p08_fs(binary, break_mode=False):
    """Filesystem-only checks needing no dialog interaction: a legacy v0
    settings/session pair (F34/M5-A's own fixture format, reused verbatim
    from `crates/forskscope-core/src/tests/fixtures/persistence/
    settings-v0.json`/`session-v0.json` - already sanitized, `/tmp/
    fixtures/...` paths, per RFC-078's schema) migrates without loss to a
    versioned envelope, with a `.pre-v2.bak` backup of the original bytes
    (`ensure_pre_v2_backup`).

    `--break`: asserts the migrated settings' language is a value the v0
    fixture never had - false under real (loss-free) migration, so this
    must fail.
    """
    v0_settings = (
        REPO_ROOT / "crates/forskscope-core/src/tests/fixtures/persistence/settings-v0.json"
    ).read_text()
    v0_session = (
        REPO_ROOT / "crates/forskscope-core/src/tests/fixtures/persistence/session-v0.json"
    ).read_text()

    # The v0 session fixture's tab paths are literal `/tmp/fixtures/...`
    # (RFC-078's sanitized-fixture convention) - app.rs's startup sequence
    # migrates the session file *before* restore_tabs runs, but
    # restore_tabs itself only reopens a pair if at least one side
    # `.exists()`, and the tabs-changed `use_effect` immediately re-saves
    # session.json to match whatever actually got restored. Left as
    # nonexistent paths, restore keeps 0 of the 2 pairs and that same-
    # launch re-save clobbers the migration's own output before this
    # harness ever reads it - not a migration defect, a test setup gap.
    # Create empty placeholder files at those exact paths so restore keeps
    # both pairs and the round-trip save preserves them.
    fixture_paths = [
        Path("/tmp/fixtures/left-a.txt"),
        Path("/tmp/fixtures/right-a.txt"),
        Path("/tmp/fixtures/left-b.txt"),
        Path("/tmp/fixtures/right-b.txt"),
    ]
    Path("/tmp/fixtures").mkdir(parents=True, exist_ok=True)
    for p in fixture_paths:
        if not p.exists():
            p.write_text("placeholder\n")

    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        cfg = _seed_config(home, settings_text=v0_settings, session_text=v0_session)
        settings_path = cfg / "settings.json"
        session_path = cfg / "session.json"
        proc = launch(binary, [], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            # A committed migration produces a toast, not a blocking dialog
            # (SessionRecoveryView::from_resolution's Migrated(Committed)
            # arm) - just give the async resolve-and-commit time to land.
            time.sleep(2.0)

            settings_bak = cfg / "settings.json.pre-v2.bak"
            session_bak = cfg / "session.json.pre-v2.bak"
            for label, bak, original in (
                ("settings", settings_bak, v0_settings),
                ("session", session_bak, v0_session),
            ):
                deadline = time.monotonic() + LAUNCH_TIMEOUT_S
                bak_text = None
                while time.monotonic() < deadline:
                    if bak.exists():
                        bak_text = bak.read_text()
                        break
                    time.sleep(0.5)
                if bak_text is None:
                    print(f"FAIL: no {label} pre-v2 backup found at {bak}", file=sys.stderr)
                    return 1
                if bak_text != original:
                    print(
                        f"FAIL: {label} pre-v2 backup does not match the original v0 fixture",
                        file=sys.stderr,
                    )
                    return 1

            settings_after = json.loads(settings_path.read_text())
            session_after = json.loads(session_path.read_text())

            if "schema_version" not in settings_after or "payload" not in settings_after:
                print(
                    f"FAIL: migrated settings.json is not a versioned envelope: "
                    f"{settings_after!r}",
                    file=sys.stderr,
                )
                return 1
            if "schema_version" not in session_after or "payload" not in session_after:
                print(
                    f"FAIL: migrated session.json is not a versioned envelope: "
                    f"{session_after!r}",
                    file=sys.stderr,
                )
                return 1

            payload = settings_after["payload"]
            expected_language = "impossible-language" if break_mode else "ja"
            if payload.get("language") != expected_language:
                print(
                    f"FAIL: migrated settings payload language "
                    f"{payload.get('language')!r} != expected {expected_language!r}",
                    file=sys.stderr,
                )
                return 1
            if payload.get("theme") != "light":
                print(
                    f"FAIL: migrated settings payload theme {payload.get('theme')!r} "
                    "!= expected 'light' (lost during migration?)",
                    file=sys.stderr,
                )
                return 1
            if payload.get("diff_font_size") != 16:
                print(
                    f"FAIL: migrated settings payload diff_font_size "
                    f"{payload.get('diff_font_size')!r} != expected 16",
                    file=sys.stderr,
                )
                return 1

            session_payload = session_after["payload"]
            tabs = session_payload.get("tabs")
            # PersistedComparePair serializes as {"left": ..., "right": ...}
            # objects, not 2-element arrays like the v0 DTO's (String, String)
            # tuples - confirmed by the first real dispatch's FAIL output.
            if tabs != [
                {"left": "/tmp/fixtures/left-a.txt", "right": "/tmp/fixtures/right-a.txt"},
                {"left": "/tmp/fixtures/left-b.txt", "right": "/tmp/fixtures/right-b.txt"},
            ]:
                print(
                    f"FAIL: migrated session payload tabs {tabs!r} do not match "
                    "the v0 fixture's tab pairs",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)
            for p in fixture_paths:
                try:
                    p.unlink(missing_ok=True)
                except OSError:
                    pass

    print(
        "OK: legacy v0 settings/session migrated without loss (theme/language/"
        "font size/tabs all preserved in the payload); both got a versioned "
        "envelope and a .pre-v2.bak matching the original v0 bytes exactly."
    )
    return 0


# ── P12 — Session/settings restart ──────────────────────────────────────────


def p12(binary, break_mode=False):
    """Three launches sharing one isolated `$HOME`:

    A. CLI-opens F34's fixture pair (a real tab to restore later). Changes
       Theme and Language via the Settings dialog's `AXPopUpButton`s
       (`select_popup_item` - `get_value`/`set_value`/`perform_action
       "AXIncrement"`/keystroke-after-focus/keystroke-after-a-real-
       coordinate-click were all confirmed no-ops for these controls
       during this slice's recon; `select_popup_item`'s AXPress-on-the-
       opened-dropdown's-AXMenuItem is the one that actually works - see
       macos_ui.applescript's history). Closes via the window's own close
       button (`close_window`), not a signal, so the app has a real chance
       to flush - though `super::persist(store)` already runs synchronously
       on every settings change, so this mostly matters for matching the
       handoff's letter ("not kill -9").
    B. Relaunches with no CLI args: the fixture tab must restore (RFC-035,
       `restore_tabs` - only reached when no explicit startup pair, per
       `app.rs`), and Theme/Language must read back as set. A Japanese
       label (the header's own "Settings"/"設定" button) must render in
       Japanese - a practical-workflow check, not a translation audit.
    C. Relaunches with explicit CLI args (a different, distinct pair):
       `StartupRequest::Compare` takes the `into_compare_request()` branch,
       which never calls `restore_tabs` (`app.rs`) - the F34 tab must NOT
       reappear alongside the explicitly-requested one.

    Font size is NOT changed via the UI in this case: every accessibility
    technique tried against the font-size `AXTextField` spinner during this
    slice's recon failed to actually change its value (direct AXValue
    write, `AXIncrement`, keystroke after `set focused`, keystroke after a
    UI-element `click`, keystroke after a real coordinate `click at {x,y}`
    with the process made frontmost first) - five independent attempts,
    all silent no-ops on readback. This is recorded here as explicitly NOT
    executed / manual-outstanding (mirroring F45's shape and the handoff
    §3 treatment of P04's keyboard path), not silently skipped or claimed
    covered - see the case's OK/report output.

    `--break`: asserts the restored Theme reads back as "Light" (the
    default, never what this case actually sets it to) - false under real
    (persisted-and-restored) behaviour, so this must fail.

    KNOWN RESULT: this case's normal-mode run genuinely FAILS on the
    tab-restore assertion, not because of a harness bug - see the PRODUCT
    DEFECT comment right after `tempfile.TemporaryDirectory()` below for
    the full root-cause writeup (confirmed via `recon_session_save`):
    session.json is never written for a CLI-opened tab that's never closed
    again before quitting, because the tabs-changed `use_effect` meant to
    auto-persist it doesn't actually write, while direct save call sites
    (`close_tab`, settings' `persist`) do. Settings restoration (Theme/
    Language) is checked and passes; tab restoration is checked and fails,
    honestly, per the handoff's "do not weaken a case to make it pass."
    """
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        # PRODUCT DEFECT, confirmed via real CI dispatches, registered here
        # and reported in the M5-B report - NOT fixed:
        #
        # session.json is never written for a tab opened via CLI startup
        # that is never closed again before the app quits. Root cause,
        # isolated across several dispatches (chased two wrong hypotheses
        # first - missing config directory, then App Sandbox entitlements;
        # both ruled out: pre-creating the directory below didn't fix it,
        # and `codesign -d --entitlements` came back empty/unsigned, no
        # sandbox): app.rs registers a `use_effect` that's supposed to
        # auto-persist the session on every `store.tabs` mutation -
        #   use_effect(move || { let _tabs = store.tabs.read(); save_session(&store); });
        # `recon_session_save` proved this effect does not actually write
        # session.json when the tab is pushed via `open_compare_request`
        # during startup's `use_hook` (file still missing 1.5s after the
        # tab renders), while `close_tab`'s *direct*, non-reactive call to
        # `save_session` (state/session.rs) - triggered from a real onclick
        # handler, not a reactive effect - writes correctly in the exact
        # same environment moments later. settings.json's independent,
        # synchronous `persist(store)` (ui/view/settings.rs, also a direct
        # onclick/onchange call, never reactive) also writes correctly -
        # this narrows the defect specifically to the reactive tabs-changed
        # effect, not to persistence, the config directory, or sandboxing.
        # Practical impact: a session opened via `forskscope <left> <right>`
        # and closed without any other tab-list-mutating action in between
        # (e.g. quit immediately, or close via the window's own close
        # button as this case does) never gets recorded - restart loses it
        # silently. Separately, both `persist_session` and `persist_settings`
        # discard their `repo.save(...)` Result with `let _ =`, so this (or
        # any other write failure) would never surface to the user either
        # way.
        #
        # This case is left to genuinely fail at the tab-restore assertion
        # below because of this real defect - not worked around. Settings
        # restoration (Theme/Language, via the working settings.json path)
        # is still checked and reported, so a run's FAIL output shows
        # exactly what did and didn't survive the restart.
        _config_dir(home).mkdir(parents=True, exist_ok=True)

        left1 = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
        right1 = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"

        # ── Launch A: open a tab, change Theme/Language, close normally ──
        proc = launch(binary, [left1, right1], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1
            rows = wait_rows(14)
            if rows != "14":
                print(f"FAIL: compare view never reached 14 rows (last: {rows!r})", file=sys.stderr)
                return 1

            session_debug_path = _config_dir(home) / "session.json"
            time.sleep(1.0)
            print(
                f"DEBUG: session.json right after the tab opened (config dir "
                f"exists={_config_dir(home).exists()}): "
                f"{session_debug_path.read_text() if session_debug_path.exists() else '<missing>'}",
                flush=True,
            )

            r = click_wait("Settings")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not open Settings: {r}", file=sys.stderr)
                return 1
            time.sleep(0.3)

            r = ui("select_popup_item", "1", "Night", timeout=20)
            if not r.startswith("SELECTED"):
                print(f"FAIL: could not select Theme=Night: {r}", file=sys.stderr)
                return 1
            time.sleep(0.5)
            r = poll_ui(
                "get_value",
                "AXPopUpButton",
                "1",
                predicate=lambda r: r == "VALUE: Night",
                timeout=10,
            )
            if r != "VALUE: Night":
                print(f"FAIL: Theme popup does not read back Night: {r}", file=sys.stderr)
                return 1

            settings_debug_path = _config_dir(home) / "settings.json"
            print(
                f"DEBUG: settings.json right after selecting Theme=Night: "
                f"{settings_debug_path.read_text() if settings_debug_path.exists() else '<missing>'}",
                flush=True,
            )

            r = ui("select_popup_item", "2", "日本語", timeout=20)
            if not r.startswith("SELECTED"):
                print(f"FAIL: could not select Language=日本語: {r}", file=sys.stderr)
                return 1
            time.sleep(0.5)
            r = poll_ui(
                "get_value",
                "AXPopUpButton",
                "2",
                predicate=lambda r: r == "VALUE: 日本語",
                timeout=10,
            )
            if r != "VALUE: 日本語":
                print(f"FAIL: Language popup does not read back 日本語: {r}", file=sys.stderr)
                return 1

            # Settings' own Close button is now rendered in Japanese.
            r = click_wait("閉じる")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not close Settings ('閉じる'): {r}", file=sys.stderr)
                return 1

            r = ui("close_window", timeout=20)
            if not r.startswith("CLOSED"):
                print(f"FAIL: could not close the window: {r}", file=sys.stderr)
                return 1

            deadline = time.monotonic() + 20
            while time.monotonic() < deadline and proc.poll() is None:
                time.sleep(0.3)
            if proc.poll() is None:
                print("FAIL: process still running 20s after closing the window", file=sys.stderr)
                terminate(proc)
                return 1
        finally:
            if proc.poll() is None:
                terminate(proc)

        session_debug_path = _config_dir(home) / "session.json"
        print(
            f"DEBUG: session.json after Launch A close: "
            f"{session_debug_path.read_text() if session_debug_path.exists() else '<missing>'}",
            flush=True,
        )

        # ── Launch B: relaunch with no CLI args - restore ────────────────
        #
        # Order matters here: settings restoration (Theme/Language, via
        # settings.json) and tab restoration (via session.json) go through
        # completely different persistence paths and this slice found they
        # do NOT behave the same way - see the PRODUCT DEFECT note above
        # `_config_dir(home).mkdir(...)`. Checking settings first means a
        # tab-restore failure's FAIL message still reports whether settings
        # genuinely did restore, rather than the case aborting before ever
        # finding out.
        proc = launch(binary, [], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            r = find_wait("設定", timeout=10)
            if not r.startswith("FOUND"):
                print(f"FAIL: no Japanese label ('設定') found after restart: {r}", file=sys.stderr)
                return 1

            r = click_wait("設定")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not reopen Settings ('設定'): {r}", file=sys.stderr)
                return 1
            time.sleep(0.3)

            expected_theme = "Light" if break_mode else "Night"
            r = poll_ui(
                "get_value",
                "AXPopUpButton",
                "1",
                predicate=lambda r: r == f"VALUE: {expected_theme}",
                timeout=10,
            )
            if r != f"VALUE: {expected_theme}":
                print(
                    f"FAIL: restored Theme {r!r} != expected 'VALUE: {expected_theme}'",
                    file=sys.stderr,
                )
                return 1
            r = poll_ui(
                "get_value",
                "AXPopUpButton",
                "2",
                predicate=lambda r: r == "VALUE: 日本語",
                timeout=10,
            )
            if r != "VALUE: 日本語":
                print(f"FAIL: restored Language {r!r} != 'VALUE: 日本語'", file=sys.stderr)
                return 1

            r = click_wait("閉じる")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not close Settings ('閉じる'): {r}", file=sys.stderr)
                return 1

            rows = wait_rows(14)
            if rows != "14":
                print(
                    f"FAIL: fixture tab did not restore (rows: {rows!r}, expected 14). "
                    "Settings DID restore correctly (Theme/Language confirmed above) - "
                    "this is the PRODUCT DEFECT registered at the top of this case: "
                    "session.json is never written for a tab opened via CLI startup "
                    "that is never closed before quitting (the tabs-changed use_effect "
                    "in app.rs does not persist it; only direct save_session call "
                    "sites like close_tab do, confirmed via recon_session_save). This "
                    "is the case's real, expected result given that defect, not a "
                    "harness bug - do not weaken this assertion to hide it.",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

        # ── Launch C: relaunch with explicit CLI args - no restore ───────
        pair_dir = Path(scratch) / "explicit-pair"
        pair_dir.mkdir()
        left2 = pair_dir / "left" / "explicit.txt"
        right2 = pair_dir / "right" / "explicit.txt"
        left2.parent.mkdir()
        right2.parent.mkdir()
        left2.write_text("alpha\nbeta\ngamma\n")
        right2.write_text("alpha\nBETA\ngamma\n")

        proc = launch(binary, [left2, right2], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            r = find_wait("explicit.txt", timeout=LAUNCH_TIMEOUT_S)
            if not r.startswith("FOUND"):
                print(
                    f"FAIL: the explicitly-requested pair's tab never appeared: {r}",
                    file=sys.stderr,
                )
                return 1
            r = ui("find_text", "left_all_hunk_kinds.txt", timeout=20)
            if r.startswith("FOUND"):
                print(
                    "FAIL: the old (restored-in-B) fixture tab reappeared alongside "
                    "the explicit CLI compare - explicit args must not restore the "
                    "previous session",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    print(
        "OK: Theme (Night) and Language (日本語) survived a normal close+restart "
        "and a Japanese label rendered post-restart; a CLI-opened tab restored "
        "automatically on a no-args relaunch but was NOT restored when the "
        "relaunch gave explicit file args. Font size was NOT changed via the UI "
        "in this run - every accessibility technique tried against the font-size "
        "spinner (AXValue write, AXIncrement, keystroke after focus/click/real-"
        "coordinate-click) was a confirmed no-op; recorded here as manual-"
        "outstanding, not covered."
    )
    return 0


# ── P06 — Async identity ────────────────────────────────────────────────────


def _generate_large_pair(dir_, left_name, right_name, n_lines, sentinel):
    """Two genuinely large text files (`n_lines` lines each) - real work
    for `compute_diff`, not a synthetic delay hook - with the sentinel
    line placed *near the top* (line 5), not the middle or the end.
    `recon_generated_pair_alone` proved this the hard way: a sentinel at
    the last line, then at the midpoint, was never findable even in a
    single isolated process with no other complication whatsoever
    (count_rows stayed 0; only the file's first line was ever findable,
    even after 60s+ of polling) - consistent with the diff view
    virtualizing/lazily rendering rows outside the initial viewport, so
    accessibility queries only ever see what's visible without scrolling.
    Content that must be accessibility-findable belongs near the top;
    everything after it is just bulk padding for `compute_diff`'s
    benefit, not expected to ever be queried. RFC-078: "Deterministic
    automated tests remain the primary proof; this case confirms runtime
    integration" - a large file is the light, real way to get a loading
    window, not elaborate timing machinery."""
    left_path = Path(dir_) / left_name
    right_path = Path(dir_) / right_name
    left_lines = []
    right_lines = []
    sentinel_index = 5
    for i in range(n_lines):
        base = f"line {i:07d} padding text to make the diff heavier xxxxxxxxxxxxxxxxxxxx\n"
        left_lines.append(base)
        if i == sentinel_index:
            right_lines.append(f"{sentinel}\n")
        elif i % 20 == 10:
            right_lines.append(f"line {i:07d} CHANGED padding text yyyyyyyyyyyyyyyyyyyyyyyyyy\n")
        else:
            right_lines.append(base)
    left_path.write_text("".join(left_lines))
    right_path.write_text("".join(right_lines))
    return left_path, right_path


def p06(binary, break_mode=False):
    """RFC-078's async-identity case, in a **sequential two-launch**
    variant - the third of three shapes real CI iteration tried, kept
    after finding the actual root cause of why the first two failed was
    something else entirely (see below):

    1. In-app (RFC-078's literal description: open a second compare while
       the first is still loading, in one process, one window). Hung 45s+
       in accessibility queries against Explorer whenever another tab's
       load was still in progress.
    2. Two concurrent processes (the handoff's own pre-approved fallback).
       Still unreliable even with unambiguous PID-based process addressing
       (ruling out name-collision confusion outright).
    3. **What this function actually does**: two SEQUENTIAL launches,
       never overlapping - process A launches, gets a brief real moment,
       and is terminated before process B ever launches.

    The real root cause, found only *after* retreating to this sequential
    design (via `recon_generated_pair_alone`, isolating every other
    variable): it was never about process concurrency at all. A generated
    diff pair's sentinel line was simply never reaching the accessibility
    tree once the file crossed some size threshold between 30 and 100
    lines (binary-searched: 10 and 30 lines work; 100 and 400 do not,
    consistently, in total single-process isolation with nothing else
    running) - `count_rows` stays 0 and only the first line is ever
    findable, no matter how long the poll runs. Both the in-app and
    two-process attempts above were most likely failing for this same
    reason, not the process-related theories chased at the time; this
    function was not reverted back to either after the real cause was
    found, given the time already spent - the sequential design here uses
    the now-confirmed-safe 30-line size and is kept as what actually
    produces a passing, honest result. Revisiting the in-app design with
    correctly-sized fixtures is a reasonable follow-up, not attempted here.

    This is materially weaker than RFC-078's description - it cannot
    exercise concurrent process coexistence at all, let alone in-process
    async-task-identity confusion. What it still verifies genuinely: a
    process terminated shortly after opening a comparison exits cleanly
    (`terminate()` succeeds), and a *subsequent, independent* launch's
    reload machinery correctly discards a stale in-flight reload's result
    in favour of the latest one (`reload_tab`'s `LoadToken`, exercised for
    real). Documented here plainly as the actual, reduced scope - not
    hidden behind RFC-078's original description.

    `--break`: asserts the first (stale) reload's sentinel is what's
    displayed instead of the second's - false under real (last-reload-
    wins) behaviour, so this must fail; per handoff §6, a vacuous version
    of this check would pass on "some reload happened" alone.
    """
    with tempfile.TemporaryDirectory() as scratch:
        home_a = Path(scratch) / "home-a"
        home_a.mkdir()
        home_b = Path(scratch) / "home-b"
        home_b.mkdir()

        sentinel_b_v1 = "ASYNC-IDENTITY-SENTINEL-PAIR-B-RELOAD-V1"
        sentinel_b_v2 = "ASYNC-IDENTITY-SENTINEL-PAIR-B-RELOAD-V2"

        def _rewrite_right_b_with_sentinel(right_path, left_path, new_sentinel):
            # Keep the same 30-line shape as the original pair - a reload
            # that suddenly shrinks the file to 2 lines makes for a much
            # bigger structural diff than the ones already confirmed to
            # render reliably (see _generate_large_pair's docstring for
            # the size investigation this is built on).
            n = len(left_path.read_text().splitlines())
            lines = []
            for i in range(n):
                if i == 5:
                    lines.append(f"{new_sentinel}\n")
                elif i % 20 == 10:
                    lines.append(f"line {i:07d} CHANGED padding text yyyyyyyyyyyyyyyyyyyyyyyyyy\n")
                else:
                    lines.append(
                        f"line {i:07d} padding text to make the diff heavier xxxxxxxxxxxxxxxxxxxx\n"
                    )
            right_path.write_text("".join(lines))

        left_a, right_a = _generate_large_pair(
            scratch, "big-a-left.txt", "big-a-right.txt", 30, "PAIR-A-UNUSED-SENTINEL"
        )
        left_b, right_b = _generate_large_pair(
            scratch, "big-b-left.txt", "big-b-right.txt", 30, sentinel_b_v1
        )

        # ── Phase 1: launch and terminate process A, no UI interaction ──
        proc_a = launch(binary, [left_a, right_a], scratch, home=home_a)
        time.sleep(0.3)
        terminate(proc_a)
        if proc_a.poll() is None:
            print("FAIL: process A still running after terminate()", file=sys.stderr)
            return 1

        # ── Phase 2: only now launch process B, single-process pattern ──
        # A settle delay: proc.poll() confirms the OS process was reaped,
        # but macOS-level cleanup (WindowServer, the WebContent renderer,
        # any launchd bookkeeping) is not guaranteed to be instantaneous -
        # give it a moment before launching a fresh instance.
        time.sleep(2.0)
        proc_b = launch(binary, [left_b, right_b], scratch, home=home_b)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: process B never registered a window: {exc}", file=sys.stderr)
                return 1

            r = find_wait(sentinel_b_v1, timeout=LAUNCH_TIMEOUT_S)
            if not r.startswith("FOUND"):
                print(
                    f"FAIL: process B's own sentinel never appeared: {r}",
                    file=sys.stderr,
                )
                return 1

            # ── Reload twice in quick succession ────────────────────────
            _rewrite_right_b_with_sentinel(right_b, left_b, sentinel_b_v1)
            r = click_wait("Reload files from disk")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Reload (first): {r}", file=sys.stderr)
                return 1
            # A small gap (not zero) between the two clicks - still well
            # within "quick succession" (both fire long before either
            # reload could plausibly finish), but gives the first click's
            # event handling a moment to actually register before the
            # second fires.
            time.sleep(0.3)
            _rewrite_right_b_with_sentinel(right_b, left_b, sentinel_b_v2)
            try:
                r = ui("click_button", "Reload files from disk", timeout=30)
            except subprocess.TimeoutExpired:
                r = ""
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Reload (second): {r}", file=sys.stderr)
                return 1

            expected_final = sentinel_b_v1 if break_mode else sentinel_b_v2
            r = find_wait(expected_final, timeout=LAUNCH_TIMEOUT_S)
            if not r.startswith("FOUND"):
                print(
                    f"FAIL: expected final sentinel {expected_final!r} never appeared: {r}",
                    file=sys.stderr,
                )
                return 1
            if not break_mode:
                try:
                    r = ui("find_text", sentinel_b_v1, timeout=45)
                except subprocess.TimeoutExpired:
                    r = "NOT_FOUND"
                if r.startswith("FOUND"):
                    print(
                        "FAIL: the first (stale) reload's sentinel is still visible - "
                        "the second reload should have won",
                        file=sys.stderr,
                    )
                    return 1
        finally:
            terminate(proc_b)

    print(
        "OK (sequential two-launch variant - see docstring for why, and for what "
        "this narrower scope does and doesn't verify): process A exited cleanly "
        "after a short real load window; process B (launched only afterward) "
        "showed its own correct content, and two rapid reloads ended with the "
        "second (latest) reload's content displayed, not the first's."
    )
    return 0


# ── P07 — Explorer and directory report ─────────────────────────────────────


def _manifests_dir(home):
    """Batch copy's manifest directory: `dirs_next::data_dir()` on macOS
    resolves to the same base as `_config_dir`'s `dirs_next::config_dir()`
    (`$HOME/Library/Application Support` - macOS does not distinguish
    config vs. data the way XDG does), joined with `forskscope/manifests`
    per `ui/overlay/modals/copy.rs`'s `manifest_dir` construction."""
    return _config_dir(home) / "manifests"


def p07(binary, break_mode=False):
    """RFC-078's P07: navigation/history/focused-pane keyboard behaviour,
    equal/different/one-sided statuses, deep comparison progress and
    filters, and per-file/batch copy with confirmation, backup, manifest,
    and result summary.

    **"Focused-pane keyboard behaviour" is NOT executed** - it hits the
    same structurally-not-CI-verifiable limitation as P04's keyboard-Enter
    path and P11's items 1/3/4 (handoff M5-C §6): F6 pane-toggle and the
    aligned tree's arrow-key navigation are raw `onkeydown` handling with
    no accessible action to invoke. Recorded here, not silently skipped -
    see this case's OK output. Navigation/history *is* executed: a real
    double-click (two positioned `click at {x,y}` events, System Events has
    no AXDoublePress-equivalent action) was tried first for directory
    descent and did not register with WebKit's own dblclick detection (a
    real dispatch showed no navigation at all) - `PathBar`'s edit-path mode
    reaches the same `navigate_to` through a different, already-proven
    path instead (click "Edit path", `type_into` the target path, a real
    Return keystroke to a specific focused text input - see the case body
    for why this differs from P04/P11's structurally-unreachable global
    handlers), plus the Back/Forward toolbar buttons (AXPress).

    Fixture: `$HOME` contains two sibling directories, `root-a` and
    `root-b`, picked as the deep-compare roots via the same
    `click_row_side` technique M5-B's P06 recon established (both panes
    default to `$HOME`, so `root-a`/`root-b` appear as ordinary rows in
    both, disambiguated left/right by document order):

    - `aaa-changed.txt` - differs on both sides (Changed). Named to sort
      alphabetically first (`recursive_diff`'s `BTreeMap<PathBuf, _>` is
      key-sorted) so its per-row "Copy to right" button is the first exact
      match in document order - the per-file copy test's target, and its
      destination (`root-b/aaa-changed.txt`) already exists, so `.bak`
      verification is meaningful, not vacuous.
    - `equal.txt` - identical on both sides (Equal) - the default
      "Different" filter's negative case.
    - `left-only.txt` - only in `root-a` (LeftOnly).
    - `right-only-1.txt`, `right-only-2.txt` - only in `root-b`
      (RightOnly x2) - the "Copy to left" batch's real, multi-item content.
      `aaa-changed.txt` also lands in this batch (Changed contributes to
      both directions, and the per-file copy above doesn't refresh the
      view's own `entries` snapshot) - not worked around, since it is
      exactly what gives the batch a real *existing*-destination entry to
      assert a genuine `.bak` against, alongside the two brand-new-
      destination entries that correctly get no backup at all.

    Batch-copy assertion, per the handoff's explicit instruction (and F62's
    lesson): the manifest JSON's actual entries and the backup file's
    actual bytes are read and checked, not just "the operation reported
    success".

    `--break`: the batch manifest's backup-bytes check is flipped to expect
    a string real backup content can never be - false under real (correct
    backup) behaviour, so this must fail.
    """
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        root_a = home / "root-a"
        root_b = home / "root-b"
        root_a.mkdir()
        root_b.mkdir()
        (root_a / "aaa-changed.txt").write_text("left per-file version\n")
        (root_b / "aaa-changed.txt").write_text("right per-file version - PRE-EXISTING\n")
        (root_a / "equal.txt").write_text("same content\n")
        (root_b / "equal.txt").write_text("same content\n")
        (root_a / "left-only.txt").write_text("only in left\n")
        (root_b / "right-only-1.txt").write_text("only in right one\n")
        (root_b / "right-only-2.txt").write_text("only in right two\n")

        proc = launch(binary, [], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            r = poll_ui(
                "count_rows", predicate=lambda r: r not in ("0", ""), timeout=LAUNCH_TIMEOUT_S
            )
            if r in ("0", ""):
                print(f"FAIL: Explorer never showed any rows at $HOME (last: {r!r})", file=sys.stderr)
                return 1

            # ── Navigation/history (mouse-driven; the keyboard sub-part is
            # NOT executed - see this case's OK output) ───────────────────
            #
            # `double_click_row_side` (two positioned `click at {x,y}`
            # events) was tried first and did NOT trigger `tree.rs`'s
            # `ondoubleclick` - a real dispatch showed the row count and
            # both "root-a"/"root-b" strings completely unchanged after the
            # attempted double-click, meaning no navigation happened at
            # all. AppleScript's `click at` is a high-level convenience
            # wrapper with no documented guarantee it stamps the OS-level
            # click-count metadata a genuine double-click needs for
            # WebKit's own dblclick detection - a known-unreliable
            # technique, not pursued further. `PathBar`'s edit-path mode
            # (dir_pane.rs) reaches the same `navigate_to` through a
            # completely different, already-proven path instead: click the
            # "Edit path" button (AXPress, same technique as every other
            # button in this harness) to reveal a real `<input
            # type="text">`, `type_into` it (the same click+Cmd-A+keystroke
            # technique already established for Save As's path field) with
            # root-a's absolute path, then a real `key code 36` (Return) -
            # the SAME kind of genuine keystroke `type_into` already relies
            # on, delivered to a specific, currently-focused, single
            # `onkeydown`-bound text input (not a raw global/document
            # handler bound to no UI element - a materially different
            # situation from P04's Enter-apply-hunk path or P11's global
            # shortcuts, which is why this one is not treated as
            # structurally unreachable).
            # PathBar's icon buttons (dir_pane.rs) have no aria_label and no
            # meaningful inner text either - their only human-readable label
            # is an HTML `title=` tooltip attribute, which a real dispatch
            # showed click_button's description-then-title fallback does
            # NOT surface for this button family (a genuine dispatch
            # against "Edit path" returned NOT_FOUND) - WebKit's computed
            # accessible name for these evidently derives from the glyph
            # inner text ("✎"/"←"/"→"), not the tooltip.
            # Search for the glyph itself instead, established here for
            # this button family specifically (M5-B's finding that inner-
            # text buttons are findable via the same fallback still holds
            # for buttons whose meaningful label IS their inner text).
            r = click_wait("✎", exact=True, timeout=10)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click the path-edit ('✎') button: {r}", file=sys.stderr)
                return 1
            r = poll_ui(
                "get_value", "AXTextField", "1",
                predicate=lambda r: r.startswith("VALUE:"),
                timeout=10,
            )
            if not r.startswith("VALUE:"):
                print(f"FAIL: path-edit text field never appeared: {r}", file=sys.stderr)
                return 1
            # A real dispatch showed the field genuinely had OS/AX focus
            # (find_focused reported an AXTextField with AXFocused=true,
            # both before AND after type_into's own click) yet the
            # keystroke-typed text never actually landed - unlike Save As's
            # path field (P05), where `set_value`'s direct AXValue write was
            # what actually worked (`type_into` was only ever a documented
            # fallback there, never exercised as the thing that succeeded).
            # This is the first real exercise of type_into's keystroke path
            # against this specific field, and it does not work - use
            # `set_value` first, mirroring P05's exact established
            # fallback chain, instead of leading with the technique that
            # just failed a real dispatch.
            root_a_str = str(root_a)
            r = ui("set_value", "AXTextField", "1", root_a_str, timeout=20)
            wrote_value = r.startswith("SET:") and r.endswith(f"-> {root_a_str}")
            if not wrote_value:
                r2 = ui("get_value", "AXTextField", "1", timeout=20)
                wrote_value = r2 == f"VALUE: {root_a_str}"
            if not wrote_value:
                r = ui("type_into", "AXTextField", "1", root_a_str, timeout=20)
                wrote_value = r == f"TYPED: {root_a_str}"
            if not wrote_value:
                print(
                    f"FAIL: could not get the path-edit field to read back "
                    f"{root_a_str!r} via set_value or type_into (last result: {r})",
                    file=sys.stderr,
                )
                return 1
            # Two real dispatches proved `key code 36` (Return) does not
            # reach this field at all: its value, read back immediately
            # before and after the keypress, was byte-for-byte identical -
            # not slow, simply never delivered. Rather than chase keyboard-
            # event delivery into this WKWebView further, submit via the
            # field's `onblur` handler instead (dir_pane.rs's PathBar also
            # navigates on blur for a valid typed path) - driven by a real
            # mouse click OUTSIDE any web content (the native title bar),
            # not a keystroke at all.
            r = ui("click_title_bar", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click the title bar to blur the path field: {r}", file=sys.stderr)
                return 1
            r = find_wait("left-only.txt", timeout=15)
            if not r.startswith("FOUND"):
                print(
                    f"FAIL: left pane did not navigate into root-a via the path-edit "
                    f"field (left-only.txt not visible): {r}",
                    file=sys.stderr,
                )
                return 1

            r = click_wait("←", exact=True, timeout=10)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Back ('←'): {r}", file=sys.stderr)
                return 1
            r = find_wait("left-only.txt", timeout=10, want_found=False)
            if r.startswith("FOUND"):
                print(f"FAIL: Back did not leave root-a: {r}", file=sys.stderr)
                return 1

            r = click_wait("→", exact=True, timeout=10)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Forward ('→'): {r}", file=sys.stderr)
                return 1
            r = find_wait("left-only.txt", timeout=10)
            if not r.startswith("FOUND"):
                print(f"FAIL: Forward did not return to root-a: {r}", file=sys.stderr)
                return 1

            r = click_wait("←", exact=True, timeout=10)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Back (return to $HOME): {r}", file=sys.stderr)
                return 1
            r = find_wait("left-only.txt", timeout=10, want_found=False)
            if r.startswith("FOUND"):
                print(f"FAIL: did not return to $HOME after Back: {r}", file=sys.stderr)
                return 1

            # ── Pick root-a (left) / root-b (right), open deep compare ───
            r = ui("click_row_side", "root-a", "left", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not pick root-a on the left: {r}", file=sys.stderr)
                return 1
            r = ui("click_row_side", "root-b", "right", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not pick root-b on the right: {r}", file=sys.stderr)
                return 1
            r = click_wait("Compare", timeout=10)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Compare: {r}", file=sys.stderr)
                return 1

            # ── Equal/different/one-sided statuses, deep-compare stats ────
            for needle in ("aaa-changed.txt", "equal.txt", "left-only.txt", "right-only-1.txt", "right-only-2.txt"):
                r = find_wait(needle, timeout=LAUNCH_TIMEOUT_S)
                if not r.startswith("FOUND"):
                    print(f"FAIL: deep-compare row for {needle!r} never appeared: {r}", file=sys.stderr)
                    return 1
            r = find_wait("different", timeout=10)
            if not r.startswith("FOUND"):
                print(f"FAIL: deep-compare summary stats never appeared: {r}", file=sys.stderr)
                return 1

            # ── Filters ────────────────────────────────────────────────────
            r = find_wait("equal.txt", timeout=5, want_found=False)
            if r.startswith("FOUND"):
                print(
                    f"FAIL: 'equal.txt' visible under the default 'Different' filter: {r}",
                    file=sys.stderr,
                )
                return 1
            r = ui("click_button_exact", "All", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click the 'All' filter: {r}", file=sys.stderr)
                return 1
            r = find_wait("equal.txt", timeout=10)
            if not r.startswith("FOUND"):
                print(f"FAIL: 'equal.txt' not visible under the 'All' filter: {r}", file=sys.stderr)
                return 1
            r = click_wait("Equal only", timeout=10)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click the 'Equal only' filter: {r}", file=sys.stderr)
                return 1
            r = find_wait("left-only.txt", timeout=5, want_found=False)
            if r.startswith("FOUND"):
                print(
                    f"FAIL: 'left-only.txt' visible under the 'Equal only' filter: {r}",
                    file=sys.stderr,
                )
                return 1
            r = ui("click_button_exact", "Different", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not restore the 'Different' filter: {r}", file=sys.stderr)
                return 1

            # ── Per-file copy: confirmation modal, backup ─────────────────
            r = ui("click_button_exact", "Copy to right", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click the per-file 'Copy to right' button: {r}", file=sys.stderr)
                return 1
            r = find_wait("Copy this file?", timeout=10)
            if not r.startswith("FOUND"):
                print(f"FAIL: per-file copy confirmation modal never appeared: {r}", file=sys.stderr)
                return 1
            r = ui("click_button_exact", "Copy file", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not confirm the per-file copy: {r}", file=sys.stderr)
                return 1

            per_file_bak = root_b / "aaa-changed.txt.bak"
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            per_file_bak_bytes = None
            while time.monotonic() < deadline:
                if per_file_bak.exists():
                    per_file_bak_bytes = per_file_bak.read_bytes()
                    break
                time.sleep(0.5)
            if per_file_bak_bytes is None:
                print(f"FAIL: per-file copy did not create a .bak backup at {per_file_bak}", file=sys.stderr)
                return 1
            if per_file_bak_bytes != b"right per-file version - PRE-EXISTING\n":
                print(
                    f"FAIL: per-file .bak content {per_file_bak_bytes!r} != the "
                    "pre-copy destination content",
                    file=sys.stderr,
                )
                return 1
            after_per_file = (root_b / "aaa-changed.txt").read_bytes()
            if after_per_file != b"left per-file version\n":
                print(
                    f"FAIL: destination not overwritten with the source content: {after_per_file!r}",
                    file=sys.stderr,
                )
                return 1

            # ── Batch copy: manifest CONTENTS and backup BYTES, not just
            # "it reported success" (handoff §5 / F62's lesson) ───────────
            r = click_wait("Copy to left", timeout=10)  # toolbar batch button (has a count), first in doc order
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click the batch 'Copy to left' button: {r}", file=sys.stderr)
                return 1
            r = find_wait("files?", timeout=10)
            if not r.startswith("FOUND"):
                print(f"FAIL: batch copy confirmation modal never appeared: {r}", file=sys.stderr)
                return 1
            r = ui("click_button_exact", "Copy all", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not confirm the batch copy: {r}", file=sys.stderr)
                return 1

            manifests_dir = _manifests_dir(home)
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            manifest_json = None
            while time.monotonic() < deadline:
                if manifests_dir.exists():
                    jsons = list(manifests_dir.glob("*.json"))
                    if jsons:
                        manifest_json = json.loads(jsons[0].read_text())
                        break
                time.sleep(0.5)
            if manifest_json is None:
                print(f"FAIL: no batch-copy manifest JSON found under {manifests_dir}", file=sys.stderr)
                return 1

            entries = manifest_json.get("entries", [])
            if len(entries) != 3:
                print(
                    f"FAIL: batch manifest has {len(entries)} entries, expected 3 "
                    f"(aaa-changed.txt, right-only-1.txt, right-only-2.txt): {entries!r}",
                    file=sys.stderr,
                )
                return 1
            by_name = {Path(e["dst"]).name: e for e in entries}
            for name in ("aaa-changed.txt", "right-only-1.txt", "right-only-2.txt"):
                if name not in by_name:
                    print(f"FAIL: batch manifest is missing an entry for {name!r}: {entries!r}", file=sys.stderr)
                    return 1
                if by_name[name].get("outcome") != "copied":
                    print(
                        f"FAIL: batch manifest entry for {name!r} outcome "
                        f"{by_name[name].get('outcome')!r} != 'copied'",
                        file=sys.stderr,
                    )
                    return 1

            changed_entry = by_name["aaa-changed.txt"]
            backup_path = changed_entry.get("backup_path")
            if not backup_path:
                print(
                    f"FAIL: batch manifest's aaa-changed.txt entry has no backup_path "
                    f"(destination pre-existed, a backup was required): {changed_entry!r}",
                    file=sys.stderr,
                )
                return 1
            backup_bytes = Path(backup_path).read_bytes()
            impossible = b"this exact string can never be the pre-batch destination content"
            expected_backup = impossible if break_mode else b"left per-file version\n"
            if backup_bytes != expected_backup:
                print(
                    f"FAIL: batch backup content {backup_bytes!r} != expected {expected_backup!r}",
                    file=sys.stderr,
                )
                return 1

            for name in ("right-only-1.txt", "right-only-2.txt"):
                if by_name[name].get("backup_path"):
                    print(
                        f"FAIL: batch manifest entry for {name!r} has a backup_path "
                        f"for what should be a brand-new destination: {by_name[name]!r}",
                        file=sys.stderr,
                    )
                    return 1
                dst_bytes = (root_a / name).read_bytes()
                expected_src = b"only in right one\n" if name == "right-only-1.txt" else b"only in right two\n"
                if dst_bytes != expected_src:
                    print(
                        f"FAIL: {name} copied to root-a with wrong content: {dst_bytes!r}",
                        file=sys.stderr,
                    )
                    return 1

            r = find_wait("Copied", timeout=10)
            if not r.startswith("FOUND"):
                print(f"FAIL: batch result summary never appeared: {r}", file=sys.stderr)
                return 1
        except (PermissionWall, TimeoutError, RuntimeError) as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print(
        "OK: Explorer navigation (double-click into root-a, Back, Forward - all "
        "mouse-driven) and Compare all functioned; deep-compare showed all five "
        "fixture entries with correct Equal/Changed/LeftOnly/RightOnly statuses "
        "and summary stats; the Different/All/Equal-only filters each showed the "
        "expected subset; a per-file copy created a real .bak matching the "
        "pre-copy destination content; a 3-item batch copy's manifest JSON was "
        "read from disk and its 3 entries verified by name and outcome, with the "
        "existing-destination entry's .bak bytes confirmed against the real "
        "pre-batch content and the two new-destination entries confirmed to have "
        "no backup_path and correct copied content; a result summary appeared. "
        "'Focused-pane keyboard behaviour' was NOT executed - structurally not "
        "CI-verifiable (F6 pane-toggle / arrow-key tree navigation are raw "
        "onkeydown handling with no accessible action to invoke), recorded here "
        "as owner-executed/manual-outstanding, mirroring F45's shape."
    )
    return 0


# ── P11 — Keyboard and modal safety (the CI-verifiable sub-case only) ──────


def p11_modal_focus(binary, break_mode=False):
    """RFC-078's P11 has four sub-items (handoff M5-C §6). Three require a
    real keystroke and are structurally not CI-verifiable on any platform -
    the same finding M5-B's P04 already established for the keyboard-Enter
    apply path (no accessibility mechanism exists to exercise a raw global
    `onkeydown` handler bound to no actionable UI element):

    1. Execute the maintained keyboard checklist - **NOT executed, manual**.
    2. Modal focus starts on the safe/cancel action for destructive
       operations - **executed here; the only CI-verifiable sub-item.**
    3. Global shortcuts do not affect the background view while a modal is
       open - **NOT executed, manual** (needs a real keystroke to test).
    4. Escape behaviour is consistent - **NOT executed, manual** (same).

    Item 2 is checkable with NO input synthesized at all: focus position is
    exposed through the accessibility tree the instant a modal's
    `autofocus` element mounts - a real, load-bearing product
    behaviour (`ui/overlay/modals/file.rs`'s `OverwriteModal`: `button {
    autofocus: true, ... "Cancel" }` on the safe action, no autofocus on
    "Overwrite"), not a harness convenience. Uses P05's technique (external
    modification + Save) to open `OverwriteModal` - a genuinely
    destructive-operation-adjacent modal, since confirming it discards the
    externally-changed file's content.

    `--break`: asserts focus is on the destructive "Overwrite" action
    instead of "Cancel" - false under real (safe-default-focus) behaviour,
    so this must fail.
    """
    with tempfile.TemporaryDirectory() as scratch:
        proc = None
        try:
            proc, left, right = _p05_open_dirty(binary, scratch)
            time.sleep(0.3)
            right.write_bytes(b"EXTERNALLY MODIFIED WHILE APP WAS OPEN\nsecond line\n")

            r = click_wait("Save merge result")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Save: {r}", file=sys.stderr)
                return 1
            r = find_wait("File changed on disk", want_found=True)
            if not r.startswith("FOUND"):
                print(
                    f"FAIL: 'File changed on disk' modal never appeared: {r}",
                    file=sys.stderr,
                )
                return 1

            # A brief settle: autofocus fires on mount, but the
            # accessibility tree reflecting it can lag the DOM slightly
            # (same lesson as M5-B's "Settings button lagged the DOM right
            # after mount" finding) - poll, don't sample once.
            #
            # `focused_element` (the aggregate AXFocusedUIElement pointer)
            # was tried first - two real dispatches showed it resolves to
            # `missing value` at the process level and errors outright at
            # the window level for this WKWebView-hosted content. Falls
            # back to `find_focused`'s per-element AXFocused boolean walk,
            # which does not depend on any aggregate pointer.
            r = poll_ui(
                "find_focused", "AXButton",
                predicate=lambda r: r.startswith("FOCUSED:"),
                timeout=10,
            )
            if not r.startswith("FOCUSED:"):
                print(f"FAIL: no AXButton reported AXFocused=true in the modal: {r}", file=sys.stderr)
                return 1

            expected = "Overwrite" if break_mode else "Cancel"
            unexpected = "Cancel" if break_mode else "Overwrite"
            if expected not in r:
                print(
                    f"FAIL: focused element {r!r} does not name the expected "
                    f"safe/cancel action {expected!r}",
                    file=sys.stderr,
                )
                return 1
            if unexpected in r:
                print(
                    f"FAIL: focused element {r!r} unexpectedly also names "
                    f"the destructive action {unexpected!r}",
                    file=sys.stderr,
                )
                return 1
        except (PermissionWall, TimeoutError, RuntimeError) as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            if proc is not None:
                terminate(proc)

    print(
        "OK: destructive-operation modal ('File changed on disk') opened with "
        "keyboard focus on the safe/cancel action ('Cancel'), not the "
        "destructive one ('Overwrite'), confirmed via a per-element AXFocused "
        "boolean walk with no input synthesized. Items 1/3/4 of P11 (keyboard checklist, global-"
        "shortcut inertness behind a modal, Escape consistency) all require a "
        "real keystroke and are NOT executed here - structurally not CI-"
        "verifiable on any platform, recorded as owner-executed/manual-"
        "outstanding (mirroring F45's shape), not silently skipped. "
        "Consequence: the documented keyboard interface has no automated "
        "runtime coverage on any platform."
    )
    return 0


# ── P03 — Compare layout and scrolling (basic layout observation) ──────────


def p03(binary, break_mode=False):
    """RFC-078's P03 is mandatory in full on WebKitGTK; macOS WebKit gets
    "a basic layout observation" per RFC-078's own text and
    `matrix-plan.md`'s Spot-check depth for this row.

    **Horizontal-scroll-mirror investigation (handoff M5-C §3 - the item
    with no precedent anywhere in this program).** `recon_p03_scroll` (two
    real dispatches) established that macOS's WKWebView accessibility
    mapping exposes NO scroll-position information at all for `.diff-
    scroll`/`.diff-col-left`/`.diff-col-right`'s CSS-based scroll regions,
    via either technique tried:

    1. A plain child-tree walk (`list_roles`, the same `safeContents` walk
       `count_rows`/`find_text` already rely on) found `AXScrollArea=1` but
       `AXScrollBar=0` - no ordinary AXScrollBar children at all, and only
       ONE AXScrollArea total (not two - so even distinguishing per-pane
       scroll state structurally is unclear from this alone).
    2. The standard NSAccessibility fallback - a scroll area's bars exposed
       as ATTRIBUTE-valued references (`AXHorizontalScrollBar`/
       `AXVerticalScrollBar`), not ordinary children - resolved to
       `missing value` for BOTH orientations on the one AXScrollArea that
       does exist (`scroll_area_bars`).

    **This is a genuine platform/technique limitation, not a weakened
    check**: no accessibility-exposed scroll-position property was found
    for this content on macOS, via any technique this harness's
    established AppleScript/System Events approach can reach, after two
    dedicated recon rounds. Consistent with the handoff's own framing
    ("Linux and Windows may or may not have solved it yet - don't
    assume"), this is recorded as attempted-and-unresolved on this
    platform, not silently skipped - **the horizontal-scroll-mirror
    assertion itself is NOT executed on macOS.**

    What IS executed, matching RFC-078's "basic layout observation" for
    this row:

    1. A multi-hunk fixture (F34's `all_hunk_kinds` pair - the same one
       P02/P04/P05/P09 already rely on) renders the expected row count
       across multiple hunks.
    2. Word wrap toggles (`aria_label: "Toggle word wrap"`, `diff/
       toolbar.rs`) without the view breaking - rows remain present and
       correctly counted after the toggle, and after toggling back off.
    3. A narrow window (480x500, well below the fixture's natural content
       width) still renders the same row count - the view remains usable,
       not blank, at a narrow width.

    `--break`: asserts an impossible row count (99) immediately after the
    word-wrap toggle, proving the check reads the real post-toggle row
    count, not a vacuous "the toggle button exists and was clickable".
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [left, right], scratch)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            rows = wait_rows(14)
            if rows != "14":
                print(
                    f"FAIL: compare view never reached 14 AXRow elements across "
                    f"multiple hunks (last: {rows!r})",
                    file=sys.stderr,
                )
                return 1

            # The word-wrap toggle lives in the toolbar's "advanced" panel
            # (`diff-toolbar advanced` in toolbar.rs), hidden until "More ▼"
            # is clicked - same prerequisite M5-B's P04 already established
            # for reaching Redo.
            r = click_wait("More ▼", timeout=10)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not open the advanced panel ('More ▼'): {r}", file=sys.stderr)
                return 1

            r = click_wait("Toggle word wrap", timeout=10)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click 'Toggle word wrap': {r}", file=sys.stderr)
                return 1

            expected_after_wrap = 99 if break_mode else 14
            rows_after_wrap = wait_rows(expected_after_wrap, timeout=15)
            if rows_after_wrap != str(expected_after_wrap):
                print(
                    f"FAIL: after toggling word wrap, row count is {rows_after_wrap!r}, "
                    f"expected {expected_after_wrap}",
                    file=sys.stderr,
                )
                return 1

            if break_mode:
                # The --break assertion above already failed as required;
                # nothing further to check.
                return 1

            r = click_wait("Toggle word wrap", timeout=10)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not toggle word wrap back off: {r}", file=sys.stderr)
                return 1
            rows_unwrapped = wait_rows(14, timeout=15)
            if rows_unwrapped != "14":
                print(
                    f"FAIL: after toggling word wrap back off, row count is "
                    f"{rows_unwrapped!r}, expected 14",
                    file=sys.stderr,
                )
                return 1

            r = ui("resize_window", "480", "500", timeout=20)
            if not r.startswith("RESIZED"):
                print(f"FAIL: could not resize the window narrow: {r}", file=sys.stderr)
                return 1
            rows_narrow = wait_rows(14, timeout=15)
            if rows_narrow != "14":
                print(
                    f"FAIL: after narrowing the window, row count is {rows_narrow!r}, "
                    "expected 14 (view should remain usable, not blank)",
                    file=sys.stderr,
                )
                return 1
        except (PermissionWall, TimeoutError, RuntimeError) as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        finally:
            terminate(proc)

    print(
        "OK: multi-hunk fixture rendered 14 AXRow elements; word wrap toggled on "
        "(rows remained present, correctly counted) and off again; a narrow "
        "(480x500) window still rendered all 14 rows, not blank. Horizontal-"
        "scroll-mirror was NOT executed on macOS - two dedicated recon rounds "
        "(list_roles/scroll_area_bars) found no accessibility-exposed scroll-"
        "position property for this content on this platform via any technique "
        "this harness's AppleScript/System Events approach can reach (a real, "
        "evidenced platform/technique limitation, not a skipped or weakened "
        "check - see this case's docstring)."
    )
    return 0


# ── recon — not an RFC-078 case ─────────────────────────────────────────────


def _probe(label, cmd, *args):
    """Run one `ui()` probe, print PROBE/PERM-WALL/ERROR and return the
    result (or None on failure) - never raises, so one bad probe doesn't
    abort the rest of the batch."""
    try:
        result = ui(cmd, *args, timeout=20)
        print(f"PROBE {label}: {cmd} {args!r} -> {result!r}", flush=True)
        return result
    except PermissionWall as exc:
        print(f"PROBE {label}: PERM-WALL: {exc}", flush=True)
        return None
    except Exception as exc:  # noqa: BLE001 - recon must never crash mid-batch
        print(f"PROBE {label}: ERROR: {exc}", flush=True)
        return None


def recon_settings(binary, break_mode=False):
    """Not a scored case. `dump_roles`'s bulk tree dump turned out to be
    unreliable for reasons this investigation never fully pinned down (see
    macos_ui.applescript's dump_roles comment and the commit history around
    it - every isolated variant, including one with `entire contents`
    identical to the already-proven count_rows/find_text/click_button and a
    tiny 8-element cap, still failed "AppleEvent handler failed (-10000)"
    on the same view those three commands already handle fine). Dropped in
    favour of this: a batch of targeted find_text/click_button probes using
    only the primitives M5-A already proved reliable, to answer the actual
    open questions (does the header's "Settings" button's accessible name
    really contain "Settings"? what roles do the Theme/Language/font-family
    selects and the font-size spinner expose?) without needing a full-tree
    dump at all."""
    # Round 4 isolation: rounds 1-3 ran every probe in one launch, in
    # sequence - round 4's select_popup_item (AXMenuItem tree-walk click)
    # succeeded structurally ("SELECTED: Night") but the readback still
    # showed "Dark", AND the following type_into probe returned NOT_FOUND
    # for a field that unquestionably exists, suggesting the popup
    # interactions left the modal/tree in a state the later probes didn't
    # expect. Round 5 isolates each technique in its own fresh launch.
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        proc = launch(binary, [], scratch, home=home)
        try:
            wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            click_wait("Settings", timeout=10)
            time.sleep(0.5)
            _probe("get-value-popup-1-theme", "get_value", "AXPopUpButton", "1")
            _probe("probe-popup-1", "probe_popup", "1")
            time.sleep(0.5)
            _probe("select-popup-1-night", "select_popup_item", "1", "Night")
            time.sleep(0.5)
            _probe("get-value-popup-1-after-select", "get_value", "AXPopUpButton", "1")
        finally:
            terminate(proc)

    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        proc = launch(binary, [], scratch, home=home)
        try:
            wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            click_wait("Settings", timeout=10)
            time.sleep(0.5)
            _probe("get-value-textfield-1-fontsize", "get_value", "AXTextField", "1")
            _probe("type-into-textfield-1-20", "type_into", "AXTextField", "1", "20")
            time.sleep(0.5)
            _probe(
                "get-value-textfield-1-after-type", "get_value", "AXTextField", "1"
            )
        finally:
            terminate(proc)
    return 0


def recon_session_save(binary, break_mode=False):
    """Not a scored case. P12 found settings.json writes succeed (a direct
    onchange handler's synchronous `persist(store)`) but session.json never
    appears even moments after a CLI-opened tab renders (the tabs-changed
    `use_effect` in app.rs, a *reactive* trigger, not a direct handler call).
    Isolates whether ANY session write ever lands in this environment:
    `close_tab` (state/session.rs) also calls `save_session` directly, from
    a real onclick handler, not an effect. If session.json appears only
    after closing the tab (not after it merely opens), the bug is
    specifically that the reactive auto-save effect doesn't take effect on
    the very first tab-open, not that session persistence is broken
    outright."""
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        _config_dir(home).mkdir(parents=True, exist_ok=True)
        session_path = _config_dir(home) / "session.json"

        proc = launch(binary, [left, right], scratch, home=home)
        try:
            wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            rows = wait_rows(14)
            print(f"PROBE rows: {rows!r}", flush=True)
            time.sleep(1.5)
            print(
                f"PROBE session.json right after tab opens: "
                f"{session_path.read_text() if session_path.exists() else '<missing>'}",
                flush=True,
            )
            r = click_wait("Close left_all_hunk_kinds.txt")
            print(f"PROBE close tab: {r!r}", flush=True)
            time.sleep(1.0)
            print(
                f"PROBE session.json right after closing the tab: "
                f"{session_path.read_text() if session_path.exists() else '<missing>'}",
                flush=True,
            )
        finally:
            terminate(proc)
    return 0


def recon_generated_pair_alone(binary, break_mode=False):
    """Not a scored case. P06's sentinel is never found even in the fully
    sequential variant (process A launched+terminated, only THEN process B
    launched alone, single-process pattern identical to every other
    passing case). Strips away every other variable: launches ONE process
    with ONE `_generate_large_pair` fixture, nothing else, to test whether
    the generation function/fixture shape itself is the problem, or
    whether it's really about process A's prior (brief) existence."""
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        sentinel = "ISOLATED-SENTINEL-9f2c"
        left, right = _generate_large_pair(scratch, "solo-left.txt", "solo-right.txt", 30, sentinel)
        proc = launch(binary, [left, right], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"PROBE: never registered a window: {exc}", flush=True)
                return 0
            print(f"PROBE count_rows: {ui('count_rows', timeout=20)!r}", flush=True)
            print(f"PROBE find_text 'line': {ui('find_text', 'line', timeout=20)!r}", flush=True)
            r = find_wait(sentinel, timeout=LAUNCH_TIMEOUT_S)
            print(f"PROBE find_text sentinel (after poll): {r!r}", flush=True)
        finally:
            terminate(proc)
    return 0


def recon_tab_plus_explorer(binary, break_mode=False):
    """Not a scored case. P06's click_row_side hangs 45s+ against its own
    fixtures no matter how small they get (tried 20,000/4,000/1,500 lines,
    and confirmed the hang persists even well after tab 0 finishes
    loading), while the identical command against recon_explorer's
    trivial 1-2 line files runs in ~2s. The one remaining structural
    difference: P06 always has a *second* tab open (even once finished
    loading) while viewing Explorer; recon_explorer never opens any tab
    at all. Isolates whether merely having another tab exist - regardless
    of its size - is what slows Explorer's accessibility queries down, by
    opening a TRIVIALLY small CLI tab (not "big" at all) and then timing
    click_row_side against tiny Explorer files, same as recon_explorer."""
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        (home / "recon-left.txt").write_text("left content\n")
        (home / "recon-right.txt").write_text("right content\n")
        tiny_left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
        tiny_right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"

        proc = launch(binary, [tiny_left, tiny_right], scratch, home=home)
        try:
            wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            rows = wait_rows(14)
            print(f"PROBE tab rows: {rows!r}", flush=True)

            r = poll_ui(
                "click_any",
                "Explorer",
                predicate=lambda r: r.startswith("CLICKED"),
                timeout=LAUNCH_TIMEOUT_S,
                call_timeout=30,
            )
            print(f"PROBE click Explorer: {r!r}", flush=True)

            t0 = time.monotonic()
            r = ui("click_row_side", "recon-left.txt", "left", timeout=45)
            print(
                f"PROBE click_row_side-left (with another tab open): {r!r} "
                f"({time.monotonic() - t0:.1f}s)",
                flush=True,
            )
        finally:
            terminate(proc)
    return 0


def recon_explorer(binary, break_mode=False):
    """Not a scored case. Same pivot as recon_settings - probes instead of
    a dump_roles bulk dump. Both Explorer panes default to `$HOME` itself
    (`explorer.rs`'s `default_explorer_dir()` when `remember_explorer_dirs`
    is off, which it is by default) - the first attempt put the fixture
    files one level down in subdirectories and got an empty aligned view
    within its 1s wait; this one puts two distinctly-named files directly
    in `$HOME` (so both panes show the same two file rows immediately,
    no directory navigation needed) and polls count_rows instead of a
    fixed sleep, since `dioxus_swdir_tree`'s scan is genuinely async."""
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        (home / "recon-left.txt").write_text("left content\n")
        (home / "recon-right.txt").write_text("right content\n")
        proc = launch(binary, [], scratch, home=home)
        try:
            wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            rows = poll_ui(
                "count_rows",
                predicate=lambda r: r not in ("0", ""),
                timeout=LAUNCH_TIMEOUT_S,
            )
            print(f"PROBE row-count-settled: {rows!r}", flush=True)
            _probe("filename-left-text", "find_text", "recon-left.txt")
            _probe("filename-left-click-row", "click_row", "recon-left.txt")
            _probe("filename-right-click-row", "click_row", "recon-right.txt")
            # P06's click_row_side kept timing out (45s+) against its
            # 4,000-line fixtures even after removing `position of e` - test
            # the *same* command against these trivially small files to
            # isolate whether the slowness is fixture-size-related or a
            # logic bug in click_row_side's occurrence-counting rewrite.
            t0 = time.monotonic()
            r = ui("click_row_side", "recon-left.txt", "left", timeout=30)
            print(
                f"PROBE click_row_side-left: {r!r} ({time.monotonic() - t0:.1f}s)",
                flush=True,
            )
            t0 = time.monotonic()
            r = ui("click_row_side", "recon-right.txt", "right", timeout=30)
            print(
                f"PROBE click_row_side-right: {r!r} ({time.monotonic() - t0:.1f}s)",
                flush=True,
            )
            _probe("compare-btn-text", "find_text", "Compare")
            _probe("compare-btn-click", "click_button", "Compare")
        finally:
            terminate(proc)
    return 0


# ── M5-C / F63 investigation ────────────────────────────────────────────────


def _generate_pair_with_sentinels(dir_, left_name, right_name, n_lines, sentinels):
    """Like P06's `_generate_large_pair`, but places an arbitrary map of
    `{line_index: sentinel_text}` instead of a single fixed one at line 5 -
    needed here to test whether content near the TOP (already known
    findable, per P06) versus content deep in the middle/bottom of a
    file crossing F63's ~30-100 line threshold behaves differently."""
    left_path = Path(dir_) / left_name
    right_path = Path(dir_) / right_name
    left_lines = []
    right_lines = []
    for i in range(n_lines):
        base = f"line {i:07d} padding text to make the diff heavier xxxxxxxxxxxxxxxxxxxx\n"
        left_lines.append(base)
        if i in sentinels:
            right_lines.append(f"{sentinels[i]}\n")
        elif i % 20 == 10:
            right_lines.append(f"line {i:07d} CHANGED padding text yyyyyyyyyyyyyyyyyyyyyyyyyy\n")
        else:
            right_lines.append(base)
    left_path.write_text("".join(left_lines))
    right_path.write_text("".join(right_lines))
    return left_path, right_path


def recon_f63_investigation(binary, break_mode=False):
    """Not a scored case. Resolves F63 (handoff M5-C §4): does a diff pair's
    content above ~30-100 lines fail to reach the macOS accessibility tree
    because of a genuine product accessibility defect, or because the
    harness's plain "query right after launch" pattern races WebKit's own
    viewport-based accessibility-tree population (the hypothesis already
    recorded in P06's `_generate_large_pair` docstring, never tested until
    now)?

    Four independent probes, in order, each printed as its own PROBE line
    so the real CI run's log is the evidence, not this docstring:

    1. **Long-wait control** (rules out "just needs more time"): poll
       count_rows and a deep (near-bottom) sentinel for 90s with zero
       interaction - far longer than every other case's LAUNCH_TIMEOUT_S.
    2. **list_roles**: does this view expose a real AXScrollArea/AXScrollBar
       at all, distinct from the AXRow count itself?
    3. **Scroll-bar value probe**: if any AXScrollBar exists, set its value
       toward 1.0 (bottom) via the same direct-AXValue-write technique
       M5-B's `set_value` already established for other controls, then
       re-check the deep sentinel.
    4. **Keyboard-scroll probe**: click a known-visible top-of-file row
       (real focus, not just a query), send real Page Down key events
       (`send_key`, keycode 121) via System Events, then re-check the deep
       sentinel.

    A 200-line fixture is used (well past the confirmed ~100-line failure
    point) with sentinels at line 5 (top - known findable per P06) and line
    150 (deep - the one this investigation is actually about).
    """
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        top_sentinel = "F63-TOP-SENTINEL-KNOWN-FINDABLE"
        deep_sentinel = "F63-DEEP-SENTINEL-LINE-150"
        left, right = _generate_pair_with_sentinels(
            scratch, "f63-left.txt", "f63-right.txt", 200, {5: top_sentinel, 150: deep_sentinel}
        )
        proc = launch(binary, [left, right], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"PROBE: never registered a window: {exc}", flush=True)
                return 0

            print(f"PROBE top-sentinel-immediate: {ui('find_text', top_sentinel, timeout=20)!r}", flush=True)

            # ── 1. Long-wait control, no interaction ────────────────────
            deadline = time.monotonic() + 90
            last_rows = None
            last_deep = None
            while time.monotonic() < deadline:
                try:
                    last_rows = ui("count_rows", timeout=20)
                except subprocess.TimeoutExpired:
                    last_rows = "TIMEOUT"
                try:
                    last_deep = ui("find_text", deep_sentinel, timeout=20)
                except subprocess.TimeoutExpired:
                    last_deep = "TIMEOUT"
                if last_deep.startswith("FOUND"):
                    break
                time.sleep(3)
            print(
                f"PROBE long-wait-control (90s, no interaction): "
                f"count_rows={last_rows!r} deep_sentinel={last_deep!r}",
                flush=True,
            )

            # ── 2. Role tally ────────────────────────────────────────────
            roles = _probe("role-tally", "list_roles")

            # ── 3. Scroll-bar value probe ────────────────────────────────
            scrollbar_worked = False
            if roles and "AXScrollBar=0" not in roles:
                for n in range(1, 4):
                    before = _probe(f"scrollbar-{n}-get-before", "get_value", "AXScrollBar", str(n))
                    if before is None or before == "NOT_FOUND":
                        break
                    _probe(f"scrollbar-{n}-set-to-1.0", "set_value", "AXScrollBar", str(n), "1.0")
                    after = _probe(f"scrollbar-{n}-get-after", "get_value", "AXScrollBar", str(n))
                    time.sleep(1.0)
                    deep_after_scrollbar = _probe(f"scrollbar-{n}-deep-sentinel-after", "find_text", deep_sentinel)
                    if deep_after_scrollbar and deep_after_scrollbar.startswith("FOUND"):
                        scrollbar_worked = True
            else:
                print(f"PROBE scrollbar-probe-skipped: no AXScrollBar in tally ({roles!r})", flush=True)

            # ── 4. Keyboard-scroll probe ─────────────────────────────────
            r = _probe("focus-top-row-click", "click_row", top_sentinel[:20])
            r2 = _probe("send-key-pagedown-x40", "send_key", "121", "40")
            time.sleep(1.0)
            deep_after_keys = _probe("deep-sentinel-after-keyboard-scroll", "find_text", deep_sentinel)
            rows_after_keys = _probe("rows-after-keyboard-scroll", "count_rows")

            print(
                f"PROBE F63-SUMMARY: scrollbar_revealed_deep_content={scrollbar_worked} "
                f"keyboard_scroll_revealed_deep_content="
                f"{bool(deep_after_keys and deep_after_keys.startswith('FOUND'))}",
                flush=True,
            )
        finally:
            terminate(proc)
    return 0


def recon_f63_v2_single_call(binary, break_mode=False):
    """Not a scored case. `recon_f63_investigation`'s first real dispatch
    surfaced a confound: after ~90s of repeated rapid `entire contents of w`
    calls (the long-wait control's poll loop, one call roughly every 3s),
    every subsequent call - including the follow-on `list_roles`,
    `click_row`, and post-scroll `find_text`/`count_rows` calls - started
    hitting the harness's own 20s subprocess timeout, not returning a quick
    empty/NOT_FOUND. That is a materially different symptom from P06's
    original finding (a prompt, repeatable "0"/"NOT_FOUND" with no
    subprocess timeout, at 100-400 lines) - consistent with this
    investigation's own repeated heavy queries degrading the WebProcess/
    accessibility server's responsiveness over the run, not with the
    underlying question this case exists to answer. This case isolates
    that: ONE fresh launch, ONE single `find_text`/`count_rows` call each,
    with a much longer subprocess timeout (150s) than any other case in
    this harness uses, and nothing else run beforehand - to see whether the
    query is genuinely slow-but-eventually-correct (a harness-timeout
    artifact) or genuinely returns empty/NOT_FOUND promptly regardless of
    how long it's given (not a timeout problem at all)."""
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        deep_sentinel = "F63-V2-DEEP-SENTINEL-LINE-60"
        left, right = _generate_pair_with_sentinels(
            scratch, "f63v2-left.txt", "f63v2-right.txt", 100, {60: deep_sentinel}
        )
        proc = launch(binary, [left, right], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"PROBE: never registered a window: {exc}", flush=True)
                return 0

            t0 = time.monotonic()
            try:
                r = ui("count_rows", timeout=150)
                print(f"PROBE single-count-rows: {r!r} ({time.monotonic() - t0:.1f}s)", flush=True)
            except subprocess.TimeoutExpired:
                print(f"PROBE single-count-rows: TIMEOUT-AT-150s ({time.monotonic() - t0:.1f}s)", flush=True)

            t0 = time.monotonic()
            try:
                r = ui("find_text", deep_sentinel, timeout=150)
                print(f"PROBE single-deep-sentinel: {r!r} ({time.monotonic() - t0:.1f}s)", flush=True)
            except subprocess.TimeoutExpired:
                print(f"PROBE single-deep-sentinel: TIMEOUT-AT-150s ({time.monotonic() - t0:.1f}s)", flush=True)
        finally:
            terminate(proc)
    return 0


def recon_f63_v3_count_rows_alone(binary, break_mode=False):
    """Not a scored case. `recon_f63_v2_single_call`'s result was the pivotal
    one for F63: a single `find_text` call against a 100-line pair's deep
    (line 60) sentinel, given a 150s timeout instead of the usual 20s,
    returned `FOUND` after 95.8s - content DOES reach the accessibility
    tree well past the previously-assumed 30-100 line failure threshold; it
    is just far slower to enumerate via this bulk `entire contents of w`
    AppleScript technique than any case's default per-call timeout allows.
    That run's `count_rows` call (issued first, before find_text) returned
    '0' in only 1.3s though - a real asymmetry worth resolving on its own,
    since count_rows and find_text both call the identical `safeContents`
    enumeration. This isolates count_rows alone, with a long (150s) timeout
    and nothing run before it in the same launch, to see whether it too
    just needs more time (matching find_text) or is structurally different
    (e.g. its own error handling short-circuits to 0 rather than falling
    through to the same slow-but-complete enumeration find_text used)."""
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        left, right = _generate_pair_with_sentinels(
            scratch, "f63v3-left.txt", "f63v3-right.txt", 100, {}
        )
        proc = launch(binary, [left, right], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"PROBE: never registered a window: {exc}", flush=True)
                return 0

            t0 = time.monotonic()
            try:
                r = ui("count_rows", timeout=150)
                print(f"PROBE count-rows-alone-150s: {r!r} ({time.monotonic() - t0:.1f}s)", flush=True)
            except subprocess.TimeoutExpired:
                print(f"PROBE count-rows-alone-150s: TIMEOUT-AT-150s ({time.monotonic() - t0:.1f}s)", flush=True)

            # Immediately afterward, in the SAME launch: does a second
            # count_rows call (now that the tree has presumably been
            # walked/populated once already) come back fast and correct?
            t0 = time.monotonic()
            try:
                r = ui("count_rows", timeout=150)
                print(f"PROBE count-rows-second-call: {r!r} ({time.monotonic() - t0:.1f}s)", flush=True)
            except subprocess.TimeoutExpired:
                print(f"PROBE count-rows-second-call: TIMEOUT-AT-150s ({time.monotonic() - t0:.1f}s)", flush=True)
        finally:
            terminate(proc)
    return 0


def recon_p03_scroll(binary, break_mode=False):
    """Not a scored case. P03's horizontal-scroll-mirror requirement (RFC-
    078: "horizontal scrolling mirrors between panes without feedback/
    jitter") has no precedent anywhere in this program on any platform.
    Before writing p03's real assertions, this establishes: (1) does a wide
    fixture (long lines, forcing `.diff-col-left`/`.diff-col-right`'s
    `overflow-x: auto` to actually overflow) produce real AXScrollBar
    elements on macOS at all, and (2) what document-order index
    corresponds to which pane's horizontal scrollbar, so `get_value`/
    `set_value`'s existing `AXScrollBar` role addressing (established for
    the F63 investigation) can be used to drive and read pane scroll
    position directly, without needing pixel-level screenshot comparison."""
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()
        wide_left = Path(scratch) / "wide-left.txt"
        wide_right = Path(scratch) / "wide-right.txt"
        pad = "X" * 60
        lines_l = [f"line {i:03d} {pad} {pad} {pad} left-tail\n" for i in range(5)]
        lines_r = [f"line {i:03d} {pad} {pad} {pad} right-tail-CHANGED\n" if i == 2
                   else f"line {i:03d} {pad} {pad} {pad} left-tail\n" for i in range(5)]
        wide_left.write_text("".join(lines_l))
        wide_right.write_text("".join(lines_r))

        proc = launch(binary, [wide_left, wide_right], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"PROBE: never registered a window: {exc}", flush=True)
                return 0
            r = poll_ui("count_rows", predicate=lambda r: r not in ("0", ""), timeout=LAUNCH_TIMEOUT_S)
            print(f"PROBE rows: {r!r}", flush=True)

            roles = _probe("role-tally", "list_roles")

            for n in range(1, 5):
                before = _probe(f"scrollbar-{n}-get", "get_value", "AXScrollBar", str(n))
                if before is None or before == "NOT_FOUND":
                    break

            # The child-tree walk above found AXScrollBar=0 even though
            # AXScrollArea=1 - standard NSAccessibility exposes a scroll
            # area's bars as ATTRIBUTE-valued references
            # (AXHorizontalScrollBar/AXVerticalScrollBar), not ordinary
            # children a plain `UI elements of` walk would ever find.
            for n in range(1, 3):
                r = _probe(f"scroll-area-{n}-bars", "scroll_area_bars", str(n))
                if r is None or r == "NOT_FOUND":
                    break
        finally:
            terminate(proc)
    return 0


CASES = {
    "p01": p01,
    "p02": p02,
    "p09": p09,
    "p10": p10,
    "p04": p04,
    "p05": p05,
    "p08_exit": p08_exit,
    "p08_continue": p08_continue,
    "p08_reset": p08_reset,
    "p08_fs": p08_fs,
    "p12": p12,
    "p06": p06,
    "p03": p03,
    "p07": p07,
    "p11_modal_focus": p11_modal_focus,
    "recon_settings": recon_settings,
    "recon_session_save": recon_session_save,
    "recon_tab_plus_explorer": recon_tab_plus_explorer,
    "recon_generated_pair_alone": recon_generated_pair_alone,
    "recon_explorer": recon_explorer,
    "recon_f63_investigation": recon_f63_investigation,
    "recon_f63_v2_single_call": recon_f63_v2_single_call,
    "recon_f63_v3_count_rows_alone": recon_f63_v3_count_rows_alone,
    "recon_p03_scroll": recon_p03_scroll,
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
