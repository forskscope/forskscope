# Accessibility

ForskScope's accessibility work ships in stages (RFC-097). This page covers
what has shipped so far.

---

## Text size

Settings → *Text size* enlarges the whole app — text, controls, click
targets and the code panes together, not just the diff panes' own font size
— on top of whatever your system already does. See
[Settings → Text size](settings.md#text-size) for the setting itself, and
the [Keyboard Reference](../intermediate/keyboard.md#text-size) for
**Ctrl+=**/**Ctrl++**/**Ctrl+-**/**Ctrl+0**.

### Does your system's own text size already reach ForskScope?

Before adding *Text size*, this was measured directly rather than assumed —
on **Windows** (*Settings → Accessibility → Text size*, "Make text
bigger") and on **GNOME, Wayland session** (`text-scaling-factor`), both
already apply automatically, with no setting or code of ForskScope's own
involved. ForskScope's own *Text size* setting adds to that; it does not
replace or duplicate it.

This was measured only on those two: Windows and GNOME on Wayland. No other
combination — X11 sessions, KDE, or any other desktop environment — has
been measured, so none is named here one way or the other. If your system
has its own text-scaling setting and it does not seem to reach ForskScope,
ForskScope's own *Text size* setting still works regardless.

---

## Colour vision

See [Settings → Colour vision](settings.md#colour-vision): a red–green-safe
diff palette, and every change is marked with a glyph as well as colour, in
both palettes.
