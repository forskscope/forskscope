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

            r = ui("type_into", "AXTextField", "1", str(other_path), timeout=20)
            if not r.startswith("TYPED"):
                print(f"FAIL: could not type into the Save As path field: {r}", file=sys.stderr)
                return 1
            typed_value = r[len("TYPED: ") :]
            if typed_value != str(other_path):
                print(
                    f"FAIL: Save As path field reads back {typed_value!r}, expected "
                    f"{str(other_path)!r} - typing did not take effect",
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
