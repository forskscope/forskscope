#!/usr/bin/env python3
"""M5-C prerequisite A: inject a geometry-only misalignment, for
render_check.py's falsifiability demonstration only.

F34's rendering check (`render_check.py`'s `check_pane`) has two branches:
an accessible-child-*count* comparison (would catch F32's exact shape — an
extra accessible child on rows carrying a label) and a content-cell
*x-origin* comparison (would catch a pure geometry shift with no DOM/AT-SPI
structure change at all). `reintroduce_f32_defect.py` demonstrates the
first branch. Nothing demonstrated the second until this: review 055 tried
CSS `padding`/`margin` mutations on `.cell` twice and never produced a
shift `Atspi.Component.get_extents` would observe, because a `<div
display:table-cell>`'s own accessible box position comes from the table
layout algorithm's column placement, not its internal padding - padding
moves *content inside* the cell's box, not the box itself.

Confirmed empirically (measuring `extents(cell).x` directly against a
locally-built binary before writing this script) that `transform:
translateX()` *does* move the accessible's own reported box, because it
affects the painted position directly rather than layout-computed padding.
Targets `.hunk-rep .cell` specifically (Replace-kind rows only) so exactly
one row per pane shifts while the rest of the pane's rows stay at their
original x - matching `check_pane`'s comparison, which is relative
(row-to-baseline-row), not absolute, and would not detect a *uniform*
shift applied to every row alike.

Edits `crates/forskscope-ui/assets/css/11-view-diff.css` and regenerates
`assets/main.css` (`cargo xtask css`) in this run's checkout only - never
committed. See `.github/workflows/render-check.yml`'s
`inject_geometry_defect` input, the only caller.

Usage: inject_geometry_defect.py
Exits 1 if the expected marker line isn't found exactly once - fails
loudly rather than silently no-op'ing if the CSS has changed shape since
this was written.
"""

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CSS_FILE = REPO_ROOT / "crates/forskscope-ui/assets/css/11-view-diff.css"
XTASK_DIR = REPO_ROOT / "xtask"

MARKER = ".hunk-rep .diff-row { background: var(--rep-bg); }\n"
INJECTED = MARKER + ".hunk-rep .cell { transform: translateX(20px); }\n"


def main():
    src = CSS_FILE.read_text()
    count = src.count(MARKER)
    if count != 1:
        print(
            f"FAIL: expected the Replace-hunk background rule exactly once "
            f"in {CSS_FILE}, found {count} - the CSS has changed shape "
            "since this script was written; update it rather than let it "
            "silently no-op",
            file=sys.stderr,
        )
        return 1
    CSS_FILE.write_text(src.replace(MARKER, INJECTED))

    result = subprocess.run(
        ["cargo", "run", "--quiet", "--", "css"], cwd=XTASK_DIR
    )
    if result.returncode != 0:
        print("FAIL: `cargo xtask css` failed to regenerate main.css", file=sys.stderr)
        return 1

    print(
        f"Injected a geometry-only defect into {CSS_FILE} "
        "(.hunk-rep .cell { transform: translateX(20px); }) and regenerated main.css."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
