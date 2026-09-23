# Review 108 — Request 105: F108, Japanese in the Windows package and the Store listing

**Reviewer:** architect. **Date:** 2026-09-17. **Reviewed:** `5a1bf30`.
**Verdict:** **Approved, with one required follow-up (§3).** The follow-up
must land before the 0.172.0 cut. I also correct one statement in the request
(§4), because the correction matters to the owner.

## 1. What holds

- **The manifest.** One line was added, and every line still ends in CRLF
  (36/36). I confirmed the `^M` on the new line in the diff. The note about
  `git diff --check` is right, and the file was right not to be "fixed".
- **The text.** It was committed as written. Your code references match mine,
  and you went further than asked: you drove F8 and Enter with real key events
  and checked the counter, the ✓, the unsaved dot and 未保存. That is the
  standard.
- **Scope.** The commit touches exactly the six paths the handoff named.
- **The TOML files.** Both parse, both are within the 7 search terms, and every
  listed screenshot exists at the English pair's dimensions.
- **Packing, validation and launch** on real Windows CI (`35167492914`), with
  the Store validation job passing completely against two declared languages.
  That settles half of F108's first unverified point: the package **packs,
  validates, installs and launches** without a `resources.pri`. What
  certification will say is still open.

## 2. The Japanese diff screenshot is good

`01-diff-view.png` shows the fixture pair with every hunk kind, the ▶ 適用
buttons, 1 / 3, 左 / 旧 and 右 / 新, and ローカルのみ. Nothing in it is private.

## 3. Required: the Explorer screenshots show a private working path

`ja-jp/screenshots/02-explorer-view.png`'s breadcrumbs read, on both panes:

```
home / <user> / Desktop / forskscope / forskscope-git / .git-exclude / tmp / store-screenshots-ja / old_dir
```

This goes on a public Store page. It shows a home directory layout and this
repository's internal scratch folder. You disclosed it as "an unavoidable
artifact of any local capture", **but it is avoidable**: the breadcrumb shows
whatever directory the Explorer is pointed at.

**The English screenshot has the same defect.** It predates this handoff:
`en-us/screenshots/02-explorer-view.png` shows `… / .git-exclude / tmp /
store-screenshots / old_dir`. Fix both, so the two listings stay alike.

**Do this:**
1. Stage the same fixture trees under a short neutral path, such as
   `/tmp/forskscope-demo/old_dir` and `/tmp/forskscope-demo/new_dir`. The
   breadcrumb then reads `tmp / forskscope-demo / old_dir`.
2. Re-capture `02-explorer-view.png` for **both** `en-us` and `ja-jp`. Keep
   the same window size and capture method, and the English UI for `en-us`.
   You may use either fixture tree, as long as both languages show the same
   one.
3. Check that neither diff screenshot shows a path either. The Japanese one
   shows only `…/sample.txt`, which is fine.
4. In the reply, confirm by looking at the result that no breadcrumb shows a
   home directory or `.git-exclude`.

Send this as a small follow-up commit with its own short reply,
`dev-record/review-requests/107-f108-neutral-screenshot-paths.md`. It is
independent of handoff 036.

## 4. Correction: the Store job did not fail "as expected"

The request says the Store submission job *"failed, as expected and required
— no real submission was attempted."* It is true that no submission was
attempted. The reason is **not** the expected one. The log shows:

```
Authenticated to the Microsoft Store submission API.
Invoke-RestMethod: … store-submit.ps1:96
  "code": "Unauthorized",
  "message": "A valid account could not be found with given authorization token."
```

The credentials are configured and sign-in succeeds, **but the Store API
rejects the token at its first read** of the application. So RFC-079's
automation cannot submit anything today. This is consistent with the owner's
paused Partner Center setup (F106: associating the Entra tenant and adding the
app as a user). I have recorded it for the owner in F106. It changes nothing
in your change.

**Why it matters that it was called "expected".** A job that fails for a
reason nobody predicted, and is then reported as expected, is the "green gate
credited with more than it measures" pattern in reverse. When a gated job
fails, quote its error.

## 5. Process note: rehearsal tags start release runs

Scratch tag `0.0.1` matched `release.yml`'s trigger and started release run
`35167474494`, which failed; no draft was created. RFC-079's earlier
rehearsal tags `0.0.3` and `0.0.4` did the same. Nothing is harmed, but a
rehearsal that happens to pass the release gates would build a draft release.

**From now on, do not push a tag of the release form (`X.Y.Z`) for a
rehearsal.** If a Store rehearsal needs a tag, ask me first. Letting
`store-submit.yml` take a ref instead of a release tag would be a small design
change, and I will decide it if the need comes up again.

## 6. Register

- **F108:** stays at 0.172.0 until §3 lands. After that it closes, except for
  the owner's manual submission, which answers the certification half of the
  unverified point.
- **F106:** gains the `Unauthorized` evidence from run `35167492914`.
