# RFC 094: Headless Comparison for Scripts and AI Agents

**Status.** Implemented (`0.195.0`, `10ca2ac`; reviews 182–186).
**Superseded in part.** RFC-096 (accepted 2026-10-22) replaces §1's second program and §5's packaging with one executable: `forskscope compare` and `forskscope mcp`.
**Accepted.** 2026-10-20 by the project owner.
**Scheduling.** `0.195.0`, after RFC-093. See `ROADMAP.md` § "Remaining proposed RFCs".
**Tracks.** The owner's direction, 2026-10-20:
- extend ForskScope to support AI agents, through command-line arguments or a
  low-memory server mode;
- make collaboration between agents and people easier;
- **without bloating the program.**

**Amends.** RFC-027's JSON report: it was never shipped, and this RFC's
versioned format replaces it.
**Touches.**
- a new crate and program, `crates/forskscope-cli` (`forskscope-cli`);
- `forskscope-report` (the versioned JSON);
- `release.yml`, packaging and the AUR `PKGBUILD`;
- the user guide;
- `docs/src/maintainers/threat-model.md`.

## Summary

A second program, **`forskscope-cli`**, built only from `forskscope-compare` and
`forskscope-report` (RFC-093). It has two modes:

1. **`forskscope-cli compare <left> <right>`:** compares two files or two
   folders, and prints the result as text, versioned JSON, a unified diff or
   the HTML report. The exit code follows `diff`.
2. **`forskscope-cli mcp --root <dir>`:** a Model Context Protocol server on
   standard input and output, for AI agents.

**The program has four fixed properties:**
- **Read-only by construction:** it does not link the code that writes.
- **No network:** no socket is opened, and no port is listened on.
- **Stateless:** it reads no settings and writes no session.
- **Small:** no GTK, no WebKitGTK, no WebView2.

**The GUI program does not change.**

## Motivation

**Agents and scripts need what a person gets from the window:**
- whether two things differ;
- what differs, with line numbers;
- the app's own verdicts for a folder (*Identical* only after the contents
  were compared);
- something they can hand to a person.

Today all of this exists only behind a GUI.

**Why a second program, not a mode of the GUI program:**
- On Linux the GUI program links 16 libraries, among them GTK and WebKitGTK.
  Its loader maps them before `main` runs, even for a mode that never opens a
  window.
- So such a mode would need a desktop stack installed, and it **would not start
  at all on a server or in a CI container without GTK**, which is where agents
  often run.
- A separate program built from the engine alone has none of that, and leaves
  the GUI program exactly as it is.

**Why stdio MCP, not a local HTTP server:**
- **MCP over standard input and output is how current agents attach local
  tools.** The agent starts the program, and only that agent can talk to it.
- **A local HTTP port can be reached by any local process,** and by web pages
  through DNS rebinding. It would also be ForskScope's first listening socket,
  for no capability stdio lacks.
- **The app's posture stays whole:** no network request except the one a
  person makes by clicking *Check for updates*.

**Why read-only first.**
- An agent acts on text it reads, and files can contain instructions written
  to manipulate it.
- A comparison that can only read cannot be turned into a write by such a file.
- Writing, as merge, apply or copy, is a separate decision, for a later RFC
  with its own safety design.

## Design

### 1. The program

- **The crate:** `crates/forskscope-cli`, a binary named `forskscope-cli`.
  - It depends on `forskscope-compare` and `forskscope-report`, and on nothing
    from `edit`, `desktop`, `ui-logic` or `ui`. RFC-093's gate enforces this.
  - Its own argument parsing stays hand-written, like the GUI's. No parser
    framework.
- **Distribution:**
  - **The Linux `.tar.gz` and the Windows `.zip`** carry it beside the GUI
    program.
  - **The AUR package** installs `/usr/bin/forskscope-cli`.
  - **On macOS** it sits inside the bundle, at
    `ForskScope.app/Contents/MacOS/forskscope-cli`, and the guide says how to
    put it on `PATH`.
  - **The Microsoft Store package** is out of scope (Non-goals).
- **Release gates:**
  - **Linux:** `forskscope-cli` links only the C runtime (`libc`, `libm`,
    `libgcc_s` and the loader), checked like the GUI's 16-library check.
  - **Windows:** the import gate's script checks it too.
  - The release log reports both programs' sizes.

### 2. `compare`

```text
forskscope-cli compare <left> <right> [options]
forskscope-cli --version | --help
```

- **Two files or two folders,** decided from the paths. A file against a folder
  is refused.
