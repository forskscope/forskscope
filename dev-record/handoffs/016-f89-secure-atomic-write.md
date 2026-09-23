# Developer Handoff 016 — F89: every save must use an unpredictable temp file

**From:** architect
**Date:** 2026-09-01
**Register:** F89 (High, security). **Governing: RFC-082 §D5** (accepted).
**Gate:** Release-blocking (audit blocker B5).

---

## 1. Task title

Replace `atomic_replace`'s hand-rolled temp file with the `tempfile` primitive
already used 80 lines below it.

## 2. Purpose

```rust
pub(crate) fn atomic_replace(target: &Path, bytes: &[u8]) -> Result<()> {
    let temp = temp_path_for(target);          // .{filename}.fsk-tmp — predictable
    fs::write(&temp, bytes)                    // follows symlinks, no O_EXCL
```

**Two defects, both reproduced.**

**Symlink following, CWE-59 / CWE-378.** The architect pre-created
`.doc.txt.fsk-tmp` as a symlink to an unrelated file:

```
victim.txt now = Ok("user's new content\n")   ← unrelated file overwritten
doc.txt is_symlink = true
doc.txt -> "/tmp/.../victim.txt"              ← the user's document became a link
```

**Collision.** Two concurrent saves of one target — two tabs, or two ForskScope
instances — write the same path and both rename. Interleaved writes can produce a
corrupt temp that is then renamed onto the target, defeating this module's own
headline promise that a reader sees the old file or the new one, never a partial
write.

**The blast radius is wider than document saves.** `atomic_replace` has two
callers: `save_text` (`save.rs:89`) and `atomic_write_envelope`
(`persist/schema/repository.rs:140`) — **every settings and session write**.

## 3. Required implementation — RFC-082 §D5

**Use the primitive already in the file.** `persist_noclobber` does this
correctly; copy its shape, including the permissions reasoning its comment
records:

```rust
let mut builder = tempfile::Builder::new();
#[cfg(unix)]
{ builder.permissions(fs::Permissions::from_mode(0o666)); }
let mut tmp = builder.tempfile_in(dir)?;
std::io::Write::write_all(&mut tmp, bytes)?;
tmp.persist(target)?;
```

Random `O_EXCL` name; no symlink vector; no collision. `tempfile` is already a
`forskscope-core` dependency.

**`persist`, not `persist_noclobber`.** `atomic_replace` overwrites by contract —
that is its whole job. Using the no-clobber variant would break every save to an
existing file.

### 3a. Do not copy the parent-directory creation — the contracts differ

`persist_noclobber` begins by `create_dir_all`-ing the target's parent.
**`atomic_replace` must not**, and this is the trap in "copy its shape".

Its caller `atomic_write_envelope` creates the parent itself, deliberately —
F61/F62 record that this is the *first* write a fresh install makes and that
every first write failed silently until the caller started creating
`~/.config/forskscope`. Moving that into `atomic_replace` would make the caller's
line redundant **and** change document-save semantics: saving into a
non-existent directory would start succeeding instead of failing.

Copy the temp-file handling. Nothing else.

### 3b. Do not leak the random temp name into errors

The current code maps a write failure to the **temp** path
(`CoreError::io(&temp, …)`). With a predictable name that was merely unhelpful;
with a random one it is noise a user cannot act on — `.tmpA7f3Kq` means nothing.

Follow `persist_noclobber`: a `tempfile_in` failure reports the **directory**, a
write failure reports the **target**. A `persist` failure reports the target with
`IoOperation::Rename`.

## 4. Explicit non-change scope

- **`persist_noclobber` itself** — already correct; do not refactor the two into
  a shared helper. Their contracts differ (§3a) and merging them is how the
  difference gets lost.
- **The `.bak` copy** (`save.rs:70-77`) — it clobbers an existing backup and
  follows symlinks. That is a separate audit finding, not this handoff.
- **`temp_path_for`** — delete it if nothing else uses it; check first.
- F87/F88 (handoff 017), `ROADMAP.md`, the RFCs.

## 5. Required tests

1. **The symlink attack fails.** Create `doc.txt` and `victim.txt`; pre-create
   `.doc.txt.fsk-tmp` as a symlink to `victim.txt`; call `atomic_replace` on
   `doc.txt`. Assert **`victim.txt` is unchanged** and **`doc.txt` is a regular
   file** with the new content.
   **Falsify by restoring `fs::write(temp_path_for(target), …)`** — the attack
   must succeed and the test must fail.

   **`#[cfg(unix)]`** for `symlink`. **No root-skip is needed here** — unlike
   handoff 006's permission test, this one creates its own symlink and root
   changes nothing about link following. Do not add a skip you do not need.

2. **An ordinary overwrite still works**, and the file's permissions are what a
   plain `fs::write` would have produced — that is what `persist_noclobber`'s
   `0o666`-plus-umask comment exists to preserve. A save that lands `0600`
   because `NamedTempFile` defaults there is a regression a test should catch.

3. **Both callers still work**: a document save, and a settings write into a
   **non-existent** parent directory — the F61/F62 case §3a protects. That second
   one must still succeed, via the caller's own `create_dir_all`.

## 6. Acceptance criteria

- Restoring the `fs::write` temp path fails a test.
- No predictable temp path is written anywhere in the save path.
- Saved files keep umask-derived permissions, not `0600`.
- A settings write into a missing directory still succeeds.
- `persist_noclobber` is untouched.
- Gates green: `fmt`, `clippy --workspace --all-targets -- -D warnings`,
  `test --workspace`, `xtask css --check`, `xtask version-sync`, `xtask i18n`,
  `xtask rfc-sync`, `git diff --check`.

## 7. Prohibited shortcuts

- **Do not add `create_dir_all` to `atomic_replace`.** §3a.
- **Do not use `persist_noclobber` for the overwrite path.**
- **Do not merge the two functions.**
- **Do not add a root-skip to test 1.** It is not needed and it would hide a real
  failure.
- **Do not report a falsification you did not run.**

## 8. Required review-request format

Lead with test 1's falsification and its output — the attack succeeding against
the old code, then failing against the new.

State plainly:
- the permissions a saved file ends up with, and how you checked;
- that `atomic_replace` still does not create parent directories;
- whether `temp_path_for` survives, and why.
