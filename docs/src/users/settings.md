# Settings Reference

Open Settings via the **Settings** button in the header.

---

## Appearance

### Theme

Selects the colour scheme applied to the entire application.

| Value | Description |
|-------|-------------|
| **Dark** (default) | Dark background, light text. |
| **Light** | Light background, dark text. |
| **Night** | High-contrast dark theme with deeper blacks. |

Changes take effect immediately.

### Diff Font Size

Size (in pixels) of the text in the diff view. Range: 8–32. Default: 14.

---

## Diff View

### Context lines

Number of equal lines shown around each change before the rest collapses.

| Value | Behaviour |
|-------|-----------|
| **0** | Collapse all equal context — show changed lines only. |
| **3** (default) | Three lines of context above and below each change. |
| **5** | Five lines. |
| **10** | Ten lines. |

Click the `···` divider in any collapsed region to expand it.

---

## Compare Profiles

A profile stores a combination of diff options. Selecting a profile makes it
the default for new comparisons. Open tabs are unaffected until you change their
options directly in the toolbar.

**Built-in profiles** (read-only):

| Profile | Algorithm | Ignore WS | Ignore case |
|---------|-----------|-----------|-------------|
| Exact (default) | Myers | – | – |
| Ignore whitespace | Myers | ✓ | – |
| Ignore case | Myers | – | ✓ |
| Histogram | Histogram | – | – |

**Custom profiles:** click `×` to delete any profile you created. Built-in
profiles cannot be deleted (no `×` button).

**Add a profile:** fill in the name, check the options you want, pick an
algorithm, and click **Add**.

---

## Locale

### Language

| Value | Description |
|-------|-------------|
| **English** (default) | English interface. |
| **日本語** | Japanese interface. |

Changes take effect immediately across all open workspaces.

---

## Keyboard reference and About

These are accessible from the header buttons (**?** and **ℹ**) rather than from
Settings. See [Keyboard Reference](../intermediate/keyboard.md) and the
[About panel](../maintainers/release.md) for diagnostics.
