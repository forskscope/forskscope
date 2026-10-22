# RFC 096: One Program — the App and Its Commands

**Status.** Accepted — review complete; implementer may start. Moves to `done/`
when the work ships (RFC-000, 5-folder variant).
**Proposed.** 2026-10-22 by the architect. The owner approved the design in
discussion the same day.
**Accepted.** 2026-10-22 by the project owner.
**Scheduling.** `0.196.0`, before RFC-095 stage 2's documentation.
**Supersedes.** RFC-094 §1's second program and §5's packaging. RFC-094's
commands, JSON contract, MCP server and guarantees stay. Only where they live
changes.
**Tracks.** The owner's direction, 2026-10-22:
- the feature exists for **collaboration between a person and an AI agent**;
- there is no good reason to split the program on purpose;
- **one executable:** without a subcommand it is the app, and with one it is
  the command or the server.

**Touches.**
- `crates/forskscope-cli`, which becomes a library, `forskscope-commands`;
- `crates/forskscope-ui`'s `main`;
- `xtask` (`crate-boundaries`, `json-schema`);
- the workflows: `ci.yml`, `release.yml`, `windows-check.yml`;
- packaging: `build-release.sh`, `install.sh`, `PKGBUILD`, `build-dmg.sh`,
  `build-zip.sh`;
- the guide and the maintainer docs;
- `threat-model.md` entry 10.

## Summary

`0.195.0` shipped two executables, `forskscope` and `forskscope-cli`. From
`0.196.0` there is one, `forskscope`:

```text
forskscope                                   the app
forskscope <left> <right>                    the app, comparing two files or folders
forskscope <local> <remote> <merged>         the app, as git mergetool
forskscope compare <left> <right> [options]  prints a comparison; no window
forskscope mcp --root <dir> [--allow-show]   the MCP server for AI agents; no window
```

**What does not change:**
- what `compare` prints;
- its exit codes;
- the `forskscope.comparison/1` JSON and its schema;
- the MCP tools and their protocol revisions;
- the roots rule.

`forskscope-cli` is removed.

## Motivation

**RFC-094 split the program for one reason.** On Linux, the app links GTK and
WebKitGTK, which must be present before the program can start at all. A
separate program could run on a server or a CI container without a desktop.

**The purpose is collaboration on the same machine:** a person and an AI agent
working together where the app already runs. That machine has the desktop
libraries. For it, the split costs:
- a second file in every download;
- a second name to learn;
- a rule that `show_to_person` finds the app beside it;
- two of everything in the release checks.

None of that is needed. One file is simpler to deliver and to carry, and
portable mode is one file again.

## 1. Dispatch

**The first argument decides,** before anything else in `main`:
- **exactly `compare`:** the comparison command;
- **exactly `mcp`:** the agent server;
- **anything else, or nothing:** the app, exactly as today, including
  `--version`, `--help`, `--diagnostics`, `--data-dir` and `--private`.

**A subcommand starts nothing the app uses:**
- no GTK, tao, web view or WebKit;
- no settings, no session, no data folder;
- no Flatpak start-up changes (RFC-095 §4.2 and §4.7).

**`main` checks the first argument and hands over** before any of that code
runs.
- **Test it:** a subcommand run creates no file at all. Use the
  writable-overlay method against a sandbox home, as RFC-094's proofs did.
- **Test it on Linux:** a subcommand run never connects to a display. Run it
  with `DISPLAY` and `WAYLAND_DISPLAY` unset, and it still works.

**A file literally named `compare` or `mcp`:**
- it is written `./compare`;
- the guide says so;
- `git difftool` and `git mergetool` pass a temporary path first, so they
  cannot collide.

**Help:**
- `forskscope --help` lists the app's usage and the two subcommands.
- `forskscope compare --help` and `forskscope mcp --help` give each one's
  options.

## 2. Read-only, kept at the source level

RFC-094's `forskscope-cli` was **read-only by construction**: it did not link
the code that writes. In one executable, that code is present. **The guarantee
moves to the source:**

- **`crates/forskscope-cli` becomes the library `forskscope-commands`.**
  - It holds `compare` and `mcp`.
  - It has one entry point per subcommand, each taking the arguments and
    returning the exit code.
  - **Its dependencies stay exactly `forskscope-compare` and
    `forskscope-report`.**
- **`crate-boundaries`:**
  - its table names `forskscope-commands` with those two dependencies;
  - **its `std::net` rule covers it,** as it covered `forskscope-cli`;
  - **rule 2 now applies to it:** no process, except `show_to_person`'s one
    launch, listed by file and function as the only exception.
  - **Falsify each one:**
    - a `forskscope-edit` dependency added;
    - a `std::net` use;
    - a second `Command`.
- **The app crate depends on `forskscope-commands`** only to dispatch to it.
  **No code in `forskscope-commands` calls into the app.**

**The threat model says it plainly:** the commands cannot reach the writing
code, by the dependency graph and its gate, even though the executable contains
it.

## 3. `show_to_person`

- **It starts `std::env::current_exe()`,** the same executable, with the two
  paths as separate arguments and no subcommand. That opens the app.
- **The "beside it" lookup is gone.**
- **Unchanged:**
  - no shell;
  - the user's environment;
  - null standard input and output;
  - the early-exit report;
  - the reaping thread.

## 4. Platforms

