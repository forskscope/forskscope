# Git Integration

ForskScope is compatible with `git difftool` and `git mergetool` without
any plugin or wrapper — the CLI interface matches what git expects.

## difftool (view differences)

Configure git to open ForskScope for file diffs:

```sh
git config --global diff.tool forskscope
git config --global difftool.forskscope.cmd 'forskscope "$LOCAL" "$REMOTE"'
git config --global difftool.prompt false
```

Then use it:

```sh
git difftool HEAD -- path/to/file.rs
git difftool main..feature
```

Git passes the two versions of the file as `$LOCAL` (old) and `$REMOTE`
(new). ForskScope opens them side by side in the diff workspace.

## mergetool (resolve conflicts)

Configure git to use ForskScope for conflict resolution:

```sh
git config --global merge.tool forskscope
git config --global mergetool.forskscope.cmd \
  'forskscope "$LOCAL" "$REMOTE" "$MERGED"'
git config --global mergetool.keepBackup false
```

Then resolve a conflict:

```sh
git mergetool path/to/conflicted.rs
```

Git passes three paths:

| Variable | Meaning |
|----------|---------|
| `$LOCAL` | Your version (current branch) — shown as **left/old** |
| `$REMOTE` | Their version (branch being merged) — shown as **right/new** |
| `$MERGED` | File that git expects to contain the final result |

ForskScope opens `$LOCAL` vs `$REMOTE` for comparison. When you **Save**,
the merged result is written to `$MERGED` (not to `$REMOTE`). After
ForskScope closes, git marks the conflict as resolved if `$MERGED` was
written successfully.

## JJ (Jujutsu)

JJ also supports external diff and merge tools via its configuration.
The same `<left> <right>` interface works for `jj diff --tool`:

```sh
jj config set --user ui.diff.tool '["forskscope", "$left", "$right"]'
```

For merge (when JJ conflict resolution is configured):

```sh
jj config set --user ui.merge-tool '["forskscope", "$left", "$right", "$output"]'
```

## Tips

- ForskScope opens immediately and stays open until you close it. Git
  waits for the process to exit before marking the file as resolved.
- If you close ForskScope **without saving** in mergetool mode, git will
  ask whether to keep or discard the conflict markers.
- The `difftool.prompt false` setting prevents git from asking
  confirmation before opening ForskScope for each file.
