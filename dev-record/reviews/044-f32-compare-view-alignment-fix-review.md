# F32 — compare-view alignment fix review

**Review date:** 2026-08-04
**Request:** `dev-record/review-requests/041-f32-compare-view-alignment-fix.md`
**Baseline:** `cb6a852` (`fix: move sr-only diff label inside .cell to fix WebKitGTK row shift (F32)`)
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/f32-compare-view-alignment-handoff.md`
**Review mode:** Independent verification, including my own build, run, and
screenshot capture — not acceptance of the submitted images.

## 1. Verdict

**Approved. F32 is closed.**

The release-blocking defect is fixed, verified by my own capture from a build I
made at this commit rather than by inspecting theirs. The accessibility label
survives, confirmed against the real AT-SPI bus.

One observation, surfaced by their own evidence, registered for the
accessibility track. Not a regression and not blocking.

B3 and B4 remain open; v1/public release stays **No-Go**.

## 2. The change

Two lines, one file, and exactly the mechanism the handoff named:

```rust
div { class: "cell",
    if let Some(ref lbl) = sr_label { span { class: "sr-only", "{lbl}: " } }   // now here
    if let Some(ref spans) = inline_left { ... }
```

Every `.diff-row` now contains exactly three table cells — gutter, diff-mark,
cell — regardless of `HunkKind`. No CSS change was needed, which is the right
outcome: the CSS was never wrong.

## 3. Independent visual verification

I built `forskscope-ui` at `cb6a852`, ran it under an isolated `HOME` and
`XDG_CONFIG_HOME`, and captured the compare view myself
(`.git-exclude/tmp/shots/verify-f32.png`).

Changed rows now begin at the same left offset as unchanged rows, `−`/`+`
markers sit immediately after the line number, and no content is clipped at the
pane edge. The rendering is **identical to the diagnostic mutation capture**
(`02-mutation.png`) — which is what the handoff predicted, since deleting the
span and relocating it produce the same three-cell row.

I also reviewed their pure-Delete capture. `line two` aligns with the unchanged
rows around it, marker in place.

Worth crediting: the handoff pointed at the demo fixture, they diffed it, noticed
it produces only Replace and Insert hunks, and **built a second fixture to
exercise `HunkKind::Delete`** because that was the third code path. Nobody asked
for that. It is the difference between testing the fix and testing the fixture.

## 4. Accessibility verification

The handoff asked for confirmation "not merely that it exists in the source."
Querying `org.a11y.Bus` through `gi.repository.Atspi` — the bus Orca actually
reads — exceeds what I asked for, and it is the right instrument: it proves what
an assistive technology receives, not what the DOM contains.

Confirmed for all three label kinds, with the label preceding the content:

```text
'3 | Changed: #[derive(Debug, Clone)]'
'8 | Inserted:     pub ignore_hidden: bool,'
'2 | Deleted: line two'
```

## 5. Observation — from their own evidence

### N1 — Empty counterpart rows announce a label with no content

Their AT-SPI output includes:

```text
row 23: ' | Changed:'
row 24: ' | Changed:'
```

Those are the left pane's empty counterpart rows opposite the right pane's added
lines in a multi-line Replace hunk. They carry `kind == Replace`, so they receive
the `Changed:` label, but they have no gutter number and no content. A screen
reader announces "Changed:" on a blank row — four times for the one logical
change in this fixture.

**This is not a regression.** The label logic is untouched by F32; only the
span's position moved. It has behaved this way since the labels were introduced,
and the defect being fixed here is what made it visible, because the AT-SPI query
was run at all.

Whether an empty counterpart row should be labelled `Changed`, labelled
something else, or left unlabelled is an accessibility-semantics question for the
RFC-061 track, not for a layout fix. **Registered as F35.**

That their verification surfaced a finding beyond its own scope is the point of
doing verification properly rather than confirming the expected result.

## 6. Answers to the requested review focus

### 6.1 The mechanism

**Exactly what was intended**, and the better of the two options the handoff
permitted. The `aria-label`-on-row alternative would have satisfied the same
property, but the AT-SPI output settles it: putting the span inside `.cell`
produces `<gutter> | <label>: <content>` as one coherent accessible string with
the label first. An `aria-label` on the row would more likely have replaced or
competed with the row's computed name rather than prefixing it.

You picked the mechanism the evidence supports.

### 6.2 Promoting the fixtures and evidence

**Leave them where they are; F34 should decide the shape, not inherit these
artifacts.**

The demo fixture and the delete fixture were built for two different reasons and
neither is quite the right shape for a permanent regression check. If F34 adds a
visual step to release preflight, what it wants is **one fixture producing all
three hunk kinds** — Replace, Insert, and a pure Delete — in a single file pair,
so one capture covers every code path that carries a label. Building that
deliberately beats promoting two fixtures that happen to exist.

I have noted that in F34 so the requirement is recorded rather than rediscovered.

## 7. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test --workspace` | Pass — 1007, unchanged |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo xtask css --check` | Pass — `main.css` up to date |
| CI run `30864375956` | `success` on `cb6a8520` |
| Diff scope | One file, `+2/−2`; `forskscope-core` untouched |
| Three table cells per row, all hunk kinds | Confirmed by reading the DOM construction |
| **My own build + capture at `cb6a852`** | **Changed rows aligned, nothing clipped** |
| Pure-Delete hunk | Confirmed aligned |
| AT-SPI label survival, all three kinds | Confirmed, label precedes content |

Reporting the unchanged test count as evidence the gates ran clean rather than as
evidence the fix works was the correct framing, and it is what the handoff asked
for.

## 8. Recommended next action

1. Treat **F32 as closed**. The release-blocking defect is gone.
2. **F33** is now unblocked — the README and docs work, including screenshots
   from a build that renders correctly.
3. Patch 6 — recovery UI and documentation, carrying F28, F28b, and review 043's
   N1.
4. F23 before M2's cut; F24, F25/F25b, F31, F34, F35 at M4.
