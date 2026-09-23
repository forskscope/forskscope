# RFC-076 patch 4 — production switch review

**Review date:** 2026-08-03
**Request:** `dev-record/review-requests/037-rfc076-patch4-production-switch.md`
**Baseline:** `2a26f09` (`persist: switch forskscope-ui to RFC-076 versioned settings/session (patch 4)`)
**Governing documents:** RFC-076 §"User-facing behavior", §"UI integration"; handoff §4.4 step 4, §6, §9, §10; audit finding B2
**Review mode:** Independent verification against the repository; code read, not
description. No implementation changes made.

## 1. Verdict

**Corrections Required.** One mandatory correction, in the newly added
protection itself.

The patch is otherwise strong and B2's mechanism is genuinely in place:
`ConfigManager` is gone from `forskscope-ui` entirely, the adapter's
`..base.clone()` gives real forward-compatibility, and the manual run against
real backed-up config files is the kind of evidence this milestone has been
short of.

But `write_disabled` — the protection this patch adds — **does not engage in CLI
mode**, which is the project's flagship documented invocation. A future-version
or corrupt `session.json` is overwritten there. That is the exact outcome
RFC-076 forbids in as many words.

B2 is not yet closed. B3 and B4 remain open; v1/public release stays **No-Go**.

## 2. Mandatory correction

### C1 — `session_write_disabled` is never set in CLI mode, so the session file is overwritten

`session_write_disabled` is assigned in exactly one place — inside
`restore_session` (`state/session.rs:72`). And `restore_session` runs only in the
`else` branch of `app.rs`'s startup hook:

```rust
if let Some(Some((left, right))) = STARTUP_PAIR.get() {
    open_compare(...);            // CLI mode — restore_session never runs
} else {
    restore_session(&mut store);  // sets session_write_disabled
}
```

So with `forskscope left right`:

```text
1. STARTUP_PAIR is set   → restore_session never called
2. session_write_disabled stays at Store::new's default, false
3. open_compare pushes a tab → the tabs use_effect fires
4. save_session() sees write_disabled == false → proceeds
5. SessionRepository::save() atomically writes a v2 envelope
   over the user's future-version or corrupt session.json
```

`save()` performs no verification — established in patch 2's review — so there is
no second line of defence.

RFC-076 §"User-facing behavior" is explicit for `FutureVersion`: *"do not
overwrite … Continuing must not save over the future file."* For `Corrupt`:
*"preserve the file."* This path does neither.

**Reachability is high, not theoretical.** `forskscope old/src/main.rs
new/src/main.rs` is the first code block in the README, and the git
difftool/mergetool configuration in the same file makes CLI mode the way the
product is meant to be used day to day. A user in that mode with a future-version
session file loses it silently.

**Root cause:** settings resolution and session resolution are asymmetric.
`load()` runs unconditionally inside `use_context_provider`; session resolution
is buried inside `restore_session`, which is conditional. **Resolving a file and
restoring tabs from it are two different jobs** — the first must always happen,
the second only when no CLI pair was given.

**Required:** resolve the session file unconditionally at startup, setting
`session_write_disabled` and producing any notice; restore tabs only in the
no-startup-pair case. Add a test covering the CLI path — a future-version
session fixture plus a startup pair must leave the file byte-identical.

**Note this is a new hole in new protection, not a pre-existing bug.** Before
patch 4 there was no future/corrupt detection at all, so nothing was being
protected. Patch 4 introduces the protection and leaves this path outside it.

## 3. Verified

- **`ConfigManager` is gone.** Zero references to `ConfigManager` or
  `app_json_settings` anywhere under `crates/forskscope-ui/src/`. RFC-076's
  acceptance criterion is met for settings; the session half is met in code but
  undermined by C1's bypass.
- **`merge_into_v2` preserves unmentioned fields.** The `..base.clone()` tail
  means `appearance_font_size`, `density`, `show_line_numbers`,
  `wrap_long_lines`, `newline_policy`, `restore_session`, `recent_limit`, and
  `performance` survive a UI-driven save — **and so will any field added to
  `PersistedSettingsV2` after this code was written**, with no matching edit
  here. That is the right property, and identifying the naive-adapter trap before
  falling into it is the most valuable judgement in this patch.
- **Startup runs once.** The repository load and any migration commit sit inside
  `use_context_provider`'s initialiser, so they execute once per scope rather
  than per render. A migration attempted on every re-render would have been an
  easy and expensive mistake.
- **`persist()` and `save_session()` both honour their flag** as early returns.
- **Manual verification** against real backed-up v0 config files, with
  `.pre-v2.bak` byte-matching the originals and the theme conversion confirmed in
  rendered CSS rather than only in a unit test.

## 4. Answers to the requested review focus

