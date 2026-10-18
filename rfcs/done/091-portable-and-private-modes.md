# RFC 091: Portable and Private Modes, and a Windows Program That Runs by Copying

**Status.** Implemented (`0.193.0`, `a4fb730`; reviews 170, 171).
**Accepted.** 2026-10-15 by the project owner.
**Amended.** 2026-10-17, by the owner's decision: on macOS the two modes refuse to start (§2a). The system web view's own storage on Linux is redirected through the XDG directories (§2). F200.
**Scheduling.** `0.193.0`. See `ROADMAP.md` §
"Remaining proposed RFCs".
**Tracks.** The owner's idea, 2026-10-15: improve portability, store settings
beside the program or not at all, and say so on the website. It also resolves
F45 (a): the Windows program depends on the Visual C++ runtime. The owner
accepted the direction on 2026-10-15.
**Amends.** RFC-076 (where settings and session are stored) and
RFC-011 (session persistence). Each carries a one-line Status note pointing here.
**Touches.**
- `crates/forskscope-ui-logic/src/compare/startup.rs` (argument parsing)
- `crates/forskscope-ui/src/main.rs`
- `crates/forskscope-ui/src/state.rs` (`config_file_path`)
- `ui/overlay/modals/copy.rs` (manifest folder)
- the status bar and About
- the WebView data directory (`dioxus-desktop` `Config::with_data_directory`)
- `.cargo/config.toml` (Windows linking)
- `release.yml` (a Windows import check)
- `docs/src/users/installation.md`, `docs/src/intermediate/cli.md`

## Summary

Three additions, so that ForskScope runs where the home folder is shared,
restricted or not writable at all:

1. **Portable mode.**
   - A folder named `forskscope-data` beside the program, or a folder given with
     `--data-dir <path>`, holds everything the app writes.
   - Nothing is written to the home folder.
2. **Private mode** (`--private`).
   - ForskScope starts from default settings, reads no saved settings or
     session, and writes nothing that outlives the run.
3. **The Windows program carries its own Visual C++ runtime.**
   - `forskscope.exe` runs by copying it, on any Windows with the WebView2
     runtime.

## Motivation

**Where ForskScope runs today.** It writes in four places, all under the user's
home folder:

| What | Where |
|---|---|
| Settings and session | the platform config folder (`dirs_next::config_dir`) |
| Batch-copy manifests | the platform data folder |
| The web view's own storage | Linux: `~/.local/share/forskscope` and `~/.cache/forskscope`; Windows: `%LOCALAPPDATA%\forskscope` |

So it cannot be carried on a USB drive with its settings. On a virtual desktop
or a locked-down machine where the home folder is not writable, it cannot keep
settings at all. On a shared machine, everything a user compared is recorded in
that user's session file. A user may be **forbidden to write to the home
folder**, and portability is what lets the app work there.

**Windows needs more than the program.** `forskscope.exe` imports
`VCRUNTIME140.dll` and `VCRUNTIME140_1.dll` (F45). On a Windows without the
Visual C++ redistributable, it fails at launch with *"VCRUNTIME140.dll was not
found"*. The Microsoft Store package does not declare that dependency either
(`AppxManifest.xml` lists only the device family).

F45 recorded this in August 2026, and the WebView2 half got a startup check.
Linking the runtime in was never proposed: an oversight, not a decision.

## Design

### 1. How ForskScope decides where to keep its data: once, at startup

**Before reading any file,** ForskScope resolves one *data location* in this
order. The first match wins:

| # | Condition | Mode | Location |
|---|---|---|---|
| 1 | `--private` | **Private** | nothing kept (§3) |
| 2 | `--data-dir <path>` | **Portable** | `<path>` |
| 3 | A folder named `forskscope-data` beside the program | **Portable** | that folder |
| 4 | Otherwise | **Installed** (today) | the platform folders, unchanged |

- **"Beside the program"** is the folder of the program file, with symbolic links
  resolved (`std::env::current_exe`, canonicalized). A link in `~/bin` to a
  program elsewhere uses the real program's folder.
- **Portable mode is never guessed.** A writable program folder alone does not
  switch it on. Otherwise files would appear beside a program in `~/bin` or a
  download folder.
- **`--private` with `--data-dir`** is a contradiction. ForskScope prints why and
  exits with status 2, before opening a window.
- **`--data-dir`'s folder** is created if it does not exist. A relative path is
  taken from the current directory.
- **The decision is made once.** It cannot change while running, so all four
  kinds of data always agree on where they are.

