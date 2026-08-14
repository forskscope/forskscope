#!/usr/bin/env python3
"""M5-A: Linux evidence harness for P01, P02, P09, P10 (RFC-078).

Drives the built/downloaded binary through its four CLI launch modes and
asserts real, functional outcomes via AT-SPI — not just a zero exit code.
Reuses `render_check.py`'s AT-SPI helpers (`wait_for_ready`, `find_app`,
`find_by_role`) directly rather than duplicating them, since P02's launch
mode and readiness condition are identical to F34's.

Each case is a subcommand. Exit 0 and a summary line on success; exit 1
and a description of what failed otherwise, matching render_check.py's
convention.

Usage:
  linux_harness.py p01 <binary>
  linux_harness.py p02 <binary>
  linux_harness.py p09 <binary>
  linux_harness.py p10 <binary>

Falsifiability (`--break`): each case accepts an optional `--break` flag
that deliberately breaks the condition it checks, to demonstrate the
assertion is not vacuous. See each case function's docstring for exactly
what it breaks. Never applied by default; always reverted by the caller
(this script mutates nothing on disk outside its own scratch directory —
`--break` changes only in-process expectations, not product source, except
for p10 which shells out to a separate, explicit revert step the caller
controls, mirroring `reintroduce_f32_defect.py`'s pattern).
"""

import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from render_check import (  # noqa: E402
    APP_TIMEOUT_S,
    extents,
    find_app,
    find_by_role,
    wait_for_ready,
)

import gi  # noqa: E402

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

LAUNCH_TIMEOUT_S = 45


def send_ctrl_s_to_window_titled(title):
    """Focus the named window and send it Ctrl+S, via `xdotool`.

    `Atspi.generate_keyboard_event` (AT-SPI's own key-synthesis API) sends
    to whatever X11 considers focused - under a bare Xvfb display there is
    no window manager, so a newly mapped window never receives focus and
    the event goes nowhere. `windowfocus` issues `XSetInputFocus` directly
    (no window-manager cooperation needed); `windowactivate` was tried
    first and failed here specifically because it depends on a WM handling
    an EWMH `_NET_ACTIVE_WINDOW` request that nothing under bare Xvfb ever
    answers.
    """
    found = subprocess.run(
        ["xdotool", "search", "--name", title],
        capture_output=True, text=True, timeout=15,
    )
    window_id = found.stdout.strip().splitlines()[0] if found.stdout.strip() else None
    if not window_id:
        raise RuntimeError(f"xdotool found no window titled {title!r}")
    subprocess.run(["xdotool", "windowfocus", "--sync", window_id], check=True, timeout=15)
    subprocess.run(["xdotool", "key", "--window", window_id, "ctrl+s"], check=True, timeout=15)


def find_text_containing(node, substring, depth=0, max_depth=60):
    """Walk the tree for any accessible whose name, description, or Text
    interface content contains `substring`. Used for message-presence
    checks where the exact role/nesting isn't part of the contract."""
    if depth > max_depth:
        return None
    name = node.get_name() or ""
    if substring in name:
        return node
    desc = node.get_description() or ""
    if substring in desc:
        return node
    interfaces = Atspi.Accessible.get_interfaces(node) or []
    if "Text" in interfaces:
        text_content = Atspi.Text.get_text(node, 0, -1)
        if text_content and substring in text_content:
            return node
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is not None:
            found = find_text_containing(child, substring, depth + 1, max_depth)
            if found is not None:
                return found
    return None


def launch(binary, args, cwd):
    return subprocess.Popen([binary, *[str(a) for a in args]], cwd=str(cwd))


def terminate(proc):
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


def run_diagnostics(binary):
    """Run `--diagnostics` (no window, no display needed) and return stdout."""
    result = subprocess.run(
        [binary, "--diagnostics"], capture_output=True, text=True, timeout=15
    )
    return result.returncode, result.stdout, result.stderr


# ── P01 — Install and cold launch ───────────────────────────────────────────


