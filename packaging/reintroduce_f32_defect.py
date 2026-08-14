#!/usr/bin/env python3
"""F57: reintroduce F32's exact defect shape, for render_check.py's
falsifiability demonstration only.

F32 was: `hunk.rs` emitted the `sr_label` accessibility span as a *sibling*
of `.cell` (inside the row, outside the content cell) instead of as a
*child* of `.cell`. WebKitGTK wraps a bare sibling span in an anonymous
table cell, adding one accessible child to exactly the rows that carry a
label (Delete/Replace) and shifting their content one column right — the
defect F34's rendering check exists to catch.

This script moves `sr_label`'s span back out of `.cell` in both RowLeft
and RowRight, reproducing that exact shape in a checkout used only to
prove render_check.py still goes red when the defect it exists for
reappears. It is never run as part of a normal build, and its edit is
never committed — see `.github/workflows/render-check.yml`'s
`inject_f32_defect` input, the only caller.

Usage: reintroduce_f32_defect.py
Exits 1 if the expected fixed-state pattern isn't found exactly twice
(RowLeft, RowRight) — fails loudly rather than silently no-op'ing if
hunk.rs has changed shape since this was written.
"""

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
HUNK_RS = REPO_ROOT / "crates/forskscope-ui/src/ui/view/hunk.rs"

FIXED = '''            div { class: "cell",
                if let Some(ref lbl) = sr_label { span { class: "sr-only", "{lbl}: " } }
'''
DEFECT = '''            if let Some(ref lbl) = sr_label { span { class: "sr-only", "{lbl}: " } }
            div { class: "cell",
'''


def main():
    src = HUNK_RS.read_text()
    count = src.count(FIXED)
    if count != 2:
        print(
            f"FAIL: expected the fixed sr_label placement exactly twice "
            f"(RowLeft, RowRight) in {HUNK_RS}, found {count} - hunk.rs "
            "has changed shape since this script was written; update it "
            "rather than let it silently no-op",
            file=sys.stderr,
        )
        return 1
    HUNK_RS.write_text(src.replace(FIXED, DEFECT))
    print(f"Reintroduced F32's defect shape in {HUNK_RS} ({count} occurrences moved).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
