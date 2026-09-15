#!/usr/bin/env python3
"""F34: rendering-geometry check for the compare view (would have caught F32).

Launches the built binary against a fixed file pair producing Replace,
Delete, and Insert hunks together (tests/fixtures/text/{left,right}_all_hunk_kinds.txt,
pinned by a core corpus test), then walks the AT-SPI accessible tree for the
diff view's rows and asserts every row in each pane has the same number of
accessible children and the same content-cell x-origin as every other row
in that pane.

This is deliberately a *geometry* check, not a DOM-structure check: F32 was
"WebKitGTK wraps a screen-reader label in an anonymous table cell when it's
a sibling of .cell instead of a child of it," which shifted Delete/Replace
rows one column right with content clipped. A row gaining an extra
accessible child, or its content starting at a different x than its
neighbours, is the visible symptom regardless of which future markup change
produces it - checking the outcome is more general than pattern-matching
the one historical cause.

F57: the check waits (`wait_for_ready`) until the accessible tree reaches
the fixture's known, pinned row shape before checking alignment - not just
until the app registers on the accessibility bus, and not just until a
single tree walk happens to find the landmark. A window registers as soon
as it exists; the WebView's DOM, and the row content that alignment
checking needs, only appears after first paint, which a software-rendered
CI runner can take much longer to reach than a developer machine with a
GPU. The first real release run failed here: the app registered in under a
second, one tree walk found no landmark yet, and the check gave up with
twenty-seven of its thirty-second budget unspent.

Usage: render_check.py <path-to-forskscope-binary>
Exit 0 and prints a summary on success. Exit 1 with a description of every
misaligned row on failure.
"""

import subprocess
import sys
import tempfile
import time
from pathlib import Path

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, GLib  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parent.parent
LEFT_FIXTURE = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
RIGHT_FIXTURE = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"

APP_TIMEOUT_S = 30
# F102: a start-up crash (the process exits before ever registering on the
# accessibility bus - e.g. 0.170.0's release cut, "Failed to initialize
# GTK" under xvfb) is an infrastructure flake, not a render defect, and
# this is the only phase retried. A small fixed bound, not a policy knob:
# see main()'s docstring-adjacent comment on launch_until_registered for
# why nothing past this phase is ever retried.
STARTUP_MAX_ATTEMPTS = 3
# F57: the app registering on the accessibility bus (APP_TIMEOUT_S, above)
# happens as soon as its window exists - well before the WebView's DOM
# exists, let alone paints. READY_TIMEOUT_S is the separate budget for
# waiting on *that*: the compare view's accessible tree reaching its full,
# expected shape. A CI runner's software rendering (no GPU, `libEGL
# warning: DRI3 error`) makes first paint far slower than on a developer
# machine - the original release failure hit at 3s into a 30s budget only
# because the check never retried at all, not because 30s was too short.
READY_TIMEOUT_S = 45
POLL_INTERVAL_S = 0.5

# F57: exactly how many `DiffRow`s (one accessible "table row" per side)
# the fixture pair produces, pinned by
# `all_hunk_kinds_fixture_produces_exactly_seven_visual_rows` in
# forskscope-core's diff_corpus.rs. The readiness condition waits for
# *this* rather than "the landmark exists" - collect_rows does a single
# tree traversal, so a tree caught mid-render could yield a partial row
# set that either fails confusingly ("found only N rows") or, worse,
# compares a subset and passes. Waiting for the known shape is what makes
# this check slow-tolerant instead of merely lucky.
EXPECTED_ROWS_PER_PANE = 7


def find_app(name, timeout_s=APP_TIMEOUT_S):
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        desktop = Atspi.get_desktop(0)
        for i in range(desktop.get_child_count()):
            a = desktop.get_child_at_index(i)
            if a.get_name() == name:
                return a
        time.sleep(POLL_INTERVAL_S)
    return None


# F57 held that a tree caught mid-render yields a partial-but-consistent
# row set (fixed by waiting for the pinned shape); it did not anticipate
# a tree caught mid-*mutation*, where a proxy for a node that the DOM has
# already torn down raises GLib.GError ("Object does not exist at path
# ...") the instant a walk touches it - not just here, at any depth, at
# any of get_role_name/get_child_count/get_child_at_index. That crashes
# the whole script instead of retrying, unlike every other partial-tree
# case wait_for_ready already tolerates. Treating a GError node as simply
# absent from this walk lets the outer poll loop retry once the tree
# settles, exactly like any other not-ready-yet state.
def _role_name_or_none(node):
    try:
        return node.get_role_name()
    except GLib.GError:
        return None


