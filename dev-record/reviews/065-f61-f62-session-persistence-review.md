# F61/F62 review — session persistence and its silence

**Review date:** 2026-08-16
**Request:** `dev-record/review-requests/062-f61-f62-session-persistence.md`
**Baseline:** `ecc5a28`
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/f61-f62-session-persistence-handoff.md`
**Review mode:** Independent verification against the real application on a real desktop.

## 1. Verdict

**F62: approved.** The error-handling work is correct and valuable.

**F61: not resolved. Returned.** I re-ran review 064's reproduction against your
fixed build, and **a CLI-opened tab still does not persist** — in both directory
conditions. The register currently marks F61 Resolved; that must be corrected
before anything else, because a Gate D blocker is recorded as cleared when it
is not.

This is not a criticism of the work. You fixed a real bug and built real
infrastructure. But the mechanism in §1 of your request is not the mechanism the
product exhibits, and the regression test now passes while the product still
fails — which is a worse position than before, because a green test will guard
the wrong thing.

## 2. The evidence

All on `ecc5a28`, real GUI process, real desktop AT-SPI bus, isolated
`XDG_CONFIG_HOME`.

**Case A — config directory pre-created** (review 064's exact conditions):

```text
forskscope <left> <right>, 14s   → session.json? NO
```

**Case B — genuinely fresh profile, no `forskscope/` subdirectory** (the case
your fix targets):

```text
forskscope <left> <right>, 14s   → session.json? NO
                                 → forskscope/ directory: NOT CREATED
```

**Case B is the decisive one.** Your fix calls `fs::create_dir_all(parent)` as
the *first* statement of `atomic_write_envelope`, before any write is attempted.
If the effect had fired and reached that function, the directory would exist
even if the write then failed. It does not exist. **Therefore
`atomic_write_envelope` is never called** for a CLI-opened tab in the real
application.

**Control, same run as Case A:** invoking the tab's close button via AT-SPI
produced `session.json` immediately. So the process can write to that directory,
the flag permits it, and the repository layer works. The defect is upstream of
all of it, exactly as review 064 found.

**Method verified before concluding**, because this contradicts your report:

| Check | Result |
|---|---|
| Binary is the fixed one | Confirmed — contains F62's `Could not save session` string |
| `XDG_CONFIG_HOME` honoured | Confirmed via `--diagnostics` |
| The tab actually opened | Confirmed — `Close <left> ↔ <right>` present in the AT-SPI tree |
| The app can write there | Confirmed — close-tab wrote the file |

## 3. What this means about the diagnosis

Your §1 states: *"the reactive `use_effect` on `store.tabs` was firing correctly
the whole time… The write beneath it was failing."*

Case B contradicts that in the real app. The write is not failing — it is not
being attempted.

**Both observations can be true of different environments.** Your regression
test renders `App()` inside a `VirtualDom` and drives
`wait_for_work`/`render_immediate` manually. That harness evidently *does* run
the effect, which is why you saw a genuine `NotFound` toast. The real desktop
runtime evidently does not, or not before the process is asked to do anything
else. **The harness and the product disagree**, and the fix was made against the
harness.

That is also why the `--test-threads<=2` dependence matters more than it looked:
a test whose behaviour changes with scheduler pressure is a test whose
scheduling differs from production. Your §10 flagged it as an unresolved
infrastructure limitation; I would now read it as a signal that the harness is
not reproducing the production effect lifecycle.

**Leading hypotheses, none established — do not fix from this list:**

- the effect runs once before the startup tab is added and is never re-triggered
  by the asynchronous `open_compare` completion (RFC-075's load path);
- `session_write_disabled` is true during the only window in which the effect
  fires, making `save_session_if_allowed` a silent `Ok(())` no-op — note
  `resolve_session`'s own doc anticipates precisely this ordering concern;
- the effect's subscription does not register as intended.

Establish which by observing the real process, not the harness. Since F62 now
surfaces failures, an instrumented run that logs whether `save_session` is
entered at all would separate "never called" from "called and returned early" in
one attempt.

## 4. F62 — approved, and it did its job

The handoff predicted F62 would convert guesswork into observation, and it did —
just not to the conclusion you drew. Specifics that are right:

- `Result` returned rather than discarded, with `Result`'s own `#[must_use]`
  doing the enforcement, so a future `let _ =` is a compile error under
  `-D warnings` rather than silence. Removing the redundant `#[must_use]` that
  clippy flagged rather than suppressing the lint is the right call.
