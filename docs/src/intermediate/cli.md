# CLI Reference

This page covers the GUI program's own command line (`forskscope`). For a
separate, headless program built for scripts and AI agents — no GTK, no
window, prints to standard output only — see
[Using ForskScope from scripts and AI agents](./scripts-and-agents.md).

## Startup modes

### Open the explorer workspace

```sh
forskscope
```

Opens the two-pane directory explorer. The last-used left and right directories are
restored from the previous session.

### Compare two files

```sh
forskscope <left> <right>
```

Opens a diff tab comparing `left` (old/source) and `right` (new/result) directly,
bypassing the explorer. This is the form used by `git difftool`.

### Git mergetool mode

```sh
forskscope <local> <remote> <merged>
```

Opens a diff of `local` (your branch) vs `remote` (the other branch). When you
**Save**, the result is written to `<merged>` — the path git expects to contain
the resolved file, never to `remote`. The tab title carries a "(merge)"
suffix, and a quiet `Result: <merged>` line under the file header names the
save destination throughout — it is not a button; the normal save/overwrite
guards are the only path to writing there.

### `--data-dir <path>` — portable mode

```sh
forskscope --data-dir /path/to/folder
forskscope --data-dir /path/to/folder <left> <right>
```

Keeps settings, session and everything else ForskScope writes inside
`<path>` instead of the platform's usual locations — nothing is written to
your home folder. The folder is created if it does not exist. Combines
with the normal startup forms: `--data-dir` is parsed out of the argument
list before the file arguments are, the same way `--version` is, so
`forskscope --data-dir /path/to/folder a.txt b.txt` still opens a
comparison. See [Running portably](../users/installation.md#running-portably)
for what is and is not moved, and for the alternative — a `forskscope-data`
folder kept beside the program — that needs no flag at all.

### `--private` — private mode

```sh
forskscope --private
forskscope --private <left> <right>
```

Starts with default settings, and saves nothing: no settings file is read
or written, no session is read or written, and nothing survives after
ForskScope exits. The files you compare and save are still your files,
written when you save — private mode is about ForskScope's own records,
not about the documents you open. See
[Running portably](../users/installation.md#running-portably) for the full
list of what each mode writes and does not.

`--private` and `--data-dir` cannot be combined (exit code `2`, above).

**Windows and Linux only.** On macOS, the system web view keeps its own
cache in your home folder regardless of either mode, so neither can keep
its promise there. Both refuse to start on macOS: one plain message to
stderr, then exit code `2` (above) — the same code the two flags'
contradiction uses, since both are "this cannot be satisfied," not two
different kinds of failure. `--help` does not list `--data-dir`/`--private`
on macOS, rather than listing them with a caveat.

---

## Git integration

### Configure as difftool

```ini
# ~/.gitconfig
[diff]
    tool = forskscope

[difftool "forskscope"]
    cmd = forskscope "$LOCAL" "$REMOTE"

[difftool]
    prompt = false
```

```sh
git difftool HEAD -- path/to/file.rs
git difftool main..feature
```

### Configure as mergetool

```ini
[merge]
    tool = forskscope

[mergetool "forskscope"]
    cmd = forskscope "$LOCAL" "$REMOTE" "$MERGED"
```

```sh
git mergetool path/to/conflicted.rs
```

Git passes three paths to the command:

| Variable   | Meaning |
|-----------|---------|
| `$LOCAL`  | Your version (current branch) — shown as **left/old** |
| `$REMOTE` | Their version (branch being merged) — shown as **right/new** |
| `$MERGED` | The file git expects to contain the final resolution |

After ForskScope writes to `$MERGED` and exits, git marks the conflict resolved.

### JJ (Jujutsu)

```toml
# ~/.config/jj/config.toml
[ui]
diff.tool = ["forskscope", "$left", "$right"]
merge-tool = ["forskscope", "$left", "$right", "$output"]
```

---

## Exit codes

| Code | Meaning |
|------|---------|
| `0`  | Normal exit |
| `1`  | Unsupported argument count — anything other than 0, 2, or 3 positional arguments (`--diagnostics`, `--version`/`-V` and `--help`/`-h` are all handled separately, before this check). An error naming the argument count is printed to stderr, along with a pointer to `--version`/`--diagnostics` for the question that error most often turns out to be. The same code is used for an unrecognized `--` flag and for `--data-dir` with no path after it — both are startup-argument errors, not a file name ForskScope tried and failed to open. |
| `2`  | Either of two things `--private`/`--data-dir` cannot resolve: the two combined — a contradiction, not a priority to resolve by picking one, since `--private` starts from nothing and keeps nothing while `--data-dir` names a folder to keep everything in (the error names both flags) — or, **on macOS,** either flag used at all, because the system web view there keeps its own cache in your home folder regardless of the mode (the error names the one flag used). |
| `3`  | **Windows only.** The Microsoft Edge WebView2 Runtime is not installed. ForskScope shows a message box offering the download page and exits with `3` whichever button is pressed. |

A missing file path is **not** a startup error: one side of a comparison may
be absent (e.g. a newly-added or deleted file), so `forskscope <left> <right>`
opens normally with the missing side shown as empty — and saving can create
it, restoring a deleted file. The exit code does not indicate whether changes
were saved, either — git determines that by inspecting whether `$MERGED` was
written.

### Print the version

```sh
forskscope --version
forskscope -V
```

Prints `forskscope <version>` and exits without launching the UI — the quick
answer when `--diagnostics`' full report is more than you need (issue #146:
the version is also shown in the app itself, under **Settings → ℹ About**).

### Print usage

```sh
forskscope --help
forskscope -h
```

Prints the startup modes above and exits without launching the UI.

### Print platform diagnostics

```sh
forskscope --diagnostics
```

Prints platform information (OS, architecture, CPU count, app version, Rust
version, config directory) and exits without launching the UI. Useful for
diagnosing startup failures and for including in bug reports.

Example output (Linux, 0.171.1):

```
ForskScope 0.171.1
Rust: unknown
OS: linux (unix)
Arch: x86_64
CPUs: 32
Home: /home/***
Config: ~/.config (default)
```

The home directory's user name is redacted (`***`) for privacy when copying
into bug reports. `Rust:` currently reads `unknown` in release builds.

On Windows one more line follows, reporting the WebView2 Runtime the window is
drawn with (from a release build on a CI runner):

```
WebView2 runtime: 152.0.4191.66
```

or, when it is missing, `WebView2 runtime: not found (<reason>)`.
