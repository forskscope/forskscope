# Platform Evidence — `linux-wayland`

**Artifact filename:** `forskscope-v0.167.0-linux-x86_64.tar.gz`
**SHA-256:** `e17baa26abbb91e5e8e046d3812b08203f0d1ddfd6f8dc9fb9182326ed04bf09`
**Source commit:** `cb6f5b6`

**Not executed in this slice.** Per `matrix-plan.md` §1/§4 Q2, this row is
**Manual**, executed by the owner — CI's Xvfb is X11-family, not a real
Wayland compositor, so no CI substitute is claimed here the way
`linux-x11` stands in for basic functional coverage. M5-A built the
CI-automatable rows (`linux-x11`, and the Windows/macOS rows); this row
remains outstanding until the owner runs it on a real Wayland session.

## Cases

| Case | Result |
|---|---|
| P01 — Install and cold launch | Not yet executed (owner) |
| P02 — CLI file compare | Not yet executed (owner) |
| P09 — Mergetool | Not yet executed (owner) |
| P10 — Binary/XLSX fail-closed policy | Not yet executed (owner) |

`linux-x11`'s CI evidence in this same directory is the closest available
substitute for now — WebKitGTK rendering and core save/mergetool/XLSX
logic are shared between the X11 and Wayland windowing backends; only
the Wayland-specific windowing/event-loop integration (P06, out of this
slice's scope) and the manual host's real desktop environment are
untested here.