- **Output:** `--format text` (the default), `json`, `patch` or `html`, to
  standard output only.
  - The program never creates a file. A user who wants one redirects.
  - `html` is RFC-092's report, unchanged.
- **Options:**
  - **The comparison profile's settings**, with the GUI's defaults:
    `--algorithm`, `--ignore-whitespace`, `--ignore-case`, `--ignore-newlines`,
    `--context <n>`.
  - **For folders, the ignore rules:** `--ignore-ext`, `--ignore-dir`,
    `--hide-dotfiles`.
  - **`--paths names|relative|full`:** how paths appear. File names only, by
    default, as RFC-092.
- **The exit code follows `diff`:**
  - **0:** the same;
  - **1:** different;
  - **2:** an error or a refusal, with **one line on standard error** that
    names the path and the reason.
  - *Clarified 2026-10-21 (architect, review 183):* **0 only when everything
    was compared and found the same.** An entry that was not compared (an
    unreadable file, a symbolic link not followed) is not *the same*, so the
    code is 1, and the output names the entry. 2 stays for a comparison that
    could not run at all.
- **Nothing is read but the two paths,** and nothing is written.

### 3. The JSON format: versioned, and the app's own vocabulary

- **The contract has its own types,** in `forskscope-report`, built from the
  engine's results. The engine's types are never serialized directly, so the
  engine can change shape without the contract changing. This is the rule the
  stored settings already follow with their `Persisted*` types (RFC-093,
  *No separate data crate*).
- **Every document starts with its schema:**
  `{"schema": "forskscope.comparison/1", …}`.
  - Adding a field keeps `/1`.
  - Removing a field, or changing its meaning, moves to `/2`.
  - A JSON Schema file in the repository describes each version, and a gate
    checks emitted documents against it.
- **A file comparison holds:**
  - **for each side:** its name (per `--paths`), size, SHA-256 and encoding;
  - `identical`;
  - **counts:** changes, lines removed, lines added;
  - **the changes:** old and new line numbers, `removed`, `added` or `context`
    for each line, and the line text;
  - **`notes`, only when one applies:** a limit that changed what is shown, an
    encoding that is not UTF-8, an undecodable side.
- **A folder comparison holds:**
  - **counts by status;**
  - **every entry,** with its path, status, sizes and SHA-256 where read;
  - **the changes of each different text file.**
  
  The status identifiers are stable and machine-readable: `identical`,
  `different`, `left_only`, `right_only`, `unreadable`, `not_compared`,
  `symlink_not_followed`, `left_only_empty_folder`, `right_only_empty_folder`.
  Each maps one-to-one to the words the app shows.
- **Never `identical` without a content comparison** (F74/F82). That rule holds
  here as everywhere.
- **Caps that are never silent:**
  - `--max-files` and `--max-change-bytes` default to RFC-092's 50 files and
    5 MiB;
  - anything left out is listed by path in `omitted`, with the reason.
  
  An agent's context is finite, and so is a person's patience.
- **Spreadsheets and binary files** report their status only, as in RFC-092.

### 4. `mcp`: the agent server

```text
forskscope-cli mcp --root <dir> [--root <dir> …] [--allow-show]
```

- **Transport:** MCP over standard input and output, at the protocol revision
  current when this is implemented. The guide names that revision.
  - *Clarified 2026-10-21 (architect, review 182):*
    - **The current revision, 2026-07-28, is stateless.** It drops the
      `initialize` handshake, and its clients fall back to `initialize` when
      they reach a server on 2025-11-25 or earlier.
    - **The server therefore implements 2025-11-25,** which both client
      generations can use today.
    - **Supporting 2026-07-28 natively is a later step.** It is triggered when
      a major client stops falling back.
    - *Clarified (review 183):* the server also answers a client that asks
      for **2025-06-18** with that revision. The subset this server uses is the
      same in both, and older clients reject a revision they do not know.
      Earlier revisions are not offered: 2025-03-26 requires JSON-RPC batches.
    - **By the measurement rule below,** the protocol is **hand-written**, with
      no new dependency. `rmcp` would add 48 crates and an async runtime
      (`tokio` I/O with `mio` and `socket2`), and its stdio server alone
      measured larger than the whole `compare` program.
  - JSON-RPC requests in.
  - Nothing else on standard output.
  - Diagnostics on standard error.
- **Roots are required, and are the whole world:**
  - every path an agent passes is resolved, following symbolic links;
  - a path that is not inside a `--root` is refused, with an error naming the
    rule;
  - the person who configures the server decides the roots, and the agent
    cannot widen them.
