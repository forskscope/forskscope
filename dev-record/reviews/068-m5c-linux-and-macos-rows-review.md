# M5-C Linux and macOS rows review — P03, P07, P11

**Review date:** 2026-08-16
**Requests:** `dev-record/review-requests/065-m5c-linux-visual-navigation.md`, `dev-record/review-requests/066-m5c-macos-visual-navigation-and-assembly.md`
**Governing document:** `rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md`
**Review mode:** Independent verification of the shared root cause behind F68/F73. Reviewed together because the slice's most consequential finding spans both rows.

## 1. Verdict

**Both rows approved as evidence.** All three M5-C prerequisites are now
resolved — Prerequisite A (Linux, §2), Prerequisite B (Windows, review 067), and
F63 (macOS, §3) — each before any evidence was gathered against them, as the
handoff required.

**One finding changes Gate D.** F73 is not an ordinary defect: on the reading of
RFC-078's own waiver policy it is **un-waivable**, which makes it a **third Gate
D blocker** alongside F44 (§4). That is the answer to macOS's question 5, and it
matters more than any case result in either request.

I also established that **F68 and F73 share one root cause and one fix** (§5) —
neither request had connected them.

## 2. Prerequisite A — resolved, and it answers a question I left open

Review 055 raised that F34's geometry branch had never been observed firing, and
I tried twice to trigger it with CSS mutations and failed. This explains why:

> a `display:table-cell` div's own AT-SPI accessible box position comes from the
> table-layout algorithm's column placement, not its CSS padding/margin —
> padding moves content *inside* the cell's box, not the box
> `Atspi.Component.get_extents` reports.

`transform: translateX()` moves the painted position and therefore the reported
box. Verified in the request against a locally-built binary: `padding-left`
produced zero change across all seven rows; `translateX(20px)` shifted exactly
the mutated row.

That closes review 055's N1 properly — the branch is real, not vacuous — and it
corrects my own diagnosis, which assumed my mutations were reaching the layout
and something else was swallowing the effect.

**On question 1 (is `translateX` too narrow a mutation?):** No. The check
asserts a *property* — every content cell in a pane shares an x-origin — and any
defect that breaks that property is caught regardless of what causes it. The
injection only needs to violate the property once, plausibly; it does not need
to resemble F32's specific cause. `inject_f32_defect.py` already covers the
structural shape, so the two together span both branches.

## 3. F63 — resolved as a harness artifact, and the evidence is decisive

The three dispatches are well designed, and the second and third are what settle
it:

- one isolated `find_text` call → **`FOUND` after 95.8s**, well past the
  30–100 line threshold M5-B recorded;
- `count_rows` alone → `'0'` in 0.8s, then **`'82'` in 24.0s on an immediate
  second call, same launch**.

A first bulk query returning fast-but-incomplete, and a second returning correct
results slowly, is not "content invisible to assistive technology." It is
WebKit's accessibility-tree computation being slower than the harness's
timeouts. **F63 is a harness artifact**, and M5-B's macOS P06 reduced scope was
built on a misdiagnosis rather than a product limit.

**On question 1 (does the VoiceOver caveat block closure?):** No, and you were
right to state it rather than resolve it. VoiceOver queries incrementally rather
than by bulk fetch, so the harness's latency says little about a real user's
experience — but that is an *unmeasured* question, not an unresolved finding.
Closing F63 as a harness artifact is correct; the VoiceOver question is a
separate, new, and much narrower thing. If it matters to Gate D it should be
raised as its own item, not left attached to a closed finding.

**Retrofitting M5-B's P06** with this understanding is worth doing eventually,
and you were right not to do it here — it is not this slice's case, and touching
a completed row's evidence mid-matrix is exactly what the freeze prevents.

## 4. F73 is un-waivable — the answer to macOS question 5

You asked whether P07's **Pass** is the right Gate D input given it contains two
real product defects. **The Pass is accurate; the framing is not.**

The case's assertions genuinely passed. But RFC-078's waiver policy says:

> **No waiver may turn these into a release pass:** … **wrong-file/stale-load
> behavior** …

F73 is wrong-file behaviour in the plainest sense. Your own evidence: a per-row
"Copy to right" click landed at `$HOME/aaa-changed.txt` — **with a real backup
and overwrite** — while the file actually shown as Changed was verified
untouched, and **no error surfaced**. A copy tool writing to a location the user
did not choose, silently, overwriting what was there, is precisely the category
the policy names.

So:

- **P07 stays Pass** — its assertions passed, and rewriting that would misstate
  what was observed.
- **F73 becomes a separately weighted Gate D input**, and on this reading a
  **blocker**, not a note inside a passing case.

That is exactly why the handoff asked for a Gate D input list distinct from the
case results: a case result says what the checks found; the input list says what
the evidence *means*. Folding a blocker into a Pass loses the second.

