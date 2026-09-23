# Developer Handoff 008 — F80: Deep Compare's status glyphs need accessible labels

**From:** architect
**Date:** 2026-08-22
**Register:** F80. Not a Gate D blocker.
**Size:** Small. Most of the strings already exist.

---

## 1. Task title

Give every Deep Compare status an accessible label, and collapse the one-status
special case F79 left behind.

## 2. Purpose

`deep_compare.rs`'s row renders `span { class: "dir-status {cls}", "{icon}" }` for
`Changed`, `LeftOnly`, `RightOnly`, `Equal`, `Computing` and `Symlink` — glyph and
colour only. RFC-009 §7 lists **screen-reader text as its own requirement**
beside symbol-or-label, and a bare `⚠` or `←` announces as its character rather
than its meaning.

**It matters more here than it did in the Explorer**, because these statuses gate
the copy controls: `can_copy_left_to_right`, `can_copy_right_to_left` and
`can_cmp` all key off them. A screen-reader user currently cannot tell **why** a
row offers no copy button.

## 3. Background

This is the F74 defect in the view F74 never touched. Review 072 fixed it in
`dir_pane.rs`; `deep_compare.rs` was not in that handoff's scope and kept the
defect while the Explorer lost it.

Raised by you, in request 074's Q3, asking whether F79 should have retrofitted
the other five. **It should not have** — five new localised strings and a design
pass do not belong inside a defect fix. Declining was right, and this handoff is
the result of asking.

## 4. Change scope

- `crates/forskscope-ui/src/ui/view/deep_compare.rs`
- `crates/forskscope-ui/src/i18n.rs` — one new key (§5)

## 5. Required implementation

Every status gets `role="img"` + a localised `aria_label`, using the pattern F74
established and F79 already used for `Unreadable`.

**Collapse the special case.** `42a9ccb` left:

```rust
if entry.status == RecStatus::Unreadable { …labelled span… } else { …bare span… }
```

That was honest at the time — one status labelled, five not. **It must become a
single span** for all seven. Review 076 recorded that a second special case added
later would be worse than the uniform absence it started from.

**Six of the seven strings already exist**, added by the Explorer work — reuse
them exactly rather than inventing near-duplicates:

| Status | Label | Key exists? |
|---|---|---|
| `Changed` | Different | yes |
| `LeftOnly` | Only on the left | yes |
| `RightOnly` | Only on the right | yes |
| `Equal` | Identical | yes |
| `Computing` | Comparing… | yes |
| `Unreadable` | Unreadable | yes |
| `Symlink` | *(new — see below)* | **no** |

**`Symlink` is the one that needs thought, not just a string.** Core's own doc
says ForskScope *"does not follow cross-root symlinks to avoid cycles; the entry
is reported and left to the caller to act on."* So the honest label says the link
was **not followed**, not merely that one exists — otherwise it reads as a
verdict about the target. Propose the wording in your review request; I will
review it as wording, not rubber-stamp it.

## 6. Explicit non-change scope

- **Do not change any glyph.** See §8 — the divergence is real and is not yours
  to resolve inside this handoff.
- **Do not touch `RecStatus`**, its variants, or any classification logic. This
  is presentation only.
- **No behaviour change.** Copy gating, filtering and counts stay exactly as they
  are.
- F75(b), RFC-080 — untouched.

## 7. Required tests

Weaker than usual, and I want that stated rather than disguised: **whether a
label reaches the accessibility tree is a P07 assertion (AT-SPI/UIA), not a unit
test.** That limit is already recorded for F74 and has not changed.

What is unit-testable, and what I want:

1. **Every `RecStatus` variant yields a non-empty label in every language** — the
   same shape as `dir_pane.rs`'s existing
   `every_digest_state_has_a_non_empty_label_in_every_language`. Falsify by
   returning an empty string for one variant.
2. **No two statuses share a label** except where deliberate — and after this
   change, none should. Falsify by pointing two variants at the same key.

Do **not** claim these prove the labels reach a screen reader. They prove the
strings exist and are distinct.

## 8. Recorded, and deliberately not fixed here

**The two views now show different glyphs for the same concepts**, and that
divergence is *new* — review 077 moved the Explorer onto `RowStatusKind`'s glyphs
(`=`, `≠`, `…`) while Deep Compare kept its own (`✓`, `⚠`, `⊙`). Same app, same
meaning, two symbols.

That is recorded against F80 as an observation. It is a **design decision about a
shared display vocabulary**, not a defect to fix in passing, and resolving it
badly — by copying one set into the other file — would create a third place where
glyph choices live. Leave it. If you have a view, put it in the review request.

## 9. Acceptance criteria

- One span, no per-status branch.
- Every status: `role="img"`, a localised `aria_label`, and its existing glyph and
  CSS class unchanged.
- `cargo xtask i18n` passes with the new key translated.
- No glyph, no class, no behaviour changed.
- Gates green: `fmt`, `clippy --workspace --all-targets -- -D warnings`, `test
  --workspace`, `xtask css --check`, `xtask i18n`, `git diff --check`.

## 10. Prohibited shortcuts

- **Do not add a second `if status == …` branch.** One span.
- **Do not invent new strings for the six that exist.** A near-duplicate is a
  translation burden and a divergence waiting to happen.
- **Do not describe a symlink as compared.** It was not.
- **Do not report a falsification you did not run.**

## 11. Required review-request format

Short is fine — this is a small change. Lead with the two falsifications.

State plainly:
- the `Symlink` wording you chose and why;
- that no glyph, class or behaviour changed;
- whether you think the glyph divergence in §8 should be resolved, and how.