def _child_count_or_zero(node):
    try:
        return node.get_child_count()
    except GLib.GError:
        return 0


def _child_at_or_none(node, i):
    try:
        return node.get_child_at_index(i)
    except GLib.GError:
        return None


def find_by_role(node, role, limit=None):
    if _role_name_or_none(node) == role:
        return node
    for i in range(_child_count_or_zero(node)):
        if limit is not None and time.monotonic() > limit:
            return None
        child = _child_at_or_none(node, i)
        if child is not None:
            found = find_by_role(child, role, limit)
            if found is not None:
                return found
    return None


def collect_rows(node, rows):
    if _role_name_or_none(node) == "table row":
        rows.append(node)
    for i in range(_child_count_or_zero(node)):
        child = _child_at_or_none(node, i)
        if child is not None:
            collect_rows(child, rows)


def extents(accessible):
    return Atspi.Component.get_extents(accessible, Atspi.CoordType.SCREEN)


def wait_for_ready(app, expected_rows_per_pane, timeout_s=READY_TIMEOUT_S):
    """Poll until the compare view's accessible tree has fully rendered:
    the 'File comparison' landmark and application frame both exist, and
    each pane has exactly `expected_rows_per_pane` table rows (F57).

    Every step here retries against the deadline rather than trying once -
    the original defect was exactly this: a single traversal immediately
    after the app registered on the bus, with three seconds of a
    thirty-second budget consumed and twenty-seven never spent.

    Returns (landmark, frame, left_rows, right_rows) once ready, or
    (None, None, [], []) if `timeout_s` elapses first.
    """
    deadline = time.monotonic() + timeout_s
    frame = None
    while time.monotonic() < deadline:
        landmark = find_by_role(app, "landmark", deadline)
        if landmark is not None:
            if frame is None:
                frame = find_by_role(app, "frame", deadline)
            if frame is not None:
                frame_extents = extents(frame)
                midline_x = frame_extents.x + frame_extents.width / 2
                rows = []
                collect_rows(landmark, rows)
                left_rows = [r for r in rows if extents(r).x < midline_x]
                right_rows = [r for r in rows if extents(r).x >= midline_x]
                if (
                    len(left_rows) == expected_rows_per_pane
                    and len(right_rows) == expected_rows_per_pane
                ):
                    return landmark, frame, left_rows, right_rows
        time.sleep(POLL_INTERVAL_S)
    return None, None, [], []


def check_pane(rows, pane_name):
    failures = []
    if not rows:
        return [f"{pane_name}: no rows found"]
    baseline_count = rows[0].get_child_count()
    baseline_x = None
    for row in rows:
        n = row.get_child_count()
        if n != baseline_count:
            failures.append(
                f"{pane_name}: a row has {n} accessible children, other "
                f"rows have {baseline_count} - a label is likely rendering "
                f"as a sibling of the content cell instead of inside it "
                f"(F32's defect shape)"
            )
            continue
        cell = row.get_child_at_index(n - 1)
        x = extents(cell).x
        if baseline_x is None:
            baseline_x = x
        elif x != baseline_x:
            failures.append(
                f"{pane_name}: a row's content starts at x={x}, other rows "
                f"start at x={baseline_x} - a column shift, the visual "
                f"symptom of F32"
            )
    return failures


def tail_lines(path, n=20):
    """Last `n` lines of the file at `path`, or "" if it doesn't exist or
    is empty. Review 105 §2: a pipe (`subprocess.PIPE`) is wrong for this -
    nothing drains it on the success path (the app's own stderr writes
    eventually block once the pipe buffer fills, stalling the app and
    turning a live process into a false `wait_for_ready` timeout), and on
    the crash path `.read()` blocks until every process still holding the
    write end closes it - which, for a crashed app, is however long its
    WebKitNetworkProcess/WebKitWebProcess children take to notice their
    IPC peer is gone, not until forskscope itself exits. A plain file has
    no reader to stall and no EOF to wait for.
    """
    try:
        with open(path, "r", errors="replace") as f:
            return "\n".join(f.read().splitlines()[-n:])
    except OSError:
        return ""


