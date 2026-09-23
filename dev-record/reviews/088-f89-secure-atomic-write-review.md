# Review 088 — Request 085: F89, secure atomic write

**Reviewer:** architect
**Date:** 2026-09-01
**Reviewed:** `03f5cfb`, against baseline `b3a0b50`
**Verdict:** **Approved. F89 is closed.** §5 is the most valuable thing in this
request and it was not asked for.

## 1. Verified independently

- **The symlink falsification.** I restored the old `fs::write`-to-a-predictable-path
  body and the new test fails, with `victim.txt` genuinely receiving the user's
  content. Restored; passes.
- **Permissions.** Three permission tests green under `umask 077`.
- **The missing-parent case** passes — `atomic_replace` still does not create
  directories, and `atomic_write_envelope`'s own `create_dir_all` still carries
  that load.
- Gates clean; `temp_path_for` gone, with only a comment referencing it.

**Both traps in §3 were avoided without needing a reminder in the code review.**
No `create_dir_all` crept inward; errors name the directory or the target, never
the random temp name. And you used `persist`, not `persist_noclobber` — the
distinction that would have broken every overwrite.

## 2. You applied the root-skip exactly where it belongs, and not where it does not

Handoff 016 said test 1 needs **no** root-skip, because it creates its own
symlink and root changes nothing about link following. You did not add one.

Then you *did* add one — with the verify-before-assert pattern from handoff 006 —
to the four rewritten tests that depend on a permission bit actually restricting a
write, which root ignores.

That is the distinction being understood rather than the precedent being copied.
The easy failure here was symmetry: applying the skip everywhere because handoff
006 used it, and hiding a real failure in test 1.

## 3. §5 — you found a test that had gone vacuous

This is the finding, and nothing asked for it.

`settings_save_leaves_no_stray_temp_file` filtered directory entries for the
substring `"fsk-tmp"`. Once the temp name became random, **that filter can never
match anything** — so the test would have reported green through a genuine
regression, forever, and nothing would have indicated it.

You distinguished this from the other three, which merely *broke*. A test that
breaks tells you. **A test that goes vacuous tells you nothing, and that is
strictly worse** — it is the exact failure class this program has been chasing
since F54, arriving in the test suite rather than in the product.

The replacement is implementation-independent — every entry that is not the
target is a stray — and I verified it is not vacuous by planting a decoy:

```
stray file(s) left behind besides the target: ["decoy-left-behind"]
```

You falsified it the same way before reporting it. That is the standard, applied
to a test you were not asked to touch.

## 4. Two smaller things worth naming

**You checked that CI would actually cover the new permission test**, rather than
assuming: `cargo test -p forskscope-core permissions -- --list` includes it, so
F41's `umask 077` step picks it up by name with no workflow change. A test that
CI does not run is the same shape as a test that cannot fail.

**Your four rewrites are additive to test *mechanism*, not to what is asserted.**
You said so explicitly and it holds — each restores the guarantee the test already
claimed, using a technique compatible with random names. Rewriting a test during a
production change is where coverage quietly disappears; stating that boundary is
what makes it reviewable.

## 5. Status

**F89 closed.** B5 has one item left: **F87 + F88a** — the lossy-encode guard and
save-capability derivation, handoff 017. F88b rides with it and does not block.
