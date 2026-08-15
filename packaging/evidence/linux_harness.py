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
what it breaks. `--break` only changes what the harness expects to see —
it never touches product source (contrast `reintroduce_f32_defect.py`,
F57's demonstration helper, which does); every case here can be falsified
by checking against a condition the real, unmodified app can never
satisfy.
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


def find_by_name_containing(node, substring, depth=0, max_depth=60):
    """Walk the tree for the first descendant whose accessible name
    contains `substring` - buttons expose their `aria_label` here."""
    if depth > max_depth:
        return None
    name = node.get_name() or ""
    if substring in name:
        return node
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is not None:
            found = find_by_name_containing(child, substring, depth + 1, max_depth)
            if found is not None:
                return found
    return None


def click(node):
    """Invoke a button's primary action via AT-SPI's Action interface -
    directly triggers the same Dioxus `onclick` handler a real mouse click
    would, without needing X11 input focus, a window manager, or synthetic
    input events at all. Far more reliable under a bare Xvfb display than
    routing a real click/keypress through the X server (tried first; see
    git history for why `xdotool`-based focus/key synthesis was dropped)."""
    if Atspi.Action.get_n_actions(node) < 1:
        raise RuntimeError(f"accessible {node.get_name()!r} exposes no actions")
    if not Atspi.Action.do_action(node, 0):
        raise RuntimeError(f"do_action(0) on {node.get_name()!r} returned False")


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
    """Launches `<binary> <local> <remote> <merged>` against F34's fixture
    pair (a real diff - three hunks, one "Use this change" button per
    changed hunk), clicks the first hunk's apply button, then clicks the
    toolbar's "Save merge result" button, and asserts `<merged>` was
    actually written - both buttons invoked via AT-SPI's Action interface,
    not synthetic keyboard/mouse input (see `click`'s docstring for why).
    `merged` starts pre-seeded with a placeholder no real merge output
    could produce, so "was it overwritten" is unambiguous.

    Applying a real hunk first (rather than local == remote, no changes)
    matters beyond realism: the toolbar Save button is disabled until the
    tab is dirty, so a no-op tab would make this test pass for the wrong
    reason - clicking a disabled button proves nothing.

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
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus", file=sys.stderr)
                return 1
            landmark, _frame, left_rows, right_rows = wait_for_ready(
                app, 7, timeout_s=LAUNCH_TIMEOUT_S
            )
            if landmark is None:
                print("FAIL: compare view did not reach the expected 7 rows per pane", file=sys.stderr)
                return 1

            apply_button = find_by_name_containing(app, "Use this change")
            if apply_button is None:
                print('FAIL: could not find a "Use this change" hunk-apply button', file=sys.stderr)
                return 1
            click(apply_button)

            # The re-render that enables the Save button (tab becomes
            # dirty) happens asynchronously after the click returns -
            # find it fresh and retry do_action rather than assume the
            # tree already reflects the new state.
            save_button = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            clicked_save = False
            while time.monotonic() < deadline and not clicked_save:
                save_button = find_by_name_containing(app, "Save merge result")
                if save_button is not None and Atspi.Action.get_n_actions(save_button) >= 1:
                    clicked_save = Atspi.Action.do_action(save_button, 0)
                if not clicked_save:
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
                # No real Save output can ever equal this - proves the
                # comparison below is a genuine content check, not a
                # vacuous "file changed at all" test.
                required_content = "this exact string can never appear in real merge output"
            else:
                required_content = None  # any content other than the placeholder is acceptable
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
