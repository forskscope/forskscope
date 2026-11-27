# Handoff 126 — RFC-081 amended: retire the recipe-fix path

**From:** architect. **Date:** 2026-11-27. **Release:** `0.203.0`.
**RFC:** `rfcs/accepted/081-aur-publication-automation.md`, amended by
the owner on 2026-11-27. Read its *Amended* line, and the note at the top
of Q3: they hold the reasoning.

**Background (F176):**
- `recipe-fix` compares `main`'s `PKGBUILD` with the AUR's;
- `main`'s `pkgver` moves ahead in the post-release commit;
- so between releases the mode always refuses. All 5 runs ever made were
  rehearsals, and all 5 were refused.

**A packaging-only fix now ships as a patch release.**

## Do, in one commit

1. **`.github/workflows/aur-publish.yml`:**
   - remove the `mode` input, and the recipe-fix path in every job;
   - **release is the only path,** with `tag` its input;
   - **keep `dry_run` defaulting to true;**
   - keep every validation the release path runs today.
2. **`packaging/linux/aur-publish.sh`:** remove the recipe-fix branch.
   **Keep the release branch's behaviour identical.**
3. **`docs/src/maintainers/release.md`,** and anywhere else that
   documents the mode: the dispatch is now
   `gh workflow run aur-publish.yml -f tag=X -f dry_run=false`.
   - **Grep for `recipe-fix` and `mode=`** across the repository, and
     list every place changed.
4. **The workflow's own lint** (`actionlint` and `zizmor`, if CI runs
   them) stays green.

## Show

- **A dry run of the release path against the current tag** (`0.202.0`,
  `dry_run` left at its default `true`), dispatched once, reaches
  *Validate: success* and **skips the publish job.**
  - **This is the only dispatch in this handoff.**
  - **Never with `dry_run=false`.**
- **`git grep recipe-fix`** shows only the history in RFC-081 and the
  register.

**The register is mine.** Reply in
`.git-exclude/review-request/214-retire-recipe-fix.md`.
