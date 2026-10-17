# RFC 092: Comparison Reports — HTML and Unified Diff, for Files and Folders

**Status.** Proposed — awaiting the owner's acceptance. Implementation does not
start before this moves to `accepted/` (RFC-000, 5-folder variant).
**Scheduling.** After `0.193.0` (RFC-091). See `ROADMAP.md` § "Remaining proposed
RFCs".
**Tracks.** The owner's question, 2026-10-17: is a report of a diff supported, in
a format that is easy to read and hard to change? The owner accepted the
direction the same day.
**Amends, on acceptance.** RFC-027 (Report Export and Session Evidence). This RFC
takes over its deferred items: the export dialog and the HTML report. It leaves
PDF to the system's printing. RFC-027 gets a one-line Status note pointing here.
**Touches.**
- `crates/forskscope-core/src/report/` (a new HTML renderer)
- `crates/forskscope-core/src/patch/` (unchanged, newly reached from the UI)
- a checksum helper in core (`sha2`)
- the diff toolbar's *More* row and the folder report's toolbar (`ui/view/deep_compare.rs`)
- a small export dialog
- i18n
- `docs/src/intermediate/patch-export.md` (renamed *Exporting a comparison*)
- `docs/src/users/features.md`

## Summary

A user can save what a comparison found, as **one HTML file** or a **unified
diff**, from a file comparison **or a folder comparison**.

- **The HTML report** is self-contained: no scripts, nothing loaded from
  elsewhere.
  - It reads without colour and prints cleanly, so *Print → Save as PDF* in any
    browser produces the PDF.
  - It records the **SHA-256 of every compared file**, so anyone can confirm it
    describes exactly those files.

## Motivation

**What exists today:**

| Capability | State |
|---|---|
| Unified diff of a file comparison | **Shipped**: *More ▼ → Export patch* (`patch_from_file_diff`) |
| Unified diff of a folder comparison | **Built, tested, never offered**: `patch_from_directories` (core, F79: it fails rather than silently omit an unreadable entry) |
| Markdown and JSON reports, file and folder | **Built, tested, never offered**: `report::FileComparisonReport` and `DirComparisonReport` (RFC-027, v0.49.0, 20 tests), with name-only paths by default |
| An export dialog, HTML, PDF | **Not built**: RFC-027 deferred them |

**Why reports, in the owner's words:** a result *"easy to read and kind enough to
understand"*, to give to someone, in a form *"difficult to change"*.

- **A `.patch` is precise but written for tools.**
- **Markdown is readable but plain,** and edited as easily as it is read.

### What "difficult to change" can honestly mean

**No unsigned file format resists editing.** A PDF only discourages casual edits,
and signing is a larger question (F46).

What a report *can* carry cheaply is **the SHA-256 of each compared file**:

- anyone can run `sha256sum` on the files and confirm the report describes
  exactly those;
- it does not prove the report itself is unaltered, **and the report says so**.

## Design

### 1. Where it is offered

| View | Control |
|---|---|
| **A file comparison** | *More ▼* row: **Export report…** beside the existing *Export patch* |
| **A folder comparison** (the deep compare report) | Its toolbar: **Export report…** and **Export patch…** |

- **A spreadsheet or binary comparison** shows *Export report…* disabled, with a
  tooltip saying reports cover text comparisons for now.
- **Nothing is exported automatically.** Every export is the user's click and a
  save dialog.

### 2. The export dialog: small on purpose

| Choice | Default | Other values |
|---|---|---|
| **Format** | *HTML report* | *Unified diff (.patch)* |
| **Paths** (under **More options ▸**, collapsed) | *File names only* | *Paths relative to the compared folders*; *Full paths* |

- The path choice is RFC-027's privacy rule (`ReportPathMode`): full paths
  never appear unless chosen. It sits behind *More options*, following the
  owner's rule that detailed settings stay hidden.
- *Export patch* in the toolbar keeps working as today. It is the same as
  choosing *Unified diff* here.
- After saving, a notice offers **Open**, which opens the report in the
  system's default program. RFC-091 already gives child processes the user's
  own environment back.

### 3. The HTML report

**One file:**

- inline CSS, **no JavaScript**, no fonts, images or anything else loaded from
  elsewhere;
- the UI's language (English or Japanese).

**Contents, in reading order:**

1. **Header:**
   - *Comparison report*;
   - the date and time it was made, local time with the offset;
   - *ForskScope X.Y.Z*.
2. **What was compared.** Each side's name, per the path choice, then:
   - **a file:** its size and **SHA-256**;
   - **a folder:** its root, and the counts in §3a.
3. **How:**
   - the compare profile's name and options (whitespace, case, line endings,
     algorithm);
   - the context lines used.
4. **Summary in one sentence first,** for example *"3 changes: 2 lines removed,
   5 added, 1 replaced."* Then the counts.
