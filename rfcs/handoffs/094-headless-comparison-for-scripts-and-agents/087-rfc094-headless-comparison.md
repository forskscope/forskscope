# Handoff 087 — 0.195.0, part 2: headless comparison for scripts and AI agents

**From:** architect. **Date:** 2026-10-21. **Release:** `0.195.0`.
**Implements:** RFC-094
(`rfcs/accepted/094-headless-comparison-for-scripts-and-agents.md`), accepted by
the owner on 2026-10-20, on RFC-093's crates.

**The RFC is the authority.** Where this document and the RFC differ, the RFC
wins. If implementation shows the RFC is wrong anywhere, **stop and tell me
before changing course.** Do not change the design yourselves, and do not edit
any RFC.

**Two hard stops are built in:**
- **§4 step A:** measure the MCP library choice, report, and wait.
- **Any need for the program to write, open a socket, or link anything beyond
  `forskscope-compare` and `forskscope-report`:** stop and tell. The answer is
  almost certainly "no".

**Desktop safety, for every live check:** **AT-SPI actions only, never
keystrokes** (review 181 §2). If a step cannot be done that way, stop and say
so.

---

## 0. RFC-093 follow-ups (review 181 §3)

1. **`crate-boundaries` rule 1 catches imported write functions.**
   - Fail on `use std::fs::write;`, `use std::fs::{…, write, …}` and the like,
     for every listed function.
   - Fail on `File`/`OpenOptions` together with a write builder call.
   - Falsify it with the two-line evasion: `use std::fs::write;` and a bare
     `write(p, b)`.
2. **The live Deep Compare,** through AT-SPI only:
   - pick two sandbox folders with the rows' own action;
   - press *Compare ▶*;
   - take a screenshot, and describe it.

**Commit these first.**

## 1. The program

- **The crate:** `crates/forskscope-cli`. The package and the binary are both
  `forskscope-cli`.
- **Dependencies:** `forskscope-compare` and `forskscope-report`, and nothing
  else from this workspace.
  - **Add `forskscope-cli` to `crate-boundaries`' expected table** with exactly
    those two.
  - **Falsify:** a `forskscope-edit` dependency added to it fails.
- **Arguments are parsed by hand,** like the GUI's.
  - `--version` and `--help`.
  - **An unknown flag is an error, with exit code 2.**
- **Nothing reads the GUI's settings or session.**

## 2. `compare` (RFC-094 §2)

- **Files or folders,** decided by the paths. A file against a folder is
  refused.
- **The four formats,** to standard output.
  - `patch` for a file calls the same code as the GUI's *Export patch*.
  - `html` calls `forskscope-report`'s builders, the ones RFC-093 moved down.
    **Never a copy:** if the CLI needs something the GUI has, it moves into
    `forskscope-report` first.
- **The options,** with the GUI's defaults, mapped onto the existing types.
  There are no new comparison semantics.
- **Exit codes:**
  - **0:** the same;
  - **1:** different;
  - **2:** an error or a refusal, with one line on standard error naming the
    path and the reason.

## 3. The JSON contract (RFC-094 §3)

- **Its own types, in `forskscope-report`.** `Serialize` derives on those types
  only, never on the engine's.
- **The schema file:** `docs/schemas/forskscope.comparison.v1.json`.
  - **A gate validates emitted documents against it:** a planned set of file
    and folder cases, including every status and a capped folder.
  - **Put the validator in `xtask`,** whose dependencies do not reach the
    shipped programs.
  - **Falsify it:** a field renamed in the emitted output fails the gate.
- **The status identifiers:**
  - **one function maps `RecStatus` to the identifier,** beside RFC-093's
    status-word function;
  - **a test iterates every `RecStatus`,** and asserts the identifier and that
    the app's word exists for it.
- **The caps:** `--max-files` and `--max-change-bytes`, with defaults of 50 and
  5 MiB.
  - The `omitted` list names each path left out, with the reason.
  - **Test the boundary:** 50 files, 51 files, and exactly 5 MiB.

## 4. `mcp` (RFC-094 §4)

### Step A: measure, report, wait

