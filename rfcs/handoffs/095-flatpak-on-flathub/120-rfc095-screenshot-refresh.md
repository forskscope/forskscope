# Handoff 120 — a screenshot refresh: the Linux listing and the README

**From:** architect. **Date:** 2026-11-26. **Release:** `0.202.0`, last.
The owner accepted and authorized it on 2026-11-26.

**Start only when I say so,** after handoffs 118 and 119 (and any fixes
they lead to) have landed, so the pictures show the final interface.

**Why:**
- **The README's two pictures** (`docs/src/assets/screenshot-diff.png`,
  `screenshot-explorer.png`) date from 2026-10-12. **They show the header
  row RFC-089 removed,** on the project's front page.
- **The Linux listing's three** (`packaging/linux/screenshots/explorer.png`,
  `file-comparison.png`, `folder-comparison.png`) date from `0.198.1`. They
  predate:
  - the quiet × buttons;
  - *Settings* at the end of the tab bar;
  - the ignore-list badges;
  - the direction markers;
  - the path bar's "…".

## Rules: privacy first

**These images are public.**
- **Use the established privacy-safe recipe:**
  - invented demo files, written for this purpose, never anything from
    the maintainer's machine;
  - a sandboxed home (`session.sh --home-remap`), so paths read
    `/home/you/…`;
  - the isolated session's settings and buses (handoffs 106 and 112).
- **Check each image** for any local path, user name, host name or real
  file name, **by looking at it in full and at the path bar and title
  zoomed.** State what you checked.
- **Nothing is committed until the owner approves the images.** Send them
  in the reply; I put them to the owner.

## What to take

- **The same scenes as today's,** so they replace like for like:
  - the README's file comparison and Explorer;
  - the listing's Explorer, file comparison and folder comparison.
- **The README's alt text and the metainfo's captions** are checked
  against what each new image shows. **Proposals only; the owner approves
  the wording.**
- **One theme throughout,** the same as today's, unless you show a reason.
  Text at 100 %, and a window size that reads well at the listing's
  display size.

## At the cut (mine)

The metainfo's image URLs are pinned to a tag (today `0.198.1`). They
move to `0.202.0` in the cut commit. The `metainfo-release` gate and
`pages-link-check` confirm they resolve after the tag.

## Reply

**The images, with the privacy check, and the caption and alt-text
proposals,** in
`.git-exclude/review-request/208-screenshots.md`. **Not committed.**

## Go (2026-11-26, review 248)

**Start now.** Handoffs 118 and 119 are complete.

## Round 2 (2026-11-26, review 249)

- **The three listing images are approved,** pending the owner's look at
  all five.
- **The README pair is content only** (the owner): crop the title strip
  exactly, and measure the edges.
- **`screenshot-explorer.png`:** one retry, with about 1 s of settle.
  **If it still lags, stop.**
- **Look at each capture in full before describing it.**

Reply in `.git-exclude/review-request/208b-screenshots.md`. **Not
committed.**

## Round 3 (2026-11-26, review 250)

**Approved by the architect. Waiting for the owner's approval of the
five images.** Then one commit, by you: the five images in place, nothing
else. The metainfo URLs move in the cut commit.

**Tooling, any time:** the session script's default settle after a
click becomes 1 s.

## Owner-approved (2026-11-26)

**The owner approved all five images.** Commit them now:
- `explorer.png`, `file-comparison.png` and `folder-comparison.png`
  into `packaging/linux/screenshots/`;
- `screenshot-diff.png` and `screenshot-explorer.png` into
  `docs/src/assets/`;
- **one commit, nothing else.**

Check `git log origin/main..main` before you push. Reply in one line
with the commit hash, in
`.git-exclude/review-request/208c-screenshots-committed.md`.

## Closed (2026-11-26, review 250)

**Complete.** The five committed images are byte-identical to the owner-approved ones (`ebee817`).
