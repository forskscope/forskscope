# Quick Start

## Install

Build from source (Rust ≥ 1.85 required):

```sh
# Linux: install WebKitGTK and GTK3 runtime libraries
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev

git clone https://github.com/forskscope/forskscope
cd forskscope
cargo build --release
```

The binary is `target/release/forskscope`.  Copy it anywhere on your `PATH`.

## First comparison

**From the command line:**

```sh
forskscope old_version.rs new_version.rs
```

The diff opens immediately as a tab.

**From the GUI:**

1. Launch `forskscope`.
2. The Explorer workspace opens with two directory panes.
3. Navigate each pane to a directory.
4. Click a file in the left pane, then a file in the right pane.
5. Click **Compare**.

## Core workflow

Once a diff tab is open:

| Step | How |
|---|---|
| Move to next change | **▶** button or `F8` |
| Move to previous change | **◀** button or `F7` |
| Apply a change | Click **▶ Use** in the action column of a changed hunk |
| Undo the last apply | **Undo** button |
| Save the result | **Save** button |

The tab shows a dot (●) when the result has unsaved changes.

## Settings

Click **Settings** in the header to change the theme, language, or diff font
size.  Settings are saved automatically.
