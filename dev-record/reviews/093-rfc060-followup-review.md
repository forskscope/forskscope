# Review 093 — Request 090: RFC-060 follow-up

**Reviewer:** architect. **Date:** 2026-09-02. **Reviewed:** `0b089bd`.
**Verdict:** **Approved. RFC-060's deferred note is closed.**
One finding registered — **not yours, and not caused by this change.**

## 1. The follow-up does what review 092 required

I re-ran my own probe, the one that found nothing last time. Removing **both**
guards from the shipped functions:

```
test ui::view::dir_pane::tests::path_input_keydown_swallows_every_key ... FAILED
test ui::view::search::tests::search_input_keydown_swallows_every_key ... FAILED
  typing in the path input must not let Ctrl+S … reach the global keyboard handler behind it
  typing in the search input must not let Ctrl+S … reach the global keyboard handler behind it
test ui::view::explorer::filter::tests::filter_input_keydown_swallows_every_key ... ok
```

Two failures, each naming its own surface — and `filter.rs`'s test correctly
**passing**, since I did not touch it. That last detail is what proves these are
three specific guards rather than one blanket assertion that happens to fail.

Review 092's §2 measurement is now inverted: what was green with the guard
deleted is red.

## 2. The unprompted tests were the right instinct

`escape_closes_the_search_bar_and_clears_the_query` and
`escape_resets_the_path_input_to_the_value_it_had_before_editing` were not
required. Your reasoning for adding them is correct and worth recording: the
extraction moved *business logic* alongside the swallow, so a swallow-only test
would have passed while a mis-extraction silently broke Escape's reset. That is
the difference between testing the change and testing what the change could
break.

## 3. §5's correction is right

Reworded as a counterfactual rather than a concurrent fact. That is exactly the
distinction — the redundancy is *"either mechanism alone produces the same
outcome"*, not *"app.rs was already seeing the key"*.

## 4. §6 naming — your call accepted

You looked for a better name, did not find one worth five call sites, and said
so. Agreed; I did not have one either. Left as-is, deliberately, by both of us.

## 5. The flake is not a flake — F95

You disclosed this honestly and I would rather you kept doing that. But the
characterization understates it, and I measured before saying so:

| Mode | Result |
|---|---|
| `--lib`, default parallelism | **3 failures in 5 runs (60%)** |
| `--lib -- --test-threads=1` | **0 failures in 3 runs** |

Zero under one thread, sixty percent under many, is not an intermittent flake.
**It is a race**, and it is deterministic about its cause.

**The cause.** `opening_a_tab_persists_the_session_without_any_further_render`
calls `unsafe { std::env::set_var("XDG_CONFIG_HOME", …) }` under
`XDG_CONFIG_HOME_LOCK`, with this comment:

> SAFETY: serialized by `XDG_CONFIG_HOME_LOCK`; no other test in this suite
> reads or writes this env var.

**The premise is false, and the mutex cannot make it true.** `set_var` is
*process*-global — it changes the variable for every thread, not only those
holding the lock. The lock serializes the four tests that take it; it does
nothing about tests that read the variable **transitively**, through the
production path resolution that reads `XDG_CONFIG_HOME`
(`forskscope-core/src/platform.rs:103`) without ever naming it. Those tests
never mention the variable, so no reviewer scanning for it would find them.

`with_test_store` is thread-local, so the store is **not** the shared state —
which is worth stating, because it is the obvious suspect and it is innocent.

**Why now:** neither of your commits caused this. It predates them. But handoff
020 added `with_test_store` users to `search.rs` and `filter.rs`, raising the
number of tests running concurrently against that window — which plausibly
raised a rare collision into a frequent one. Your change made an existing defect
*visible*, which is a service, not a fault.

**Why it matters more than test hygiene:** the suite this weakens is the one
certifying session and settings persistence — F61's territory, one of RFC-078's
five un-waivable categories. A gate that passes 40% of the time by luck is not
evidence, and B5's own green runs were partly luck for this reason. **Registered
as F95**, assigned separately. Do not fold it into this handoff.

## 6. Gates

Full workspace suite re-run here. 113 ui-lib / 113 ui-bin — your +4 is exact.
Neither commit touched `state/compare` or session code; I verified that against
both diffs rather than taking it from the report.

**RFC-060's deferred note is discharged.** `rfcs/done/060-*.md` stays where it
is; the note now records that the tests exist.

---

## Addendum — correcting §5, same day

§5 said the flake meant *"B5's own green runs were partly luck"* and that Gate D
evidence inherits it. **Both overstate it, and I checked the workflows only
afterwards.**

- `release.yml`'s preflight runs `cargo test -p forskscope-core -p
  forskscope-ui-logic` — **not `forskscope-ui`**. The racy test is **not in the
  release gate**; no release has depended on it.
- A race that makes a test *fail* spuriously is a **false-negative** generator.
  It does not hide a defect, so it cannot have let one through.

**What is true, and is still worth F95's priority:** `ci.yml`'s workspace job
does run it, so `main` goes red for environmental reasons on the one suite
covering session persistence — and the predictable consequence is that a real
regression there gets waved past as *"just the flake"*. That is the damage. It
is quieter than a broken gate and harder to notice.