### 4.1 View adapter versus retyping `Store.settings`

**The adapter is right, and the RFC agrees.** Its UI-integration section offers
"the canonical core type **or** a non-serializing view adapter" as equals. The
"one public canonical domain type" language from patch 1 is about *disk
ownership* — it exists to stop two types both claiming to be the persisted
format. `AppSettings` no longer claims that; it is an in-memory projection.

Your reasoning about the cost is also correct: `PersistedDiffProfileV2` has four
independent axes where the Settings dialog offers two checkboxes, so retyping
would push UI capability changes into a stabilization patch that adds no user
capability.

**One steer for patch 5, which now includes convergence cleanup:** do **not**
narrow `PersistedDiffProfileV2`'s axes on the grounds that the UI cannot produce
them. Narrowing a persisted schema is a schema change requiring a version bump,
and those axes mirror core's `CompareProfile`, which the diff engine genuinely
uses. Keep them and document the projection.

### 4.2 Toast versus the blocking dialog

**Acceptable here — for one specific reason.** Patches 4 and 5 ship in the same
release; there is no cut between them. No user ever experiences the toast-only
state, so the interim is an internal boundary rather than a shipped behaviour.

That reasoning carries an obligation: **patch 5's dialog is release-blocking for
M2's cut.** RFC-076 does not say "notify" for `FutureVersion`, it says "show an
incompatibility dialog offering Exit, Continue with temporary defaults." A toast
a user can miss would not satisfy that if M2 were cut before patch 5. I have
recorded this so the sequence cannot quietly collapse.

Your instinct not to reach into step 5's scope was right; the answer is to keep
the boundary and make the dependency explicit.

### 4.3 `Store::new`'s three parameters

**Your discomfort is pointing at C1.** The signature is lopsided because settings
resolution is passed in at construction while session resolution happens later
and elsewhere — which is precisely the asymmetry that leaves CLI mode
unprotected.

Fix C1 first. Once both documents resolve at startup, a small struct carrying
both resolutions — settings value, base, both write-disabled flags, and the
notice — becomes the obvious shape, and it will be motivated by the code rather
than by tidiness. Do not restructure the parameters on their own.

### 4.4 `DiffProfile` round-trip lossiness

**Accept as documented — and the owner's decision of 2026-08-03 makes the
argument stronger than when you wrote it.** The core-v1 migration path is being
removed in patch 5, so after that there is no producer of `IgnoreTrailing`,
`IgnoreBlankLines`, non-`Significant` newlines, or non-`Lazy` inline mode
anywhere in the system.

The one residue worth keeping in the doc comment: a hand-edited v2 file carrying
those values would be silently normalised on the next save. Hand-editing is not a
supported input, so this does not need architectural closure — but it is the sort
of thing that should be written down rather than rediscovered.

## 5. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test --workspace` | Pass — **1063** (709+27+16+2+20+20+255+6+7+1) |
| `cargo clippy --workspace -- -D warnings` | Pass |
| CI run `30782787366` | `success` on `2a26f092` |
| `ConfigManager` / `app_json_settings` in `forskscope-ui` | **Zero references** |
| `merge_into_v2` preserves unmentioned fields | Confirmed — `..base.clone()` |
| Startup resolution runs once | Confirmed — inside `use_context_provider` |
| `persist()` honours `settings_write_disabled` | Confirmed |
| `save_session()` honours `session_write_disabled` | Confirmed |
| `session_write_disabled` assignment sites | **One — inside `restore_session`** (C1) |
| `restore_session` reached in CLI mode | **No** (C1) |

## 6. Notable quality observations

- Naming the naive-adapter field-loss trap and closing it with struct-update
  syntax is the strongest thing in this patch. The failure it avoids would have
  been silent, delayed, and indistinguishable from the bug RFC-076 exists to fix.
- Running the real binary against real backed-up config files, and confirming the
  theme reached rendered CSS rather than stopping at a unit test, is the first
  runtime evidence this milestone has produced. More of that.
- Splitting each persistence function into a `Store`-dependent wrapper and a
  testable core, then testing against real temp-path repositories rather than a
  stand-in, is exactly what the handoff's §6 asked for.
- Declining to remove `app-json-settings` despite confirming it unused respects a
  boundary that would have been easy to cross unnoticed.

## 7. Recommended next action

1. Apply C1 with its CLI-path test; submit as a short follow-up.
2. Then patch 5 — convergence cleanup, per
   `rfcs/handoffs/076-versioned-runtime-persistence/convergence-cleanup-handoff.md`.
3. Then patch 6 — recovery UI and documentation. Its dialog is release-blocking
   for M2's cut, per §4.2. F28 rides with it.
4. B2 closes when C1 lands, not before.
