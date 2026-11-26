# Handoff 091 — 0.196.0, part 1: one program, the app and its commands

**From:** architect. **Date:** 2026-10-22. **Release:** `0.196.0`.
**Implements:** RFC-096
(`rfcs/accepted/096-one-program-the-app-and-its-commands.md`), accepted by the
owner on 2026-10-22. It replaces RFC-094's second program and its packaging.

**Order:**
- **This handoff comes first.** Handoff 090 (RFC-095 stage 2) follows.
- **If you work on both,** RFC-096's dispatch owns the very first lines of
  `main` (090's banner).

**The RFC is the authority.** Where this document and the RFC differ, the RFC
wins. If the work shows the RFC is wrong anywhere, **stop and tell me.** Do not
change the design, and do not edit any RFC.

## Rules

**Desktop safety, for every live check:**
- **AT-SPI actions only, never keystrokes.**
- A sandbox `HOME`.
- Stop processes by pid. **Never `pkill -f`.**

**A real agent client,** if you drive one (`claude -p`), runs locked down:
- `--tools ""`;
- `--permission-mode dontAsk`, with `--allowed-tools` naming the one MCP tool;
- `--no-session-persistence`;
- run from the sandbox folder, with absolute paths in the prompt;
- the request states that `claude` still reads the user's settings to sign
  in.

It is optional here: the RFC-094 interoperability proof stands.

**Research online freely.**

## 1. Steps, one commit each, each green

1. **The library.** `git mv crates/forskscope-cli crates/forskscope-commands`.
   - The package becomes `forskscope-commands`. It is a `lib` only, with no
     `[[bin]]`.
   - **One public entry point per subcommand,** each taking the arguments
     after the subcommand and returning the exit code. For example
     `forskscope_commands::compare::run(&[String]) -> i32` and
     `forskscope_commands::mcp::run(&[String]) -> i32`.
   - **Dependencies stay exactly `forskscope-compare` and
     `forskscope-report`.**
   - **Messages say `forskscope compare:` and `forskscope mcp:`,** and the
     MCP `serverInfo` name is `forskscope`.
2. **The dispatch in `forskscope`'s `main`:**
   - **It is the first thing `main` does** (RFC-096 §1): `compare` or `mcp`
     as the exact first argument hands over to the library, and the process
     exits with its code.
   - **Nothing before it** touches GTK, tao, the web view, settings, the data
     folder, the Flatpak start-up changes, or logging set-up that writes
     anywhere.
   - **`--help`:**
     - `forskscope --help` adds the two subcommands;
     - `forskscope compare --help` and `forskscope mcp --help` show each
       one's options.
   - **Windows (RFC-096 §4):** when a subcommand runs with standard handles
     that are not redirected, attach to the parent console
     (`AttachConsole(ATTACH_PARENT_PROCESS)`). When they are redirected (pipes
     or files), change nothing. Explain in a comment how the two cases are
     told apart.
3. **`show_to_person`** starts `std::env::current_exe()` with the two paths,
   and no subcommand (RFC-096 §3). The "beside it" lookup is removed.
4. **The tests move:**
   - the binary-level suites run `forskscope compare` and `forskscope mcp`
     (`CARGO_BIN_EXE_forskscope`);
   - **`show_to_person`'s tests:**
     - they need a stand-in for the app the executable starts;
     - choose either a copy of the executable with a test seam, or a seam
       in the launch function;
     - **say which, and show the real launch path is still exercised once,**
       live, in §2.
5. **`xtask`:**
   - **`crate-boundaries` (RFC-096 §2):**
     - the table names `forskscope-commands` with its two dependencies;
     - the `std::net` rule covers it;
     - **rule 2 covers it,** with `show_to_person`'s launch as the one listed
       exception, by file and function;
     - **falsify all three:** a `forskscope-edit` dependency, a `std::net`
       use, and a second `Command`.
   - **`json-schema`** runs `forskscope compare`. Falsify it again with a
     renamed field.
6. **Workflows and packaging (RFC-096 §5):**
   - **`release.yml`:**
     - remove `forskscope-cli`'s link check, import check and size line;
     - the archives hold one program;
     - the 16-library check and the Windows import gate stay on
       `forskscope`.
   - **Also:** `ci.yml`, `windows-check.yml`, `build-release.sh`,
     `install.sh`, `PKGBUILD`, `build-dmg.sh` and `build-zip.sh`.
   - **Rehearse the changed release steps without a tag,** as for F206:
     - a temporary `windows-check.yml` job and an equivalent Linux check,
       both reverted;
     - show the archives' contents.
7. **Documentation (RFC-096 §6):**
   - the guide (`scripts-and-agents.md`, `cli.md`);
   - the maintainer pages and `CONTRIBUTING.md`;
   - `threat-model.md` entry 10.
   - **Do not touch** the CHANGELOG, the ROADMAP, the version or any RFC.
   - **Grep for `forskscope-cli` and `forskscope_cli`.** The request says the
     result is empty except history (CHANGELOG, RFCs, the register).

## 2. Verification (RFC-096 acceptance criteria)

1. **One executable** in every archive, from the rehearsal.
2. **Byte for byte against `0.195.0`.**
   - Build the tag's `forskscope-cli` in a clean worktree.
   - Run both over the existing test cases, in each format: text, JSON,
     patch, and HTML with `generated_at` fixed.
   - Diff the outputs, and compare the exit codes.
   - **Quote the summary,** and say how `generated_at` was fixed.
3. **`forskscope mcp`:**
   - every RFC-094 protocol test passes;
   - **live:** `show_to_person` opens the app from the same executable, with
     the window title checked through AT-SPI or `niri msg windows`.
4. **A subcommand writes nothing and needs no display.**
   - The writable-overlay method against a sandbox home: the overlay stays
     empty.
   - A run with `DISPLAY` and `WAYLAND_DISPLAY` unset works.
   - **Falsify the display check:** a test build that calls `gtk::init`
     before the dispatch fails it.
5. **No network for either subcommand,** with RFC-094's `LD_PRELOAD`
   observer, shown catching a deliberate socket first.
6. **`crate-boundaries`:** the three falsifications from §1 step 5.
7. **Windows, for the owner to run.**
   - Write the exact commands for PowerShell and for `cmd`:
     - a terminal run;
     - a piped run;
     - reading the exit code;
     - any form needed to wait for the result.
   - Use a small sandbox folder pair the commands create themselves. They
     touch nothing else.
   - Put them in the request. I check them, then hand them to the owner.
   - **The guide's Windows text waits for the owner's results.**
8. **The app is unchanged without a subcommand:**
   - every existing app test passes;
   - **live, through AT-SPI only:** the Explorer, and a file comparison from
     two paths.
9. **Test names, not sums:** every RFC-094 test exists after the move.
   Renames are listed, each with its reason.

## Scope

- **In:** RFC-096 entire.
- **Out:**
  - new subcommands;
  - any change to what `compare` or `mcp` do;
  - a headless build;
  - an alias for `forskscope-cli`;
  - RFC-095's work;
  - CHANGELOG, ROADMAP, the version.

## Gates

- **The standard set,** including Windows GNU clippy.
- **A green CI run** with its ID.
- **A Windows Check dispatch** with its ID.

Reply in `.git-exclude/review-request/167-one-program.md`.
