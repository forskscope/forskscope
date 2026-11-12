#!/usr/bin/env python3
"""F222 (review 211 §1) - scratch-branch-only Windows verification.

Proves the fix *by machine, not by eye*: finds every `.diff-row`'s own
accessible rectangle (`windows_harness.collect_diff_rows`, already used
elsewhere in this harness for exactly this kind of pixel-precise
geometry check), crops the screenshot to each row's gutter column's
leftmost 4px, and counts pixels matching the system's actual `Highlight`
colour (read live via `GetSysColor`, never assumed) - once for a known
*changed* row (the fixture's own delete/insert/replace lines) and once
for a known *unchanged* one.

Usage: forced_colors_pixel_check.py <binary> <data-dir> <left> <right> <capture-png>
Prints one `GUTTER:` line per row, in document order, with its own text,
its changed/unchanged classification (from that text - this fixture's
exact lines are known, not guessed), and the matching-pixel count in its
4px gutter strip.
"""

from __future__ import annotations

import ctypes
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from windows_harness import (  # noqa: E402
    collect_diff_rows,
    connect,
    launch,
    terminate,
)

COLOR_HIGHLIGHT = 13
# A changed row always carries one of these three fixture lines on
# whichever side it has content - not guessed, this script wrote the
# fixture itself (see the driver's own Set-Content calls).
CHANGED_LINE_FRAGMENTS = ("two", "TWO", "four", "six")


def get_sys_color_highlight() -> tuple[int, int, int]:
    rgb = ctypes.windll.user32.GetSysColor(COLOR_HIGHLIGHT)
    r = rgb & 0xFF
    g = (rgb >> 8) & 0xFF
    b = (rgb >> 16) & 0xFF
    return (r, g, b)


def row_text(row) -> str:
    try:
        return (row.window_text() or "").strip()
    except Exception:  # noqa: BLE001
        return ""


def main() -> int:
    if len(sys.argv) != 6:
        print(
            f"usage: {sys.argv[0]} <binary> <data-dir> <left> <right> <capture-png>",
            file=sys.stderr,
        )
        return 2
    binary = Path(sys.argv[1])
    data_dir = Path(sys.argv[2])
    left = Path(sys.argv[3])
    right = Path(sys.argv[4])
    capture_path = Path(sys.argv[5])
    data_dir.mkdir(parents=True, exist_ok=True)

    highlight = get_sys_color_highlight()
    print(f"HIGHLIGHT: {highlight}")

    proc = launch(
        binary,
        ["--data-dir", str(data_dir), str(left), str(right)],
        cwd=binary.parent,
    )
    try:
        _app, win = connect(proc.pid)
        time.sleep(2)  # let the diff fully render before reading rows

        rows = collect_diff_rows(win)
        if not rows:
            print("FAIL: no .diff-row elements found", file=sys.stderr)
            return 1

        win_rect = win.rectangle()
        capture_path.parent.mkdir(parents=True, exist_ok=True)
        image = win.capture_as_image()
        image.save(str(capture_path))
        print(f"CAPTURE: {capture_path}")

        # PIL import deferred: only needed once a capture exists to
        # analyse, and keeps this script's own import list minimal for
        # any caller that only wants the capture itself.
        from PIL import Image  # noqa: PLC0415

        img = Image.open(capture_path).convert("RGB")
        pixels = img.load()

        def matches_highlight(rgb, tolerance=24):
            return all(abs(a - b) <= tolerance for a, b in zip(rgb, highlight))

        ordered = sorted(rows, key=lambda r: (r.rectangle().top, r.rectangle().left))

        # Group left/right rows at the same y into one logical "line" -
        # an insert/delete's empty side carries no text at all (nothing
        # to show a line number for, the exact case review 211 §1 asked
        # to check), so classifying *per element* by its own text would
        # wrongly call that empty side "unchanged". Classifying by the
        # y-group instead means the empty side inherits its paired
        # side's real status.
        groups: dict[int, list] = {}
        for row in ordered:
            top = row.rectangle().top
            key = next((k for k in groups if abs(k - top) <= 2), top)
            groups.setdefault(key, []).append(row)
        group_changed: dict[int, bool] = {
            key: any(
                any(frag in row_text(r) for frag in CHANGED_LINE_FRAGMENTS) for r in members
            )
            for key, members in groups.items()
        }

        any_changed_has_bar = False
        any_unchanged_has_bar = False
        for row in ordered:
            rect = row.rectangle()
            x0 = rect.left - win_rect.left
            y0 = rect.top - win_rect.top
            y1 = rect.bottom - win_rect.top
            text = row_text(row)
            group_key = next((k for k in groups if abs(k - rect.top) <= 2), rect.top)
            changed = group_changed[group_key]
            count = 0
            total = 0
            for y in range(max(y0, 0), min(y1, img.height)):
                for x in range(max(x0, 0), min(x0 + 4, img.width)):
                    total += 1
                    if matches_highlight(pixels[x, y]):
                        count += 1
            kind = "changed" if changed else "unchanged"
            print(
                f"GUTTER: {kind} row text={text!r} rect=({x0},{y0})-({x0 + 4},{y1}) "
                f"highlight_px={count}/{total}"
            )
            if changed and count > 0:
                any_changed_has_bar = True
            if not changed and count > 0:
                any_unchanged_has_bar = True

        print(f"RESULT: any_changed_row_has_highlight_bar={any_changed_has_bar}")
        print(f"RESULT: any_unchanged_row_has_highlight_bar={any_unchanged_has_bar}")
        return 0
    finally:
        terminate(proc)


if __name__ == "__main__":
    raise SystemExit(main())
