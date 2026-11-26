# Handoff 084 — 0.194.0: comparison reports

**From:** architect. **Date:** 2026-10-19. **Release:** `0.194.0`.
**Implements:** RFC-092 (`rfcs/accepted/092-comparison-reports.md`), accepted by
the owner on 2026-10-17.

**The RFC is the authority.** Where this document and the RFC differ, the RFC
wins. If implementation shows the RFC is wrong anywhere, **stop and tell me
before changing course.** Do not change the design yourselves, and do not edit
any RFC.

**The owner's rule for this RFC** is *"Be careful not to print noise to users,
which results into only confusion."* Every line in a report, and in the dialog,
must help its reader. A section, an option line or a note appears only when it
applies. When in doubt, leave it out and ask me.

---

## 0. One rule the RFC implies: the checksums and the changes describe the same bytes

RFC-092 §5 hashes "each file's bytes as read at export time". A report is worth
something only if anyone running `sha256sum` gets the hashes it shows, **and**
the changes it shows are the changes between exactly those files. So:

- **The HTML report is built from one read of each file at export time.** Hash
  those bytes, and compute the changes shown from those same bytes, with the
  comparison's own options.
- **A file comparison with unsaved edits:**
  - *Export report…* is disabled, with the tooltip *"Save or undo your changes
    first: a report describes the saved files."*;
  - the toolbar's *Export patch* keeps today's behaviour, unchanged.
- **If what is read at export no longer matches what the user compared,** the
  export refuses before writing anything, with one plain message: *"A file
  changed since it was compared. Compare again, then export."*
  - **For a file:** use the save path's fingerprint where it exists, or
    compare what you read with what the tab loaded.
  - **For a folder:** a file listed *identical* whose two hashes now differ,
    or listed *different* whose two hashes now match.
- **A folder comparison still in progress** (any entry still being compared)
  has *Export report…* and *Export patch…* disabled until it finishes. A
  comparison that was cancelled can be exported. Its unfinished entries are
  *not compared*, in the app's words.

This is not new scope. It keeps the RFC's promise that the report describes
exactly those files. I am adding a one-line clarification to RFC-092 §5 myself.

## 1. Where it is offered (RFC §1)

- **The diff toolbar's *More* row:** *Export report…*, beside *Export patch*.
- **Deep Compare's toolbar:** *Export report…* and *Export patch…*.
- **Spreadsheet and binary comparisons:** *Export report…* is disabled, with
  the RFC's tooltip.
- **Every export goes through the user's click and a save dialog.** Default
  file names:
  - a file comparison: `<right file name>-report.html`;
  - a folder comparison: `<right folder name>-report.html`;
  - the patch keeps today's name.

## 2. The dialog (RFC §2)

- **Format:** *HTML report* (the default) or *Unified diff (.patch)*.
- **Paths,** under *More options ▸* and collapsed: *File names only* (the
  default), *Paths relative to the compared folders*, *Full paths*.
- **Use `SelectField`** for every dropdown (F202). Labels and controls follow
  F205's spacing.
- **After saving,** one notice offers **Open**. This needs two pieces:
  - **A notice with an action button.** Today's notices have none. Keep it to
    one action, and reuse it nowhere else in this handoff.
  - **A new `open_path` in `external_tool.rs`,** beside `open_url`, for a file
    the user just saved:
    - it calls `env_passthrough::restore_onto`;
    - **on Windows it must not pass the path through `cmd /C start`.** `cmd`
      interprets `&`, `^` and `%` in a file name. Use `explorer.exe <path>`
      or `ShellExecuteW`;
    - unit-test each platform's command shape, as `open_url_command` does,
      with a file name containing `&`.

## 3. The HTML renderer (RFC §3, §3a)

- **In `forskscope-core/src/report/`,** reading the same model as
  `to_markdown` and `to_json` (§6). Extend the model where the HTML needs
  more:
  - checksums;
  - the one-sentence summary;
  - character highlights;
  - non-default options only;
  - notes only when they apply.

  Do not build a parallel model.
- **The language comes from the UI's.** Add new strings to i18n in English and
  Japanese.
- **The colours** come from the user's *Colour vision* setting, using the same
  token values as `assets/main.css` (RFC-088/090). Do not invent new ones. The
  `colour-vision` gate already measures those tokens.