**Gate D now has three un-waivable inputs**, not one: F44 (cannot launch on a
supported platform), F61 (fixed, pending a new candidate), and F73.

## 5. F68 and F73 are one defect with one fix — verified

Neither request connected these, and the source makes it plain:

```rust
pub fn DeepCompareView(left_root: PathBuf, right_root: PathBuf, lang: Lang)  // :25
    BatchCopyButtons { entries, left_root: …, right_root: … }                // :157  ← gets the roots
fn DeepRow(entry: RecEntry, lang: Lang)                                      // :183  ← does NOT
    let has_left_root = store.settings.read().last_left_dir.is_some();       // :203  ← substitutes
```

**`DeepRow` is never passed the compare roots**, so it substitutes Explorer's
remembered pane directories. That single omission produces both registered
defects:

- **F68** — the buttons vanish when `remember_explorer_dirs` is off, because the
  substitute is `None`;
- **F73** — the buttons write to the wrong place when the substitute differs
  from the real roots.

And `BatchCopyButtons`, one line above in the same file, **already receives the
roots** — which is exactly why the batch path is correct and you confirmed its
manifest `src` came from `right_root`. The correct pattern is present in the
same component tree; `DeepRow` simply does not use it.

**One fix — pass `left_root`/`right_root` into `DeepRow` — closes both.** That
is worth knowing before either is scheduled, and it lowers the cost of fixing a
Gate D blocker considerably.

## 6. Other answers

### Linux Q3 — harden the other walkers now

Yes, pre-emptively. Your stale-node fix to `render_check.py` is **shared
infrastructure every M5 row uses**, and the two crashes you hit (`find_by_role`,
then `find_text_containing` under `--break`'s longer retry) were the same bug
surfacing in whichever walker happened to run longest against a mutating tree.
The remaining walkers differ only in how long they run, not in kind. Waiting for
each to crash means discovering it during an evidence run, which is the most
expensive moment.

### macOS Q3 — recording Finding 3 as a technique limitation is right, with a caveat

Right call. You tested and ruled out ordering races, occurrence-index
miscounting and element staleness; "AppleScript's `click` verb reports success
without effect on the right pane" is an honest description of what was observed.

The caveat worth recording: you cannot currently distinguish it from a real
accessibility gap in the right pane. If a VoiceOver user also cannot operate
those controls, that is a product defect in an area this project makes explicit
claims about. Not something to resolve now — but note it as an open question
attached to Finding 3, so a future macOS slice does not read "harness
limitation" as "product is fine."

### macOS Q4 — the scroll gap is acceptable for this row

RFC-078 requires P03 in full only on WebKitGTK; macOS's depth is a basic layout
observation, and you exceeded it. A genuine, evidenced technique limitation —
`AXScrollBar=0`, both orientation attributes `missing value` — recorded as
unverified rather than skipped is the correct outcome. It does not block macOS's
assessment.

### Linux Q2 / the scroll fallback chain

Trying three conventions and recording which worked is acceptable, and better
than asserting one — the alternative already failed once on real CI (button-7
silently swallowed by Xvfb's virtual pointer). Your §9 caveat is the right one:
it is not yet known whether the working method is stable across CI images. Note
it in the evidence rather than treating the chain as settled.

## 7. Notable quality observations

- Resolving all three prerequisites *before* gathering evidence against them, in
  three independent efforts, without being chased.
- The macOS F63 investigation isolating one variable per dispatch, and
  identifying that its own earlier heavy queries were degrading the WebProcess —
  diagnosing a confound the investigation itself introduced.
- Linux disclosing that the sandbox's local X11 synthesis is broken **for
  everything**, so every harness fix needed a real CI round-trip. That correction
  saves the next person real time and admits a prior assumption was too narrow.
- macOS preserving the ruled-out technique families in the `p07` docstring
  rather than smoothing the history away.
- Both rows finding harness bugs and calling them harness bugs, while separately
  finding product defects and calling those product defects. The distinction held
  under pressure in both.
- Linux Q3's framing — asking whether to harden pre-emptively rather than
  quietly doing it or quietly not.

## 8. Recommended next action

1. **F68 + F73 as one fix** (§5) — pass the roots into `DeepRow`. F73 is a Gate D
   blocker; F68 comes free with it. Needs a new candidate.
2. **Harden the remaining recursive walkers** (§6, Linux Q3).
3. **Evidence assembly** — all three rows are in. The **Gate D input list** now
   carries F44, F61, F73 as blockers, plus F45, F46, F60, F69, F70, F72, and the
   keyboard-coverage gap as inputs.
4. F63 closes; attach the VoiceOver-latency question to Finding 3 as an open
   item rather than to F63.
