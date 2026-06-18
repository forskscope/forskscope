# Settings Reference

Open Settings via the **Settings** button (⚙) in the header bar, or press
**Ctrl+/** to open the keyboard reference, which has a link to Settings.
Close Settings with **Esc**, the **Close** button, or by clicking outside
the dialog.

---

## Appearance

### Theme

| Value | Description |
|-------|-------------|
| **Dark** (default) | Dark background, light text. Suitable for low-light environments. |
| **Light** | Light background, dark text. Matches light system themes. |
| **Night** | Deeper blacks, higher contrast. |

Changes take effect immediately.

### Diff font size

Point size for text in the diff panes. Range: 8–32 pt. Default: 14.

The UI chrome (toolbar, tabs, status bar) scales proportionally.

---

## Language

| Value | Description |
|-------|-------------|
| **English** (default) | English interface. |
| **日本語** | Japanese interface. All labels, buttons, dialogs, and notices are translated. |

Changes take effect immediately across all open workspaces.

---

## Advanced settings

Click **▸ Advanced** at the bottom of the Settings dialog to reveal advanced
options. Click **▾ Hide advanced** to collapse them again.

The following settings are inside the Advanced section.

### Context lines

Number of unchanged lines shown above and below each change before the rest
collapses.

| Value | Behaviour |
|-------|-----------|
| **0 (show all)** | Never collapse — show the entire file. |
| **3 (default)** | Three lines of context on each side of a change. |
| **5** | Five lines. |
| **10** | Ten lines. |

Click any `···` divider in the diff to expand a collapsed region.

---

### Ignore patterns

These filters apply to the Explorer tree — files and directories that match are
hidden from the comparison panes.

#### Ignore file extensions

A comma-separated list of extensions to hide (no leading dot required). Example:

```
o, class, tmp, pyc
```

Matching is case-insensitive.

#### Ignore directory names

A comma-separated list of directory names or glob patterns (using `*` as a
wildcard) to hide. Example:

```
target, node_modules, *.cache, __pycache__
```

---

### Compare profiles

A profile stores a combination of diff options and an algorithm choice.
Selecting a profile in Settings makes it the default for new comparisons.
Open tabs are unaffected; change their options directly in the toolbar's
**More ▼** section.

**Built-in profiles (read-only):**

| Profile | Algorithm | Ignore WS | Ignore case |
|---------|-----------|-----------|-------------|
| **Exact (default)** | Myers | — | — |
| **Ignore whitespace** | Myers | ✓ | — |
| **Ignore case** | Myers | — | ✓ |
| **Histogram** | Histogram | — | — |

Built-in profiles cannot be deleted (no **×** button).

**Adding a custom profile:**

1. Click **+ New profile** at the bottom of the profile list.
2. Enter a name.
3. Check **Ignore WS** and/or **Ignore case** if desired.
4. Pick an algorithm.
5. Click **Add**.

The profile is saved immediately and appears in the list.

**Deleting a custom profile:** click the **×** button next to its name.

---

## About and Diagnostics

The **ℹ** button in the Settings header opens the About dialog. It shows:

- ForskScope version
- Build profile (debug / release)
- Platform (OS and architecture)
- UI framework and diff engine versions

Click **Copy diagnostics** to copy this information to the clipboard for
use in bug reports.
