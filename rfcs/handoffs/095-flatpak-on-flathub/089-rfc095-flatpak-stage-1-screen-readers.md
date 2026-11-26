# Handoff 089 — RFC-095 stage 1: screen readers inside the Flatpak sandbox

**From:** architect. **Date:** 2026-10-21.
**Implements:** RFC-095 §2 (`rfcs/accepted/095-flatpak-on-flathub.md`),
accepted by the owner on 2026-10-21.
**This is an investigation,** with no app change and no release. Stage 2 waits
for its result.

**Order:** after the review 184 follow-ups (request 164). It touches nothing
in the `0.195.0` cut.

**The RFC is the authority.** If the investigation shows the RFC is wrong
anywhere, **stop and tell me.** Do not change the design, and do not edit
any RFC.

## Read first

- **RFC-095,** all of it, especially §1 and §2.
- **The research notes and the test-build results:**
  `.git-exclude/specs/flatpak-research-2026-10-21.md`.

## Rules for this work

**Flathub's AI policy (RFC-095 §1):**
- You **never** write, draft or edit a Flathub manifest, `cargo-sources.json`,
  or anything for the Flathub pull request. That is the owner's, by hand.
- You **may use** the architect's private test manifest,
  `.git-exclude/tmp/flatpak-spike/spike.yml` (app ID
  `io.github.forskscope.forskscope.Spike`), to build and run test builds.
- You **may change** that file for experiments.
- **It never leaves `.git-exclude/`.** Do not commit it, quote it in a
  tracked file, or offer it to the owner.

**Desktop safety:**
- **AT-SPI actions only, never keystrokes** (reviews 176 and 181).
- **A sandbox `HOME`** for every app you launch, native or Flatpak, so
  nothing private appears in a tree dump or a screenshot. For the Flatpak, the
  test app's own data folder is its sandbox. Do not grant it `home`.
- **Stop only the test app:** with `flatpak kill
  io.github.forskscope.forskscope.Spike`, or by pid. Never `pkill -f`.

**WebKit's own sandbox:**
- **Never weaken it,** not even for diagnosis, without asking me first. That
  includes `WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS` and anything like it.
- The answer we need must hold with that sandbox intact.

**Installations:**
- Use only the runtimes already installed: GNOME 51 Platform and SDK, and
  rust-stable 26.08, all `--user`.
- **Ask before installing anything else,** for example another Flathub app
  for comparison. Give the name and the download size.

**Run the test build without network permission,** using RFC-095 §4.2's
`GIO_USE_PROXY_RESOLVER=dummy` (`--env=` on `flatpak run`). The result must
not depend on network access.

## What is known

**In the Flatpak test build:**
- **AT-SPI shows only the window frame:** Minimize, Maximize, Close.
- **The client logs** `AT-SPI: Unable to open bus connection for application
  with pid …: Failed to connect to socket /run/user/1000/at-spi2-XXXX/socket:
  No such file or directory`.

**The native build, in the same session,** exposes every button.

## Do

1. **Reproduce it,** with the AT-SPI dump quoted for both builds.
2. **Find the cause.** At least:
   - **The registration path:** how WebKitGTK's web process registers its
     accessibility tree, and the process that owns that socket path:
     the app, `WebKitWebProcess` or `WebKitNetworkProcess`. Is the socket
     inside a namespace the host's AT-SPI cannot reach?
   - **The permissions:** what Flatpak grants for accessibility by default
     (the a11y bus proxy, `--a11y-own`), and whether a manifest permission or
     an app-side setting changes the result.
   - **The versions:** the WebKitGTK version in GNOME 51
     (`flatpak run --command=…` can tell), and whether the upstream
     changelogs or bug trackers (WebKit Bugzilla, the flatpak and
     xdg-desktop-portal issues, at-spi2-core) record this case and a fix.
   - **Orca:** our AT-SPI client stands in for a screen reader. **Do not
     start Orca.** It speaks aloud and grabs keys on the owner's live
     desktop. If you think a real screen-reader run is needed, say so in the
     request and I will arrange it with the owner.
3. **Report:**
   - **the cause, with evidence;**
   - **if a fix is in our hands** (an app-side setting, or a manifest
     requirement for RFC-095 §3): show the gate passing in a test build, with
     **RFC-095 §2's three checks quoted**, by AT-SPI dump and action:
     - the toolbar buttons with their names;
     - an Explorer row's action;
     - a file comparison's *Save* button;
   - **if the fix lies upstream:** where it is tracked, its state, and the
     version that carries it or will;
   - **anything else** that reaches screen-reader users inside the sandbox.

## Scope

- **In:** investigation and test builds only.
- **Out:**
  - app code changes;
  - the manifest for Flathub;
  - RFC-095 stage 2;
  - any upstream report, which needs my approval first. Draft it in the
    request instead (*don't pressure active upstreams*).

Reply in `.git-exclude/review-request/165-flatpak-screen-readers.md`.