### 2. Portable mode

Everything the app writes goes inside the data location:

| What | Path inside the data location |
|---|---|
| Settings | `settings.json` |
| Session | `session.json` |
| Batch-copy manifests | `manifests/` |
| The web view's storage | `webview/`, given to `dioxus-desktop` as its data directory |

- **Nothing is written under the home folder by ForskScope.** The test for this
  (Acceptance 3) runs with the home folder **read-only**.
  - Some writes belong to the system's toolkit, not to ForskScope, for example
    GTK or fontconfig caches. The test lists any it observes, and says whether
    the app still works when they fail.
  - Those are documented, not hidden.
- **If the data location is not writable,** for example a read-only drive or
  share, ForskScope still starts, with saved settings if they can be read:
  - The status bar says *Portable (read-only): changes are not saved*.
  - ForskScope **never falls back silently** to the home folder.
- **The status bar always shows the mode** in portable: *Portable*, with the
  folder in its tooltip.
- **About and *Copy diagnostics*** show the mode and the data location.
- **No automatic import from an installed copy.** Moving existing settings is
  one documented step: copy `settings.json` into the folder. A surprise copy of
  someone's settings onto a USB drive is worse than one manual step.
- **Where it applies:** the downloaded program on Windows and Linux. An AUR or
  Store installation has a program folder that is not writable, so the marker
  folder cannot exist there. `--data-dir` and `--private` work on Windows and
  Linux, and refuse to start on macOS (§2a).
- **macOS:** see §2a. The marker folder is ignored there. macOS runs an unsigned
  app from a hidden, read-only copy (App Translocation), so *beside the
  program* is not a stable place.
- **Trust:** whoever can write the program's folder can choose its settings.
  Settings hold no commands or anything that runs today (verified 2026-10-15),
  so that grants nothing. **Any future setting that runs or names a program must
  be reviewed against portable mode before it is added.**

**The system web view's own storage, on Linux (F200).** `dioxus-desktop`
0.7.9's pinned `wry` sets only WebKitGTK's base data directory, so its cache and
HSTS file would stay in home. Before GTK initialises and before any thread
exists, ForskScope therefore sets `XDG_CACHE_HOME`, `XDG_DATA_HOME` and
`XDG_STATE_HOME` to folders inside the data location. That also captures the
toolkit's own writes, such as file-chooser metadata. Every child process
ForskScope starts gets the user's original values back. `XDG_CONFIG_HOME` is not
changed, so the user's GTK settings are still read.

**On Windows,** WebView2 keeps its cache inside the user data folder that
`with_data_directory` sets. This is measured, not assumed. If it is shown to
escape, Windows gets §2a's refusal until it is fixed.

### 2a. A mode that cannot keep its promise does not start

**The owner's principle, 2026-10-17:** a mode either keeps its promise fully or
is not offered. Users must never be left to rely on a guarantee that silently
does not hold.

**On macOS,** `dioxus-desktop` neither applies the data directory to WKWebView nor
exposes a non-persistent store. So the system web view would keep its cache in
`~/Library` whatever the mode, and neither mode's promise can be kept there.
Therefore, **on macOS:**

- **`--private` and `--data-dir` refuse to start.** ForskScope prints one plain
  message and exits with status 2, before any window opens. For example:
  - *"--private is not available on macOS: the system web view keeps its own
    data in your home folder, so ForskScope cannot promise to save nothing."*
  - The `--data-dir` message says that ForskScope cannot keep everything in the
    chosen folder.
- **`--help` does not list the two options on macOS,** rather than listing them
  with a caveat.
- **The installed mode is unchanged.**
- **Revisit** when the framework can relocate or disable WKWebView's storage.

**The same rule binds every platform.** No status bar, About text or document
carries a platform exception for these modes. Where a mode runs, its promise is
whole.

### 3. Private mode (`--private`)

- **Reads nothing:** no settings, no session. It starts from defaults with an
  empty Explorer at the home folder, or at the current folder if the home folder
  cannot be read.
- **Writes nothing that outlives the run.** Changes made in Settings apply for
  this run only.
- **The web view needs somewhere to write while running,** so it gets a new
  folder under the system's temporary directory, removed when ForskScope exits.
- **Compared files themselves** are still read and, when the user saves a merge,
  written. Private mode is about ForskScope's own records, not the user's
  files, and the help text says so.
- **The status bar always shows** *Private: nothing is saved*. A Settings change
  says the same next to it.
