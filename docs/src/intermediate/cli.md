# CLI Reference

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
the resolved file. The tab title carries a "(merge)" suffix as a reminder.

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
| Non-zero | Startup error (e.g. path not found) |

The exit code does not indicate whether changes were saved — git determines that
by inspecting whether `$MERGED` was written.

### Print platform diagnostics

```sh
forskscope --diagnostics
```

Prints platform information (OS, architecture, CPU count, app version, Rust
version, config directory) and exits without launching the UI. Useful for
diagnosing startup failures and for including in bug reports.

Example output:

```
ForskScope 0.112.0
OS:       linux
Arch:     x86_64
CPUs:     8
Rust:     1.85.0
Home:     ***
Config:   /home/user/.config/forskscope
```

The home directory is redacted (`***`) for privacy when copying into bug reports.
