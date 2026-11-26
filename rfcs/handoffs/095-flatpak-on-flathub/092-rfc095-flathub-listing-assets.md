# Handoff 092 — 0.196.1: the Flathub listing's screenshots and icon

**From:** architect. **Date:** 2026-10-25. **Release:** `0.196.1`, a patch
release. **Implements:** RFC-095 §5, the screenshots and the icon
(`rfcs/accepted/095-flatpak-on-flathub.md`). Nothing in the app changes.

**Why:** the owner submits ForskScope to Flathub from the `0.196.1` tag. The
listing takes its screenshots and icon from that tag.
- **The icon now exists.** The owner approved it on 2026-10-25, and RFC-095
  open question 1 is decided.
- **Two of the `0.196.0` screenshots cannot be used:**
  - `explorer.png` shows the demo home's `.cache`, `.config` and `.local`
    (review 189 §4).
  - The owner's replacements have the right content, but at 1538×962 pixels
    on a 1.2-scale display, about 1282×802 logical. That exceeds RFC-095's
    1000×700.

## Read first

- **The GUI rule, updated by the owner on 2026-10-24:**
  `.git-exclude/rules/project-instructions-gui-common.md`, especially *GUI
  Evidence and Automation Rules*. Every capture here follows it.
- **RFC-095 §5,** *The icon* and *Screenshots*.
- **The owner's reference images,** in
  `.git-exclude/tmp/forskscope-flatpak-package.zip`:
  - `upstream-assets/packaging/linux/screenshots/explorer.png`;
  - `upstream-assets/packaging/linux/screenshots/file-comparison.png`.

  Extract the zip under `.git-exclude/tmp/`. **Do not commit these images.**
  They are the reference for content, not the deliverable.

## Rules for this work

- **Flathub's AI policy (RFC-095 §1):**
  - never write, draft or edit a Flathub manifest, `cargo-sources.json`, or
    anything for the Flathub pull request;
  - the owner discloses the screenshots you make, so list exactly which
    files you made.
- **Do not draw, generate or edit the icon.** Commit the supplied file byte
  for byte (§2).
- **Live checks:** AT-SPI actions only, never keystrokes. This work needs
  none.
- **Privacy:** the sandbox-home recipe, invented demo files only, and a
  `strings` check of every PNG for the username and `/home/` before
  committing.

## 1. Retake two screenshots

`packaging/linux/screenshots/explorer.png` and `file-comparison.png`.
- **Content:** the owner's two reference images. Same demo folders, files,
  selection and state:
  - the Explorer on `website-v1` and `website-v2`, with `css` expanded;
  - `app.js` with its first hunk selected.

  Recreate the demo files from what the images show; invent anything they
  don't show.
- **Size:** exactly 1000×700 pixels, like `folder-comparison.png` and
  `html-report.png`.
  - **Set it at launch,** for example with a nested compositor at scale 1
    sized for it.
  - **Never float or resize a window** on the shared desktop.
  - The app has no window-size setting. Do not add one.
- **Default settings, English,** the build from `main`.
- **The two other screenshots stay as they are.**
- **The metainfo captions must stay true:** the four captions must describe
  the four images. If one no longer fits, say so; don't rewrite it silently.
- **Look at each image before committing it,** and describe it in the
  request.

## 2. Commit the icon

- **Source:** `.git-exclude/tmp/flathub-icon-draft/io.github.forskscope.forskscope.svg`,
  SHA-256
  `487f29bb40865b2c597edb92aa286e4853a5cc679a5e8f37055e8fe62d359df4`.
- **Destination:**
  `packaging/linux/icons/hicolor/scalable/apps/io.github.forskscope.forskscope.svg`.
  Byte for byte; check the hash after copying.
- **`install.sh` and the PKGBUILD already install it from that path.** Do
  not change their logic.
- **Update every comment and message that says no icon exists:**
  - `install.sh`'s "none yet (RFC-095 §5 Open questions 1)" output;
  - the PKGBUILD's "no real icon exists yet" comment;
  - any other place you find.
- **Keep the guard:** a missing icon still reports itself, not an error.
- **The desktop file's `Icon=` stays as it is.**

## 3. The metainfo's screenshot links

Point all four `<image>` URLs at the `0.196.1` tag. The CI gate runs with
`--no-net`, so it passes before the tag exists. **Do not add the `<release>`
entry:** I add it at the cut.

## Verification

- **The icon:** the committed file's SHA-256 is the one in §2.
- **`install.sh`:** a staged install, to a scratch prefix, installs the
  icon. Quote the output.
- **The PKGBUILD:** a `makepkg` package contains
  `usr/share/icons/hicolor/scalable/apps/io.github.forskscope.forskscope.svg`.
  Quote the listing.
- **`appstreamcli validate --no-net --strict`** through the existing gate.
- **The images:**
  - `magick identify` shows 1000×700 for both;
  - the `strings` checks are clean;
  - your description of each.

## Scope

- **In:** §1–§3.
- **Out:**
  - the icon's design;
  - the manifest;
  - CHANGELOG, the version, the `<release>` entry;
  - any app change.

## Gates

**The standard set, and a green CI run** with its ID. Reply in
`.git-exclude/review-request/171-flathub-listing-assets.md`.