- **The inert rules, both of them:**
  - escape every name and line through one escaping function, and test it on
    its own;
  - include the CSP meta exactly as the RFC writes it.
- **The checksum sentence** sits beside the checksums. Wording, English: *"Run
  `sha256sum` on a file to confirm it is the file this report describes. This
  does not prove the report itself is unaltered."* Japanese to match. **No
  general Notes section** unless something applies.
- **The folder report:**
  - **the counts and status words are the app's own,** from Deep Compare,
    never "identical" without a content comparison;
  - **a SHA-256 for every file present** on either side, since each is read
    at export anyway (§0);
  - **the cap:** 50 files or 5 MiB of change text, whichever comes first. When
    it applies, the report names every file it leaves out;
  - binary and spreadsheet files: status only.

## 4. The unified diff (RFC §4)

- **A file comparison:** the dialog's *Unified diff* calls the same code as
  *Export patch*, so the two are identical byte for byte.
- **A folder comparison:** `patch_from_directories`, with Deep Compare's diff
  options. An unreadable entry fails the export with a message naming it (F79).

## 5. Checksums and the work off the UI thread (RFC §5)

- **`sha2`** becomes a direct dependency of `forskscope-core`, at the version
  already in the tree (0.10.9). Add it to `audit-deps`'s expectations with the
  reason.
- **Reading, hashing and rendering run off the UI thread,** in the app's
  existing job/cancel machinery:
  - **progress** for a folder;
  - **Cancel** stops the export and **writes nothing**: render into memory or
    a temporary file, and write the chosen path only at the end;
  - hashes live only for the export.

## 6. Documentation

- **`docs/src/intermediate/patch-export.md`** becomes *Exporting a comparison*.
  Keep the old page's anchors working, or update every link to them, and the
  link check confirms which. It covers:
  - both formats and where they are offered;
  - the path choice;
  - what the checksums prove and do not;
  - *Print → Save as PDF*.
- **`docs/src/users/features.md`:** one entry.
- **The keyboard reference,** only if you add a shortcut. None is required.

## 7. Verification: RFC-092's criteria 1–7, one by one, with evidence

**Rules for evidence, after reviews 169–172:**
- **Quote the log or output lines you rely on.**
- **Describe each screenshot or PDF after looking at it.**
- **Every check must be able to fail.** For each test, say what you removed to
  watch it fail.

**In particular:**

- **Criterion 1:** `cmp` between the dialog's patch and *Export patch*.
- **Criterion 2:**
  - a planned fixture of 60 changed text files, which shows the cap and lists
    the 10 files left out;
  - `git apply` of the folder patch on a copy of the left folder, followed by
    `diff -r` against the right folder for the text files;
  - an unreadable entry made with `chmod 000`.
- **Criterion 3:**
  - a fixture file containing `</pre><script>alert(1)</script>` and `<img
    src=x onerror=alert(1)>`;
  - two tests, each failing when its own defence is removed;
  - `cargo xtask pages-origin-check <report>.html`.
- **Criterion 4:** `sha256sum` against each hash in a file report and a folder
  report.
- **Criterion 5:**
  - print to PDF with headless Chromium, in Standard and in red–green safe;
  - look at both and describe them: marks on every changed line, no clipped
    line, readable in black and white.
- **Criteria 6 and 7:** grep a default report for `/home` and the username, then
  a *Full paths* report; one report in Japanese.
- **§0:**
  - a file edited after comparing refuses the export;
  - a tab with unsaved edits shows *Export report…* disabled with its tooltip;
  - a cancelled folder export leaves no file at the chosen path.
- **Fixtures** are built and planned, never "real" files: invented content, in
  the scratchpad or `tests/`.

## Scope

- **In:** §0 to §6; i18n (English and Japanese).
- **Out:**
  - Markdown or JSON in the UI;
  - a PDF writer;
  - signing;
  - spreadsheet, binary and three-way reports;
  - the landing page;
  - RFCs;
  - `CHANGELOG.md`, `ROADMAP.md`, the version.

## Gates

- **The standard set:**
  - `cargo fmt --check`, for the workspace and xtask;
  - clippy: native, Windows GNU and xtask;
  - tests;
  - every `cargo xtask` gate;
  - `mdbook build docs`, `pages-link-check`, `actionlint`.
- **A green CI run,** with its ID.

Reply in `.git-exclude/review-request/159-comparison-reports.md`.
