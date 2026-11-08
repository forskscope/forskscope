# Accessibility

## Text size

Settings → *Text size* enlarges the whole app: text, controls, click
targets and the code panes together. It adds to whatever your system
already does. See [Settings → Text size](settings.md#text-size) for the
setting, and the [Keyboard Reference](../intermediate/keyboard.md#text-size)
for **Ctrl+=**/**Ctrl++**/**Ctrl+-**/**Ctrl+0**.

### Your system's own text size

On **Windows** (*Settings → Accessibility → Text size*) and on **GNOME in a
Wayland session** (*Large Text*, or `text-scaling-factor`), your system's
text size already applies to ForskScope by itself. ForskScope's *Text
size* adds to it.

Other systems and desktops haven't been checked. If your system's text
size doesn't seem to reach ForskScope, use ForskScope's *Text size*: it
works the same everywhere.

---

## Contrast and text

In every theme and colour-vision option:
- text meets the WCAG AA contrast minimum of 4.5:1;
- the edges of fields and buttons meet 3:1;
- no text is smaller than 12 px.

A check in the project's build verifies this on every change. Disabled
buttons are the exception, as WCAG allows: they are dimmed on purpose.

---

## Colour vision

See [Settings → Colour vision](settings.md#colour-vision): a red–green-safe
diff palette, and every change is marked with a glyph as well as colour, in
both palettes.
