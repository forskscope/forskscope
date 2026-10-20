# RFC 093: Crate Boundaries — a Read-Only Engine, Its Outputs, and Everything That Writes

**Status.** Proposed.
**Scheduling.** Before RFC-094, on `main`, with no release of its own. It ships
inside the release that carries RFC-094. See `ROADMAP.md` § "Remaining proposed
RFCs".
**Tracks.** The owner's questions, 2026-10-20:
- are the crate boundaries reasonable, and ready for extension?
- should `forskscope-core` be split, for example into a compare crate and a
  report crate, and should `forskscope-ui-logic`?

**Amends.** None. It changes no behaviour.
**Touches.**
- every module of `crates/forskscope-core/` (moved, none rewritten);
- `crates/forskscope-ui-logic/` (receives four view-model modules);
- the workspace `Cargo.toml`;
- `xtask` (a new `crate-boundaries` gate; `audit-deps`, `ui-logic-connectivity`
  and `ui-logic-docs` re-pointed);
- `docs/src/maintainers/architecture.md` and `testing.md`.

## Summary

`forskscope-core` holds three different things. A program for AI agents
(RFC-094) must link only the first:

1. a **read-only comparison engine**;
2. the **outputs** that describe a comparison;
3. **everything that writes or holds the desktop app's own state**.

This RFC splits `forskscope-core` into four crates **by what each may do**, not
by size:

- **`forskscope-compare`:** reads, and never writes;
- **`forskscope-report`:** turns a comparison into text;
- **`forskscope-edit`:** the only crate that writes the user's files;
- **`forskscope-desktop`:** the app's own settings, session and system
  integration.

Four view-model modules move to `forskscope-ui-logic`. **Nothing changes for
users.** A new gate keeps the boundaries from eroding.

## Motivation

### What core holds today

Measured on `72bff1c`. Line counts exclude tests.

| Kind | Modules | What they need |
|---|---|---|
| **Comparison, read-only** | `document`, `encoding`, `file_kind`, `diff`, `dir` (listing, recursive diff, digests), `xlsx`, `ignore`, `job`, `cancel`, `error`, `path`, `fnv`, `compare_prep` (the compare half) | `similar`, `encoding_rs`, `chardetng`; **`sheets-diff` → `calamine` → `zip`, `quick-xml`**, the heaviest tree, reached only from `xlsx` |
| **Outputs** | `report` (2,317 lines), `patch` (884), `checksum` | `sha2` |
| **Writing the user's files** | `merge` (1,369), `edit_op`, `save`, `dir`'s batch copy and plan/execute, `compare_prep`'s save target | `tempfile` |
| **The desktop app's own state and system integration** | `persist` (1,567), `settings`, `external_tool`, `vcs`, `watcher`, `platform`, `env_passthrough` | `serde_json`, process spawning |
| **GUI view-models** | `command`, `diff_decoration`, `line_map`, `conflict_nav` | nothing |

### Why split, and why by these lines

**Not for binary size.** The Rust linker already drops code a program never
calls, so moving modules between crates does not shrink a binary. **What sets a
program's size is its dependency tree:**
- the GUI's GTK and WebKitGTK, or WebView2;
- the workbook reader's `calamine`, `zip` and `quick-xml`.

**A crate boundary is worth having for three reasons:**

1. **A guarantee the compiler enforces.**
   - If `forskscope-compare` and `forskscope-report` cannot write files, a
     program that links only them cannot write the user's files. No runtime
     flag decides that.
   - RFC-094's agent server is read-only **because the code that writes is not
     in it**, not because an option is off.
   - This is the owner's *"safe and secure"* made structural.
2. **A dependency direction that cannot be reversed by accident.**
   - The engine must not know about report formats, editing or the GUI.
   - Today one crate holds all of these, so nothing stops an edge in the
     wrong direction.
3. **A heavy dependency behind a feature.** The workbook stack becomes the
   `spreadsheet` feature of `forskscope-compare`, on by default.
   - A consumer that never compares workbooks can leave it out.
   - The engine can be built and tested without it.

### Why not finer

The owner's concern is bloat, and that applies to crates too.

- **Every crate has a cost:** a manifest, a public API surface, cross-crate
  visibility, and a gate's attention.
- **A split earns its place only if it changes** what a program may do, what
  it depends on, or which way dependencies point.
- **Splitting by topic alone does none of these.** A `forskscope-diff` apart
  from `forskscope-folder` is an example.
- So the engine stays one crate, and so do the outputs.

### No separate data crate (IR, object model or store), for now

**Each candidate is weighed against the rule above:** a boundary pays only if
some program or crate can then **depend on one side without the other**, and that
absence is a guarantee or a real dependency cut.

- **A shared model or intermediate representation** (`forskscope-ir`,
  `forskscope-model`) would hold `DiffDocument`, `RecEntry`, `RecStatus`,
  `LoadedDocument` and the options. **Today every crate that reads these types
  also needs the engine that produces them:** report, edit, the GUI and RFC-094's
  program. No consumer would link the types without the engine, so the boundary
  would guarantee nothing.
  - **The data-boundary concern that is real is different.** An *external*
    contract must not be the engine's internal types. If it were, an internal
    refactor would silently break every script and agent.
  - **This project already keeps the two apart.** The stored settings and session
    use their own `Persisted*` types, never the domain types, and no engine
    result type derives `Serialize`. RFC-094's JSON follows the same rule, with
    its own contract types in `forskscope-report` (RFC-094 §3).
