# Using ForskScope from scripts and AI agents

**`forskscope-cli`** is a second, separate program (RFC-094) built to be
driven by a script or an AI agent rather than a person at a window. It
ships beside the GUI program (`forskscope`) in every release, and has
four fixed properties:

- **Read-only by construction** — it is not built from the code that
  writes, so it cannot save, merge, or apply a patch.
- **No network** — it opens no socket and listens on no port.
- **Stateless** — it reads no settings and writes no session; every run
  starts from nothing and leaves nothing behind.
- **Small** — no GTK, no WebKitGTK, no WebView2. It starts in a plain
  server or CI container with no desktop stack installed.

Today it has one mode, `compare`. A second mode, an MCP server for AI
agents (`forskscope-cli mcp`), is designed in RFC-094 §4 but not yet
built — this page will grow a section for it once it exists; nothing
below promises it early.

**Where to find it:** the Linux `.tar.gz` and the Windows `.zip` carry
`forskscope-cli` beside the GUI program — run it directly, or copy it
somewhere on `PATH`. The AUR package installs it at `/usr/bin/forskscope-cli`.
On macOS it sits inside the app bundle, at
`ForskScope.app/Contents/MacOS/forskscope-cli` — to use it from a
terminal or a script, either call it by that full path, or put a symlink
to it on your `PATH` (for example, `ln -s
"/Applications/ForskScope.app/Contents/MacOS/forskscope-cli"
/usr/local/bin/forskscope-cli`).

---

## `compare`

```sh
forskscope-cli compare <left> <right> [options]
```

Compares two files, or two folders — decided from the paths, not from a
flag. Comparing a file against a folder is refused (exit code `2`).

### Output formats

`--format text|json|patch|html`, printed to **standard output only** —
`forskscope-cli` never creates a file itself. Redirect if you want one:

```sh
forskscope-cli compare old.txt new.txt --format html > report.html
```

- **`text`** (the default): a short Markdown-flavored summary — meant for
  a person reading a terminal, not for a script to parse. Use `json` for
  that.
- **`json`**: the versioned `forskscope.comparison/1` document, below.
- **`patch`**: a unified diff, compatible with `patch -p1` and
  `git apply`. For a file comparison this is byte-for-byte what the GUI's
  own **Export patch** would write for the same two files and the same
  `--context`. Nothing is printed when the two sides are identical — there
  is nothing to patch.
- **`html`**: the same self-contained HTML report the GUI's **Export
  report…** produces (see [Exporting a comparison](./patch-export.md)).

### Options

- **The comparison profile:** `--algorithm myers|patience|lcs|histogram`,
  `--ignore-whitespace`, `--ignore-case`, `--ignore-newlines`, `--context
  <n>` (unified-diff context lines, default `3`). These match the GUI's
  own defaults when omitted.
- **For a folder comparison, the ignore rules:** `--ignore-ext
  <a,b,...>`, `--ignore-dir <a,b,...>`, `--hide-dotfiles` — the same rules
  the GUI's own Settings offer, applied to every output format alike
  (including `patch`).
- **`--paths names|relative|full`:** how a path appears in the output.
  File names only (`names`) by default — never a full path unless asked
  for.
- **For a folder comparison's JSON, the caps:** `--max-files <n>`
  (default `50`) and `--max-change-bytes <n>` (default `5242880`, 5 MiB)
  bound how many changed files carry full line-by-line content before the
  rest are listed, by path, in the JSON document's `omitted` array — never
  silently. A binary or workbook (`.xlsx`) file inside a folder, or
  compared directly as the two top-level paths, reports its status only
  (`identical`/`different`), never a detailed diff — the same rule the
  GUI's own folder reports and reports already follow.

### Exit codes

Follows `diff`:

| Code | Meaning |
|---|---|
| `0` | The two sides are the same. |
| `1` | They differ. |
| `2` | An error or a refusal — one line naming the path and the reason is printed to standard error. |

A missing path, or a file compared against a folder, is exit code `2`,
not a comparison result.

---

## The JSON format

Every document starts `{"schema": "forskscope.comparison/1", "kind":
"file"|"directory", ...}`. The schema name is versioned on purpose:
adding a field keeps `/1`; removing a field, or changing what one means,
moves to `/2` with a new schema file. The full shape is a JSON Schema
file in the repository, checked against what `forskscope-cli` actually
prints by `cargo xtask json-schema`:

```text
docs/schemas/forskscope.comparison.v1.json
```

**A file comparison** (`kind: "file"`) carries each side's name, byte
size, SHA-256 and text encoding; whether the two are `identical`; counts
of changed hunks and added/removed lines; the changes themselves (old and
new line numbers, and whether each line is `context`, `removed` or
`added`); and `notes` for anything the comparison itself knows that the
rest of the document does not carry on its own — a size limit that
changed what is shown, a non-UTF-8 encoding, a side that could not be
decoded.

**A folder comparison** (`kind: "directory"`) carries entry counts by
status, every entry's path/status/sizes/SHA-256, the changes of each
differing text file (subject to the two caps above), and the `omitted`
list those caps produce. Every entry's `status` is one of nine stable,
machine-readable identifiers — `identical`, `different`, `left_only`,
`right_only`, `unreadable`, `not_compared`, `symlink_not_followed`,
`left_only_empty_folder`, `right_only_empty_folder` — each one naming
exactly the same thing the GUI's own Deep Compare view and reports show
with a word, never a different, looser-defined status an agent would
have to guess the meaning of. `identical` is never reported without the
two sides actually having been compared — a folder entry whose content
was never read says `not_compared`, not `identical`.

---

## Collaborating with a person

An agent's own findings are already something a person can open:
`forskscope-cli compare --format html` is the same report the GUI's
**Export report…** produces, and `--format patch` is the same unified
diff **Export patch…** produces. Handing either file to a person, or
pointing them at it, needs nothing more from ForskScope today.