- A uniform toast for startup and user-initiated failures. Your reasoning is
  sound and I would not change it: the user's data is equally at risk either
  way, and RFC-076's recovery path genuinely addresses a different problem
  (what is safe to *read*, not a failed write). See §5.1.
- RFC-076's write-disable guarantee left intact, with the test narrowed to
  assert `Ok(())` rather than the disabled behaviour itself.

The `create_dir_all` change is also a real fix for a real bug — a genuinely
fresh profile would fail to write once the effect problem is solved. **Keep it.**
It is simply not F61.

## 5. Answers to the requested review focus

### 5.1 Uniform toast — right call

Keep it. A startup-time failure is not less serious for being unprompted; if
anything it is more, since the user has no action to associate it with and will
discover the loss at the next launch. A dedicated startup notice would be
worth revisiting only if these prove noisy in practice, which no evidence
suggests.

### 5.2 The `#[ignore]` — acceptable as a record, not as the deliverable

Documenting the limitation instead of substituting a weaker test was right, and
the `--test-threads` narrowing (5/5 at N≤2, 5/5 failing at N≥3, with
`DIOXUS_VDOM_TEST_LOCK` ruling out cross-test interference) is genuinely good
investigation.

But §2 changes what the test means. It currently asserts a behaviour the product
does not have, so it would pass while the defect ships. **The regression test
must fail on today's `main` before it is worth keeping** — that is the
falsifiability standard this program applies to every other check, and it was
not applied here because the fix and the test were developed against each other.

Of your unattempted options, **the subprocess test is now clearly the right
one**: launch the real binary with real CLI arguments against an isolated
`XDG_CONFIG_HOME` and assert on the file. It sidesteps the harness/production
divergence entirely rather than trying to model it, and review 064 and this
review have both now done exactly that by hand — it is a shell script away from
being automated.

### 5.3 Fix placement — reasoning is right

`atomic_write_envelope` for config-managed directories, not
`crate::save::atomic_replace`, is the correct line and not merely convenient. A
user's chosen Save As target not existing is a situation they should hear about;
a config directory the app owns is one it should create. Keep it as scoped, and
keep the `ensure_pre_v2_backup` reasoning too — a path just read successfully
has an existing parent by construction.

### 5.4 The sandboxed-environment limitation — this is the whole lesson

You asked whether the missing GUI verification is acceptable given the
regression test's coverage. **It was the gap that let this through**, and both
of us should take the point:

- You flagged it honestly and recommended a real desktop smoke test before
  cutting a candidate. That recommendation was correct and, followed, would have
  caught this.
- I wrote a handoff requiring "runtime confirmation that a CLI-opened tab now
  persists" without saying *on a real desktop process*, which left a
  VirtualDom-rendered test as a defensible reading. That is my omission.

The rule worth keeping: **a defect discovered by running the product is not
resolved by a test that does not run the product.**

## 6. Required to close F61

1. **Correct the register** — F61 back to open, with what is now known: the fix
   landed, the defect persists, `atomic_write_envelope` is never reached. F62
   stays resolved.
2. **Establish the real mechanism** by observing the real process (§3).
3. **Fix at that level.**
4. **A test that fails on today's `main`** — the subprocess shape from §5.2.
5. **Real-desktop confirmation**: `forskscope <left> <right>` on a real display,
   isolated config, `session.json` present without touching the tab list. If the
   sandbox still cannot launch a GUI process, say so and hand that step to the
   owner rather than substituting a harness result.

## 7. Notable quality observations

- Following the handoff's F62-first ordering exactly, and reporting what the
  toast said rather than what you expected it to say.
- The `--test-threads` narrowing, and building `DIOXUS_VDOM_TEST_LOCK` to
  eliminate a hypothesis rather than assume it away.
- Finding and fixing the `Dropped(ValueDroppedError)` ordering bug in your own
  test, and distinguishing it from the thread-count issue instead of merging two
  problems.
- Confirming `with_test_store` genuinely cannot reach `App()` rather than
  asserting it — that is a real F36 finding.
- Flagging the GUI-verification gap and recommending the smoke test. You
  identified the exact hole this review fell into.
