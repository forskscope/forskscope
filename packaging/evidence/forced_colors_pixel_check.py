#!/usr/bin/env python3
"""F222 (review 211 §1) - scratch-branch-only Windows verification.

Proves the fix *by machine, not by eye*: finds every `.diff-row`'s own
accessible rectangle, crops the screenshot to each row's gutter column's
leftmost pixels, and reads the dominant colour there - once for every
known *changed* row (the fixture's own delete/insert/replace lines) and
once for every known *unchanged* one. The fix is confirmed if the set of
changed-row gutter colours is disjoint from the set of unchanged-row
gutter colours - a relative comparison, not a match against a value read
via `GetSysColor` from this (message-loop-less) launcher process, which
measured live (run 43146676172) as the *system's* default accent colour
even under a real contrast theme (the theme's own repaint notification
has nowhere to land without a window message pump) - printed here only
as a diagnostic, never the basis of pass/fail.

Usage: forced_colors_pixel_check.py <binary> <data-dir> <left> <right> <capture-png>
Prints one `GUTTER:` line per row, in document order, with its own text,
its changed/unchanged classification (by y-position, matched against
where this fixture's own known changed lines were found - not guessed),
and its gutter strip's dominant colour.
"""

from __future__ import annotations

import ctypes
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from windows_harness import (  # noqa: E402
    connect,
    find_by_text_containing,
    launch,
    terminate,
    wait_for_row_shape,
)

# Six rows per pane, not five: the delete ("four", left-only) and the
# insert ("six", right-only) each get their own row, with the *other*
# side rendered empty rather than merging into the replace row ("two"/
# "TWO") - the row count `wait_for_row_shape` polls for, the same
# F57-safe pattern every other harness case here uses rather than a flat
# `sleep()`: a single tree walk right after `connect()` can catch
# WebView2 mid-render and see a partial row set.
EXPECTED_ROWS_PER_PANE = 6

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


def changed_row_tops(win) -> list[int]:
    """The `.rectangle().top` of each known-changed fixture line, found
    directly by its own text (`find_by_text_containing`, the same proven
    anchor this harness's `find_leaf_rect`/p03 already use) - `.diff-row`
    elements themselves measured empty (`row.window_text()`) on every
    row, live (run 43145780040), so classification goes by the content
    cell's own text instead, not the row element's."""
    tops = []
    for fragment in CHANGED_LINE_FRAGMENTS:
        elem = find_by_text_containing(win, fragment)
        if elem is not None:
            try:
                tops.append(elem.rectangle().top)
            except Exception:  # noqa: BLE001
                pass
    return tops


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
        _frame, left_rows, right_rows = wait_for_row_shape(win, EXPECTED_ROWS_PER_PANE)
        rows = left_rows + right_rows
        if not rows:
            print(
                f"FAIL: no .diff-row elements found (wanted {EXPECTED_ROWS_PER_PANE} per pane)",
                file=sys.stderr,
            )
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

        ordered = sorted(rows, key=lambda r: (r.rectangle().top, r.rectangle().left))

        # An insert/delete's empty side carries no text at all (nothing to
        # show a line number for, the exact case review 211 §1 asked to
        # check) - found by its own fixture-line text instead
        # (`changed_row_tops`), then matched to whichever row(s) share
        # that y, so the empty side inherits its paired side's real
        # status rather than being read as "unchanged" for having no
        # text of its own.
        changed_tops = changed_row_tops(win)

        def is_changed_top(top: int) -> bool:
            return any(abs(top - ct) <= 2 for ct in changed_tops)

        def dominant_colour(x0, y0, y1):
            # Trims 2px off the row's own top/bottom (anti-aliased
            # gridline pixels at a row boundary would otherwise pollute
            # the count) and widens the strip to 6px (a few pixels of
            # tolerance on exactly where the gutter's own left edge
            # lands, rather than trusting .diff-row's rectangle to the
            # pixel).
            counts: dict[tuple[int, int, int], int] = {}
            for y in range(max(y0 + 2, 0), min(y1 - 2, img.height)):
                for x in range(max(x0, 0), min(x0 + 6, img.width)):
                    c = pixels[x, y]
                    counts[c] = counts.get(c, 0) + 1
            if not counts:
                return (0, 0, 0)
            return max(counts, key=lambda c: counts[c])

        changed_colours = set()
        unchanged_colours = set()
        for row in ordered:
            rect = row.rectangle()
            x0 = rect.left - win_rect.left
            y0 = rect.top - win_rect.top
            y1 = rect.bottom - win_rect.top
            text = row_text(row)
            changed = is_changed_top(rect.top)
            colour = dominant_colour(x0, y0, y1)
            kind = "changed" if changed else "unchanged"
            print(
                f"GUTTER: {kind} row text={text!r} rect=({x0},{y0})-({x0 + 6},{y1}) "
                f"dominant_colour={colour}"
            )
            (changed_colours if changed else unchanged_colours).add(colour)

        overlap = changed_colours & unchanged_colours
        distinguishable = bool(changed_colours) and bool(unchanged_colours) and not overlap
        print(f"RESULT: changed_row_gutter_colours={sorted(changed_colours)}")
        print(f"RESULT: unchanged_row_gutter_colours={sorted(unchanged_colours)}")
        print(f"RESULT: changed_rows_are_distinguishable_by_gutter_colour={distinguishable}")
        return 0
    finally:
        terminate(proc)


if __name__ == "__main__":
    raise SystemExit(main())
