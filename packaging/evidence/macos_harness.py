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


def ui(cmd, *args, timeout=15):
    """Run one `macos_ui.applescript` command and return its stdout,
    stripped. Raises PermissionWall if System Events itself refused the
    request (assistive access not granted), rather than folding that into
    an ordinary NOT_FOUND result."""
    proc = subprocess.run(
        ["osascript", str(APPLESCRIPT), cmd, PROC_NAME, *args],
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


def run_diagnostics(binary):
    """Run `--diagnostics` (no window, no display needed) and return stdout."""
    result = subprocess.run(
        [str(binary), "--diagnostics"], capture_output=True, text=True, timeout=15
    )
    return result.returncode, result.stdout, result.stderr


def wait_for_window(deadline):
    """Poll until `forskscope` registers a window with positive extents.
    Returns (width, height) or raises on timeout/permission wall."""
    last = None
    while time.monotonic() < deadline:
        last = ui("window_size", timeout=20)
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


def poll_ui(cmd, *args, predicate, timeout, interval=0.5):
    """Poll `ui(cmd, *args)` until `predicate(result)` is true or the
    timeout elapses; returns the last result either way (callers decide
    what "never satisfied" means for their case)."""
    deadline = time.monotonic() + timeout
    result = None
    while time.monotonic() < deadline:
        result = ui(cmd, *args, timeout=20)
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


def click_wait(needle, timeout=LAUNCH_TIMEOUT_S, exact=False):
    cmd = "click_button_exact" if exact else "click_button"
    return poll_ui(
        cmd, needle, predicate=lambda r: r.startswith("CLICKED"), timeout=timeout
    )


def find_wait(needle, timeout=LAUNCH_TIMEOUT_S, want_found=True):
    def pred(r):
        return r.startswith("FOUND") == want_found

    return poll_ui("find_text", needle, predicate=pred, timeout=timeout)


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
    """
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()

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
        proc = launch(binary, [], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            rows = wait_rows(14)
            if rows != "14":
                print(
                    f"FAIL: fixture tab did not restore (rows: {rows!r}, expected 14)",
                    file=sys.stderr,
                )
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
    """Two genuinely large text files (`n_lines` lines each) differing at
    ~120 scattered points plus one final sentinel line - real work for
    `compute_diff`, not a synthetic delay hook. RFC-078: "Deterministic
    automated tests remain the primary proof; this case confirms runtime
    integration" - a large file is the light, real way to get a loading
    window, not elaborate timing machinery."""
    left_path = Path(dir_) / left_name
    right_path = Path(dir_) / right_name
    left_lines = []
    right_lines = []
    for i in range(n_lines):
        base = f"line {i:07d} padding text to make the diff heavier xxxxxxxxxxxxxxxxxxxx\n"
        left_lines.append(base)
        if i % 500 == 250:
            right_lines.append(f"line {i:07d} CHANGED padding text yyyyyyyyyyyyyyyyyyyyyyyyyy\n")
        else:
            right_lines.append(base)
    right_lines[-1] = f"{sentinel}\n"
    left_path.write_text("".join(left_lines))
    right_path.write_text("".join(right_lines))
    return left_path, right_path


def p06(binary, break_mode=False):
    """RFC-078's async-identity case, using two genuinely large (80,000-
    line) synthetic file pairs so `compute_diff` (`tokio::task::
    spawn_blocking` in `open_compare_request`/`reload_tab`) takes real,
    measurable time - a real loading window, not a synthetic hook.

    1. CLI-opens pair A (tab index 0, starts loading). Switches to the
       Explorer tab (`click_any` - `TabBar`'s Explorer tab is a plain
       `div` with no ARIA role, so neither click_button nor click_row
       matches it) and, while tab 0 is presumably still loading, picks
       pair B's two files (`click_row_side` - Explorer's Aligned view
       shows the same directory in both panes by default, so a filename
       can appear as a row on both sides; `left`/`right` disambiguate by
       X position) and clicks Compare, opening pair B as a second tab.
    2. Closes tab 0 (`Close <pair A's tab title>`) while it may still be
       loading.
    3. Asserts the surviving tab shows pair B's own sentinel line - not
       blank, not crashed, and specifically not pair A's sentinel, which
       is the falsifiable content check (a tab-count check alone would be
       vacuous per the handoff's explicit warning: "a check that passes
       whenever two tabs merely exist").
    4. Reloads twice in quick succession (toolbar's reload button),
       rewriting the right-hand file with a different sentinel between
       the two clicks, and asserts the *second* reload's sentinel is what
       ends up displayed - not the first's, regardless of which
       `spawn_blocking` diff happens to finish first.

    `--break`: asserts the first (stale) reload's sentinel is what's
    displayed instead of the second's - false under real (last-reload-
    wins) behaviour, so this must fail; per handoff §6, a vacuous version
    of this check would pass on "some reload happened" alone.
    """
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        home.mkdir()

        sentinel_a = "ASYNC-IDENTITY-SENTINEL-PAIR-A-7f3a91"
        sentinel_b_v1 = "ASYNC-IDENTITY-SENTINEL-PAIR-B-RELOAD-V1"
        sentinel_b_v2 = "ASYNC-IDENTITY-SENTINEL-PAIR-B-RELOAD-V2"

        left_a, right_a = _generate_large_pair(
            home, "big-a-left.txt", "big-a-right.txt", 80_000, sentinel_a
        )
        left_b, right_b = _generate_large_pair(
            home, "big-b-left.txt", "big-b-right.txt", 80_000, sentinel_b_v1
        )

        proc = launch(binary, [left_a, right_a], scratch, home=home)
        try:
            try:
                wait_for_window(time.monotonic() + LAUNCH_TIMEOUT_S)
            except (PermissionWall, TimeoutError) as exc:
                print(f"FAIL: {exc}", file=sys.stderr)
                return 1

            r = ui("click_any", "Explorer", timeout=20)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not switch to the Explorer tab: {r}", file=sys.stderr)
                return 1

            r = poll_ui(
                "click_row_side",
                "big-b-left.txt",
                "left",
                predicate=lambda r: r.startswith("CLICKED"),
                timeout=LAUNCH_TIMEOUT_S,
            )
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not pick big-b-left.txt on the left: {r}", file=sys.stderr)
                return 1
            r = poll_ui(
                "click_row_side",
                "big-b-right.txt",
                "right",
                predicate=lambda r: r.startswith("CLICKED"),
                timeout=LAUNCH_TIMEOUT_S,
            )
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not pick big-b-right.txt on the right: {r}", file=sys.stderr)
                return 1

            r = click_wait("Compare")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Compare: {r}", file=sys.stderr)
                return 1

            r = click_wait("Close big-a-left.txt", timeout=LAUNCH_TIMEOUT_S)
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not close pair A's tab: {r}", file=sys.stderr)
                return 1

            r = find_wait(sentinel_b_v1, timeout=LAUNCH_TIMEOUT_S)
            if not r.startswith("FOUND"):
                print(
                    f"FAIL: pair B's sentinel never appeared after closing pair A's "
                    f"tab mid-load: {r}",
                    file=sys.stderr,
                )
                return 1
            r = ui("find_text", sentinel_a, timeout=20)
            if r.startswith("FOUND"):
                print(
                    f"FAIL: pair A's sentinel is visible after its tab was closed - "
                    "stale content leaked into the surviving tab",
                    file=sys.stderr,
                )
                return 1

            # ── Reload twice in quick succession ──────────────────────
            right_b.write_text(f"content v1\n{sentinel_b_v1}\n")
            r = click_wait("Reload files from disk")
            if not r.startswith("CLICKED"):
                print(f"FAIL: could not click Reload (first): {r}", file=sys.stderr)
                return 1
            right_b.write_text(f"content v2\n{sentinel_b_v2}\n")
            r = ui("click_button", "Reload files from disk", timeout=20)
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
                r = ui("find_text", sentinel_b_v1, timeout=20)
                if r.startswith("FOUND"):
                    print(
                        "FAIL: the first (stale) reload's sentinel is still visible - "
                        "the second reload should have won",
                        file=sys.stderr,
                    )
                    return 1
        finally:
            terminate(proc)

    print(
        "OK: closing pair A's tab mid-load left pair B's tab showing its own "
        "correct content (not pair A's, not blank); two rapid reloads ended with "
        "the second (latest) reload's content displayed, not the first's."
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
            _probe("compare-btn-text", "find_text", "Compare")
            _probe("compare-btn-click", "click_button", "Compare")
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
    "recon_settings": recon_settings,
    "recon_explorer": recon_explorer,
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
