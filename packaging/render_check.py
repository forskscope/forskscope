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
from gi.repository import Atspi  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parent.parent
LEFT_FIXTURE = REPO_ROOT / "tests/fixtures/text/left_all_hunk_kinds.txt"
RIGHT_FIXTURE = REPO_ROOT / "tests/fixtures/text/right_all_hunk_kinds.txt"

APP_TIMEOUT_S = 30
POLL_INTERVAL_S = 0.5


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


def find_by_role(node, role, limit=None):
    if node.get_role_name() == role:
        return node
    for i in range(node.get_child_count()):
        if limit is not None and time.monotonic() > limit:
            return None
        child = node.get_child_at_index(i)
        if child is not None:
            found = find_by_role(child, role, limit)
            if found is not None:
                return found
    return None


def collect_rows(node, rows):
    if node.get_role_name() == "table row":
        rows.append(node)
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is not None:
            collect_rows(child, rows)


def extents(accessible):
    return Atspi.Component.get_extents(accessible, Atspi.CoordType.SCREEN)


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


def main():
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} <path-to-forskscope-binary>", file=sys.stderr)
        return 2
    binary = str(Path(sys.argv[1]).resolve())

    with tempfile.TemporaryDirectory() as scratch:
        proc = subprocess.Popen([binary, str(LEFT_FIXTURE), str(RIGHT_FIXTURE)], cwd=scratch)
        try:
            app = find_app("forskscope")
            if app is None:
                print(
                    "FAIL: forskscope never registered on the accessibility bus "
                    f"within {APP_TIMEOUT_S}s",
                    file=sys.stderr,
                )
                return 1

            landmark = find_by_role(app, "landmark", time.monotonic() + APP_TIMEOUT_S)
            if landmark is None:
                print("FAIL: could not find the 'File comparison' landmark", file=sys.stderr)
                return 1

            frame = find_by_role(app, "frame")
            if frame is None:
                print("FAIL: could not find the application frame", file=sys.stderr)
                return 1
            frame_extents = extents(frame)
            midline_x = frame_extents.x + frame_extents.width / 2

            rows = []
            collect_rows(landmark, rows)
            if len(rows) < 2:
                print(f"FAIL: found only {len(rows)} table rows, expected several", file=sys.stderr)
                return 1

            left_rows = [r for r in rows if extents(r).x < midline_x]
            right_rows = [r for r in rows if extents(r).x >= midline_x]

            failures = check_pane(left_rows, "left pane") + check_pane(right_rows, "right pane")

            if failures:
                print("FAIL: F34 rendering check found misalignment:", file=sys.stderr)
                for f in failures:
                    print(f"  - {f}", file=sys.stderr)
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
