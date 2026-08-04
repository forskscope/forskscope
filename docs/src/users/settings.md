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

### Diff font family

Font family used in the diff panes. Five presets:

| Preset | Font stack |
|---|---|
| **Monospace (default)** | System default fixed-pitch (`ui-monospace, monospace`) |
| **Sans-serif** | System default proportional (`system-ui, sans-serif`) |
| **Serif** | System default serif (`Georgia, serif`) |
| **Courier New** | Classic fixed-pitch (`Courier New, Courier, monospace`) |
| **Consolas / Menlo** | Developer fixed-pitch (`Consolas, Menlo, monospace`) |

Changes take effect immediately. The UI chrome is unaffected.

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

### Enable binary comparison

When **off** (default), binary files (detected by NUL-byte sniff) cannot be
opened for comparison. They appear in the Explorer with a `bin` badge and are
non-actionable.

When **on**, binary files can be compared using a hex-dump preview. Because
binary diffs are often voluminous and rarely meaningful, this is off by default.
The comparison runs asynchronously so the app does not freeze on large binaries.

### Explorer layout

Controls how entries are displayed in the two-pane Explorer.

| Value | Behaviour |
|---|---|
| **Aligned (default)** | Same-name entries share a row across panes; spacer rows fill gaps where one side is missing. Vertical scrolling implicitly keeps panes in sync. |
| **Compact (independent panes)** | No spacer rows. Each pane packs its own entries and scrolls independently. Cross-pane row alignment is intentionally absent. Best for directories where many files exist only on one side. |

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

---

## Where your settings and session are stored

ForskScope keeps two small JSON files in your platform config directory
(typically `~/.config/forskscope/` on Linux, `%APPDATA%\forskscope\` on
Windows): `settings.json` for everything on this page, and `session.json` for
your open tabs (see [My session was not restored](faq.md#my-session-was-not-restored-after-restarting)).
Both files stay on this computer — nothing here is uploaded.

If one of these files can't be used as-is — it was written by a newer version
of ForskScope, or it's unreadable/corrupted — a dialog appears at startup
instead of silently falling back to defaults. What it offers depends on why:

| Situation | What you see |
|---|---|
| File is from a newer ForskScope version | **Exit**, or **Continue with defaults** for this session. The file itself is left untouched — a future version can still read it. |
| File is unreadable/corrupted | **Continue with defaults**, or **Reset and back up**, which backs up the unreadable file (as `<name>.reset.bak`, next to the original) before writing a fresh one. |

Either way, until you choose an action, nothing you change in that session is
saved — the dialog says so explicitly. Choosing **Continue…** lets you keep
using ForskScope for the session without touching the file on disk; closing
and reopening will show the same dialog again until the underlying file is
fixed, replaced, or reset.

**Downgrading ForskScope:** if you install an older version after having run a
newer one, the older version will not understand the current file format and
will show the "newer version" dialog above. Your settings/session are not
lost — install the newer version again to read them, or use **Reset and back
up**/reinstall to start fresh. A settings or session file that was
automatically upgraded from an older format keeps a one-time backup of the
original alongside it, named `<name>.pre-v2.bak`.
