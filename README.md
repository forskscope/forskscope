# ForskScope

[![Releases Workflow](https://github.com/nabbisen/forskscope/actions/workflows/release-executable.yaml/badge.svg)](https://github.com/nabbisen/forskscope/actions/workflows/release-executable.yaml)
[![License](https://img.shields.io/github/license/nabbisen/forskscope)](https://github.com/nabbisen/forskscope/blob/main/LICENSE)

![logo](docs/src/assets/logo.png)

Diff through Exploring 🕵️‍♀️ GUI tool with cross-platform support 💻️ named after "*forske forskjell*" (research difference) 🤍

---

## Overview

ForskScope is a desktop diff and merge workstation for developers and operators who want a practical WinMerge-style experience on Unix/Linux, macOS, and Windows without being forced into an IDE or a Git GUI.

It is built on a GUI-independent Rust core (`forskscope-core`) and a Dioxus desktop frontend, giving a fast, local, offline experience with a calm default layout.

---

## Why / When

Use ForskScope when you need to:

- compare two text files or directories side by side before accepting a change;
- selectively apply individual hunks from a left (old/source) file into a right (new/target) file and save safely;
- inspect differences visually without a terminal, an IDE, or a remote service;
- work primarily on Unix/Linux and want the WinMerge workflow on your platform.

ForskScope is deliberately **not** a Git GUI, IDE, cloud diff service, or file synchronization suite — it has one job, and its scope is kept narrow so it stays trustworthy.

---

## Quick Start

**Requirements:** a Rust toolchain (≥ 1.85) and the GTK 3 / WebKitGTK 4.1 runtime libraries on Linux.

```sh
# Linux — install runtime libraries (Debian/Ubuntu)
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev

# Build from source
git clone https://github.com/forskscope/forskscope
cd forskscope
cargo build --release
./target/release/forskscope
```

**Open two files directly:**

```sh
forskscope path/to/old.rs path/to/new.rs
```

**Workflow at a glance:**

1. Launch the app — the Explorer workspace opens.
2. Navigate the left and right directory panes; select a file on each side, then click **Compare**.
3. Use **◀ ▶** to move between differences.
4. Click **▶** in the action column of a changed hunk to apply it from left to right.
5. Click **Save** — the merge result is written with conflict detection and an automatic `.bak` backup.

---

## Design Notes

**One job, done well.** The scope is intentionally narrow: local two-pane text diff and merge. Binary and Excel files can be compared (read-only) but not merged; three-way merge and directory synchronization are future work.

**Less is more.** The default layout shows only what the current workflow step needs. Navigation, apply, and save are always visible when they apply. Advanced controls (inline character diff, redo, future compare profiles) are one click away behind a disclosure, not front-loaded.

**Model-backed merge.** Every merge action goes through a transaction log in the Rust core. The UI never recomputes merge results from rendered content. Undo, redo, dirty state, and save safety are derived from the core model, not from the DOM.

**Local-first and private.** No accounts, no telemetry, no cloud upload. Files compared on your machine stay on your machine.

---

## More Detail

Full documentation is in [`docs/`](docs/src/SUMMARY.md), structured as an mdbook.

- [Features and tutorials](docs/src/users/features.md) — for new users.
- [Architecture overview](docs/src/maintainers/architecture.md) — for contributors.
- [RFC directory](rfcs/README.md) — design decisions and rationale.
- [Changelog](CHANGELOG.md)
