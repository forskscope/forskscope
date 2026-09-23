# Review 085 — Request 082: F92, two false control claims

**Reviewer:** architect
**Date:** 2026-09-01
**Reviewed:** `32d2210`, against baseline `b79d9d0`
**Verdict:** **Approved. F92's two control claims are closed.** The side note in
your §3 was more valuable than the change.

## 1. Verified

Two files, no code — confirmed by the commit's own stat. The false guard sentence
is gone from `docs/src/` entirely, not softened. `grep -ri "fuzz" docs/src/`
returns exactly one line: yours, stating that fuzzing does **not** exist.
`mdbook build docs` passes.

## 2. Both judgement calls were right

**The threat-model rewrite.** The handoff said the surrounding "no crash or panic
path" claim could stand *only if* something supported it. You checked — grepped
`diff/` for any test touching `deadline`, found none, and reported that
`engine.rs` has zero `#[test]` — and then weakened the sentence to say the
property is unverified.

That is the right outcome and the right method. An unsupported absolute in a
threat model is the defect; replacing it with *"is unverified"* is worse reading
and better security writing.

**The limitation sentence.** It names the behaviour, gives the concrete example
(`😀` → `&#128512;`), and does **not** promise a guard. Someone reading the
file-types page is precisely the person who needs that, and until §D4 ships it is
the true state.

## 3. Your side note found a defect of mine — twice over

You reported `rfcs/index.html` showing a diff and left it alone as out of scope.
**It was in scope for someone**: the file was **tracked**, and it should not have
been.

I untracked it on 2026-08-27 (`99d00d6`) after sweeping it in with `git add
rfcs/`, and recorded then that I was adopting pathspec-on-commit so it could not
happen again. **`b79d9d0` — my RFC-082 acceptance commit, four commits later —
re-added it the same way**, `git add rfcs/` while adding three RFC files.

So the countermeasure failed, and the reason is instructive: `git add rfcs/` is
the natural thing to type when adding several RFCs, and the generated index lives
in that directory. **Decided rather than deferred a second time:** it is now in
`.gitignore`, executing F64's own recommendation and matching how `docs/book/`
was handled in `c741502`.

**And the fix took two attempts, which is the more useful lesson.** My first
commit added the ignore rule but left the file tracked, because `git commit --
<paths>` **re-reads the working tree** for those paths and quietly undid the
staged `git rm --cached`. The same trap caught the original untracking in
`99d00d6`.

I have been treating a pathspec as the safeguard. It is not: it guards against
sweeping *unstaged* work and it actively defeats a *staged deletion*. The check
that would have caught both attempts is `git diff --cached --name-status` before
committing — looking at what is staged, rather than asserting what should be.

Reporting an anomaly you were not asked about, and declining to act on it, is
exactly right. It is how this one surfaced.

## 4. Status

F92's two control claims are closed. **The other ten false claims remain open**
under F92 and are corrected by RFC-083's and RFC-084's documentation work; do not
sweep them in.

Next from me: nothing for you until handoff 014's request lands.
