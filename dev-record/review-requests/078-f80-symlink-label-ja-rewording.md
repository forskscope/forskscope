# Review Request 078: F80 — reword the Japanese Symlink-not-followed label

**Governing.** Review 078 §3 (request 076) raised a question for the owner about
`"Symlink not followed"`'s Japanese translation. Owner decision: reword it.
**Baseline.** `main` at `c2a6a0b`
**Commit.** `f4a61a3`

## What changed

`crates/forskscope-ui/src/i18n.rs`, one line:

```
- "Symlink not followed" => "シンボリックリンク（未追跡）",
+ "Symlink not followed" => "シンボリックリンク（リンク先は未比較）",
```

**Why.** 未追跡 commonly reads as git's "untracked," which risks the label sounding
like a VCS state rather than "we did not follow the link" — exactly the
misreading review 078 §3 flagged. Reworded to drop 追跡 entirely and reuse 比較,
the word `NotCompared`'s existing label already uses
(`ディレクトリの中身は比較されていません`), so the app's two "nothing was
examined" labels share vocabulary instead of one coincidentally resembling an
unrelated concept.

**No English change, no new key, no test change.** `symlink_label_says_not_followed_not_a_verdict`
is scoped to `Lang::En` specifically so a Japanese-only rewording can't false-fail
it (review 078 §2 called this out as the reason the test was written that way).

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` (unchanged: core 697, ui-logic 257, ui 76),
`cargo xtask css --check`, `cargo xtask i18n` (236 keys, unchanged),
`cargo xtask rfc-sync`, `git diff --check`. Pushed as `f4a61a3`; CI green,
including confirmation that F83's RFC sync check now actually executes in the
pipeline (the `version-sync` blocker from request 077 was fixed separately by
the architect in `c0cdcd8`).
