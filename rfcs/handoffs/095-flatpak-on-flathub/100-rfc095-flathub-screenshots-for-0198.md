# Handoff 100 — `0.198.1`: the Flathub listing's screenshots, retaken for the current UI

**From:** architect. **Date:** 2026-11-15. **Release:** `0.198.1`, a patch
release. Nothing in the app changes.

**Why:** the owner submits ForskScope to Flathub. The listing's screenshots
(`packaging/linux/screenshots/`, linked from the metainfo at the `0.196.1`
tag) no longer match the app:
- the "?" button reads "? Shortcuts";
- the comparison toolbar shows *Previous change*, *Next change* and *Search*
  beside their symbols;
- the Explorer has its symbol key;
- **Deep Compare shows its row buttons only on the selected row.** The
  current `folder-comparison.png` shows them on every row, which is no
  longer true.

A listing that shows a different app is the confusion the owner's first
rule forbids.

## Read first

- **The GUI rules** (`.git-exclude/rules/project-instructions-gui-common.md`),
  including the owner's *Test Optimization Rules* and the 2026-11-15 line:
  *the folders the app opens or compares are fixtures too.*
- **Handoff 092 and reviews 191–194:**
  - the content;
  - the 1000×700 size;
  - the hard crop with no resampling;
  - the focus-ring decision (review 194: the Explorer tab's ring is
    accepted);
  - the privacy checks.
- **The shared session script** `.git-exclude/gui/session.sh` and its
  fixtures. Use them.

## Rules for this work

- **Flathub's AI policy (RFC-095 §1):** never write, draft or edit a
  Flathub manifest, `cargo-sources.json`, or anything for the Flathub pull
  request. **List exactly which files you made,** because the owner
  discloses them.
- **Privacy:**
  - invented demo files only, as fixtures under the project;
  - the isolated data directory;
  - a `strings` check of every PNG for the username and for `/home/`
    before committing.
  - **Never the real home folder.**
- **Evidence plan first** (the rules): the questions are listed below. The
  level is real-UI capture, because these are deliverables.

## Do

Retake, at 1000×700, the same theme and the same demo content as the files
they replace:

| File | Content | What must now be visible |
|------|---------|--------------------------|
| `explorer.png` | the Explorer on `website-v1` / `website-v2`, with `css` expanded (as in 092) | the symbol key; "? Shortcuts" |
| `file-comparison.png` | `app.js` with its first change selected (as in 092) | the toolbar's words; *Redo* beside *Undo* |
| `folder-comparison.png` | the Deep Compare of the same demo pair, *Different* filter | the first row selected, with its accent bar and buttons; no buttons on the other rows |

- **`html-report.png` stays.** The report hasn't changed in its Standard
  colours.
- **No tooltip in any frame:** park the pointer over the title bar.
- **No focus ring** beyond the one review 194 accepted. Check each frame
  against the old file at the same spot, by exact pixel count, and explain
  every difference you find.
- **Hard crop to 1000×700, with no resampling.**
- **Don't change the metainfo.** I update its image links to `0.198.1` in
  the release cut.

## Verification

- **The three PNGs:** dimensions, and the `strings` checks.
- **A short table per file:** what changed from the old file, by region,
  with an exact pixel count per region.
- **The list of files you made,** for the owner's disclosure.

## Reply

`.git-exclude/review-request/187-flathub-screenshots.md`. One commit, with
CI green.
