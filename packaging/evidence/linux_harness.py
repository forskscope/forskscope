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
    subprocess.run(["xdotool", "type", "--clearmodifiers", text], check=True, timeout=15)


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


CASES = {"p01": p01, "p02": p02, "p04": p04, "p05": p05, "p09": p09, "p10": p10}


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
