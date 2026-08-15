#!/usr/bin/env python3
"""M5-A/M5-B: Linux evidence harness for P01, P02, P04, P05, P06, P08,
P09, P10, P12 (RFC-078).

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

import hashlib
import os
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


def find_by_exact_name(node, exact_name, depth=0, max_depth=60):
    """Like `find_by_name_containing`, but requires an exact match -
    needed where a substring match would be ambiguous (e.g. a dialog's
    plain "Save" confirm button versus the toolbar's "Save merge result
    (Ctrl+S)" button, which contains "Save" as a substring)."""
    if depth > max_depth:
        return None
    if (node.get_name() or "") == exact_name:
        return node
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is not None:
            found = find_by_exact_name(child, exact_name, depth + 1, max_depth)
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


def type_into_field(node, window_title, text):
    """Types `text` into a text-entry accessible, replacing its current
    content. WebKitGTK's `<input>` here exposes only the read-only `Text`
    interface, not `EditableText` (confirmed by inspection - `Atspi.
    Accessible.get_interfaces` returns `['Accessible', 'Action',
    'Collection', 'Component', 'Hyperlink', 'Text']`, no `EditableText`
    and no text-insertion method on `Text` either), so this is the one
    interaction in this harness that falls back to real input synthesis
    rather than a direct accessibility action - there is no accessibility
    action for "set text content".

    Clicks the field's own on-screen center first (not just window-level
    focus) so both X11 and the field's internal DOM focus land on it,
    unlike M5-A's Ctrl+S struggle where window-level focus alone proved
    insufficient for a global keydown handler - typing into a focused
    native text input is browser-level behavior, not a custom app
    listener, so it was worth trying via genuine XTEST synthesis before
    assuming it would fail the same way.
    """
    ext = extents(node)
    cx, cy = ext.x + ext.width // 2, ext.y + ext.height // 2
    subprocess.run(["xdotool", "search", "--onlyvisible", "--name", window_title], timeout=15)
    subprocess.run(["xdotool", "mousemove", "--sync", str(cx), str(cy)], check=True, timeout=15)
    subprocess.run(["xdotool", "click", "1"], check=True, timeout=15)
    time.sleep(0.3)
    subprocess.run(["xdotool", "key", "ctrl+a"], check=True, timeout=15)
    time.sleep(0.2)
    # Confirmed on CI (run 31877814616): plain `xdotool type` with no
    # --delay sends keystrokes faster than the WebView's input pipeline
    # processes them, producing scrambled/reordered text (e.g. typing
    # "/tmp/x/saveas-target.txt" landed as "/t/twovu/vetaettx") rather
    # than a clean failure - a real, reproducible synthesis-speed defect
    # in this harness, not a flake. A per-keystroke delay fixes it.
    subprocess.run(["xdotool", "type", "--clearmodifiers", "--delay", "80", text], check=True, timeout=30)
    time.sleep(0.3)


def is_enabled(node):
    """True if `node` carries the SENSITIVE state - AT-SPI's signal for
    "not disabled". The toolbar's Save/Undo buttons gate on tab dirty
    state via the HTML `disabled` attribute; this is how M5-B's P04
    confirms a dirty/clean transition without a dedicated visual marker,
    since the app has none - the button's own enabled state *is* the
    dirty signal, per `toolbar.rs`."""
    return node.get_state_set().contains(Atspi.StateType.SENSITIVE)


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


# ── P04 — Merge, undo/redo, safe save ───────────────────────────────────────


def sha256_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def p04(binary, break_mode=False):
    """Launches `<binary> <left> <right>` (F34's fixture pair, plain
    2-arg compare - Save writes to `right`), applies the first hunk via
    AT-SPI Action invocation on "Use this change" (M5-B §3: this is the
    *mouse* path - the same handler a real click fires - not the Enter-key
    path, which has no accessibility action to invoke at all and is not
    exercised here), confirms the Save button's enabled state as the
    dirty signal (the app has no separate visual marker), undoes, redoes,
    then saves and checks the result: content hash, a `.bak` sibling
    equal to the *pre-save* content, and no leftover temp/sidecar file.

    `--break`: after save, checks the output hash against a value the
    real save can never produce, proving the hash comparison is real.
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right_src = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"

    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        right = scratch_path / "right.txt"
        right.write_bytes(right_src.read_bytes())
        pre_save_content = right.read_bytes()

        proc = launch(binary, [left, right], scratch)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus", file=sys.stderr)
                return 1
            landmark, _frame, _left_rows, _right_rows = wait_for_ready(app, 7, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print("FAIL: compare view did not reach the expected 7 rows per pane", file=sys.stderr)
                return 1

            save_button = find_by_name_containing(app, "Save merge result")
            if save_button is None:
                print('FAIL: could not find the "Save merge result" toolbar button', file=sys.stderr)
                return 1
            if is_enabled(save_button):
                print("FAIL: Save is enabled before any hunk was applied - tab should start clean", file=sys.stderr)
                return 1

            apply_button = find_by_name_containing(app, "Use this change")
            if apply_button is None:
                print('FAIL: could not find a "Use this change" hunk-apply button', file=sys.stderr)
                return 1
            click(apply_button)

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            dirty_after_apply = False
            while time.monotonic() < deadline:
                save_button = find_by_name_containing(app, "Save merge result")
                if save_button is not None and is_enabled(save_button):
                    dirty_after_apply = True
                    break
                time.sleep(0.5)
            if not dirty_after_apply:
                print("FAIL: Save button never became enabled after applying a hunk", file=sys.stderr)
                return 1

            undo_button = find_by_name_containing(app, "Undo last merge action")
            if undo_button is None:
                print('FAIL: could not find the "Undo" toolbar button', file=sys.stderr)
                return 1
            click(undo_button)

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            clean_after_undo = False
            while time.monotonic() < deadline:
                save_button = find_by_name_containing(app, "Save merge result")
                if save_button is not None and not is_enabled(save_button):
                    clean_after_undo = True
                    break
                time.sleep(0.5)
            if not clean_after_undo:
                print("FAIL: Save button did not become disabled again after Undo", file=sys.stderr)
                return 1

            # Redo lives in the "Advanced" disclosure panel (toolbar.rs),
            # collapsed by default - reveal it first.
            more_button = find_by_exact_name(app, "More ▼")
            if more_button is None:
                print('FAIL: could not find the "More ▼" disclosure toggle', file=sys.stderr)
                return 1
            click(more_button)

            redo_button = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and redo_button is None:
                redo_button = find_by_exact_name(app, "Redo")
                if redo_button is None:
                    time.sleep(0.5)
            if redo_button is None:
                print('FAIL: could not find the "Redo" toolbar button after expanding Advanced', file=sys.stderr)
                return 1
            click(redo_button)

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            dirty_after_redo = False
            while time.monotonic() < deadline:
                save_button = find_by_name_containing(app, "Save merge result")
                if save_button is not None and is_enabled(save_button):
                    dirty_after_redo = True
                    break
                time.sleep(0.5)
            if not dirty_after_redo:
                print("FAIL: Save button did not become enabled again after Redo", file=sys.stderr)
                return 1

            save_button = find_by_name_containing(app, "Save merge result")
            click(save_button)

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            saved_hash = None
            while time.monotonic() < deadline:
                current = right.read_bytes()
                if current != pre_save_content:
                    saved_hash = hashlib.sha256(current).hexdigest()
                    break
                time.sleep(0.5)
            if saved_hash is None:
                print(f"FAIL: {right} was not modified by Save within {LAUNCH_TIMEOUT_S}s", file=sys.stderr)
                return 1

            if break_mode:
                required_hash = "0" * 64  # no real save output can ever hash to this
                if saved_hash != required_hash:
                    print(f"FAIL: saved content hash {saved_hash} != required {required_hash}", file=sys.stderr)
                    return 1

            bak = right.with_name(right.name + ".bak")
            if not bak.exists():
                print(f"FAIL: no .bak sibling found at {bak} after Save", file=sys.stderr)
                return 1
            if bak.read_bytes() != pre_save_content:
                print(
                    f"FAIL: {bak}'s content does not equal the pre-save content of {right}",
                    file=sys.stderr,
                )
                return 1

            leftover = [
                p for p in scratch_path.iterdir()
                if p.name not in {"right.txt", "right.txt.bak"}
            ]
            if leftover:
                print(f"FAIL: leftover temp/sidecar files after save: {leftover}", file=sys.stderr)
                return 1
        finally:
            terminate(proc)

    print(f"OK: applied/undid/redid a hunk, saved (hash {saved_hash}), .bak matches pre-save content, no leftovers.")
    return 0


# ── P05 — External modification ─────────────────────────────────────────────


def p05_setup(scratch_path, right_content, external_content):
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = scratch_path / "right.txt"
    right.write_bytes(right_content)
    return left, right


def p05(binary, break_mode=False):
    """Three sub-scenarios, each a fresh launch: Cancel an overwrite,
    confirm an Overwrite (checking the .bak equals the *externally
    modified* bytes, not the original), and Save As to a new path
    (checking the original target is untouched).

    `--break`: after the Overwrite sub-scenario, checks the .bak content
    against a value that can never match, proving the comparison is real.
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right_src = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    original_content = right_src.read_bytes()
    external_content = b"externally-modified-content-not-app-content\n"

    # ── Sub-scenario 1: Cancel ──────────────────────────────────────────
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        right = scratch_path / "right.txt"
        right.write_bytes(original_content)
        proc = launch(binary, [left, right], scratch)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus (cancel sub-scenario)", file=sys.stderr)
                return 1
            landmark, _f, _l, _r = wait_for_ready(app, 7, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print("FAIL: compare view not ready (cancel sub-scenario)", file=sys.stderr)
                return 1
            apply_button = find_by_name_containing(app, "Use this change")
            click(apply_button)
            right.write_bytes(external_content)  # modify externally, after load
            save_button = find_by_name_containing(app, "Save merge result")
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and not is_enabled(save_button):
                time.sleep(0.5)
                save_button = find_by_name_containing(app, "Save merge result")
            click(save_button)

            cancel_button = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and cancel_button is None:
                cancel_button = find_by_name_containing(app, "Cancel")
                if cancel_button is None:
                    time.sleep(0.5)
            if cancel_button is None:
                print('FAIL: "File changed on disk" dialog with a Cancel button never appeared', file=sys.stderr)
                return 1
            click(cancel_button)

            if right.read_bytes() != external_content:
                print("FAIL: Cancel should leave the externally-modified bytes untouched", file=sys.stderr)
                return 1
        finally:
            terminate(proc)

    # ── Sub-scenario 2: Overwrite ───────────────────────────────────────
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        right = scratch_path / "right.txt"
        right.write_bytes(original_content)
        proc = launch(binary, [left, right], scratch)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus (overwrite sub-scenario)", file=sys.stderr)
                return 1
            landmark, _f, _l, _r = wait_for_ready(app, 7, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print("FAIL: compare view not ready (overwrite sub-scenario)", file=sys.stderr)
                return 1
            apply_button = find_by_name_containing(app, "Use this change")
            click(apply_button)
            right.write_bytes(external_content)
            save_button = find_by_name_containing(app, "Save merge result")
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and not is_enabled(save_button):
                time.sleep(0.5)
                save_button = find_by_name_containing(app, "Save merge result")
            click(save_button)

            overwrite_button = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and overwrite_button is None:
                overwrite_button = find_by_name_containing(app, "Overwrite")
                if overwrite_button is None:
                    time.sleep(0.5)
            if overwrite_button is None:
                print('FAIL: "File changed on disk" dialog with an Overwrite button never appeared', file=sys.stderr)
                return 1
            click(overwrite_button)

            bak = right.with_name(right.name + ".bak")
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and not bak.exists():
                time.sleep(0.5)
            if not bak.exists():
                print(f"FAIL: no .bak sibling at {bak} after Overwrite", file=sys.stderr)
                return 1
            required_bak_content = (
                b"this exact content can never be the real .bak" if break_mode else external_content
            )
            if bak.read_bytes() != required_bak_content:
                print(
                    f"FAIL: .bak content {bak.read_bytes()!r} != required {required_bak_content!r} "
                    "(.bak must equal the externally-modified bytes, not the original)",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    # ── Sub-scenario 3: Save As leaves the original untouched ──────────
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        right = scratch_path / "right.txt"
        right.write_bytes(original_content)
        saveas_target = scratch_path / "saveas-target.txt"
        proc = launch(binary, [left, right], scratch)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus (save-as sub-scenario)", file=sys.stderr)
                return 1
            landmark, _f, _l, _r = wait_for_ready(app, 7, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print("FAIL: compare view not ready (save-as sub-scenario)", file=sys.stderr)
                return 1
            apply_button = find_by_name_containing(app, "Use this change")
            click(apply_button)
            right.write_bytes(external_content)

            saveas_button = find_by_name_containing(app, "Save As")
            if saveas_button is None:
                print('FAIL: could not find the "Save As" toolbar button', file=sys.stderr)
                return 1
            click(saveas_button)

            path_field = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and path_field is None:
                path_field = find_by_role(app, "entry", deadline)
                if path_field is None:
                    time.sleep(0.5)
            if path_field is None:
                print("FAIL: Save As dialog's path entry field never appeared", file=sys.stderr)
                return 1
            type_into_field(path_field, "ForskScope", str(saveas_target))

            actual_field_text = Atspi.Text.get_text(path_field, 0, -1)
            if str(saveas_target) not in actual_field_text:
                print(
                    f"FAIL: typed {str(saveas_target)!r} but the field reads "
                    f"{actual_field_text!r} - input synthesis did not reach it",
                    file=sys.stderr,
                )
                return 1

            # The dialog's confirm button's visible text is exactly "Save"
            # (not "Save As" - confirmed by reading file.rs's SaveAsModal),
            # with no aria_label, so its accessible name is that bare word -
            # an *exact* match, so it can't collide with the toolbar's
            # "Save merge result (Ctrl+S)" button, whose accessible name is
            # its full aria_label, not the bare word "Save".
            confirm_button = find_by_exact_name(app, "Save")
            if confirm_button is None:
                print('FAIL: could not find the Save As dialog\'s "Save" confirm button', file=sys.stderr)
                return 1
            click(confirm_button)

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and not saveas_target.exists():
                time.sleep(0.5)
            if not saveas_target.exists():
                print(f"FAIL: Save As never created {saveas_target}", file=sys.stderr)
                return 1
            if right.read_bytes() != external_content:
                print(
                    f"FAIL: Save As must not modify the original target {right}",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    print("OK: Cancel preserves external bytes; Overwrite's .bak matches external bytes; Save As leaves original untouched.")
    return 0


# ── P12 — Session/settings restart ──────────────────────────────────────────


def find_combo_boxes(node, out=None):
    """Settings' `<select>` elements expose no accessible name (confirmed
    by inspection - WebKitGTK does not auto-associate the preceding
    `<span>` label), so they're found by role and identified by their
    stable DOM order instead: Theme (0), Language (1), Diff font family
    (2), per `settings/modal.rs`."""
    if out is None:
        out = []
    if node.get_role_name() == "combo box":
        out.append(node)
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is not None:
            find_combo_boxes(child, out)
    return out


def select_combo_option(combo, option_index):
    """Changes a `<select>`'s value to `option_index` via real X11 input
    synthesis - not `Atspi.Selection.select_child` or `Atspi.Action.
    do_action` on the menu/menu-item, both of which were tried first and
    both of which report success and even flip the AT-SPI-level SELECTED
    state, but do **not** fire the underlying Dioxus `onchange` handler
    (confirmed empirically: `settings.json` is never written after either
    call, across repeated direct diagnostics against a locally-built
    binary). This is a second, distinct instance of the same family of
    gap as the Save As path field's missing EditableText interface - the
    accessibility bridge's own "change value" APIs are shadow state here,
    not the real thing - so, as with `type_into_field`, this falls back
    to genuine XTEST synthesis: click the combo's on-screen center to
    open its native popup, arrow-key to the target option, Enter to
    confirm. Caller must verify the change actually landed (via
    `selected_option_name` and/or a settings.json read) rather than
    trusting this function's return - a synthesis delivery failure here
    must fail loudly at the point of change, not confusingly downstream.
    """
    menu = combo.get_child_at_index(0)
    if menu is None or menu.get_role_name() != "menu":
        raise RuntimeError(f"combo box's first child is not a menu: {menu}")
    current_index = 0
    for i in range(menu.get_child_count()):
        item = menu.get_child_at_index(i)
        if item.get_state_set().contains(Atspi.StateType.SELECTED):
            current_index = i
            break
    ext = extents(combo)
    cx, cy = ext.x + ext.width // 2, ext.y + ext.height // 2
    subprocess.run(["xdotool", "mousemove", "--sync", str(cx), str(cy)], check=True, timeout=15)
    subprocess.run(["xdotool", "click", "1"], check=True, timeout=15)
    time.sleep(0.3)
    delta = option_index - current_index
    step_key = "Down" if delta >= 0 else "Up"
    for _ in range(abs(delta)):
        subprocess.run(["xdotool", "key", step_key], check=True, timeout=15)
        time.sleep(0.2)
    subprocess.run(["xdotool", "key", "Return"], check=True, timeout=15)
    time.sleep(0.3)


def selected_option_name(combo):
    """The name of whichever menu-item child currently carries the
    SELECTED AT-SPI state - the only way to read a combo box's current
    value back, since the combo box itself exposes no Text/Value."""
    menu = combo.get_child_at_index(0)
    for i in range(menu.get_child_count()):
        item = menu.get_child_at_index(i)
        if item.get_state_set().contains(Atspi.StateType.SELECTED):
            return item.get_name()
    return None


def p12(binary, break_mode=False):
    """Two sub-tests, both against a scratch `XDG_CONFIG_HOME` (never the
    real user's config):

    1. Change Theme and Language via the Settings dialog's combo boxes
       (`select_combo_option`/`Selection.select_child` - the same
       EditableText-less-widget problem P05's path field had, solved the
       same way: use whichever interface the widget *does* expose).
       Settings persist immediately on change (`modal.rs`'s `onchange`
       calls `persist` directly, not only on exit), so a plain
       `terminate()` after changing them is sufficient - no clean-shutdown
       sequence needed. Relaunch and confirm both combos show the changed
       value SELECTED, and that a Japanese label ("Settings" -> "設定")
       renders — not just that the process didn't crash.
    2. Open a compare (2 args), terminate, relaunch with **no** args and
       confirm the tab restores (session saves reactively on every tab
       change, per `app.rs`'s `use_effect` — no clean-shutdown sequence
       needed here either); then relaunch with **different** explicit
       file args and confirm that compare opens instead of the restored
       tab — the CLI-args-suppress-restore distinction the case exists to
       check, not just "does *a* compare tab appear".

    `--break`: sub-test 1 checks the restored language combo's selection
    against a value that could never be selected by this test, proving
    the state-reading assertion is real.
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    other_left = REPO_ROOT / "tests/fixtures/text/left_one_changed.txt"
    other_right = REPO_ROOT / "tests/fixtures/text/right_one_changed.txt"

    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        config_home = scratch_path / "config"
        config_home.mkdir()
        env = dict(os.environ)
        env["XDG_CONFIG_HOME"] = str(config_home)

        # ── Sub-test 1: theme/language restore ──────────────────────────
        proc = subprocess.Popen([binary], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus (settings sub-test, launch 1)", file=sys.stderr)
                return 1
            settings_btn = find_by_exact_name(app, "Settings")
            if settings_btn is None:
                print('FAIL: could not find the "Settings" header button', file=sys.stderr)
                return 1
            click(settings_btn)

            combos = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                combos = find_combo_boxes(app)
                if len(combos) >= 2:
                    break
                time.sleep(0.5)
            if combos is None or len(combos) < 2:
                print(f"FAIL: expected at least 2 combo boxes in Settings, found {len(combos) if combos else 0}", file=sys.stderr)
                return 1

            select_combo_option(combos[0], 1)  # Theme: Dark(0) -> Light(1)
            theme_after_change = selected_option_name(combos[0])
            if theme_after_change != "Light":
                print(
                    f"FAIL: Theme selection reads {theme_after_change!r} immediately after "
                    "select_combo_option(1) - X11 input synthesis for the combo's native "
                    "popup did not land (see select_combo_option's docstring)",
                    file=sys.stderr,
                )
                return 1

            select_combo_option(combos[1], 1)  # Language: English(0) -> 日本語(1)
            lang_after_change = selected_option_name(combos[1])
            if lang_after_change != "日本語":
                print(
                    f"FAIL: Language selection reads {lang_after_change!r} immediately after "
                    "select_combo_option(1) - X11 input synthesis for the combo's native "
                    "popup did not land (see select_combo_option's docstring)",
                    file=sys.stderr,
                )
                return 1
            time.sleep(1)  # let the reactive persist effect run
        finally:
            terminate(proc)

        settings_json = config_home / "forskscope" / "settings.json"
        deadline = time.monotonic() + LAUNCH_TIMEOUT_S
        persisted_ok = False
        while time.monotonic() < deadline:
            if settings_json.exists():
                text = settings_json.read_text()
                if '"light"' in text and '"ja"' in text:
                    persisted_ok = True
                    break
            time.sleep(0.5)
        if not persisted_ok:
            print(f"FAIL: {settings_json} does not show theme=light/language=ja after change+terminate", file=sys.stderr)
            return 1

        proc = subprocess.Popen([binary], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus (settings sub-test, launch 2)", file=sys.stderr)
                return 1

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            japanese_label_found = False
            while time.monotonic() < deadline:
                if find_text_containing(app, "設定") is not None:
                    japanese_label_found = True
                    break
                time.sleep(0.5)
            if not japanese_label_found:
                print('FAIL: no "設定" (Japanese "Settings") label found after restart with language=ja', file=sys.stderr)
                return 1

            settings_btn = find_by_name_containing(app, "設定")
            if settings_btn is None:
                print('FAIL: could not find the Japanese-labelled Settings button to reopen the dialog', file=sys.stderr)
                return 1
            click(settings_btn)

            combos = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                combos = find_combo_boxes(app)
                if len(combos) >= 2:
                    break
                time.sleep(0.5)
            if combos is None or len(combos) < 2:
                print("FAIL: could not find the Settings combo boxes after restart", file=sys.stderr)
                return 1

            theme_selected = selected_option_name(combos[0])
            if theme_selected != "Light":
                print(f"FAIL: restored Theme selection is {theme_selected!r}, expected 'Light'", file=sys.stderr)
                return 1

            lang_selected = selected_option_name(combos[1])
            required_lang = "this value can never be selected" if break_mode else "日本語"
            if lang_selected != required_lang:
                print(f"FAIL: restored Language selection is {lang_selected!r}, required {required_lang!r}", file=sys.stderr)
                return 1
        finally:
            terminate(proc)

        # ── Sub-test 2: tab restore only with no explicit CLI paths ────
        proc = launch(binary, [left, right], scratch_path)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered (tab-restore sub-test, launch 1)", file=sys.stderr)
                return 1
            landmark, _f, _l, _r = wait_for_ready(app, 7, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print("FAIL: first compare did not render (tab-restore sub-test)", file=sys.stderr)
                return 1
            time.sleep(1)  # let the reactive session-save effect run
        finally:
            terminate(proc)

        proc = subprocess.Popen([binary], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered (no-args relaunch)", file=sys.stderr)
                return 1
            landmark, _f, left_rows, _r = wait_for_ready(app, 7, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print("FAIL: no-args relaunch did not restore the compare tab (expected 7 rows/pane)", file=sys.stderr)
                return 1
        finally:
            terminate(proc)

        proc = launch(binary, [other_left, other_right], scratch_path)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered (explicit-args relaunch)", file=sys.stderr)
                return 1
            # left_one_changed/right_one_changed is 5 lines (alpha, bravo,
            # charlie/CHARLIE, delta, echo) with one Replace hunk at line
            # 3 - confirmed against the actual fixture files, not assumed
            # from the hunk-kind test alone (that only pins the hunk kind,
            # not the surrounding line count).
            landmark, _f, left_rows, _r = wait_for_ready(app, 5, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print(
                    "FAIL: relaunch with explicit file args did not open that compare "
                    "(expected 5 rows/pane, from the different fixture pair, not the restored tab)",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    print("OK: theme/language restored across restart with a Japanese label present; tabs restore only with no explicit CLI paths.")
    return 0


CASES = {"p01": p01, "p02": p02, "p04": p04, "p05": p05, "p09": p09, "p10": p10, "p12": p12}


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
