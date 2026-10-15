# Keyboard Reference

Every shortcut below except **Escape**, the **Tabs**, **Explorer** and **Modals** sections
needs a **file-comparison tab to be active**. In the Explorer (the screen ForskScope starts
on) and in a Directory Report tab, **Ctrl+/**, **Ctrl+W** and every Diff-view shortcut do
nothing — but switching tabs (below) works from anywhere, with no active comparison tab
needed.

## Needs an active comparison tab

| Shortcut | Action |
|----------|--------|
| **Ctrl+/**   | Open this keyboard reference |
| **Ctrl+W**   | Close the active comparison tab |

The **?** button at the right end of the tab bar opens the same reference
from any screen.

## Anywhere

| Shortcut | Action |
|----------|--------|
| **Escape**   | Close an open dialog — except a startup recovery dialog, which needs an explicit choice. (The search bar has its own **Esc**; see Tips.) |

## Diff view

### Navigation

| Shortcut | Action |
|----------|--------|
| **F7** | Previous change |
| **F8** | Next change |

### Merge

| Shortcut | Action |
|----------|--------|
| **Enter**   | Apply the focused change (left → right) and advance. With a toolbar button, tab or *Use* button focused, Enter activates that control instead and applies nothing else |
| **Ctrl+Z**  | Undo last merge action |
| **Ctrl+Y**  | Redo last undone merge action |
| **Ctrl+S**  | Save the merge result |

### View

| Shortcut | Action |
|----------|--------|
| **Ctrl+F** | Open / close inline search bar |
| **F3** | Next search match |
| **Shift+F3** | Previous search match |

## Tabs

The tab bar (the Explorer tab plus every open file and directory comparison) is a standard
accessible tab list: only the active tab is a Tab stop, and arrow keys move *and* select
within it.

| Shortcut | Action |
|----------|--------|
| **Ctrl+Tab** / **Ctrl+PageDown**         | Next tab, from anywhere — wraps, includes the Explorer |
| **Ctrl+Shift+Tab** / **Ctrl+PageUp**     | Previous tab, from anywhere — wraps, includes the Explorer |
| **←** / **→** *(tab list focused)*       | Previous / next tab |
| **Home** / **End** *(tab list focused)*  | First (Explorer) / last tab |
| **Delete** *(tab list focused)*          | Close the focused tab — same path as its **×** button and **Ctrl+W**; a tab with unsaved changes asks first. Does nothing on the Explorer tab |

## Explorer

These keys work in both Explorer layouts (Aligned and Compact).

| Shortcut | Action |
|----------|--------|
| **↑ / ↓**     | Move focus between rows |
| **Enter**     | Expand or collapse the focused folder; does nothing on a file |
| **Space**     | Select focused file as left or right comparison candidate |
| **F6**        | Switch focused pane (left ↔ right) |
| **Alt+↑**     | Go up one directory level (focused pane only) |
| **Alt+←**     | Back in the focused pane's directory history (same as ◀) |
| **Alt+→**     | Forward in the focused pane's directory history (same as ▶) |
| **Alt+Home**  | Go to your home directory (focused pane only) |
| **Ctrl+O**    | Choose a folder to open (focused pane only) |

## Modals and dialogs

| Shortcut | Action |
|----------|--------|
| **Enter**     | Activate the focused button (the default action is autofocused) |
| **Escape**    | Cancel / close without action |

---

## Tips

- The toolbar **More ▼** section exposes additional toggles (inline diff, word wrap, swap
  sides, ignore whitespace, ignore case, algorithm). These do not have dedicated shortcuts
  but can be reached by Tab-navigating the toolbar.
- The diff search bar (**Ctrl+F**) supports **Esc** to dismiss. While the bar is open, the
  search runs live on every keystroke; no explicit submit is needed.
- **F3** / **Shift+F3** step through matches while focus is elsewhere in the tab. While
  focus is in the search box itself, the box takes every key typed into it — use
  **Enter** / **Shift+Enter** there instead.