**Before writing any MCP code, put this in a short request,** then wait for my
decision:
- **`rmcp` with only its stdio server features:**
  - its full dependency tree (`cargo tree -e normal`);
  - the crates it adds to this workspace;
  - whether any HTTP or network crate appears;
  - the protocol revision it implements;
  - its licence;
  - the binary size with it.
- **The minimal alternative:** an estimate of the hand-written subset
  (`initialize`, `tools/list`, `tools/call`, `ping`, cancellation), and of what
  it would need, `serde_json` included.

### Step B: after my decision

- **`--root` is required, and repeatable.**
  - Resolve every path, following symbolic links, then check it is inside a
    root.
  - **Test the refusals:** `..`, a symbolic link out of a root, an absolute
    path elsewhere, and a root that does not exist.
- **The tools:**
  - `compare`;
  - `show_to_person`, only with `--allow-show`. It starts the GUI program from
    the folder `forskscope-cli` itself is in, by its exact path, never through
    `PATH`. The two paths are separate arguments, with no shell, and the
    user's own environment.
- **The tool descriptions carry the untrusted-content sentence** from RFC-094
  §4.
- **Cancellation** maps to the engine's `CancellationToken`.
- **Standard output carries protocol messages only.** Diagnostics go to
  standard error.

## 5. Packaging and release gates (RFC-094 §1)

- **The archives:**
  - **Linux `.tar.gz` and Windows `.zip`:** `forskscope-cli` beside the GUI
    program.
  - **macOS:** inside the bundle, at `Contents/MacOS/forskscope-cli`.
  - **AUR `PKGBUILD`:** installs `/usr/bin/forskscope-cli`. Its build command
    builds both programs.
- **`release.yml`:**
  - **Linux:** `forskscope-cli` links only the C runtime, checked like the
    GUI's 16-library check.
  - **Windows:** `check-imports.ps1` runs on `forskscope-cli.exe` too.
  - Both programs' sizes appear in the log.
- **Rehearse without a tag,** as for F206: run the release jobs' new steps in a
  temporary `windows-check.yml` job, and an equivalent Linux check, both
  reverted. **Falsify the Linux check:** a test build that links GTK fails it.

## 6. Documentation

- **A guide page, *Using ForskScope from scripts and AI agents*:**
  - the commands;
  - the exit codes;
  - the JSON format and its schema;
  - the MCP tools;
  - a client configuration example;
  - what the roots mean.
- **`cli.md`** points to it.
- **`threat-model.md`:** draft the new entry points (RFC-094 §6). I review that
  text.

## 7. Verification (RFC-094 acceptance criteria 1–7)

**The rules after reviews 169–181 still apply:**
- quote the lines you rely on;
- look at screenshots, and describe them;
- every check must be able to fail;
- compare test names, not sums.

**In particular:**

- **Criterion 4:**
  - **A scripted MCP client over stdio** performs `initialize`,
    `tools/list` and `tools/call` on `compare`. A Python script in the
    scratchpad is enough. Quote the exchange.
  - **`show_to_person`** is shown absent without the flag. With the flag, it
    opens the GUI on the pair: give the window's title, through AT-SPI or
    `niri msg windows`.
- **Criterion 5:** `strace` is not installed. Use the RFC's clarified
  observer, an `LD_PRELOAD` shim logging `socket` and `connect`.
  - **Show it catching a deliberate socket** in a test build.
  - Then show it silent on both modes.
- **Criterion 6:** the RFC-091 writable-overlay method, against a sandbox home.
  The overlay stays empty.

## Scope

- **In:** §0 to §6.
- **Out:**
  - **writing of any kind,** a network server, Git revisions, the Store
    package;
  - **F209** (two folders on the GUI's command line), which has its own
    handoff;
  - CHANGELOG, ROADMAP, the version.

## Gates

- **The standard set,** with `crate-boundaries` and the new JSON-schema gate.
- **A green CI run** with its ID.
- **A Windows Check dispatch** with its ID.

Step A goes in `.git-exclude/review-request/162a-mcp-library.md`. Reply in
`.git-exclude/review-request/162-headless-comparison.md`.
