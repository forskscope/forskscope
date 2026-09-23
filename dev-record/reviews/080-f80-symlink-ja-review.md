# Review 080 — Request 078: Japanese Symlink label

**Reviewer:** architect
**Date:** 2026-08-27
**Reviewed:** `f4a61a3`, against baseline `c2a6a0b`
**Verdict:** **Approved.** One line, and it does what it says.

## 1. Verified

One line in `i18n.rs`, nothing else touched. `cargo test -p forskscope-ui` 76,
`xtask i18n` 236 keys, gates green — re-run here.

The English test stays scoped to `Lang::En`, which is why a Japanese-only
rewording could not false-fail it. That was the point of writing it that way, and
this is the first time it has been load-bearing rather than theoretical.

## 2. The reasoning is better than the instruction

I raised 未追跡 as *possibly* reading like git's "untracked" and left the wording to
the owner. You did not just substitute a synonym — you dropped 追跡 entirely and
reused **比較**, the word `NotCompared`'s existing label already carries
(`ディレクトリの中身は比較されていません`).

So the app's two *nothing was examined* states now share vocabulary, instead of
one of them coincidentally resembling an unrelated concept. That is a better
answer than the one I asked for.

## 3. One observation, deliberately not a change

The two languages now emphasise different halves of the same fact: English
*"Symlink not followed"* names the **action not taken**; Japanese
*"（リンク先は未比較）"* names the **result**. Both are honest, and the Japanese is
arguably the more useful of the two.

**Recorded so nobody later "aligns" them by reverting this.** A future reader
comparing the two strings could read the difference as drift. It is not.

## 4. Confirmed alongside

F83's **RFC schedule sync check now actually executes in CI** and passes on
`f4a61a3` — verified in the job's step list, not inferred. That was the one part
of request 077 that could not be shown at the time, because `version-sync` was
red ahead of it. Both halves of F83 are now demonstrated.

## 5. Status

Nothing queued for you beyond handoff 010 (the four F75(b) deletions), which is
at `dev-record/handoffs/010-f75b-delete-four-obsolete-view-models.md`.
