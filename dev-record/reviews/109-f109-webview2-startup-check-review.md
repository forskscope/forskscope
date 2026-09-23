# Review 109 — Request 108: F109, WebView2 startup check

**Reviewer:** architect. **Date:** 2026-09-17. **Reviewed:** `f50a954`.
**Verdict:** **The code is approved, with required follow-ups in handoff 037.**
Nothing in the change is wrong. The problem is that **no Windows machine has
compiled or run it**, and the only place it would first run is the 0.172.0
release build. F109 stays open until 037 lands.

## 1. What holds

- **Placement.** The check runs after `--diagnostics` and after argument
  parsing, and before `launch`. That is what the handoff asked for.
- **Settings are read without side effects.** I confirmed it:
  - `SettingsRepository::load()` calls only `read_to_string_or_missing`, which
    is plain `fs::read_to_string`, then parses the text.
  - `state::config_file_path` only builds a path and creates nothing.
  - Treating `MigratedLegacy` as readable is right, because loading does not
    commit the migration.
- **Opening the download page with `cmd /C start "" <url>`.** The empty title
  argument is the real pitfall with `start`, and the request explains it
  correctly. The URL contains no `cmd` metacharacters.
- **The decision is a plain function.** The falsification is real: it is a
  failing test against a broken `decide`, not only passing tests.
- **Stopping short instead of guessing.** You did not push a rehearsal tag
  and you did not edit the M5 harness. Both are right, and asking was right.

## 2. My own check: the Windows code compiles

I compiled `forskscope-ui` for Windows from Linux:

```
cargo clippy -p forskscope-ui --target x86_64-pc-windows-gnu --all-targets
```

It passed, with no warnings in `main.rs` or `webview2.rs`. So
`check_webview2_or_exit`, `saved_lang_no_side_effects` and the `--diagnostics`
line **type-check against the real `wry` and `rfd` APIs**.

It does **not** show that the code behaves correctly on Windows. It is also
the GNU target, not MSVC, although the APIs involved are the same on both.

The same run found something older. With `-D warnings`, `forskscope-core`
**fails**:

```
error: variable does not need to be mutable
   --> crates/forskscope-core/src/save.rs:179:9   (and :298:9)
    let mut builder = tempfile::Builder::new();
```

On Windows, `builder` is never mutated. The mutation must be in a Unix-only
branch. It is harmless, but it proves the point: **CI has never
compiled this project's Windows code paths at all.** `ci.yml` has one
Ubuntu job, and the release workflow builds Windows without clippy.

## 3. Required: a Windows compile check on every push (handoff 037 §A)

Code that compiles only on Windows has now appeared in two crates, and
nothing checks it until a release tag is pushed. A Windows-only type error
would fail the release, and we would find it only then.

As shown in §2, the fix does not need a Windows runner. Add a step to
`ci.yml`'s existing job that runs clippy against the Windows GNU target with
`-D warnings`, and fix the two `unused_mut` warnings in `save.rs`. Handoff 037
has the details.

## 4. Required: prove the detection on real Windows, before the cut (handoff 037 §B)

The `release.yml` step has two problems:
- **Its first run would be the 0.172.0 release itself.** A new check that has
  never run does not belong in the release workflow.
- **It asserts nothing.** It prints both outputs and passes whatever they say,
  so a regression or a broken simulation would go unnoticed.

**The answer to your question:** no rehearsal tag. Move the step into a small
**workflow you can dispatch on demand** that:
- builds the release binary on `windows-latest`;
- runs `--diagnostics` twice;
- **asserts** the results.

Dispatch it for the head commit and give the run ID. That also answers the
open question of whether `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` really
simulates a missing runtime.

## 5. Recommended: prove the Japanese text is actually used

`absent_stops_with_a_non_empty_bilingual_message` checks that the message is
non-empty **in both languages**. But if the Japanese entries were missing, `t`
would fall back to the English key and the test would still pass. Add one
assertion: the `Lang::Ja` dialog text differs from the `Lang::En` one. It is
cheap, and the test's name promises it. This is included in 037 as §C.

## 6. The message box: moves to the owner's pre-publish check

Agreed that the M5 harness cannot reach the dialog within scope. I am putting
it on the owner's 0.172.0 pre-publish list, **on the condition that 037 §B
shows the environment variable really does simulate the missing runtime**:

1. In a Command Prompt, run `set WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=C:\empty`
   (an empty folder).
2. Run `forskscope.exe` from that prompt.
3. The dialog appears. **No** exits, and **Yes** opens Microsoft's page.

## 7. Register

F109 stays at 0.172.0 until 037 lands. The 0.172.0 pre-publish list gains the
§6 check.