- **Tools:**
  - **`compare`:** `left`, `right`, the §2 options, and `format` (`json` by
    default, `patch` or `html`). It returns the §3 document as structured
    content, or the text.
  - **`show_to_person`, only with `--allow-show`:** opens the GUI program on a
    pair, so that the person can look at what the agent found.
    - **This is the collaboration step:** an agent narrows a question, and a
      person sees it in the window they know.
    - **It starts the program beside `forskscope-cli`,** in the same folder,
      never one found on `PATH`. It passes the two paths as separate arguments,
      with no shell, and with the user's own environment (RFC-091).
    - Its only effect is a window that opens.
- **The tool descriptions say plainly** that a compared file's content is data
  supplied by whoever wrote that file, and is not instructions.
- **Cancellation:** MCP's `notifications/cancelled` stops the comparison
  through the engine's `CancellationToken`.
- **Limits:**
  - the engine's existing size limits apply;
  - the §3 caps apply to every response;
  - one comparison runs at a time per server.
- **The protocol library:**
  - **The official Rust SDK (`rmcp`) is preferred,** with only its stdio server
    features enabled.
  - **It is acceptable only if its dependency tree passes `audit-deps` review**
    and adds no HTTP or network crate.
  - **Otherwise,** a minimal implementation of the subset used: `initialize`,
    `tools/list`, `tools/call`, `ping`, cancellation.
  - The implementer measures both, reports, and **waits for the architect's
    decision** before building either.

### 5. Collaboration in the other direction

A person's own exports are already readable by an agent:
- RFC-092's HTML report;
- the unified diff;
- **and now the same JSON,** from `forskscope-cli compare --format json`.

**The GUI gains no new control in this RFC.** A *"copy for an agent"* action
would be its own small decision, later.

### 6. Documentation

- **A new guide page, *Using ForskScope from scripts and AI agents*:**
  - the commands;
  - the exit codes;
  - the JSON format, with its schema file;
  - the MCP tools;
  - a configuration example for an MCP client;
  - what the roots mean.
- **`threat-model.md`** gains the new entry points:
  - **paths an agent supplies**, bounded by the roots;
  - **content returned to an agent**, which is untrusted data;
  - **the opt-in GUI launch.**

## Non-goals

- **Writing of any kind:** merge, applying a patch, save, copy. That is a later
  RFC with its own safety design, using `forskscope-edit`.
- **A network or HTTP server.** A background daemon.
- **Reading or writing the GUI's settings or session.**
- **Git revisions as inputs.**
- **Workbook and binary changes in detail.** Status only, as RFC-092.
- **The Microsoft Store package.** An app execution alias there is a separate
  packaging question.
- **Any change to the GUI program.**

## Acceptance criteria

1. **The program:**
   - `forskscope-cli` depends only on `forskscope-compare` and
     `forskscope-report`, enforced by RFC-093's gate;
   - **on Linux it links only the C runtime;**
   - Windows' import gate passes;
   - both sizes are reported.
2. **`compare`, for files and for folders:**
   - every format;
   - the exit codes 0, 1 and 2, each tested;
   - `--format patch` for a file is byte-identical to the GUI's *Export patch*;
   - `--format html` is RFC-092's report.
3. **The JSON:**
   - schema `forskscope.comparison/1`, with a schema file;
   - a gate validates emitted documents;
   - every status identifier maps to the app's word, and a test covers every
     `RecStatus`;
   - the caps list what they omit.
4. **`mcp`:**
   - an MCP client completes `initialize`, `tools/list` and `tools/call` on
     `compare`;
   - **a path outside the roots is refused,** including through a symbolic
     link and through `..`;
   - cancellation stops a long folder comparison;
   - `show_to_person` is absent without `--allow-show`, and opens the GUI on
     the pair with it.
5. **No network:** under `strace -f -e trace=network` on Linux, neither mode
   opens a socket.
   *Clarified 2026-10-21 (architect):* `strace` is not installed on the
   project's Linux machine. An equivalent observer is accepted if it can fail:
   for example, an `LD_PRELOAD` library logging every `socket` and `connect`
   call, shown catching a deliberate socket in a test build.
   *Clarified (review 183):* the observer is backed by a structural check:
   `crate-boundaries` fails on `std::net` or `std::os::*::net` in
   `forskscope-compare`, `forskscope-report` and `forskscope-cli`.
6. **No writes:** a run of each mode, against a home folder overlaid as
   writable, leaves the overlay empty. This is the RFC-091 method.
7. **The guide page and the threat model** describe exactly the behaviour
   above.

## Open questions

None. The protocol library is decided by measurement, under §4.
