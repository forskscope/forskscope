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


def launch(binary, args, cwd):
    return subprocess.Popen([str(binary), *[str(a) for a in args]], cwd=str(cwd))


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
