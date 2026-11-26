# Handoff 114 — RFC-101 stage B: catalogs, whole sentences, plurals

**From:** architect. **Date:** 2026-11-24. **Release:** `0.201.0`, first.
**RFC:** `rfcs/accepted/101-translation-groundwork.md`. Read §1–§3 and
*Stage A's decisions*. Stage A's measurement and lists are in reply 198,
decided in review 232.

**Runs alongside handoff 115** (RFC-102 stage A). Handoff 116 (RFC-102
stage B) waits for this one, so its new words go straight into the
catalog.

## Rules

- **Tests run locally with the XDG folders redirected** only if needed.
  F237 made the suite safe by construction, so plain `cargo test` is fine
  now.
- **The register is mine.**
- **The owner's review is on the critical path.** Send the list (§3)
  **first, before the migration is finished.**

## Order of work

1. **The format and its build step:**
   - our own format (`{ $name }` placeholders, `id.one` / `id.other` plural
     forms);
   - **parsed at build time** into Rust tables. A malformed line fails the
     build;
   - one file per language under `crates/forskscope-ui/i18n/`;
   - **stable message IDs,** not English text, as keys.
2. **The API:**
   - `t(lang, id)` for a plain message;
   - one call with named arguments and an optional count for the rest;
   - **CLDR plural rules** for English (*one*, *other*) and Japanese
     (*other*), with tests from CLDR's examples.
3. **The owner's list, sent first** (as its own reply,
   `.git-exclude/review-request/202a-owner-wording-list.md`):
   - stage A's items 2, 5 and 12–17, each with the Japanese before and
     after, and the English for context;
   - **plus** the Japanese list separator for the Deep Compare summary
     (§3.3 of review 232).
   
   I pass it to the owner together with handoff 115's list. **Until the
   owner answers,** those items keep today's wording in the catalog.
4. **The migration:** every existing message into the catalogs. **The
   about-18 assembled messages are rewritten as whole sentences,** using
   the wording the owner approves for the flagged ones.
   - **The Deep Compare summary is a list:** one message per item, joined
     by the catalog's separator.
   - **User data in a placeholder** (a name, a path, error text) is
     isolated in the interface (`<bdi>` or `unicode-bidi: isolate`).
     Tested with a Hebrew file name in an English and a Japanese sentence.
   - **The HTML report's `tr()`** moves to the same catalogs.
5. **The audit's checks:**
   - every language has every message;
   - no unused message;
   - **each translation uses exactly the English placeholders;**
   - plural forms match each language's rule;
   - **no code joins two translated results,** except the named list
     pattern and the About dialog's diagnostics (each exception named,
     with its reason).

## The safeguard: a snapshot of every message

**Before the migration,** render every message in English and Japanese
with fixed example arguments, and write the result to a file. **After the
migration,** render again. **Diff the two.**
- **Only the owner-approved rewrites may differ.** Every other line is
  identical, byte for byte.
- **Show the diff in the reply,** with every differing line matched to its
  approved item.

## Verification

- **Each new audit check falsified** (RFC-101's verification §3).
- **The snapshot diff** (above).
- **Test names diffed.**
- **RFC-098's gate green on CI.** The rewritten sentences change some
  lengths.
- **No visible change** beyond the approved rewrites. One capture each of
  the settings recovery dialog and the Deep Compare summary, in Japanese.
  Look at them in full.

## Gates

The standard set, including `i18n`, Windows GNU clippy and both audits.
Reply in `.git-exclude/review-request/202-catalogs.md`, after the list
in `202a`.

## Round 2 (2026-11-25, review 238)

**The mechanism is approved.** In this order:
1. **Settle the settings-loading question first:** run the `0.200.0`
   binary and `main`'s with the same seeded settings. Reply in
   `.git-exclude/review-request/202b-settings-load.md`.
2. **`tf()` never panics in a release build,** and the audit checks that
   every plural call passes a count.
3. **Isolate every placeholder that holds user data,** including the tab
   error, which needs structured content.
4. **F238:** every message that bypasses translation goes into the
   catalogs, and the audit catches a notice whose text isn't from the
   catalog.
5. **Apply the owner's answers** to `202a` (file
   `.git-exclude/tasks/dev-team/owner-answers-202a-203a.md`, to follow).

Reply in `.git-exclude/review-request/202c-catalogs.md`.

## Round 3 (2026-11-25, review 239)

**Approved, apart from F238's remainder:** the recovery dialogs'
`.detail` texts and button labels (`action_label`).
- **List them first:** the count, and where each is shown.
- **Then migrate them,** through the same `AppErrorKind` mapping.
- **Extend the notice audit** to whatever renders them, or say why it
  can't reach them.

**Any Japanese wording change goes to the owner first.** The two short
copy labels are being shown to the owner now (review 239 §2.3).

Reply in `.git-exclude/review-request/202d-catalogs.md`.

## Closed (2026-11-25, review 240)

**Complete.** F238 is done.