- **Linux: the executable needs GTK and WebKitGTK installed,** for the
  subcommands too. This is the one case RFC-094's split covered and this RFC
  gives up: a machine with no desktop.
  - The guide says it in one sentence, under *Using ForskScope from scripts
    and AI agents*.
  - **If headless use is ever wanted,** it returns as its own decision.
- **macOS:** `ForskScope.app/Contents/MacOS/forskscope compare …` from a
  terminal. The guide's symlink example points at `forskscope`.
- **Windows: the app is a windowed program** (`windows_subsystem =
  "windows"`).
  - **A subcommand attaches to the calling console** (`AttachConsole` with
    `ATTACH_PARENT_PROCESS`) only when its standard handles are not already
    redirected. **Piped use, which is how MCP clients start it, needs
    nothing.**
  - **Verify on a real Windows PC.** The owner runs the commands the dev team
    gives, in PowerShell and in `cmd`:
    - a terminal run shows the output;
    - the exit code reaches `$LASTEXITCODE` and `%ERRORLEVEL%`;
    - a piped run works.
  - **The guide documents exactly what works.** That includes any form needed
    to wait for the result, such as piping in PowerShell or `start /wait` in
    `cmd`, as verified.
- **Flatpak (RFC-095):** `flatpak run io.github.forskscope.forskscope compare
  …` and `… mcp …`, with no `--command=`.
  - **The dispatch comes first,** so RFC-095 §4.7's start-up changes never
    run for a subcommand.

## 5. Packaging and release

**The archives return to one program:**
- **Linux `.tar.gz`:** `forskscope` only.
- **Windows `.zip`:** `forskscope.exe` and the documents.
- **macOS:** the bundle's `forskscope` only.
- **AUR:** installs `/usr/bin/forskscope` only.
- `install.sh` and `build-zip.sh` follow.

**`release.yml`:**
- **The 16-library check and the Windows import gate run on the one program,
  as before `0.195.0`.**
- **Remove** `forskscope-cli`'s link check and its import check, and their
  sizes.

**`xtask json-schema` runs `forskscope compare`.**
- CI builds the app already, so the gate needs nothing new.
- **Falsify it again:** a renamed field fails.

**The tests move with the code:**
- The binary-level tests run `forskscope compare` and `forskscope mcp`:
  - exit codes;
  - patch identical to *Export patch*;
  - the MCP protocol;
  - `show_to_person`.
- **`show_to_person`'s tests** use a copy of `forskscope` with a stand-in
  build, or a test seam. Say which, and that the real launch path is still
  covered.
- **Compare test names, not sums.** Every RFC-094 test exists after the move,
  renamed only where its subject changed.

## 6. Documentation and record

- **`scripts-and-agents.md`:**
  - one program;
  - the subcommands;
  - the Linux desktop-libraries sentence;
  - the Windows forms, as verified;
  - the MCP configuration example with `"command": "/path/to/forskscope"`,
    `"args": ["mcp", "--root", …]`.
- **`cli.md`:** the two subcommands, beside the app's own arguments.
- **The maintainer docs** that name `forskscope-cli`: the architecture,
  testing, release, local-dev and GTK smoke-test pages, and `CONTRIBUTING.md`.
- **`threat-model.md` entry 10:**
  - one executable;
  - §2's source-level guarantee;
  - §3's launch.
- **RFC-094 gets a one-line note:** superseded in part by RFC-096.
- **The CHANGELOG says, under *Changed*:**
  - `forskscope-cli` is replaced by `forskscope compare` and `forskscope mcp`;
  - an MCP client's configuration changes its `command` to `forskscope`, and
    adds the subcommand to `args`.

## Acceptance criteria

1. **One executable on every platform,** and none named `forskscope-cli`.
2. **`forskscope compare` matches `0.195.0`'s `forskscope-cli compare`
   byte for byte:**
   - text, JSON, patch and HTML;
   - the same exit codes;
   - over the existing test cases, with `generated_at` fixed for HTML.
3. **`forskscope mcp`:**
   - passes every RFC-094 protocol test;
   - `show_to_person` opens the app from the same executable;
   - the window title is checked live, through AT-SPI only.
4. **A subcommand writes nothing and needs no display** (§1). Shown by the
   overlay, and by a run with no display variables.
5. **No network for either subcommand,** by RFC-094's `LD_PRELOAD` observer.
6. **`crate-boundaries` enforces §2,** with three falsifications.
7. **Windows:** the owner's PowerShell and `cmd` runs (§4), quoted. The guide
   matches them.
8. **The app is unchanged without a subcommand:**
   - every existing app test passes;
   - a live run opens the Explorer and a file comparison.
9. **The release checks run on the one program,** and the archives hold one
   program each.

## Non-goals

- **A headless build for machines without a desktop** (§4).
- **New subcommands, or changes to `compare` or `mcp` themselves.**
- **Keeping `forskscope-cli` as an alias.** It shipped one day before this
  RFC, and a second name is what this RFC removes.

## Alternatives considered

- **Keep two programs** (RFC-094). Rejected: the split serves only machines
  without a desktop, which is not this feature's purpose.
- **Separate downloads for the two programs.** Rejected: still two programs.
- **Flags instead of subcommands** (`--compare`). Rejected:
  - subcommands read better;
  - each has its own options;
  - they cannot be mistaken for the app's own flags.
- **The names `serve`, `get` or `cli`.** Rejected:
  - `serve` suggests a network server, and there is none;
  - `get` is vague;
  - `cli` names a mode, not a task.
