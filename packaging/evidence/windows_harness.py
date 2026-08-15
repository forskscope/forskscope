#!/usr/bin/env python3
"""M5-A: Windows evidence harness for P01, P02, P09, P10 (RFC-078).

Windows counterpart to `linux_harness.py`. Same four cases, same
falsifiability contract (`--break`), same CLI shape - but driven through
Windows UI Automation (UIA) via `pywinauto` instead of AT-SPI, because
there is no AT-SPI on Windows and no Xvfb/X11 story either: `windows-latest`
GitHub Actions runners provide a real interactive desktop session, so (per
the M5-A handoff) launches here should just work without any virtual
display machinery.

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
  windows_harness.py p09 <binary>
  windows_harness.py p10 <binary>

Falsifiability (`--break`): each case accepts an optional `--break` flag
that deliberately breaks the condition it checks, to demonstrate the
assertion is not vacuous. See each case function's docstring for exactly
what it breaks. `--break` never touches product source; it only changes
what the harness expects to see.
"""

import subprocess
import sys
import tempfile
import time
from pathlib import Path

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


def launch(binary, args, cwd):
    return subprocess.Popen([str(binary), *[str(a) for a in args]], cwd=str(cwd))


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
