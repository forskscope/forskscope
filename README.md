# ForskScope

[![CI](https://github.com/forskscope/forskscope/actions/workflows/ci.yml/badge.svg)](https://github.com/forskscope/forskscope/actions/workflows/ci.yml)
[![Release](https://github.com/forskscope/forskscope/actions/workflows/release.yml/badge.svg)](https://github.com/forskscope/forskscope/actions/workflows/release.yml)
[![License](https://img.shields.io/github/license/forskscope/forskscope)](https://github.com/forskscope/forskscope/blob/main/LICENSE)

![logo](docs/src/assets/logo.png)

Diff and merge through Exploring 🕵️‍♀️ GUI tool, local-first, with cross-platform support 💻️ named after "*forske forskjell*" (research difference) 🤍

```
forskscope old/src/main.rs new/src/main.rs
```

ForskScope opens two files (or two directories) side by side, highlights every change at line and character level, and lets you apply hunks from left to right with a single keystroke. Everything runs locally — no accounts, no uploads, no telemetry.

---

## Why ForskScope

Most Unix/Linux workers reach for `vimdiff`, `git diff`, or a web-based paste tool when they need to compare files. These work but they don't give a persistent, navigable side-by-side view with merge support. WinMerge does — but only on Windows.

ForskScope fills that gap: a desktop app built on [Dioxus](https://dioxuslabs.com/) and a pure-Rust diff engine ([similar v3](https://docs.rs/similar)), packaged for Linux, macOS, and Windows.

---

## Quick start

### Build from source

```sh
# Prerequisites: Rust 1.85+, WebKitGTK 4.1 (Linux only)
# Ubuntu / Debian:
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev pkg-config

cargo build --release -p forskscope-ui
./target/release/forskscope
```

### Compare two files

```sh
forskscope old.rs new.rs
```

### Use with Git

```sh
# .gitconfig
[diff]
  tool = forskscope
[difftool "forskscope"]
  cmd = forskscope "$LOCAL" "$REMOTE"
[merge]
  tool = forskscope
[mergetool "forskscope"]
  cmd = forskscope "$LOCAL" "$REMOTE" "$MERGED"

# Then:
git difftool HEAD -- src/main.rs
git mergetool
```

---

## Features

- **Side-by-side diff** with line-level and character-level highlighting
- **Merge** — apply changes from left to right one hunk at a time; undo / redo
- **Three-way merge model** — base-aware diff3 with automatic merge of non-conflicting changes and structured conflict resolution (core model shipped; conflict workspace UI in progress)
- **Enter to apply** the focused hunk; F7/F8 to navigate; Ctrl+S to save
- **Explorer** — browse two directories, see digest equality indicators, compare same-name files with one click
- **Deep compare** — recursive directory scan with live progress; batch copy changed files between trees
- **Directory compare filter** — All / Different / Equal in the Directory Report view
- **Git difftool / mergetool** compatible (`forskscope old new` or `old remote merged`)
- **Compare profiles** — named presets for ignore-whitespace, ignore-case, and algorithm (Myers / Patience / Histogram)
- **Session persistence** — open tabs are restored on next launch
- **Patch export** — export a unified-diff `.patch` file from any file or directory comparison; compatible with `patch -p1` and `git apply`
- **Safe saves** — atomic write, `.bak` backup, external-change detection
- **Search within diff** — Ctrl+F highlights matching rows across both panes
- **Navigation history** — back/forward per explorer pane
- **Dark, Light, and Night themes**
- **English and Japanese UI** (i18n)
- **GitHub Actions CI/CD** — Linux x86_64, macOS aarch64, Windows x64 release builds on tag push

---

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| F7 / F8 | Previous / next change |
| Enter | Apply focused change |
| Ctrl+Z | Undo |
| Ctrl+S | Save |
| Ctrl+F | Search within diff |
| F3 / Shift+F3 | Next / previous search match |
| Ctrl+/ | Keyboard shortcut reference |

Press **?** in the header or **Ctrl+/** for the full reference.

---

## Documentation

Full documentation (built with mdbook): [`docs/src/`](docs/src/SUMMARY.md)

---

## License

Apache-2.0 — see [LICENSE](LICENSE). Author: **nabbisen**.