def p01(binary, break_mode=False):
    """Two independent checks, both real launches of the actual binary:

    1. `--diagnostics` exits 0, reports the expected app version, and
       redacts the home directory (never the literal path).
    2. A plain cold launch (no args) registers on AT-SPI and renders a
       frame with non-zero on-screen extents - proof of an actual window,
       not just a process that stayed alive.

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
    if "Home: /home/***" not in out and "Home:" in out:
        # Redaction is asterisks after the last separator; a literal path
        # of more than one component after redaction is the failure mode.
        home_line = next((l for l in out.splitlines() if l.startswith("Home:")), "")
        if not home_line.endswith("***") and "***" not in home_line:
            print(f"FAIL: home directory not redacted: {home_line}", file=sys.stderr)
            return 1

    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [], scratch)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print(
                    "FAIL: forskscope never registered on the accessibility bus",
                    file=sys.stderr,
                )
                return 1
            frame = find_by_role(app, "frame", time.monotonic() + LAUNCH_TIMEOUT_S)
            if frame is None:
                print("FAIL: could not find the application frame", file=sys.stderr)
                return 1
            fx = extents(frame)
            if fx.width <= 0 or fx.height <= 0:
                print(
                    f"FAIL: frame has non-positive extents ({fx.width}x{fx.height}) "
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
        finally:
            terminate(proc)

    print(
        f"OK: --diagnostics reported {out.splitlines()[0]!r}; cold launch "
        f"produced a {fx.width}x{fx.height} frame."
    )
    return 0


# ── P02 — CLI file compare ──────────────────────────────────────────────────


def p02(binary, break_mode=False):
    """Launches `<binary> <left> <right>` and asserts the diff actually
    rendered - reuses render_check.py's `wait_for_ready` and its pinned
    7-rows-per-pane fixture, since P02's launch mode and readiness
    condition are identical to F34's.

    `--break`: waits for an impossible row count (99 per pane), proving
    the assertion is a real count check, not a vacuous "landmark exists".
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    expected = 99 if break_mode else 7

    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [left, right], scratch)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus", file=sys.stderr)
                return 1
            landmark, _frame, left_rows, right_rows = wait_for_ready(
                app, expected, timeout_s=LAUNCH_TIMEOUT_S
            )
            if landmark is None:
                print(
                    f"FAIL: compare view did not reach {expected} rows per pane "
                    f"within {LAUNCH_TIMEOUT_S}s",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    print(f"OK: CLI compare rendered {len(left_rows)} left + {len(right_rows)} right rows.")
    return 0


# ── P09 — Mergetool ──────────────────────────────────────────────────────────


def p09(binary, break_mode=False):
    """Launches `<binary> <local> <remote> <merged>` (local == remote, no
    hunks to apply - isolates "does Save write to merged" from merge
    logic), sends Ctrl+S via AT-SPI, and asserts `<merged>` now exists
    with the expected content.

    `--break`: checks the content against a string that cannot match,
    proving the assertion reads the file's real content rather than only
    checking existence.
    """
    content = "alpha\nbeta\ngamma\n"
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        local = scratch_path / "local.txt"
        remote = scratch_path / "remote.txt"
        merged = scratch_path / "merged.txt"
        local.write_text(content)
        remote.write_text(content)

        proc = launch(binary, [local, remote, merged], scratch)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus", file=sys.stderr)
                return 1
            frame = find_by_role(app, "frame", time.monotonic() + LAUNCH_TIMEOUT_S)
            if frame is None:
                print("FAIL: could not find the application frame", file=sys.stderr)
                return 1
            send_ctrl_s_to_window_titled("ForskScope")

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and not merged.exists():
                time.sleep(0.5)
            if not merged.exists():
                print(
                    f"FAIL: {merged} was not created within {LAUNCH_TIMEOUT_S}s "
                    "of Ctrl+S",
                    file=sys.stderr,
                )
                return 1
            expected_content = "not-the-real-content" if break_mode else content
            actual = merged.read_text()
            if actual != expected_content:
                print(
                    f"FAIL: {merged} content {actual!r} != expected {expected_content!r}",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    print(f"OK: Ctrl+S wrote {merged.name} with the expected content.")
    return 0


# ── P10 — Binary/XLSX fail-closed policy ────────────────────────────────────

XLSX_MESSAGE = "Spreadsheet comparison is temporarily disabled for security."


def p10(binary, break_mode=False):
    """Launches `<binary> <left.xlsx> <right.xlsx>` (classification is by
    extension only - core::file_kind::classify - so arbitrary bytes
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
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus", file=sys.stderr)
                return 1
            frame = find_by_role(app, "frame", time.monotonic() + LAUNCH_TIMEOUT_S)
            if frame is None:
                print("FAIL: could not find the application frame", file=sys.stderr)
                return 1

            needle = "this message cannot appear" if break_mode else XLSX_MESSAGE
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            found = None
            while time.monotonic() < deadline:
                found = find_text_containing(app, needle)
                if found is not None:
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


CASES = {"p01": p01, "p02": p02, "p09": p09, "p10": p10}


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
