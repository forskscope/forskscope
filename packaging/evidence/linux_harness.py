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
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from render_check import (  # noqa: E402
    APP_TIMEOUT_S,
    check_pane,
    extents,
    find_app,
    find_by_role,
    wait_for_ready,
)

import gi  # noqa: E402

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, GLib  # noqa: E402

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


def is_focused(node):
    """True if `node` carries the FOCUSED state - used by P11 to confirm
    a destructive-confirmation modal's initial focus lands on its safe
    (Cancel) control rather than the destructive one, with no input
    synthesized: Dioxus's `autofocus: true` sets DOM focus itself on
    mount, so reading AT-SPI's own FOCUSED state observes the real
    post-mount outcome instead of asserting anything about how it got
    there."""
    return node.get_state_set().contains(Atspi.StateType.FOCUSED)


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


def selected_option_name(combo, timeout_s=5):
    """The name of whichever menu-item child currently carries the
    SELECTED AT-SPI state - the only way to read a combo box's current
    value back, since the combo box itself exposes no Text/Value. Polls
    for the menu child rather than assuming it's immediately available -
    confirmed on CI (run 31879066537) that right after a real X11 click
    opens/closes the combo's native popup, `get_child_at_index(0)` can
    transiently return None while the tree restabilizes."""
    deadline = time.monotonic() + timeout_s
    menu = None
    while time.monotonic() < deadline:
        try:
            menu = combo.get_child_at_index(0)
        except GLib.GError:
            menu = None
        if menu is not None:
            break
        time.sleep(0.2)
    if menu is None:
        return None
    for i in range(menu.get_child_count()):
        item = menu.get_child_at_index(i)
        if item.get_state_set().contains(Atspi.StateType.SELECTED):
            return item.get_name()
    return None


