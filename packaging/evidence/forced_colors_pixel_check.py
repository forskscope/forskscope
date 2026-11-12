#!/usr/bin/env python3
"""F222 (review 211 §1) - scratch-branch-only Windows verification.

Proves the fix *by machine, not by eye*: finds every `.diff-row`'s own
accessible rectangle, crops the screenshot to a narrow strip just inside
the gutter cell's own left edge (not the focused hunk's own outline,
which sits right at that edge too - review 212 §1's own finding), and
reads the dominant colour there - once for every known *changed* row
(the fixture's own delete/insert/replace lines) and once for every known
*unchanged* one. The fix is confirmed only if **every** changed row
shares the *same* one colour and that colour never appears on an
unchanged row - not merely "differs somewhere", which a focus outline
alone could also satisfy with no bar at all. Not a match against a value
read via `GetSysColor` from this (message-loop-less) launcher process,
which measured live (run 43146676172) as the *system's* default accent
colour even under a real contrast theme (the theme's own repaint
notification has nowhere to land without a window message pump) -
printed here only as a diagnostic, never the basis of pass/fail.

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
            # Review 212 §1: the first version sampled 6px from the row's
            # own left edge (x0 to x0+6) - wide enough to also catch the
            # focused hunk's own outline (`.hunk.focused`'s `outline-
            # offset: -2px` draws *inside* the hunk's border box, right
            # at that same edge), which is why the "TWO" row - inside the
            # first, default-focused hunk - sampled white (the outline's
            # own colour) instead of the bar's. Narrowed to 2px, inset 1px
            # from the true edge (x0+1 to x0+3): still solidly inside the
            # 4px `border-left` the fix itself paints, but past where a
            # -2px-offset outline can reach. Trims 3px off the row's own
            # top/bottom too (anti-aliased gridline pixels at a row
            # boundary would otherwise pollute the count).
            counts: dict[tuple[int, int, int], int] = {}
            for y in range(max(y0 + 3, 0), min(y1 - 3, img.height)):
                for x in range(max(x0 + 1, 0), min(x0 + 3, img.width)):
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
                f"GUTTER: {kind} row text={text!r} rect=({x0 + 1},{y0})-({x0 + 3},{y1}) "
                f"dominant_colour={colour}"
            )
            (changed_colours if changed else unchanged_colours).add(colour)

        # Review 212 §1's own stronger assertion: not just "disjoint from
        # unchanged" (a focus outline alone would already satisfy that
        # with no bar at all) but every changed row sampling the *same*
        # one colour, and that colour being neither the canvas colour nor
        # (by the same disjointness) a text colour either.
        one_colour = len(changed_colours) == 1
        bar_colour = next(iter(changed_colours)) if one_colour else None
        overlap = changed_colours & unchanged_colours
        distinguishable = bool(changed_colours) and bool(unchanged_colours) and not overlap
        print(f"RESULT: changed_row_gutter_colours={sorted(changed_colours)}")
        print(f"RESULT: unchanged_row_gutter_colours={sorted(unchanged_colours)}")
        print(f"RESULT: all_changed_rows_share_one_colour={one_colour}")
        print(f"RESULT: shared_bar_colour={bar_colour}")
        print(f"RESULT: changed_rows_are_distinguishable_by_gutter_colour={distinguishable}")
        return 0
    finally:
        terminate(proc)


if __name__ == "__main__":
    raise SystemExit(main())