def launch_until_registered(binary, scratch, max_attempts=STARTUP_MAX_ATTEMPTS):
    """Launch `binary` and wait for it to register on the accessibility bus,
    retrying only a start-up crash - the process exiting before it ever
    registers (F102: an infrastructure flake, distinct from a slow but
    live process, which is `find_app`'s existing, unretried timeout).

    Nothing past this function is ever retried: not `wait_for_ready`'s own
    timeout (a process that registered but whose WebView never reaches the
    expected render shape is not a start-up flake), and not a geometry
    mismatch (retrying a real render defect until it happens to pass is
    the one outcome this whole check exists to prevent).

    Returns (proc, app, stderr_path) on success - the caller owns `proc`
    from here, and `stderr_path` is where the app's stderr keeps landing
    for the rest of the run, so a later failure (wait_for_ready timeout,
    geometry FAIL) can still show its tail.
    Returns (None, None, None) on failure, having already printed a FAIL
    message and cleaned up any process it started.
    """
    for attempt in range(1, max_attempts + 1):
        stderr_path = Path(scratch) / f"forskscope-stderr-{attempt}.log"
        with open(stderr_path, "w") as stderr_handle:
            proc = subprocess.Popen(
                [binary, str(LEFT_FIXTURE), str(RIGHT_FIXTURE)],
                cwd=scratch,
                stderr=stderr_handle,
            )
        # The file handle above is closed as soon as Popen returns - the
        # child keeps writing through its own duplicated descriptor, and
        # this process holds no reference that could ever need draining.

        app = find_app("forskscope")
        if app is not None:
            return proc, app, stderr_path

        exit_code = proc.poll()
        if exit_code is None:
            # Still running: a genuine slow-registration timeout, not a
            # start-up crash. Not retried - reported and killed as before.
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
            print(
                "FAIL: forskscope never registered on the accessibility bus "
                f"within {APP_TIMEOUT_S}s (the process was still running - "
                "not a start-up crash, so this was not retried)",
                file=sys.stderr,
            )
            tail = tail_lines(stderr_path)
            if tail:
                print(f"  stderr tail:\n{tail}", file=sys.stderr)
            return None, None, None

        # Exited before registering: F102's start-up flake. Retry.
        print(
            f"attempt {attempt}/{max_attempts}: forskscope exited during "
            f"start-up (exit code {exit_code}) before registering on the "
            "accessibility bus",
            file=sys.stderr,
        )
        tail = tail_lines(stderr_path)
        if tail:
            print(f"  stderr tail:\n{tail}", file=sys.stderr)

    print(
        f"FAIL: forskscope exited during start-up on all {max_attempts} "
        "attempts",
        file=sys.stderr,
    )
    return None, None, None


def main():
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} <path-to-forskscope-binary>", file=sys.stderr)
        return 2
    binary = str(Path(sys.argv[1]).resolve())

    with tempfile.TemporaryDirectory() as scratch:
        proc, app, stderr_path = launch_until_registered(binary, scratch)
        if app is None:
            return 1
        try:
            landmark, _frame, left_rows, right_rows = wait_for_ready(
                app, EXPECTED_ROWS_PER_PANE
            )
            if landmark is None:
                print(
                    "FAIL: compare view did not reach the expected render "
                    f"shape ({EXPECTED_ROWS_PER_PANE} rows per pane) within "
                    f"{READY_TIMEOUT_S}s",
                    file=sys.stderr,
                )
                tail = tail_lines(stderr_path)
                if tail:
                    print(f"  stderr tail:\n{tail}", file=sys.stderr)
                return 1

            failures = check_pane(left_rows, "left pane") + check_pane(right_rows, "right pane")

            if failures:
                print("FAIL: F34 rendering check found misalignment:", file=sys.stderr)
                for f in failures:
                    print(f"  - {f}", file=sys.stderr)
                tail = tail_lines(stderr_path)
                if tail:
                    print(f"  stderr tail:\n{tail}", file=sys.stderr)
                return 1

            print(
                f"OK: {len(left_rows)} left rows + {len(right_rows)} right "
                "rows all aligned within their pane."
            )
            return 0
        finally:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()


if __name__ == "__main__":
    sys.exit(main())