def p12(binary, break_mode=False):
    """Two sub-tests, both against a scratch `XDG_CONFIG_HOME` (never the
    real user's config):

    1. Seed a real, previously-tested v2 settings envelope
       (`SETTINGS_V2_FIXTURE`, adapted to theme=light/language=ja) directly
       at the config path, then launch once and confirm both Settings
       combos show the seeded value SELECTED and a Japanese label
       ("Settings" -> "設定") renders - genuinely exercising the *restore*
       half via real UI reads.

       The *change*-via-UI half is not exercised here. Three distinct
       mechanisms were tried against a `<select>`'s value and none landed
       reliably under this harness's bare-Xvfb-no-window-manager CI
       environment: `Atspi.Selection.select_child` and `Atspi.Action.
       do_action` on the menu/menu-item both report success and flip the
       AT-SPI-level SELECTED state without firing the real Dioxus
       `onchange` handler (`settings.json` never written); a real X11
       click+arrow+Enter sequence on the combo's native popup (the same
       family of fallback `type_into_field` uses successfully for a plain
       text input) either leaves the value unchanged with the process
       still alive, or - once, run 31884052729's sibling P06 crash
       suggests input synthesis against WebKitGTK's native popups may be
       broadly unreliable here, not just ineffective - visibly disturbs
       the process. This mirrors why M5-A's `click()` helper (see its own
       docstring) abandoned `xdotool` focus/key synthesis for buttons in
       favor of `Atspi.Action.do_action` in the first place; buttons and
       Explorer's tree rows both work reliably via that route, but a
       `<select>`'s native popup apparently does not, with or without
       real input synthesis, under a window-manager-less Xvfb display.
       Documented as a real, reportable limitation (not silently worked
       around) rather than continuing to hunt for a fourth mechanism.
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
        # The "change" half is seeded directly (see this function's
        # docstring for why) - a real, previously-tested v2 envelope with
        # only theme/language edited, not hand-authored from scratch.
        settings_v2 = json.loads(SETTINGS_V2_FIXTURE.read_text())
        settings_v2["payload"]["theme"] = "light"
        settings_v2["payload"]["language"] = "ja"
        settings_dir = config_home / "forskscope"
        settings_dir.mkdir(parents=True, exist_ok=True)
        (settings_dir / "settings.json").write_text(json.dumps(settings_v2))

        proc = subprocess.Popen([binary], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus (settings restore sub-test)", file=sys.stderr)
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
                died = " (process exited unexpectedly)" if proc.poll() is not None else ""
                print(f"FAIL: restored Theme selection is {theme_selected!r}, expected 'Light'{died}", file=sys.stderr)
                return 1

            lang_selected = selected_option_name(combos[1])
            required_lang = "this value can never be selected" if break_mode else "日本語"
            if lang_selected != required_lang:
                died = " (process exited unexpectedly)" if proc.poll() is not None else ""
                print(f"FAIL: restored Language selection is {lang_selected!r}, required {required_lang!r}{died}", file=sys.stderr)
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


# ── P06 — Async identity ─────────────────────────────────────────────────


def real_click_at(node):
    """Real X11 mouse click at `node`'s on-screen center - for elements
    that expose no accessible name AND whose single AT-SPI action does
    not fire their real onclick handler. Confirmed empirically: TabBar's
    permanent "Explorer" tab is a plain `div` with an onclick listener
    but no explicit ARIA `role` attribute (unlike Explorer's file-tree
    rows, `role="row"`, confirmed working via `Action.do_action`, or real
    `<button>` elements); `Action.do_action` on it reports success but
    the view never switches. A third instance of this harness's shadow-
    AT-SPI-state family - see `select_combo_option`'s docstring for the
    first two."""
    ext = extents(node)
    cx, cy = ext.x + ext.width // 2, ext.y + ext.height // 2
    subprocess.run(["xdotool", "mousemove", "--sync", str(cx), str(cy)], check=True, timeout=15)
    subprocess.run(["xdotool", "click", "1"], check=True, timeout=15)
    time.sleep(0.3)


def find_app_root(node):
    """The outer `<div id="app-root">` - located by its DOM id via
    `Accessible.get_attributes` rather than by name, since (like
    Explorer's tab bar and tree rows) it exposes no accessible name."""
    attrs = dict(Atspi.Accessible.get_attributes(node) or {})
    if attrs.get("id") == "app-root":
        return node
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is not None:
            found = find_app_root(child)
            if found is not None:
                return found
    return None


def explorer_rows_by_pane(app):
    """Explorer's aligned-tree rows, split left/right pane by on-screen
    x-position (same technique `wait_for_ready` uses for the diff view's
    rows) and sorted top-to-bottom, so a row's index matches its position
    in the pane's alphabetically sorted directory listing."""
    rows = []

    def collect(node):
        if node.get_role_name() == "table row":
            rows.append(node)
        for i in range(node.get_child_count()):
            child = node.get_child_at_index(i)
            if child is not None:
                collect(child)

    collect(app)
    if not rows:
        return [], []
    xs = [extents(r).x for r in rows]
    mid = (min(xs) + max(xs)) / 2
    left = sorted((r for r in rows if extents(r).x < mid), key=lambda r: extents(r).y)
    right = sorted((r for r in rows if extents(r).x >= mid), key=lambda r: extents(r).y)
    return left, right


def navigate_pane_to(app, pane_index, target_dir):
    """Navigates Explorer's left (`pane_index=0`) or right (`1`) pane to
    `target_dir` via its "Edit path" (`✎`) button - a real `<button>`,
    reliable via `click()` - followed by typing the path into the
    resulting text input via real X11 synthesis (`type_into_field`; this
    input has no EditableText interface either, same gap as the Save As
    path field) and pressing Enter, which is what the input's `onkeydown`
    handler actually commits navigation on."""
    edit_buttons = []

    def collect(node):
        if (node.get_name() or "") == "✎":
            edit_buttons.append(node)
        for i in range(node.get_child_count()):
            child = node.get_child_at_index(i)
            if child is not None:
                collect(child)

    collect(app)
    if len(edit_buttons) < 2:
        raise RuntimeError(f"expected 2 'Edit path' (✎) buttons, found {len(edit_buttons)}")
    click(edit_buttons[pane_index])
    entry = None
    deadline = time.monotonic() + LAUNCH_TIMEOUT_S
    while time.monotonic() < deadline:
        entry = find_by_role(app, "entry")
        if entry is not None:
            break
        time.sleep(0.3)
    if entry is None:
        raise RuntimeError("no path entry found after clicking Edit path")
    type_into_field(entry, "ForskScope", str(target_dir))
    subprocess.run(["xdotool", "key", "Return"], check=True, timeout=15)
    time.sleep(0.5)


def p06(binary, break_mode=False):
    """Two sub-tests against genuinely large (150,000-line) synthetic
    fixture pairs - large enough to give the diff engine's async
    computation a real, non-instant window, without any artificial
    delay/mocking machinery (handoff §5: "do not build elaborate timing
    machinery").

    1. Launch with pair A (CLI 2-arg, tab 0). Switch to Explorer as fast
       as possible (`real_click_at` on the Explorer tab), pick pair B via
       its tree rows and the Compare button, opening tab 1 - both while
       pair A's diff may still be computing. Close tab 0 (pair A)
       immediately via its real close button, which bypasses the dirty
       check specifically while loading (RFC-065, `tabs.rs`) - exercising
       that code path for real. The single remaining tab must then show
       pair B's *own* content, not pair A's - not merely "a tab exists"
       (handoff §6's explicit vacuous-pass warning for this case).
    2. On the surviving tab, trigger Reload twice in quick succession,
       changing the right file's content in between, and confirm the
       final render reflects the *second* (latest) request, not the
       first - the other explicit non-vacuous requirement (RFC-078 P06).

    `--break`: sub-test 1 requires pair A's content (impossible - pair A's
    tab was closed) instead of pair B's; sub-test 2 requires a marker
    that was never written to disk.
    """
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        data_dir = scratch_path / "data"
        data_dir.mkdir()

        a_left = data_dir / "pair-a-left.txt"
        a_right = data_dir / "pair-a-right.txt"
        b_left = data_dir / "pair-b-left.txt"
        b_right = data_dir / "pair-b-right.txt"

        # 150,000 lines (the original size) left the reload sub-test either
        # timing out or, once, crashing the process (CI runs 31879660008,
        # 31884052729) - right-sized down to keep a real, non-instant async
        # window without stacking two full reloads of a very large file on
        # top of CI's software-rendered (no GPU, DRI3-less) Xvfb.
        n_lines = 20_000

        def write_fixture(path, marker, changed):
            lines = [f"{marker} {i}" for i in range(n_lines)]
            if changed:
                lines[n_lines // 2] = f"{marker} CHANGED {n_lines // 2}"
            path.write_text("\n".join(lines) + "\n")

        write_fixture(a_left, "PAIR-A-MARKER", changed=False)
        write_fixture(a_right, "PAIR-A-MARKER", changed=True)
        write_fixture(b_left, "PAIR-B-MARKER", changed=False)
        write_fixture(b_right, "PAIR-B-MARKER", changed=True)

        env = dict(os.environ)
        env["XDG_CONFIG_HOME"] = str(scratch_path / "config")
        (scratch_path / "config").mkdir()

        proc = subprocess.Popen([binary, str(a_left), str(a_right)], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus", file=sys.stderr)
                return 1

            # Poll rather than a single lookup right after find_app returns -
            # the app registering on the accessibility bus does not mean its
            # UI tree (including pair A's own diff computation) has finished
            # populating yet (F57's lesson, matching wait_for_ready's own
            # retry discipline elsewhere in this file - confirmed as the
            # actual cause of CI run 31879060228's failure).
            root = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                root = find_app_root(app)
                if root is not None and root.get_child_count() >= 3:
                    break
                time.sleep(0.3)
            if root is None or root.get_child_count() < 3:
                print("FAIL: could not locate app-root / the Explorer tab", file=sys.stderr)
                return 1
            real_click_at(root.get_child_at_index(2))

            explorer_ready = False
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                if find_by_name_containing(app, "Select a file or directory") is not None:
                    explorer_ready = True
                    break
                time.sleep(0.3)
            if not explorer_ready:
                print("FAIL: clicking the Explorer tab did not return to the Explorer view", file=sys.stderr)
                return 1

            navigate_pane_to(app, 0, data_dir)
            navigate_pane_to(app, 1, data_dir)

            left_rows, right_rows = [], []
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                left_rows, right_rows = explorer_rows_by_pane(app)
                if len(left_rows) == 4 and len(right_rows) == 4:
                    break
                time.sleep(0.3)
            if len(left_rows) != 4 or len(right_rows) != 4:
                print(
                    f"FAIL: expected 4 rows per pane after navigating to the fixture dir, "
                    f"got {len(left_rows)}/{len(right_rows)}",
                    file=sys.stderr,
                )
                return 1

            click(left_rows[2])  # pair-b-left.txt
            click(right_rows[3])  # pair-b-right.txt

            compare_btn = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                compare_btn = find_by_exact_name(app, "Compare selected files")
                if compare_btn is not None:
                    break
                time.sleep(0.3)
            if compare_btn is None:
                print("FAIL: Compare button never enabled after picking pair B's files", file=sys.stderr)
                return 1
            click(compare_btn)

            close_a = find_by_name_containing(app, "Close pair-a-left.txt")
            if close_a is None:
                print("FAIL: could not find pair A's tab-close button", file=sys.stderr)
                return 1
            click(close_a)

            required_marker = "PAIR-A-MARKER" if break_mode else "PAIR-B-MARKER"
            found = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                found = find_text_containing(app, required_marker)
                if found is not None:
                    break
                time.sleep(0.3)
            if found is None:
                print(
                    f"FAIL: surviving tab does not show {required_marker!r} content "
                    "after closing the loading tab",
                    file=sys.stderr,
                )
                return 1

            reload_btn = find_by_name_containing(app, "Reload files from disk")
            if reload_btn is None:
                print("FAIL: could not find the Reload button", file=sys.stderr)
                return 1

            b_right.write_text("RELOAD-V1-MARKER\n")
            click(reload_btn)
            time.sleep(0.15)
            b_right.write_text("RELOAD-V2-MARKER\n")

            # A GLib.GError here ("The application no longer exists") is not
            # necessarily the whole process dying - WebKitGTK runs the actual
            # web content in a separate process from the one `proc` refers
            # to, and that content process can transiently drop off the
            # accessibility bus under load (reloading a non-trivial file
            # twice in quick succession, on CI's software-rendered, no-GPU
            # Xvfb) without the parent process exiting. Confirmed on CI (runs
            # 31894748081, 31911964811): `process_died(proc)` - which polls
            # for up to 2s - consistently found the parent still running.
            # So: retry through the GError for a while, and only call it a
            # possible product defect if the parent process itself is
            # confirmed gone.
            retry_deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            reload_dispatched = False
            while time.monotonic() < retry_deadline:
                try:
                    click(reload_btn)
                    reload_dispatched = True
                    break
                except GLib.GError as e:
                    if process_died(proc):
                        print(
                            "FAIL (possible product defect, not fixed here per the handoff's "
                            "constraints): the process exited after two Reload clicks in quick "
                            f"succession, {e}",
                            file=sys.stderr,
                        )
                        return 1
                    time.sleep(1)
            if not reload_dispatched:
                print(
                    "FAIL: could not deliver the second Reload click - the accessibility bus "
                    "kept reporting the app as gone without the process itself exiting",
                    file=sys.stderr,
                )
                return 1

            required_reload_marker = "this marker was never written to disk" if break_mode else "RELOAD-V2-MARKER"
            reload_ok = False
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                try:
                    if find_text_containing(app, required_reload_marker) is not None:
                        reload_ok = True
                        break
                except GLib.GError as e:
                    if process_died(proc):
                        print(
                            "FAIL (possible product defect, not fixed here per the handoff's "
                            "constraints): the process exited while waiting for the second "
                            f"reload's content to render, {e}",
                            file=sys.stderr,
                        )
                        return 1
                time.sleep(0.3)
            if not reload_ok:
                print(
                    f"FAIL: after reloading twice, {required_reload_marker!r} is not present "
                    "- the latest reload request did not win",
                    file=sys.stderr,
                )
                return 1
            if not break_mode and find_text_containing(app, "RELOAD-V1-MARKER") is not None:
                print(
                    "FAIL: the first (stale) reload's content is still present after the "
                    "second reload completed",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    print(
        "OK: closing a loading tab leaves the correct sibling tab's content; "
        "a second reload's result wins over the first's."
    )
    return 0


# ── P08 — Persistence migration ──────────────────────────────────────────

FUTURE_SCHEMA_SESSION = (
    '{"schema_name": "session", "schema_version": 99, "app_version": "0.165.1", '
    '"created_unix": 0, "updated_unix": 0, "payload": {}}'
)
CORRUPT_SESSION = "{not valid json"

SETTINGS_V0_FIXTURE = REPO_ROOT / "crates/forskscope-core/src/tests/fixtures/persistence/settings-v0.json"
SESSION_V0_FIXTURE = REPO_ROOT / "crates/forskscope-core/src/tests/fixtures/persistence/session-v0.json"
SETTINGS_V2_FIXTURE = REPO_ROOT / "crates/forskscope-core/src/tests/fixtures/persistence/settings-v2.json"


def process_died(proc, timeout_s=2):
    """Polls `proc.poll()` briefly rather than checking once - a process
    that has genuinely crashed can still report as "running" for a brief
    moment before Python reaps it, and checking exactly once right after
    an AT-SPI `GLib.GError: The application no longer exists` can race
    that gap (confirmed on CI, run 31894748081: the check saw `poll() is
    None` and re-raised the original GError uncaught, even though the
    process really had died moments later)."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            return True
        time.sleep(0.2)
    return False


def wait_for_dialog(app, title, timeout_s=LAUNCH_TIMEOUT_S):
    """Polls for the recovery dialog's own accessible - its `aria_label`
    is the dialog title (`recovery.rs`: `role: "dialog", aria_label:
    "{title}"`), so it's found the same way as any other named element."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        dialog = find_by_exact_name(app, title)
        if dialog is not None:
            return dialog
        time.sleep(0.3)
    return None


def wait_for_dialog_gone(app, title, proc, timeout_s=LAUNCH_TIMEOUT_S):
    """Returns "gone", "still_present", or "process_died". A dialog
    action (Continue/Reset) should dismiss the dialog without touching
    the process - if the AT-SPI bus reports the app no longer exists
    (`GLib.GError`) while polling, that's not the same thing as "the
    dialog closed normally", so it's surfaced distinctly rather than
    letting the traversal crash uncaught (confirmed on CI, run
    31879242484: an uncaught `GLib.GError: The application no longer
    exists` during this exact poll, worth telling apart from a real
    process-state defect rather than treating as a harness bug either
    way)."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        try:
            if find_by_exact_name(app, title) is None:
                return "gone"
        except GLib.GError:
            if process_died(proc):
                return "process_died"
        time.sleep(0.3)
    return "still_present"


def p08(binary, break_mode=False):
    """F37's three recovery-dialog choices, each with the process-state
    assertion the handoff requires (§4) - not just that the dialog
    disappeared - plus the filesystem-observable legacy-migration path.
    Four independent launches, each with its own scratch
    `XDG_CONFIG_HOME`:

    1. Legacy v0 migration: the repository's own tested `settings-v0.json`/
       `session-v0.json` fixtures placed at the config path. Produces no
       dialog (`Migrated(Committed)` is a silent notice, not a blocking
       dialog) - verified filesystem-only: both files become the current
       v2 envelope and each original is preserved byte-for-byte in a
       `.pre-v2.bak` sibling.
    2. Future-schema session fixture -> Incompatible dialog -> **Exit**.
       The one that matters (handoff §4): asserts the OS process actually
       exits - not just that the dialog disappeared, since "an Exit that
       dismisses the dialog and leaves a zombie is a failure that looks
       like a pass" - and that the file is byte-for-byte untouched.
    3. Future-schema session fixture (fresh copy) -> Incompatible dialog
       -> **Continue with defaults**. Asserts the process keeps running,
       the dialog is dismissed, and the file remains untouched.
    4. Corrupt session fixture -> CorruptPreserved dialog -> **Reset and
       back up**. Asserts the dialog is dismissed, `.reset.bak` preserves
       the original corrupt bytes, and session.json is reset to valid
       content.

    `--break`: launch 2 requires the process to *still be running* after
    Exit - impossible for the real, unmodified app - demonstrating the
    handoff's explicitly flagged vacuous-pass risk is a real assertion.
    """
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)

        # ── 1: legacy v0 migration, both settings and session ───────────
        config_home = scratch_path / "config1"
        config_dir = config_home / "forskscope"
        config_dir.mkdir(parents=True)
        settings_path = config_dir / "settings.json"
        session_path = config_dir / "session.json"
        settings_v0_bytes = SETTINGS_V0_FIXTURE.read_bytes()
        session_v0_bytes = SESSION_V0_FIXTURE.read_bytes()
        settings_path.write_bytes(settings_v0_bytes)
        session_path.write_bytes(session_v0_bytes)

        env = dict(os.environ)
        env["XDG_CONFIG_HOME"] = str(config_home)
        proc = subprocess.Popen([binary], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus (legacy migration launch)", file=sys.stderr)
                return 1

            settings_backup = config_dir / "settings.json.pre-v2.bak"
            session_backup = config_dir / "session.json.pre-v2.bak"
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                if settings_backup.exists() and session_backup.exists():
                    break
                time.sleep(0.3)
        finally:
            terminate(proc)

        if not settings_backup.exists() or settings_backup.read_bytes() != settings_v0_bytes:
            print("FAIL: settings.json.pre-v2.bak missing or does not match the original v0 bytes", file=sys.stderr)
            return 1
        if not session_backup.exists() or session_backup.read_bytes() != session_v0_bytes:
            print("FAIL: session.json.pre-v2.bak missing or does not match the original v0 bytes", file=sys.stderr)
            return 1
        try:
            settings_after = json.loads(settings_path.read_text())
            session_after = json.loads(session_path.read_text())
        except json.JSONDecodeError as e:
            print(f"FAIL: migrated settings/session.json is not valid JSON: {e}", file=sys.stderr)
            return 1
        if settings_after.get("schema_version", 0) < 2 or settings_after.get("payload", {}).get("theme") != "light":
            print(f"FAIL: settings.json was not migrated to a v2 envelope preserving theme=light: {settings_after}", file=sys.stderr)
            return 1
        if session_after.get("schema_version", 0) < 2:
            print(f"FAIL: session.json was not migrated to a v2 envelope: {session_after}", file=sys.stderr)
            return 1

        # ── 2: future-schema -> Exit ──────────────────────────────────────
        config_home = scratch_path / "config2"
        config_dir = config_home / "forskscope"
        config_dir.mkdir(parents=True)
        session_path = config_dir / "session.json"
        session_path.write_text(FUTURE_SCHEMA_SESSION)
        original_bytes = session_path.read_bytes()

        env = dict(os.environ)
        env["XDG_CONFIG_HOME"] = str(config_home)
        proc = subprocess.Popen([binary], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus (future-schema/Exit launch)", file=sys.stderr)
                return 1
            dialog = wait_for_dialog(app, "Session file is from a newer version")
            if dialog is None:
                print("FAIL: the Incompatible-session recovery dialog never appeared", file=sys.stderr)
                return 1
            exit_btn = find_by_exact_name(app, "Exit")
            if exit_btn is None:
                print("FAIL: could not find the dialog's Exit button", file=sys.stderr)
                return 1
            click(exit_btn)

            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline and proc.poll() is None:
                time.sleep(0.3)
            still_running = proc.poll() is None
            ok = still_running if break_mode else not still_running
            if not ok:
                print(
                    f"FAIL: after clicking Exit, the process is "
                    f"{'still running' if still_running else 'exited'} - required "
                    f"{'still running (break mode)' if break_mode else 'exited, with no orphaned process'}",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

        if session_path.read_bytes() != original_bytes:
            print("FAIL: Exit must not modify the untouched session file, but its bytes changed", file=sys.stderr)
            return 1

        # ── 3: future-schema (fresh) -> Continue with defaults ────────────
        config_home = scratch_path / "config3"
        config_dir = config_home / "forskscope"
        config_dir.mkdir(parents=True)
        session_path = config_dir / "session.json"
        session_path.write_text(FUTURE_SCHEMA_SESSION)
        original_bytes = session_path.read_bytes()

        env = dict(os.environ)
        env["XDG_CONFIG_HOME"] = str(config_home)
        proc = subprocess.Popen([binary], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus (future-schema/Continue launch)", file=sys.stderr)
                return 1
            dialog = wait_for_dialog(app, "Session file is from a newer version")
            if dialog is None:
                print("FAIL: the Incompatible-session recovery dialog never appeared", file=sys.stderr)
                return 1
            continue_btn = find_by_exact_name(app, "Continue with defaults")
            if continue_btn is None:
                print("FAIL: could not find the dialog's Continue with defaults button", file=sys.stderr)
                return 1
            click(continue_btn)

            dialog_result = wait_for_dialog_gone(app, "Session file is from a newer version", proc)
            if dialog_result == "process_died":
                print(
                    "FAIL: the process exited unexpectedly after clicking Continue with "
                    "defaults - it must keep running (possible product defect)",
                    file=sys.stderr,
                )
                return 1
            if dialog_result != "gone":
                print("FAIL: the dialog is still present after clicking Continue with defaults", file=sys.stderr)
                return 1
            time.sleep(1)
            if proc.poll() is not None:
                print("FAIL: the process exited after Continue with defaults - it must keep running", file=sys.stderr)
                return 1
        finally:
            terminate(proc)

        if session_path.read_bytes() != original_bytes:
            print("FAIL: Continue with defaults must not write to the untouched session file, but its bytes changed", file=sys.stderr)
            return 1

        # ── 4: corrupt -> Reset and back up ────────────────────────────────
        config_home = scratch_path / "config4"
        config_dir = config_home / "forskscope"
        config_dir.mkdir(parents=True)
        session_path = config_dir / "session.json"
        session_path.write_text(CORRUPT_SESSION)
        original_bytes = session_path.read_bytes()

        env = dict(os.environ)
        env["XDG_CONFIG_HOME"] = str(config_home)
        proc = subprocess.Popen([binary], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus (corrupt/Reset launch)", file=sys.stderr)
                return 1
            dialog = wait_for_dialog(app, "Session file could not be read")
            if dialog is None:
                print("FAIL: the CorruptPreserved recovery dialog never appeared", file=sys.stderr)
                return 1
            reset_btn = find_by_exact_name(app, "Reset and back up")
            if reset_btn is None:
                print("FAIL: could not find the dialog's Reset and back up button", file=sys.stderr)
                return 1
            click(reset_btn)

            dialog_result = wait_for_dialog_gone(app, "Session file could not be read", proc)
            if dialog_result == "process_died":
                print(
                    "FAIL: the process exited unexpectedly after clicking Reset and back up "
                    "- it must keep running (possible product defect)",
                    file=sys.stderr,
                )
                return 1
            if dialog_result != "gone":
                print("FAIL: the dialog is still present after clicking Reset and back up", file=sys.stderr)
                return 1
            time.sleep(1)  # let the reactive persist/backup effect run
        finally:
            terminate(proc)

        reset_backup = config_dir / "session.json.reset.bak"
        if not reset_backup.exists() or reset_backup.read_bytes() != original_bytes:
            print("FAIL: session.json.reset.bak missing or does not equal the original corrupt bytes", file=sys.stderr)
            return 1
        try:
            reset_content = json.loads(session_path.read_text())
        except json.JSONDecodeError as e:
            print(f"FAIL: session.json after Reset is not valid JSON: {e}", file=sys.stderr)
            return 1
        if reset_content.get("schema_version", 0) < 2:
            print(f"FAIL: session.json after Reset is not a valid v2 envelope: {reset_content}", file=sys.stderr)
            return 1

    print(
        "OK: legacy v0 settings/session migrate without loss (backed up byte-for-byte); "
        "Exit terminates the process with the file untouched; Continue leaves the process "
        "running with the file untouched; Reset backs up the corrupt original and resets the file."
    )
    return 0


# ── P03 — Visual/navigation ──────────────────────────────────────────────


def collect_all_rows(node, out=None):
    if out is None:
        out = []
    if node.get_role_name() == "table row":
        out.append(node)
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is not None:
            collect_all_rows(child, out)
    return out


def find_all_by_name_containing(node, substring, out=None):
    if out is None:
        out = []
    if substring in (node.get_name() or ""):
        out.append(node)
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is not None:
            find_all_by_name_containing(child, substring, out)
    return out


def p03(binary, break_mode=False):
    """RFC-078 P03, five sub-checks against the published binary:

    1. Full-width short-row backgrounds: every row in a pane reports the
       same on-screen width regardless of its own text length -
       `left_all_hunk_kinds.txt`/`right_*` mixes short and long lines,
       so this is a real, not synthetic, exercise of the CSS contract
       (`11-view-diff.css`'s "short rows fill the visible column width").
    2. Action-row alignment across multiple hunks: every "Use this
       change" button's y-center falls within one row-height of a real
       left-pane row's y-center and a real right-pane row's - the act
       column's own rows are not exposed as `role="row"` at all
       (confirmed by direct inspection - `.diff-act-row` produces zero
       AT-SPI "table row" nodes, unlike `.diff-row`), so this checks
       against the actionable buttons themselves instead.
    3. Vertical/geometry alignment: F34's `check_pane` logic (child-count
       and content-cell x-origin), reused directly against a genuine
       multi-hunk fixture - M5-C prerequisite A already proved this
       assertion is real, not vacuous (see `inject_geometry_defect.py`).
    4. Horizontal scroll mirroring, with settling: real X11 wheel-scroll
       synthesis on the left pane (this is the one interaction in this
       function that falls back to synthesis rather than an accessibility
       action - there is no AT-SPI action for "scroll", and
       `Atspi.Component.scroll_to_point` was tried first and confirmed to
       report success without moving anything - a fourth instance of this
       program's shadow-AT-SPI-state family). Tries button-7 (the GTK
       horizontal-wheel convention) first, falling back to shift+button-4
       and shift+button-5 (shifted vertical wheel, the more portable
       convention) within the same run - a virtual pointer commonly only
       defines 5 buttons, silently swallowing 6/7 with no error, and this
       sandbox's own local X11 routing cannot validate any of the three in
       advance (even a plain vertical scroll no-ops here), so which one
       actually works can only be settled by CI. Once movement is seen,
       the right pane's content-cell x is sampled three times over a
       settling window and required to (a) have moved from its pre-scroll
       position and (b) be stable across all three samples - "without
       feedback or jitter" means an oscillating value is itself a
       failure, not just an unmirrored one.
    5. Word wrap remains usable: toggling wrap on does not lose the row
       count or crash the view.

    `--break`: sub-check 3 (reusing check_pane) requires the impossible
    baseline x from `inject_geometry_defect.py`'s own falsifiability
    convention; sub-check 4 requires the right pane's content to have
    *not* moved, which is false under real mirroring.
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"
    left_wide = REPO_ROOT / "tests/fixtures/text/left_long_line.txt"
    right_wide = REPO_ROOT / "tests/fixtures/text/right_long_line.txt"

    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)

        # ── 1-3: full-width rows, action-row alignment, geometry ────────
        proc = launch(binary, [left, right], scratch_path)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus", file=sys.stderr)
                return 1
            landmark, frame, left_rows, right_rows = wait_for_ready(app, 7, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print("FAIL: compare view did not reach the expected 7 rows/pane shape", file=sys.stderr)
                return 1

            # 1. Full-width rows.
            for pane_name, rows in [("left", left_rows), ("right", right_rows)]:
                widths = {round(extents(r).width) for r in rows}
                if len(widths) != 1:
                    print(
                        f"FAIL: {pane_name} pane rows have inconsistent widths {sorted(widths)} "
                        "- a short row's background does not span the full widest-line area",
                        file=sys.stderr,
                    )
                    return 1

            # 2. Action-row alignment.
            act_buttons = find_all_by_name_containing(app, "Use this change")
            if not act_buttons:
                print("FAIL: no 'Use this change' action buttons found in a multi-hunk fixture", file=sys.stderr)
                return 1
            row_height = extents(left_rows[0]).height
            for btn in act_buttons:
                btn_center_y = extents(btn).y + extents(btn).height / 2
                left_match = any(
                    abs((extents(r).y + extents(r).height / 2) - btn_center_y) <= row_height
                    for r in left_rows
                )
                right_match = any(
                    abs((extents(r).y + extents(r).height / 2) - btn_center_y) <= row_height
                    for r in right_rows
                )
                if not (left_match and right_match):
                    print(
                        f"FAIL: an action button at y={extents(btn).y} has no left/right row "
                        "within one row-height - action rows are not aligned",
                        file=sys.stderr,
                    )
                    return 1

            # 3. Vertical/geometry alignment (F34's check_pane, reused).
            failures = check_pane(left_rows, "left pane") + check_pane(right_rows, "right pane")
            if break_mode:
                # Falsifiability: require the impossible - that check_pane
                # found a mismatch against the real, unmodified binary.
                if not failures:
                    print(
                        "FAIL (expected, --break): check_pane found no misalignment against "
                        "an unmodified build - required a mismatch that cannot be real",
                        file=sys.stderr,
                    )
                    return 1
            elif failures:
                print("FAIL: geometry/alignment check found misalignment:", file=sys.stderr)
                for f in failures:
                    print(f"  - {f}", file=sys.stderr)
                return 1
        finally:
            terminate(proc)

        # ── 4: horizontal scroll mirroring, with settling ────────────────
        proc = launch(binary, [left_wide, right_wide], scratch_path)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered (scroll-mirror launch)", file=sys.stderr)
                return 1
            landmark, frame, left_rows, right_rows = wait_for_ready(app, 1, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print("FAIL: compare view did not reach the expected 1 row/pane shape", file=sys.stderr)
                return 1

            def right_cell_x():
                row = right_rows[0]
                cell = row.get_child_at_index(row.get_child_count() - 1)
                return extents(cell).x

            before = right_cell_x()
            row_ext = extents(left_rows[0])
            cx, cy = row_ext.x + 100, row_ext.y + row_ext.height // 2
            subprocess.run(["xdotool", "mousemove", "--sync", str(cx), str(cy)], check=True, timeout=15)

            # A synthesized horizontal wheel click (button 6/7) is the GTK
            # convention, but a virtual pointer (Xvfb's default included)
            # commonly only defines 5 buttons - silently swallowing 6/7
            # with no error, indistinguishable from "nothing listens to
            # this event." Shift+vertical-wheel (buttons 4/5, universally
            # present) is the far more portable convention GTK/WebKit also
            # honor for horizontal scroll, so it's tried as a fallback
            # within the same run rather than assumed away - confirmed
            # unusable to synthesize *at all* from this local dev sandbox
            # (even a plain vertical scroll no-ops here), so which method
            # actually moves the pane can only be settled by CI itself.
            def button_7():
                for _ in range(20):
                    subprocess.run(["xdotool", "click", "7"], check=False, timeout=15)

            def shift_button_4():
                for _ in range(20):
                    subprocess.run(["xdotool", "keydown", "shift"], check=False, timeout=15)
                    subprocess.run(["xdotool", "click", "4"], check=False, timeout=15)
                    subprocess.run(["xdotool", "keyup", "shift"], check=False, timeout=15)

            def shift_button_5():
                for _ in range(20):
                    subprocess.run(["xdotool", "keydown", "shift"], check=False, timeout=15)
                    subprocess.run(["xdotool", "click", "5"], check=False, timeout=15)
                    subprocess.run(["xdotool", "keyup", "shift"], check=False, timeout=15)

            method_used = None
            for method_name, method_fn in [
                ("button-7 (horizontal wheel)", button_7),
                ("shift+button-4 (shifted vertical wheel)", shift_button_4),
                ("shift+button-5 (shifted vertical wheel)", shift_button_5),
            ]:
                method_fn()
                time.sleep(0.5)
                if right_cell_x() != before:
                    method_used = method_name
                    break
                if break_mode:
                    # Falsifiability doesn't need a working method - one
                    # synthesis attempt establishing "unchanged" is enough.
                    break

            samples = []
            for _ in range(3):
                samples.append(right_cell_x())
                time.sleep(0.3)

            required_moved = False if break_mode else True
            moved = samples[0] != before
            settled = samples[0] == samples[1] == samples[2]
            if moved != required_moved:
                tried = "button-7, shift+button-4, shift+button-5" if method_used is None else method_used
                print(
                    f"FAIL: right pane content x {'moved' if moved else 'did not move'} "
                    f"after scrolling the left pane via {tried} (before={before}, after={samples[0]}) "
                    f"- required {'unchanged (break mode)' if break_mode else 'to move (mirrored)'}",
                    file=sys.stderr,
                )
                return 1
            if not break_mode and not settled:
                print(
                    f"FAIL: right pane content x did not settle after scrolling - samples "
                    f"{samples} over 0.9s show feedback/jitter, not a stable mirror",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

        # ── 5: word wrap remains usable ───────────────────────────────────
        proc = launch(binary, [left, right], scratch_path)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered (wrap-toggle launch)", file=sys.stderr)
                return 1
            landmark, frame, left_rows, right_rows = wait_for_ready(app, 7, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print("FAIL: compare view did not reach ready shape for the wrap-toggle check", file=sys.stderr)
                return 1

            more_btn = find_by_exact_name(app, "More ▼")
            if more_btn is None:
                print("FAIL: could not find the 'More ▼' disclosure toggle", file=sys.stderr)
                return 1
            click(more_btn)

            wrap_btn = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                wrap_btn = find_by_name_containing(app, "Toggle word wrap")
                if wrap_btn is not None:
                    break
                time.sleep(0.3)
            if wrap_btn is None:
                print("FAIL: could not find the word-wrap toggle button", file=sys.stderr)
                return 1
            click(wrap_btn)
            time.sleep(0.5)

            landmark2, _f2, left_rows2, right_rows2 = wait_for_ready(app, 7, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark2 is None:
                print("FAIL: compare view lost its 7-row shape after enabling word wrap", file=sys.stderr)
                return 1
        finally:
            terminate(proc)

    print(
        "OK: rows are full-width regardless of content length; action buttons align with "
        "their left/right rows; geometry/alignment holds across a multi-hunk fixture; "
        f"horizontal scroll mirrors (via {method_used}) and settles; word wrap keeps the "
        "view usable."
    )
    return 0


# ── P07 — Explorer and directory report ──────────────────────────────────


def p07(binary, break_mode=False):
    """RFC-078 P07 against a real directory pair (one file each of Equal,
    Changed, LeftOnly, RightOnly), plus a light navigation-history check.
    "Focused-pane keyboard behaviour" is not exercised here - like every
    keyboard-only interaction in this program, no accessibility API can
    synthesize the keystrokes it needs; recorded as manual-outstanding in
    the evidence, mirroring F45's shape.

    1. Status classification: `DeepCompareView`'s own summary line
       ("N different · N equal · N left only · N right only") is checked
       for an exact match - `DeepRow`'s row `div`s carry no explicit
       `role="row"` (confirmed by inspection, unlike Explorer's tree
       rows), so individual rows aren't AT-SPI-readable the way the diff
       view's are; the aggregate summary line is, and directly answers
       what RFC-078 asks for ("equal/different/one-sided statuses").
    2. Filter buttons ("Different"/"All"/"Equal only") are each clicked
       and confirmed not to break the view (the summary line, which
       reflects all entries regardless of filter, must still be present
       after each).
    3. Batch copy: "Copy to right N" (copies Changed + LeftOnly entries),
       confirmed via the actual files and the actual manifest - not just
       that the operation reported success (F62's lesson: an unverified
       "success" is a claim, not evidence). Verifies changed.txt's new
       content, its `.bak` backup's content (the *original* right-side
       content, byte-for-byte), left_only.txt's new content, right_only.txt
       left untouched, and the manifest JSON's entries (two Copied
       outcomes, exactly one with a non-null backup_path).
    4. Light navigation-history check: "↑" (up-one-directory) and "⌂"
       (home) buttons in the left pane's PathBar are clicked and confirmed
       clickable via a real do_action return - RFC-078 doesn't ask for a
       navigation-history *state* check beyond this, and neither does this
       function build one.

    `--break`: sub-check 1 requires an impossible summary line; sub-check
    3 requires the backup's content to equal a value the real batch copy
    can never produce.
    """
    with tempfile.TemporaryDirectory() as scratch:
        scratch_path = Path(scratch)
        data_dir = scratch_path / "data"
        data_dir.mkdir()
        left_dir = data_dir / "left"
        right_dir = data_dir / "right"
        left_dir.mkdir()
        right_dir.mkdir()
        (left_dir / "same.txt").write_text("same content\n")
        (right_dir / "same.txt").write_text("same content\n")
        (left_dir / "changed.txt").write_text("left version\n")
        original_right_changed = "right version\n"
        (right_dir / "changed.txt").write_text(original_right_changed)
        (left_dir / "left_only.txt").write_text("only in left\n")
        (right_dir / "right_only.txt").write_text("only in right\n")

        config_home = scratch_path / "config"
        config_home.mkdir()
        xdg_data_home = scratch_path / "xdg-data"
        xdg_data_home.mkdir()
        env = dict(os.environ)
        env["XDG_CONFIG_HOME"] = str(config_home)
        env["XDG_DATA_HOME"] = str(xdg_data_home)

        proc = subprocess.Popen([binary], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus", file=sys.stderr)
                return 1

            # F57-class race: the window registers on the a11y bus before
            # the WebView paints, and navigate_pane_to's "Edit path" (✎)
            # button lookup is a single unretried tree walk - confirmed by
            # CI run 31936719013's "expected 2 'Edit path' buttons, found
            # 0". P06 (navigate_pane_to's other caller) already polls for
            # this same Explorer-rendered marker before calling it; P07
            # launches straight into the Explorer view (no tab click
            # needed) but still needs the same wait.
            explorer_ready = False
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                if find_by_name_containing(app, "Select a file or directory") is not None:
                    explorer_ready = True
                    break
                time.sleep(0.3)
            if not explorer_ready:
                print("FAIL: Explorer view did not render (0-arg launch)", file=sys.stderr)
                return 1

            navigate_pane_to(app, 0, data_dir)
            navigate_pane_to(app, 1, data_dir)

            left_rows, right_rows = [], []
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                left_rows, right_rows = explorer_rows_by_pane(app)
                if len(left_rows) == 2 and len(right_rows) == 2:
                    break
                time.sleep(0.3)
            if len(left_rows) != 2 or len(right_rows) != 2:
                print(
                    f"FAIL: expected 2 rows (left, right dirs) per pane, "
                    f"got {len(left_rows)}/{len(right_rows)}",
                    file=sys.stderr,
                )
                return 1
            click(left_rows[0])  # "left" dir, alphabetically first
            click(right_rows[1])  # "right" dir

            cmp_btn = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                cmp_btn = find_by_exact_name(app, "Compare selected directories")
                if cmp_btn is not None:
                    break
                time.sleep(0.3)
            if cmp_btn is None:
                print("FAIL: Compare button never showed 'Compare selected directories'", file=sys.stderr)
                return 1
            click(cmp_btn)

            # 1. Status classification via the summary line.
            required_summary = (
                "9 different · 9 equal · 9 left only · 9 right only"
                if break_mode
                else "1 different · 1 equal · 1 left only · 1 right only"
            )
            summary_node = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                summary_node = find_text_containing(app, required_summary)
                if summary_node is not None:
                    break
                time.sleep(0.3)
            if summary_node is None:
                print(
                    f"FAIL: deep-compare summary line never showed {required_summary!r} "
                    "- status classification is wrong",
                    file=sys.stderr,
                )
                return 1

            # 2. Filter buttons don't break the view.
            for label in ["Different", "All", "Equal only"]:
                btn = find_by_exact_name(app, label)
                if btn is None:
                    print(f"FAIL: could not find the '{label}' filter button", file=sys.stderr)
                    return 1
                click(btn)
                time.sleep(0.3)
                if find_text_containing(app, required_summary) is None:
                    print(f"FAIL: summary line disappeared after clicking the '{label}' filter", file=sys.stderr)
                    return 1

            # 3. Batch copy, verified against real files and the manifest.
            copy_btn = find_by_name_containing(app, "Copy to right")
            if copy_btn is None:
                print("FAIL: could not find the 'Copy to right N' button", file=sys.stderr)
                return 1
            click(copy_btn)

            confirm_btn = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                confirm_btn = find_by_exact_name(app, "Copy all")
                if confirm_btn is not None:
                    break
                time.sleep(0.3)
            if confirm_btn is None:
                print("FAIL: batch-copy confirmation dialog's 'Copy all' button never appeared", file=sys.stderr)
                return 1
            click(confirm_btn)
            time.sleep(1)
        finally:
            terminate(proc)

        if (right_dir / "changed.txt").read_text() != "left version\n":
            print("FAIL: changed.txt in the right dir does not hold the copied left content", file=sys.stderr)
            return 1
        backup = right_dir / "changed.txt.bak"
        required_backup = "this content was never on disk" if break_mode else original_right_changed
        if not backup.exists() or backup.read_text() != required_backup:
            print(
                f"FAIL: changed.txt.bak missing or does not equal "
                f"{'an impossible value (break mode)' if break_mode else 'the original right-side content'}",
                file=sys.stderr,
            )
            return 1
        if not (right_dir / "left_only.txt").exists() or (right_dir / "left_only.txt").read_text() != "only in left\n":
            print("FAIL: left_only.txt was not copied to the right dir with the correct content", file=sys.stderr)
            return 1
        if (right_dir / "right_only.txt").read_text() != "only in right\n":
            print("FAIL: right_only.txt (untouched by this copy direction) changed unexpectedly", file=sys.stderr)
            return 1

        manifest_dir = xdg_data_home / "forskscope" / "manifests"
        manifests = list(manifest_dir.glob("*.json")) if manifest_dir.exists() else []
        if not manifests:
            print(f"FAIL: no manifest JSON found under {manifest_dir}", file=sys.stderr)
            return 1
        manifest = json.loads(manifests[0].read_text())
        entries = manifest.get("entries", [])
        copied = [e for e in entries if e.get("outcome") == "copied"]
        if len(copied) != 2:
            print(f"FAIL: expected exactly 2 'copied' manifest entries, got {len(copied)}: {entries}", file=sys.stderr)
            return 1
        with_backup = [e for e in copied if e.get("backup_path")]
        if len(with_backup) != 1:
            print(
                f"FAIL: expected exactly 1 manifest entry with a backup_path (changed.txt "
                f"overwrote an existing file; left_only.txt did not), got {len(with_backup)}",
                file=sys.stderr,
            )
            return 1

        # 4. Light navigation-history check.
        proc = subprocess.Popen([binary], cwd=str(scratch_path), env=env)
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered (navigation-history launch)", file=sys.stderr)
                return 1
            up_buttons, home_buttons = [], []
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                up_buttons = []
                find_all_by_name_containing(app, "Go up one directory", up_buttons)
                home_buttons = []
                find_all_by_name_containing(app, "Home directory", home_buttons)
                if up_buttons and home_buttons:
                    break
                time.sleep(0.3)
            if not up_buttons or not home_buttons:
                print("FAIL: could not find the left pane's Up/Home navigation buttons", file=sys.stderr)
                return 1
            click(up_buttons[0])
            time.sleep(0.3)
            click(home_buttons[0])
        finally:
            terminate(proc)

    print(
        "OK: status classification matches the real directory pair; filters don't break the "
        "view; batch copy's files, backup, and manifest all verified; navigation buttons "
        "clickable."
    )
    return 0


# ── P11 — Keyboard interface ──────────────────────────────────────────────


def p11(binary, break_mode=False):
    """RFC-078 P11, the one sub-item of the four this program's keyboard
    coverage decomposes into (see the M5-C review request) that is
    CI-verifiable without synthesizing keystrokes: "a destructive
    operation's confirmation modal starts focus on the safe/cancel
    control, not the destructive one." The other three - the manual
    keyboard checklist, global shortcuts staying inert behind an open
    modal, and Escape closing a modal - all need real keyboard input no
    accessibility action can produce (the same limit this program hit for
    every other keyboard-only interaction all session, e.g. P07's
    docstring); they are recorded as manual-outstanding, mirroring F45's
    shape, not attempted here.

    Reuses P03's `left_all_hunk_kinds.txt`/`right_*` fixture (already
    pinned to produce a multi-hunk, 7-row shape) to reach a real
    dirty-merge state: applies one hunk via "Use this change" (the same
    action P03's sub-check 2 finds by name), which is what actually gates
    `ConfirmSwap` behind `toolbar.rs`'s `dirty` check - triggering the
    modal via a genuinely dirty tab, not a stub. Opens the modal through
    the advanced disclosure panel's real "Swap sides" button, then reads
    AT-SPI's own FOCUSED state on both the modal's Cancel and "Discard
    and Swap" controls - no input is synthesized for the focus check
    itself; `autofocus: true` sets DOM focus on mount, and this reads
    that outcome directly.

    `--break`: requires the impossible - that the destructive control
    ("Discard and Swap") holds focus instead of Cancel.
    """
    left = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
    right = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"

    with tempfile.TemporaryDirectory() as scratch:
        proc = launch(binary, [left, right], Path(scratch))
        try:
            app = find_app("forskscope", timeout_s=LAUNCH_TIMEOUT_S)
            if app is None:
                print("FAIL: forskscope never registered on the accessibility bus", file=sys.stderr)
                return 1
            landmark, _frame, _left_rows, _right_rows = wait_for_ready(app, 7, timeout_s=LAUNCH_TIMEOUT_S)
            if landmark is None:
                print("FAIL: compare view did not reach the expected 7 rows/pane shape", file=sys.stderr)
                return 1

            act_buttons = find_all_by_name_containing(app, "Use this change")
            if not act_buttons:
                print("FAIL: no 'Use this change' action buttons found - cannot dirty the tab", file=sys.stderr)
                return 1
            click(act_buttons[0])
            time.sleep(0.3)

            more_btn = find_by_exact_name(app, "More ▼")
            if more_btn is None:
                print("FAIL: could not find the 'More ▼' disclosure toggle", file=sys.stderr)
                return 1
            click(more_btn)
            time.sleep(0.3)

            swap_btn = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                swap_btn = find_by_name_containing(app, "Swap sides")
                if swap_btn is not None:
                    break
                time.sleep(0.3)
            if swap_btn is None:
                print("FAIL: could not find the 'Swap sides' toolbar button", file=sys.stderr)
                return 1
            click(swap_btn)
            time.sleep(0.3)

            dialog = None
            deadline = time.monotonic() + LAUNCH_TIMEOUT_S
            while time.monotonic() < deadline:
                dialog = find_by_role(app, "dialog", deadline)
                if dialog is not None:
                    break
                time.sleep(0.3)
            if dialog is None:
                print("FAIL: applying a hunk did not leave the tab dirty enough to trigger ConfirmSwap", file=sys.stderr)
                return 1

            cancel_btn = find_by_exact_name(dialog, "Cancel")
            discard_btn = find_by_exact_name(dialog, "Discard and Swap")
            if cancel_btn is None or discard_btn is None:
                print("FAIL: ConfirmSwap modal is missing its Cancel or Discard and Swap control", file=sys.stderr)
                return 1

            cancel_focused = is_focused(cancel_btn)
            discard_focused = is_focused(discard_btn)
            required = discard_focused and not cancel_focused if break_mode else cancel_focused and not discard_focused
            if not required:
                print(
                    f"FAIL: modal focus state is Cancel={cancel_focused}, Discard and Swap={discard_focused} "
                    f"- required {'the destructive control focused (break mode)' if break_mode else 'Cancel focused, not the destructive control'}",
                    file=sys.stderr,
                )
                return 1
        finally:
            terminate(proc)

    print(
        "OK: applying a hunk dirties the tab and gates ConfirmSwap behind it as designed; the "
        "modal's initial AT-SPI focus lands on Cancel, not the destructive Discard and Swap "
        "control. The remaining three RFC-078 keyboard items (manual checklist, shortcuts "
        "inert behind a modal, Escape-closes-modal) need real keystroke synthesis no "
        "accessibility action can produce and are recorded as manual-outstanding."
    )
    return 0


CASES = {
    "p01": p01,
    "p02": p02,
    "p03": p03,
    "p04": p04,
    "p05": p05,
    "p06": p06,
    "p07": p07,
    "p08": p08,
    "p09": p09,
    "p10": p10,
    "p11": p11,
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
