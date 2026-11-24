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

For more, choose [Settings → Contrast → High](settings.md#contrast), or
*System* to follow your system's contrast setting.

### Windows contrast themes

Under a Windows contrast theme, Windows replaces the app's colours with the
theme's own. Changed lines then keep a bar in the theme's highlight colour
beside their line numbers, as well as their `−`/`+` marks.

---

## Light and dark

[Settings → Theme → System](settings.md#theme) follows your system's
light/dark setting, on Windows and on Linux desktops with the standard
settings service.

---

## Less motion

When your system asks for reduced motion, the loading spinners stop turning.
Their progress is still shown in words. There is no setting for this in
ForskScope: it follows the system on Windows, macOS and Linux.

---

## Colour vision

See [Settings → Colour vision](settings.md#colour-vision): a red–green-safe
diff palette, and every change is marked with a glyph as well as colour, in
both palettes.

## Screen readers

Every control has a name a screen reader can read, and the whole app works
from the keyboard (see the [Keyboard reference](../intermediate/keyboard.md)).

**On Linux, status messages aren't spoken.** Messages that appear on their
own (a notice such as *Could not save settings*, the number of search
matches, the Explorer's status, a change to an ignore list) are marked for
screen readers in the standard way, but the web engine ForskScope uses on
Linux (WebKitGTK) doesn't pass them on, so Orca stays silent. What has
focus *is* spoken. Measured with WebKitGTK 2.54.3 and Orca 51.2; see
[Known limitations](known-limitations.md#linux-status-messages-arent-spoken-by-screen-readers).
Making the messages that need your attention take focus, so they're
spoken, is planned for the next release.
