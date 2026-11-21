# RFC 100: Bidirectional Text in Diffs — Show the Invisible, Keep Each Line in Place

**Status.** Proposed.
**Proposed.** 2026-11-21 by the architect, for **F234**. The owner asked
about right-to-left support. The answer separated a right-to-left
*interface*, which is deferred, from right-to-left *content* in the files
being compared, which matters to every user and is fixed here. The owner
accepted that direction the same day ("All accepted. Proceed."). This
document is the design for that acceptance.
**Scheduling.** `0.200.0`.
**Touches.**
- `crates/forskscope-compare`: one detector;
- `crates/forskscope-ui`: the diff and merge views, the status bar, the
  CSS, the i18n table;
- `crates/forskscope-report`: the HTML report;
- `crates/forskscope-commands`: the text output, and one additive JSON
  field;
- the guide's comparing and FAQ pages.

## Summary

- **Every bidirectional control character in a compared file is shown as
  a visible marker,** in place, and never takes effect on screen.
- **Each line's text is kept inside its own line:** nothing a line
  contains can reorder the gutter, the marks or the next line.
- **A file that contains such characters says so,** in the status bar,
  the HTML report and the command-line output.
- **What is saved is untouched:** the markers exist only on screen.

## Motivation

Unicode's bidirectional controls are invisible, and they reorder the text
around them. Examples:
- U+202E RIGHT-TO-LEFT OVERRIDE;
- the isolates U+2066–U+2069.

**"Trojan Source"** (CVE-2021-42574) used them to make code read one way and
run another: a comment that seems to end early, or a string that seems to
hold something else.

**A diff tool is where they should be seen.** Today, ForskScope:
- renders them like any other text, so the browser engine applies them;
- has nothing in the engine, the view or the CSS that marks or limits
  them;
- **can show a change that only adds or moves one as if nothing changed,**
  or reorder a line so that the highlighted change sits somewhere else.

Hosted code review tools warn about such files. ForskScope doesn't.

## Design

### 1. Which characters

**The bidirectional formatting characters:**

| Code point | Name | Marker |
|---|---|---|
| U+202A | LEFT-TO-RIGHT EMBEDDING | `LRE` |
| U+202B | RIGHT-TO-LEFT EMBEDDING | `RLE` |
| U+202C | POP DIRECTIONAL FORMATTING | `PDF` |
| U+202D | LEFT-TO-RIGHT OVERRIDE | `LRO` |
| U+202E | RIGHT-TO-LEFT OVERRIDE | `RLO` |
| U+2066 | LEFT-TO-RIGHT ISOLATE | `LRI` |
| U+2067 | RIGHT-TO-LEFT ISOLATE | `RLI` |
| U+2068 | FIRST STRONG ISOLATE | `FSI` |
| U+2069 | POP DIRECTIONAL ISOLATE | `PDI` |
| U+200E | LEFT-TO-RIGHT MARK | `LRM` |
| U+200F | RIGHT-TO-LEFT MARK | `RLM` |
| U+061C | ARABIC LETTER MARK | `ALM` |

**The list lives in one place in `forskscope-compare`,** with a test that
holds it equal to Unicode's `Bidi_Control` property.

### 2. On screen: a visible marker, never the character

- **In the diff and merge views,** each such character is rendered as a
  marker **in place of** the character:
  - the abbreviation in a small bordered token, for example `RLO`;
  - in the warning colour;
  - with an accessible name ("right-to-left override character"), in both
    languages.
- **The character itself never reaches the page,** so it can't reorder
  anything.
- **A change that only adds, removes or moves one** shows that marker as
  the change, with the usual character-level highlighting.
- **Copying from the diff copies what is shown,** the marker's text, not
  the invisible character. That's a deliberate choice: a diff that silently
  carries a reordering character into your clipboard would repeat the
  problem it just showed you.
- **The layout and contrast checks (RFC-098) cover the marker,** through a
  fixture that contains every listed character.

### 3. Each line stays in place

**Every line's text cell is isolated** (`unicode-bidi: isolate`), with a
left-to-right paragraph direction:
- **right-to-left text in a file** (Hebrew, Arabic and others) is still
  ordered correctly within its line, by the browser's bidirectional
  algorithm;
- **nothing in it can move the gutter, the marks, the *Use* buttons or the
  next line.**

**Per-line automatic direction is a non-goal** (it would right-align a
Hebrew line in a code pane). Code editors keep a left-to-right paragraph
too.

### 4. Saying so

- **The engine reports, for each side, whether the text contains any of
  §1's characters, and how many.** It is a pure function over the
  decoded text, next to the existing text checks.
- **The status bar** shows, while such a file is open, "Contains *N*
  direction-control characters" in the warning colour. **The marker's
  colour isn't the only signal:** the words carry it.
- **The HTML report** renders markers as in §2, and shows the same notice
  above the file's diff.
- **`forskscope compare`'s text output** prints each character escaped
  (`<U+202E>`), and adds one notice line per affected side.
- **The JSON output and the MCP `compare` tool** gain an additive field
  per side, `bidi_controls`: the count. **The file's text in JSON stays
  exact,** because JSON escaping is already safe.
- **The JSON schema** (`docs/schemas/`) gains the field, and
  `cargo xtask json-schema` checks it.

### 5. What is saved

**The markers are display only.** Saving, *Use*, merging and exporting a
patch work on the file's real text, unchanged. A test saves a file that
contains every listed character and compares it byte for byte.

## Non-goals

- **A right-to-left interface** (mirrored layout, Arabic or Hebrew
  translations). That's deferred until a right-to-left language has a
  committed reviewer (RFC-101's language policy).
- **Other invisible or confusable characters,** such as zero-width spaces
  and homoglyphs. Many are legitimate in some scripts, so they need a design
  of their own. A finding will be registered if needed.
- **Per-line automatic direction** (§3).

## Security and privacy

- **No network.**
- **No new data flow:** the detector reads text the app already reads.
- **A safety improvement:** what a diff shows matches what a file holds.

## Verification

1. **Fixtures, generated by the test code:**
   - one file per listed character;
   - pairs where the only change is one inserted, removed or moved
     control;
   - three pairs in the shapes the Trojan Source paper describes:
     - an early return hidden in a comment;
     - code commented out;
     - a string that seems to end;
   - right-to-left prose (Hebrew and Arabic) with no controls.
   
   **Written for this project.** No example code is copied.
2. **The view:**
   - no listed character reaches the rendered page. A markup test scans
     the rendered text;
   - every marker has its accessible name, through the real i18n table;
   - the right-to-left prose renders in the right order within its line.
     Check it once by eye, with the capture looked at in full.
3. **The engine:** the counts per side, against the fixtures.
4. **The CLI, the JSON and MCP outputs,** and the schema gate.
5. **Saving round-trips byte for byte** (§5).
6. **RFC-098's gate stays green,** with the new fixture view in the
   matrix.

## Open questions

None. One choice is decided here and can be revisited: **copying copies
the marker** (§2).