- **A store crate** (`forskscope-store`) would hold settings and session
  persistence apart from Git, external tools and file watching. **Only the GUI
  links either half,** so splitting them guarantees nothing for any program.
  Persistence stays in `forskscope-desktop`.
- **When to revisit, and the trigger is concrete:** create a model crate when a
  consumer must read comparison data **without** linking the engine. Two
  examples:
  - a published client library for agents;
  - a frontend that talks to the engine across a process boundary.
  
  Then move RFC-094's contract types into it, not the engine's internal types.

### `forskscope-ui-logic`: not split

- It has **one consumer, the GUI**.
- It is **pure**: no I/O, no heavy dependency.
- It is about 6,200 lines.

None of the three reasons applies. Instead it **receives** the four GUI
view-models that sit in core today. Agents and scripts have no use for a gutter
symbol or a command palette.

**When to revisit:** split it when a second frontend needs only part of it, or
when part of it gains a dependency the rest must not carry.

## Design

### 1. The crates, and what each may do

| Crate | Owns | May write files | May spawn processes | Depends on |
|---|---|---|---|---|
| `forskscope-compare` | loading, decoding and classifying documents, with fingerprints; line diff and the comparison profile; folder listing, recursive comparison and digests (`EqualityEvidence`, `RecStatus`); workbook comparison (feature `spreadsheet`); ignore rules; jobs and cancellation; errors | **No** | No | — |
| `forskscope-report` | unified diff (`patch`); Markdown, JSON and HTML reports; SHA-256 | **No**. It returns text and bytes, and the caller writes them | No | compare |
| `forskscope-edit` | merge sessions, three-way included; edit operations; atomic save with fingerprint preconditions; folder operations (batch copy, plan and execute, manifests); the save-target half of `compare_prep` | **Yes, and the only crate that writes the user's files** | No | compare |
| `forskscope-desktop` | settings and session persistence; settings display types; external tools; Git; file watching; platform detection; environment pass-through | **Only its own settings and session** | Yes: external tools, Git, the system opener | compare, edit |
| `forskscope-ui-logic` | GUI view-models, now including `command`, `diff_decoration`, `line_map` and `conflict_nav` | No | No | compare, report, edit, desktop |
| `forskscope-ui` | the GUI program `forskscope` | — | — | all of the above |

**`forskscope-core` ceases to exist.** No crate is published, so the name has no
outside user.

**Three couplings must be cut along the way:**
- **`report` reads merge history** (`TransactionLog`) for a file report's
  history section. `forskscope-report` defines its own small input type for
  that section, and the GUI fills it from `forskscope-edit`. That keeps `report`
  free of `edit`.
- **`dir` mixes reading and writing.** Listing, recursive comparison and
  digests go to `compare`. Batch copy, plan and execute, and manifests go to
  `edit`.
- **`compare_prep`** splits along the same line: what is compared goes to
  `compare`, and what is saved to goes to `edit`.

### 2. A gate that keeps it so: `cargo xtask crate-boundaries`

It fails, naming the file and line, when:

1. **`forskscope-compare` or `forskscope-report` writes a file.** Outside
   `#[cfg(test)]`, it reports any of these:
   - `std::fs::write`, `create_dir`, `create_dir_all`, `remove_file`,
     `remove_dir`, `remove_dir_all`, `rename`, `copy`, `set_permissions`;
   - `File::create`, `OpenOptions` with `.write(true)`, `.append(true)` or
     `.create(true)`;
   - any `tempfile` use.
2. **Either crate spawns a process** (`std::process::Command`).
3. **The dependency graph has an edge not in §1's table.** The workspace's own
   crates are read from `cargo metadata`.

**Each rule is falsified on a temporary commit**, and the request cites it.

### 3. The move itself

- **Mechanical, in this order,** each step green before the next:
  1. create `forskscope-compare` and move its modules;
  2. `forskscope-report`;
  3. `forskscope-edit`;
  4. `forskscope-desktop`;
  5. move the view-models into `forskscope-ui-logic`;
  6. remove `forskscope-core`.
- **Tests move with their modules.** The workspace's test count is the same
  before and after, apart from the gate's own new tests. Report both numbers.
- **Public paths change** (`forskscope_core::diff::…` becomes
  `forskscope_compare::diff::…`). There are **no re-export shims**: a shim would
  keep the old grab-bag alive under a new name.
- **`.xlsx` fixtures, `audit-deps`'s expected dependents, and `ui-logic-docs`'
  tables** follow the modules.

### 4. What does not change

- **Behaviour:** the GUI is the same program.
- **The GUI binary:**
  - its size stays within 1 %;
  - on Linux it links the same 16 libraries;
  - on Windows it passes the same import gate.

  Report the size before and after.

## Non-goals

- Any behaviour change.
- Publishing crates to crates.io.
- Splitting `forskscope-ui-logic`.
- Splitting the engine or the outputs further.
- Anything RFC-094 adds.

## Acceptance criteria

1. **The crates and modules** are as §1, and `forskscope-core` is gone.
2. **`cargo xtask crate-boundaries`** passes in CI. Each of its three rules is
   shown failing on a temporary commit.
3. **The test count** is unchanged apart from the gate's own tests, and every
   test passes.
4. **`forskscope-compare` builds and tests without the `spreadsheet` feature.**
5. **The GUI binary's size and linked libraries** are unchanged, as §4.
6. **`architecture.md`, `testing.md`** and every gate that names a crate
   describe the new layout.

## Open questions

None.
