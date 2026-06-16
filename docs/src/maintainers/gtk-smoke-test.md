# GTK Smoke Test Checklist

This checklist covers the three RFC-041 items that require a running GTK
display server. Run these checks after building the UI crate in a GTK
environment.

## Prerequisites

```sh
# Linux: WebKitGTK 4.1 required
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev pkg-config

# Build
cargo build --release -p forskscope-ui

# Smoke diagnostics (no GTK needed)
./target/release/forskscope --diagnostics
```

All 936 non-GTK tests must pass before running UI smoke tests:

```sh
cargo test -p forskscope-core -p forskscope-ui-logic
cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings
```

---

## RFC-041 item 1 — Two-way file compare works end to end

### 1a. Open via CLI

```sh
./target/release/forskscope tests/fixtures/text/left_function.rs \
                             tests/fixtures/text/right_function.rs
```

Expected: diff tab opens, left and right panes render, changed lines
highlighted with gutter markers.

### 1b. Open via file picker

- Launch `./target/release/forskscope`
- Click `+` (Add tab) or drag two files onto the window
- Verify diff renders with line-level highlighting

### 1c. Navigate hunks

| Action | Expected |
|--------|----------|
| Press **F7** | Focus moves to previous changed hunk |
| Press **F8** | Focus moves to next changed hunk |
| Focused hunk has visible outline | ✓ |
| Status bar shows hunk count | ✓ |

### 1d. Apply a hunk

- Navigate to a changed hunk
- Press **Enter** (or click the → merge button)
- Expected: right-side lines update to match left; hunk becomes equal;
  tab title gains `*` dirty marker

### 1e. Undo

- Press **Ctrl+Z**
- Expected: hunk reverts to original right-side content; dirty marker
  removed if no other changes remain

### 1f. Save

- Apply at least one hunk
- Press **Ctrl+S**
- Expected: save dialog appears with target path; after confirming, dirty
  marker removed; file on disk updated

### 1g. Search

- Press **Ctrl+F**
- Type a string that appears in the diff
- Expected: matching rows highlighted; **F3** / next button advances through
  matches

---

## RFC-041 item 2 — Directory compare works for practical trees

### 2a. Explorer loads

- Launch without arguments
- Expected: Explorer workspace visible; left and right directory panes
  show current working directory contents

### 2b. Navigate directories

| Action | Expected |
|--------|----------|
| Click directory row | Opens that directory in the pane |
| Click ↑ (Up) button | Navigates to parent |
| Click ◀ / ▶ (history) | Back / forward through visited directories |

### 2c. Digest indicators

- Point both panes at directories that share some files
- Wait a few seconds
- Expected: ✓ appears beside identical same-name files; ⚠ appears beside
  differing same-name files; unique files show no icon

### 2d. Open same-name file diff

- Both panes show a directory with a common file that differs
- Double-click the file on either side
- Expected: new diff tab opens comparing the two versions

### 2e. Deep compare

- Click the **Directory Report** mode button
- Expected: recursive scan starts; progress visible; entries show
  status (different / equal / unique)
- Click a "different" entry
- Expected: diff tab opens for that file pair

---

## RFC-041 item 3 — Basic keyboard navigation is complete

### 3a. Explorer keyboard

| Key | Expected |
|-----|----------|
| **↑ / ↓** | Moves focus through file/directory rows |
| **Enter** on a directory | Opens that directory |
| **Enter** on a same-name file | Opens diff tab |
| **Space** | Selects file as comparison candidate |
| **Alt+↑** | Navigates up to parent directory |

### 3b. Diff keyboard

| Key | Expected |
|-----|----------|
| **F7** | Previous changed hunk |
| **F8** | Next changed hunk |
| **Enter** | Apply focused hunk |
| **Ctrl+Z** | Undo last applied hunk |
| **Ctrl+Y** | Redo |
| **Ctrl+S** | Save merge result |
| **Ctrl+F** | Open / close inline search |
| **F3** | Next search match |
| **Shift+F3** | Previous search match |
| **Ctrl+W** | Close active tab (with dirty-state guard) |
| **Ctrl+/** | Open keyboard shortcut reference modal |
| **Escape** | Close open modal or search bar |

### 3c. Tab navigation

| Key | Expected |
|-----|----------|
| Click Explorer tab | Returns to Explorer workspace |
| Click diff tab | Activates that comparison |
| Click × on dirty tab | Shows unsaved-changes dialog |

---

## Pass criteria

All items checked → tick the three RFC-041 boxes:

```
[x] Two-way file compare works end to end
[x] Directory compare works for practical trees
[x] Basic keyboard navigation is complete
```

Then update `rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md`
and cut a release.

---

## Known GTK-environment issues

### NVIDIA + Wayland blank window

```sh
WEBKIT_DISABLE_DMABUF_RENDERER=1 ./target/release/forskscope
```

### Missing WebKitGTK library

```
error while loading shared libraries: libwebkit2gtk-4.1.so.0
```

Install: `sudo apt-get install libwebkit2gtk-4.1-0 libgtk-3-0`

See `docs/src/users/troubleshooting.md` for full platform-specific guidance.
