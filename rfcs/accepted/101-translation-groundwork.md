# RFC 101: Translation Groundwork — Whole Sentences, Plurals, Catalogs and a Pseudo-Language

**Status.** Accepted.
**Proposed.** 2026-11-21 by the architect. The owner asked whether to add
right-to-left support and more languages, weighing maintenance cost
against usability. The answer:
- **no new languages yet,** and no right-to-left interface;
- **first, groundwork** that lowers the cost of every future language;
- **a policy** for when a language is added.

The owner accepted that direction the same day ("All accepted. Proceed.").
This document is the design for that acceptance.
**Accepted.** 2026-11-21 by the project owner ("Yes. Both accepted.").
**Scheduling.** Stage A (the measurement) in the `0.200.0` cycle (handoff
110); stages B and C in `0.201.0` (owner's scope, 2026-11-21).
**Touches.**
- `crates/forskscope-ui`: `i18n.rs`, and every call site that assembles a
  sentence;
- `xtask`: the `i18n` audit, and RFC-098's layout matrix;
- `CONTRIBUTING.md`: the language policy (§5);
- `docs/src/maintainers/`: the architecture and testing pages.

## Summary

- **Every message is one whole sentence with named placeholders,** never
  pieces joined in English word order.
- **Counts use plural forms,** by each language's rules.
- **Translations live in one catalog file per language,** not inside Rust
  code.
- **A test-only pseudo-language joins RFC-098's layout matrix,** so every
  screen is proven to survive long translations, with no translator
  needed.
- **A written policy decides when a language is added, and when it's
  removed.**

## Motivation

**Today** (`crates/forskscope-ui/src/i18n.rs`):
- **English strings are the keys,** and Japanese is a Rust `match` of about
  430 entries. Anything missing falls back to English.
- **About 20 messages are built from translated pieces in English word
  order,** for example `format!("{} \"{}\" {} {}, {} {}", …)` in the
  settings recovery dialog. Such sentences read wrongly in most languages,
  and some already read awkwardly in Japanese.
- **There are no plural forms,** so "1 different" relies on the language
  having none.
- **A translator has to edit Rust** to contribute.
- **The layout checks know English and Japanese only.** Nothing tells us
  whether German's longer words or a 40 % longer translation fit.

**Each of these makes every future language more expensive.** Fixing them
once is cheaper than paying for them per language.

## Design

### 1. Whole sentences with named placeholders

- **Every user-visible message becomes one catalog entry,** a complete
  sentence with named placeholders: `{file}`, `{count}`, `{version}`.
  The translator moves them as the language needs.
- **The about-20 assembled messages are rewritten.** No two translated
  pieces are ever joined in code. Glue text (`": "`, `" — "`) belongs
  inside the sentence.
- **The `i18n` audit rejects a new assembly:** a `format!` whose arguments
  include two or more translated results. A named exception needs a
  stated reason, as the existing gates' allowlists do.

### 2. Plurals

- **A message with a count names its plural forms:**
  - English uses *one* and *other*;
  - Japanese uses *other* only.
- **The rules come from CLDR,** written as one small function per
  supported language, with tests from CLDR's own examples. A new
  language's rule arrives with that language.
- **The call sites that show counts use it:**
  - Deep Compare's summary;
  - the change counter;
  - search results;
  - the status notices.

### 3. One catalog file per language

- **The translations move out of Rust,** into `crates/forskscope-ui/i18n/<lang>.<ext>`:
  - stable message IDs, not English text, as keys;
  - placeholders and plural forms;
  - a translator's comment where the meaning isn't obvious.
- **The format is decided in stage A by measurement,** between:
  - **Fluent** (`.ftl`), Mozilla's format, built for word order and
    plurals and supported by translation platforms. It costs a set of new
    runtime crates (`fluent-bundle` and its dependencies).
  - **A small format of our own,** parsed **at build time** into Rust
    tables. It adds no runtime dependency, but no tool knows it.
  
  **Stage A measures:**
  - for Fluent: the new crates (`cargo audit`, licences, upkeep), and
    binary size against `0.199.0`;
  - for ours: the parser's size and its tests.
  
  **My recommendation, before the numbers, is our own format.** There are
  no translators yet, the owner weighs maintenance cost, and every runtime
  crate is one more to audit. **Fluent wins if a translation platform is
  ever adopted,** and the catalog is designed so that a later conversion is
  mechanical.
- **The audit checks:**
  - every supported language has every message;
  - no message is unused;
  - every translation uses exactly the placeholders of the English
    message;
  - plural forms match the language's rule.

### 4. A pseudo-language in the layout check

- **A test-only language,** never offered in a release:
  - generated from English at test time;
  - every letter accented (`Séttîñgš`);
  - each message stretched by 40 %;
  - each message bracketed (`⟦…⟧`), so an untranslated or cut-off string
    shows.
- **It joins RFC-098's layout matrix as a third language** on the layout
  row. That's 180 → 270 measurements, about 30 seconds more per push.
- **Its first run is a triage,** as RFC-098's was: each finding is a real
  defect, registered, or a checker issue, fixed.

### 5. The language policy (for `CONTRIBUTING.md`)

**A language is added only when:**
- **a named native speaker commits to reviewing it,** for every release
  that changes its text;
- **the full app is translated,** and passes the audit, the layout gate
  and the reviewer's read-through;
- **its plural rule is in place** (§2).

**What is translated:** the app's interface. **The user guide stays in
English.** A translated guide multiplies the cost of every documentation
change, and the guide is maintained continuously.

**Machine translation is welcome as a first draft.** It's never the
finished text: the reviewer reads every message in context.

**A language whose reviewer has stepped down,** and which falls behind by a
release, **returns to English** (it's removed from the language list).
**Stale text confuses users more than English does.**

**A right-to-left language** (Arabic, Hebrew, Persian, Urdu) additionally
needs **its own RFC for a right-to-left interface.** It is started only
when such a language has a committed reviewer.

**English and Japanese are maintained by the project.**

## Stages

- **Stage A (measure, then stop for review):**
  - the format measurement (§3);
  - the list of every assembled message (§1), with its proposed whole
    sentence in both languages.
- **Stage B:**
  - whole sentences and plurals;
  - the catalog files and the migration of every existing message, with
    the Japanese reviewed by the owner where wording changes;
  - the audit's new checks.
- **Stage C:** the pseudo-language, and its first triage.

**The policy (§5) is already in `CONTRIBUTING.md`** (*Translations*). The
owner accepted the policy itself on 2026-11-21, and it governs requests
that may arrive before stage C.

## Non-goals

- **A new language.** This RFC only prepares for one.
- **A right-to-left interface** (§5).
- **Translating the user guide.**
- **Choosing the language automatically from the system** (it is chosen in
  Settings, as today).

## Security and privacy

- **No network.** No translation service is called, and nothing is
  downloaded at runtime.
- **If Fluent is chosen,** its crates go through the audit gate like every
  other dependency.

## Verification

1. **Test names diffed** before and after each stage.
2. **No visible change in English or Japanese,** except the messages
   rewritten as whole sentences, which are listed and reviewed one by
   one.
3. **The audit falsified**, once for each new check:
   - an assembled message;
   - a missing translation;
   - a stray placeholder;
   - a wrong plural set.
4. **RFC-098's gate green with the pseudo-language,** after its triage.
5. **The plural rules tested** against CLDR's examples.

## Open questions

- **The catalog format** is settled by stage A's measurement (§3).