- **The name follows browsers' *private window*:** nothing is kept after it
  closes. Unlike a browser, it also does not read saved settings, and the help
  text says so: *start with default settings, and save nothing*.

### 4. Windows: the Visual C++ runtime built in

- **Link the Visual C++ runtime statically, and keep the Universal C runtime
  dynamic.** This is Microsoft's *hybrid CRT*:
  - `-C target-feature=+crt-static` for `x86_64-pc-windows-msvc`;
  - plus linker arguments that exclude the static UCRT (`libucrt.lib`) and use
    the dynamic one (`ucrt.lib`).
  - It goes in `.cargo/config.toml`, so every Windows build, the zip's and the
    Store's, gets it.
- **Why hybrid, not fully static:** the UCRT is part of Windows 10 and later, and
  Windows Update services it. Linking it statically would freeze its security
  fixes into each release. The Visual C++ runtime proper (`VCRUNTIME140*.dll`)
  is the part that may be missing, and it is small.
- **Risks, and how each is met:**
  - **C code compiled into the program must use the same runtime.** Crates that
    compile C through `cc` follow `crt-static` automatically. The build fails
    on a mismatch rather than misbehaving.
  - **Memory passed between different runtimes is dangerous.** ForskScope's one
    foreign component, WebView2, is reached through COM, which uses its own
    allocators.
  - **A slightly larger program,** about 100 to 200 KB. Measure and report.
- **A release gate,** like Linux's 16-library check (F179):
  - The Windows job lists `forskscope.exe`'s imported DLLs and fails if any is
    `VCRUNTIME*.dll`, `MSVCP*.dll`, or not on an explicit allow-list of Windows
    system DLLs.
  - A new import must be justified, as on Linux.
- **The Store package needs no change.** It declares no runtime dependency
  today, and after this it needs none.

### 5. Arguments, help and documentation

- `--help` gains `--data-dir <path>` and `--private`. They combine with the
  existing forms: `forskscope --private a.txt b.txt` opens a comparison
  privately.
- The flags are parsed before `parse_startup_args`'s file arguments, like
  `--version` and `--diagnostics`. An unknown `--` flag is an error, not a file
  name.
- **The user guide gains *Running portably*:**
  - the marker folder, `--data-dir`, `--private`;
  - what each writes, and what it does not;
  - how to move existing settings;
  - one line saying the modes are for Windows and Linux, with the reason macOS
    is excluded for now.

## Non-goals

- Bundling GTK or WebKitGTK on Linux (an AppImage). The program still needs the
  system's web view, and the website says so.
- Bundling the WebView2 runtime on Windows. Windows 11 includes it, and
  ForskScope already checks for it at startup.
- Syncing or importing settings between installed and portable copies.
- A Settings control for the mode. The mode is chosen at startup, by design.

## Acceptance criteria

1. **The resolution order of §1:**
   - each row selects its mode;
   - `--private` with `--data-dir` exits 2 with a message;
   - a symbolic link to the program uses the real program's folder.
   Unit tests on the pure resolver, each falsified.
2. **Portable mode:** settings, session, manifests and web-view storage all
   appear inside the data location, and nowhere else.
3. **Read-only home, on Linux** (`bwrap --ro-bind "$HOME" "$HOME"`):
   - portable mode starts, compares, saves its settings in the portable folder,
     and restores them on the next start;
   - a file-system watch on `$HOME` lists every write attempt;
   - any attempt that is not ForskScope's own is named, with what fails, if
     anything.
4. **Read-only data location:** the app starts, says so in the status bar,
   writes nothing anywhere, and never writes to home.
5. **Private mode:**
   - starts with defaults despite an existing settings file;
   - after a run with a settings change, two comparisons and a quit, no
     ForskScope file is newer than the start, and the temporary web-view folder
     is gone.
6. **Windows:**
   - the release gate lists `forskscope.exe`'s imports, with no
     `VCRUNTIME*.dll`, and fails when the static linking is removed;
   - the size change is reported;
   - the app runs on the CI Windows runner.
   
   A clean-Windows run is not possible in CI. The import table is the proof, as
   the 16-library check is on Linux.
7. **The status bar and About** show the mode in Portable, Portable (read-only)
   and Private.
8. **`--help`, `cli.md` and *Running portably*** describe exactly the behaviour
   above.
9. **On macOS,** `--private` and `--data-dir` each exit with status 2 and their
   message, before any window, in a unit test of the resolver with the platform
   as a parameter. `--help` there omits them.

## Open questions

None.