5. **The changes,** in unified layout:
   - old and new line numbers;
   - **`−` and `+` marks on every changed line;**
   - the context lines around each change;
   - character-level highlights where the comparison computed them.
6. **Notes**, never omitted:
   - limits that applied, such as character highlighting being off for size;
   - encoding notes;
   - anything the comparison could not do;
   - **What the checksums prove,** in one sentence.

**Readable without colour.** Every change carries its `−`/`+` mark and a visible
label. The colours follow the user's *Colour vision* setting (RFC-088 / RFC-090),
so a red–green-safe user's report is red–green safe.

**It prints cleanly.** A print stylesheet:

- keeps a change's lines together where it can;
- keeps the marks and borders in black and white;
- repeats nothing that wastes paper.

*Print → Save as PDF* in any browser is how a PDF is made, and the user guide
says so.

**It is inert, whatever the files contain:**

- every file name and every line is **HTML-escaped**, so a file containing
  `</pre><script>` shows that text;
- **and** the report carries `<meta http-equiv="Content-Security-Policy"
  content="default-src 'none'; style-src 'unsafe-inline'">`, so even an
  escaping mistake could load or run nothing. Two defences, deliberately.

#### 3a. A folder comparison's report

- **The counts,** in the same words the app uses (F74/F82):
  - identical, with contents compared;
  - different;
  - only on the left;
  - only on the right;
  - could not be read;
  - not compared.
  
  **A report never calls a file identical unless its contents were compared.**
- **Every file listed,** with its status, and a SHA-256 for each file whose
  contents ForskScope reads to make the report.
- **The changes of each different text file,** in order.
  - A cap keeps a huge folder's report usable: **50 files or 5 MiB of change
    text, whichever comes first.**
  - **When the cap applies,** the report says how many files' changes it left
    out and lists them by name. It **never omits silently.**
  - The unified diff export has no cap.
- **Binary and spreadsheet files** are listed with their status, and their
  changes are not shown.

### 4. The unified diff

- **A file comparison** exports exactly what *Export patch* writes today.
- **A folder comparison** exports `patch_from_directories`.
  - It keeps F79's rule: if any entry cannot be read, the export **fails with a
    message naming it**, rather than producing a patch that looks complete and
    is not.
  - Binary entries follow `PatchOptions`, as the core already defines.

### 5. Checksums

- **SHA-256,** with `sha2` as a direct dependency of `forskscope-core`. It is
  already in the dependency tree indirectly. `cargo xtask audit-deps` is updated
  to accept it, with the reason.
- **What is hashed:** each file's bytes as read at export time. Reading happens
  off the UI thread, with progress shown for a folder and a **Cancel** that
  stops the export without writing anything.
- **Hashes are computed only for the export,** never kept or written anywhere
  else.

### 6. What stays out of the UI

- **The core's Markdown and JSON reports** stay as they are, tested and unused.
  The owner asked for two formats, and a third and fourth choice would crowd
  the dialog. JSON may be offered later for tools, by its own decision.
- **The core's `to_markdown`/`to_json` and the new HTML renderer** read the same
  report model, so the three cannot disagree.

## Non-goals

- A PDF written by ForskScope itself. Printing produces it; a built-in writer
  would need embedded fonts, several megabytes for Japanese alone, and its own
  layout.
- Signing a report (F46's territory).
- Reports for spreadsheet, binary or three-way comparisons.
- Exporting automatically, on a schedule, or anywhere but a file the user chose.
- Markdown or JSON in the UI.

## Acceptance criteria

1. **File comparison:**
   - *Export report…* saves one HTML file, which a browser opens with no
     network access, showing every section of §3.
   - *Unified diff* matches *Export patch* byte for byte.
2. **Folder comparison:**
   - the report lists every file with the app's own status words;
   - the cap's notice appears when built over 50 changed files;
   - the patch, applied with `git apply` to a copy of the left folder,
     reproduces the right folder's text files;
   - an unreadable entry fails the patch export, naming it.
3. **Inert:**
   - a fixture file containing `</pre><script>alert(1)</script>` and `<img src=x
     onerror=…>` appears as text;
   - the CSP meta is present;
   - a test fails if either defence is removed.
   - The site's origin check (F190) run on a generated report finds nothing.
4. **Checksums:** each SHA-256 in a report equals `sha256sum` of that file.
5. **Without colour and in print:**
   - a print-to-PDF of a report, from headless Chromium or a browser, shows
     every change with its mark and no clipped line;
   - the PDF is looked at and described;
   - one report each in Standard and red–green safe.
6. **Privacy:**
   - by default, no directory part of any path appears in the report;
   - *Full paths* shows them only when chosen.
7. **Japanese UI** produces a Japanese report.

## Open questions

None.
