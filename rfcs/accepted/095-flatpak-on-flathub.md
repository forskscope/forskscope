# RFC 095: ForskScope as a Flatpak on Flathub

**Status.** Accepted — review complete; implementer may start. Moves to `done/`
when the Flathub listing is live (RFC-000, 5-folder variant).
**Proposed.** 2026-10-21 by the architect, at the owner's direction.
**Accepted.** 2026-10-21 by the project owner.
**Amended.** 2026-10-21, stage 1's result (§2): two app changes make screen
readers work (§4.7), and the Flatpak requires Flatpak 1.15.11 or later (§3).
The minimum version was confirmed by the owner on 2026-10-21.
**Scheduling.** After `0.195.0`. **Stage 1 is a gate:** nothing else starts
until it passes (§2).
**Tracks.** The owner's direction, 2026-10-21:
- publish on Flatpak, to reach a broader range of Linux users;
- go to Flathub, with the owner writing the manifest and handling the review;
- **inclusive design:** a Flatpak that screen-reader users cannot use is not
  offered;
- **home folder access is a requirement of the Flatpak,** as on other
  platforms, rather than two modes.

**Evidence.** `.git-exclude/specs/flatpak-research-2026-10-21.md`:
- the research, with sources;
- a test build on this project's Linux machine (flatpak 1.18.4, GNOME 51,
  Wayland, NVIDIA).

**Touches.**
- `forskscope-desktop`: Flatpak detection;
- `forskscope` (the GUI): startup, path display, the backup, the update check;
- `packaging/linux/`: the desktop file, metainfo and icon, also used by the
  AUR package;
- the user guide;
- `docs/src/maintainers/threat-model.md`.

## Summary

ForskScope is published on Flathub as `io.github.forskscope.forskscope`.
- **The app works inside the Flatpak sandbox as it does natively,** with one
  stated requirement: access to the user's home folder.
- **It needs no network permission.**
- **The owner writes the Flathub manifest and leads the review.** Flathub's
  policy requires it (§1).
- **This project makes the app ready:**
  - six small app changes (§4);
  - the listing material (§5);
  - the documentation (§6).
- **One gate comes first:** the app's content must reach screen readers inside
  the sandbox (§2).

## Motivation

- **The release archive needs WebKitGTK 4.1 on the host.** Today Linux users
  get it from the `.tar.gz` or the AUR, and many distributions no longer
  package that library by default.
- **A Flatpak carries its runtime,** so the app runs the same on Fedora,
  Ubuntu, Debian, openSUSE and others.
- **Flathub puts it in GNOME Software and KDE Discover,** with automatic
  updates. People find software there.
- **Flathub is the ordinary route.** A self-hosted Flatpak repository was
  considered and rejected (§Alternatives).

## 1. Who does what

**Flathub's generative AI policy** (requirements, 2026; quoted in the research
notes) says:
- *"Flathub manifests must not contain AI-generated or AI-assisted
  content."*
- *"AI tools or agents must not open or automate Flathub submission pull
  requests, or generate their commit messages, descriptions, review comments,
  or replies."*
- AI-generated material in the application itself must be disclosed, with
  its extent, and *"Reviewers may reject a submission … based on the extent or
  role of generated material."*
- AI *"used only for research, discussion, or debugging"* needs no
  disclosure when nothing generated is included.

**So:**

| Part | Who |
|---|---|
| The Flathub manifest, `cargo-sources.json`, the submission PR and every review reply | **The owner**, by hand |
| The disclosure of AI-generated material in the app | **The owner**, who knows its extent |
| App ID verification (a GitHub admin of the `forskscope` organization) | **The owner** |
| Stage 1's investigation; the app changes (§4); the listing material (§5); the documentation (§6) | The dev team, reviewed by the architect, as usual |

**No AI-written Flathub manifest exists in this repository, or is offered to
the owner as a model.**
- The architect's test manifest is a private, throwaway build aid. It lives
  in `.git-exclude/tmp/` and is deleted once the owner's manifest exists.
- This RFC lists **requirements** for the manifest (§3), never manifest text.
  The owner writes the manifest from Flathub's own documentation.

## 2. Stage 1, the gate: screen readers inside the sandbox

**The test build exposes only the window frame to AT-SPI:** Minimize,
Maximize, Close.
- **None of the app's content is exposed:** no button, no text, no Explorer
  row.
- **The native build, in the same session, exposes all of it.**
- **The AT-SPI client logs** `Unable to open bus connection … Failed to
  connect to socket /run/user/1000/at-spi2-…/socket`. That is a socket private
  to the sandbox.
- **So a screen-reader user would meet an empty window.**

**Do:**
1. **Find the cause.** Check, among others:
   - how WebKitGTK's web process registers with the accessibility bus under
     Flatpak;
   - whether a permission or a WebKitGTK setting governs it;
   - whether other WebKitGTK apps on Flathub show the same.
2. **Report it, with evidence:**
   - **the cause;**
   - **the fix,** if one is in our hands: an app change, or a manifest
     requirement (§3);
   - **if the fix lies in WebKitGTK, Flatpak or AT-SPI:** where it is tracked
     upstream, and whether a fix is released.

**The gate passes** when, in a Flatpak test build, the same AT-SPI checks used
on the native build succeed:
- the toolbar buttons with their names;
- the Explorer's rows with their action;
- a file comparison's *Save* button.

**If it cannot pass,** stages 2 and 3 do not start. This RFC waits for the
upstream fix, and the owner decides whether it stays open.

**Result (2026-10-21, handoff 089 and review 185): the gate passes, with two
app changes (§4.7).**
- **WebKitGTK's web-process sandbox is opt-in** for the GTK 3 API
  (`webkit2gtk-4.1`) that wry uses, and always on for the GTK 4 API GNOME Web
  uses. wry 0.53 never turns it on.
- **Without that sandbox,** under Flatpak, the web process cannot own a name
  on the accessibility bus, so its tree is never linked to the window.
- **With the sandbox on** (`WEBKIT_FORCE_SANDBOX=1` in the test), WebKit
  starts the web process through `flatpak-spawn --sandbox` and asks the portal
  for an accessibility name derived from the application ID.
  - **The app has no GTK application ID,** so WebKit used a hash
    (`org.webkit.app-7cbff5…`).
  - **The portal refused it,** *"doesn't match app id"*, and the app aborted.
- **With the program name set to the Flatpak ID as well,** the test build,
  with no network permission, exposed the whole app to AT-SPI:
  - the toolbar buttons, with their names;
  - the Explorer's rows, with their action;
  - a file comparison's *Save merge result* button.
  
  Pressing *Use this change* and *Save merge result* through AT-SPI alone
  changed the host file. The window's Wayland app ID also became the Flatpak
  ID, which is what desktop integration expects.

## 3. Requirements for the manifest (the owner writes it)

**The runtime:**
- `org.gnome.Platform` and `org.gnome.Sdk`, **51**: the latest at the time of
  writing, and the last to include WebKitGTK 4.1;
- the Rust extension, `org.freedesktop.Sdk.Extension.rust-stable`, branch
  `26.08` (Rust 1.98, above the workspace's 1.91).

**The build:**
- **From source, offline,** as Flathub requires.
- **`flatpak-cargo-generator.py`** produces `cargo-sources.json` from
  `Cargo.lock`.
- **`vendor/muda`,** the patched path dependency, needs nothing extra: it is in
  the tagged source. There are no git dependencies today.
- **Both programs are installed:** `forskscope` and `forskscope-cli`.

**The permissions, and why each one:**

| Permission | Why |
|---|---|
| `--socket=wayland`, `--socket=fallback-x11`, `--share=ipc`, `--device=dri` | A graphical app with a GPU-rendered web view. |
| `--filesystem=home` | **The stated requirement.** A diff and merge tool works on the user's own files and folders: the Explorer browses them, and Save writes back to the files compared. `git difftool` and `git mergetool` start it on files in the user's repositories. |

**The minimum Flatpak version: 1.15.11** (`--require-version` at
`build-finish`).
- WebKit's accessible sandbox needs `flatpak-spawn --sandbox-a11y-own-name`,
  which Flatpak added in 1.15.11.
- **On an older Flatpak the app would install, but screen readers would get
  nothing,** and the sandboxed start is untested there.
- **So the Flatpak is not offered there.** Users of older Flatpak versions use
  the `.tar.gz`, or their distribution's package.
- Folder forwarding for `git difftool --dir-diff` needs 1.18. The guide says
  so (§6), without raising the minimum.

**Not requested:**
- **No network:** §4.2 makes it unnecessary.
- **No `host`:** home is what the app needs. Files elsewhere arrive by file
  forwarding (§6).

**The home permission needs a Flathub lint exception,**
`finish-args-home-filesystem-access`, *"granted on sufficient explanation"*.
- **The owner writes that explanation.**
- **The risk:** in 2026, reviewers asked two smaller diff tools to use the
  file-chooser portal instead (§Risks).

## 4. App changes

### 4.1 Knowing it runs in a Flatpak

- **One function in `forskscope-desktop`, `platform::flatpak()`:**
  - it reads `/.flatpak-info` once;
  - it returns whether the app is in a Flatpak, and whether the home folder is
    granted;
  - **the home grant is the `[Context]` section's `filesystems=` list**,
    containing `home` or `host`, with or without a `:rw` or `:ro` suffix.
- **Verified on the test build:** that list reflects the permission actually
  in force, including run-time changes (`--filesystem=home` adds `home`;
  `--nofilesystem=home` empties it).
- **Every Flatpak behaviour below calls this function,** never its own check.

### 4.2 No network permission: the web view connects directly

**The test build's window was blank without network permission.**
- **The cause:** Dioxus delivers the interface to the web view over a
  WebSocket on `127.0.0.1`. That address is reachable inside the sandbox: the
  app and WebKit's processes share one network namespace, and Dioxus was
  listening.
- **But WebKit's network process never connected.** GLib first asks for a
  proxy through Flatpak's proxy portal, and without network permission the
  answer is no route at all (`Address enumeration completed (out of
  addresses)`).
- **With `GIO_USE_PROXY_RESOLVER=dummy`, which means direct connections, the
  app renders with no network permission.**

**Do:**
- **In a Flatpak, set `GIO_USE_PROXY_RESOLVER=dummy`** at the start of
  `main`, before any thread starts and before GTK or WebKit initialise.
- **Do it only if the variable is not already set.** The user's own value
  wins.
- **The web view only loads the app's own content,** so direct connections
  lose nothing.
- **Comment why, in the code,** with the evidence above.
- **The threat model records it.**

### 4.3 The home folder is required, and its absence is said plainly

**Without the home grant,** the sandbox shows a "home" holding only `.local`
and `.var`. The Explorer must never present that as the user's home.

**Do:**
- **In a Flatpak without the home grant,** the Explorer is replaced by one
  message, in English and Japanese:
  - ForskScope needs access to your home folder to browse and compare your
    files;
  - how to restore it: `flatpak override --user --filesystem=home
    io.github.forskscope.forskscope`, or a permissions manager such as
    Flatseal;
  - then restart ForskScope.
- **Comparisons started with paths still open,** with no message. Files handed
  over by `git difftool` (§6) arrive through the document portal, and need no
  home grant.
- **Outside a Flatpak, nothing changes.**

### 4.4 Real paths, not document-portal paths

**Files and folders handed over by file forwarding** appear inside the sandbox
as `/run/user/<uid>/doc/<id>/<name>`.
- **The test build showed exactly that** in Deep Compare's header:
  `Left: /run/user/1000/doc/6ObnKD…/dl`.
- **The host path is in the file's extended attribute**
  `user.document-portal.host-path`.

**Do:**
- **Wherever a path is displayed,** show the host path from that attribute
  when it is present:
  - tab titles;
  - headers;
  - the save target;
  - status messages;
  - the HTML report with `full` paths.
- **Reading and writing keep using the sandbox path.** Display only.
- **When the attribute is absent,** the path is shown as it is.

### 4.5 A backup that cannot be kept is not claimed

**In the test build, a `<name>.bak` created beside a forwarded file reported
success inside the sandbox, but never appeared on the host.** The document
portal kept it as a sandbox-only temporary file.

**Do:**
- **A file is a document-portal file** when it carries the
  `user.document-portal.host-path` attribute (§4.4), in a Flatpak.
- **For such a file, the backup is not attempted:**
  - beside *Save*, the save-target line says no backup can be kept for this
    file, in one short sentence, in both languages;
  - the success message never mentions a backup;
  - **the save itself works.** Writing back over a forwarded file reached the
    host file in the test build.
- **For every other file,** the backup behaves as today.
- **Verify on the real build:** a save with backups on, over a forwarded file.
  The host file changes, no `.bak` is claimed, and none exists.

### 4.6 *Check for updates* is not offered in a Flatpak

- **Updates come from Flathub,** through the user's software centre.
- **In a Flatpak:**
  - the menu entry and its setting are not shown;
  - no update request is ever made;
  - where the app would mention updates, it names the software centre.

### 4.7 Screen readers: the application ID, and WebKit's web-process sandbox

**In a Flatpak** (§4.1), and only there:
1. **The program name is set to the Flatpak ID,** from `/.flatpak-info`'s
   `[Application] name=`.
   - Use `g_set_prgname`, at the start of `main`, before GTK initialises.
   - WebKit derives the web process's accessibility name from it, and the
     window's Wayland app ID follows it.
   - **Not a GTK application ID.** tao registers a unique `GApplication` when
     given one, and a second launch would hand over to the first. That would
     break `git difftool`, which starts one process per comparison.
2. **WebKit's web-process sandbox is turned on.**
   - Through the web context's `set_sandbox_enabled(true)` if the app can
     reach it before the first web process starts.
   - Otherwise, `WEBKIT_FORCE_SANDBOX=1` at the start of `main`, unless the
     user has set it.
   - Say in the request which, and why.

**Both are tied to the Flatpak.**
- **Outside a Flatpak, nothing changes.** The app ID stays `forskscope`, and
  matches the desktop file's `StartupWMClass` (§5).
- WebKit's sandbox outside Flatpak needs `bwrap` and `xdg-dbus-proxy` on the
  host. That is a separate decision, not taken here.

**Comment both in the code,** with the evidence in §2.

## 5. Listing material (upstream, in this repository)

These files are part of the application, so **AI-written material among them
is disclosed by the owner** (§1).

**AppStream metainfo:**
- **Where:** `packaging/linux/io.github.forskscope.forskscope.metainfo.xml`.
- **What it holds:**
  - `type="desktop-application"`;
  - the ID;
  - `metadata_license` CC0-1.0, and `project_license` Apache-2.0;
  - the developer;
  - the launchable;
  - the OARS rating;
  - the homepage, bug tracker and source URLs;
  - the releases;
  - screenshots linked from a **tag**, never a branch.
- **It passes `flatpak-builder-lint appstream`,** where warnings are fatal.
  This runs as a CI gate on the file, in the Flathub tools' own container,
  outside the shipped programs.

**The desktop file:**
- **Renamed `io.github.forskscope.forskscope.desktop`,** for every Linux
  package, AUR included, so the ID is the same everywhere.
- **`StartupWMClass=forskscope` added,** so the window matches its entry.

**The icon:**
- **A real square app icon,** SVG, or a PNG of 256×256 or larger, under
  `hicolor`.
- **Today there is none.** `logo.svg` wraps a 300×300 PNG.
- **The AUR package installs it too.** Its desktop file names `Icon=forskscope`
  today, and none is installed. That is a defect fixed here.
- **Who makes it is an open question** (§Open questions).

**Screenshots:**
- **Size:** at most 1000×700, of the window only, at default settings, in
  English.
- **Privacy:** made by the sandbox-home recipe, so nothing private shows.
- **Captions:** each a single sentence.
- **Committed and linked by tag.**

## 6. Documentation

**`installation.md`:** a Flatpak section.
- Installing from Flathub.
- **The home folder requirement,** and how to restore it (§4.3).

**`git-integration.md`:** the Flatpak commands.
- **File forwarding** is required, because git's temporary files live in
  `/tmp`, which no Flatpak permission exposes:
  - **difftool:** `flatpak run --file-forwarding
    io.github.forskscope.forskscope @@ "$LOCAL" @@ @@ "$REMOTE" @@`;
  - **mergetool:** the same for the three paths;
  - **`--dir-diff`:** `--no-symlinks` as today. Folder forwarding needs
    flatpak 1.18 or later, and the guide says so.
- **The test build verified files and folders** through forwarding.

**`scripts-and-agents.md`:**
- `forskscope-cli` runs as `flatpak run
  --command=forskscope-cli io.github.forskscope.forskscope`.
- **`--root` must be inside the home folder.** Paths sent inside MCP messages
  cannot be forwarded.

**`threat-model.md`:**
- the Flatpak sandbox as an outer boundary;
- the home permission;
- §4.2's environment variable.

## 7. Stages and release

1. **Stage 1, the gate (§2):** investigation and report. No release.
2. **Stage 2, the app (§4–§6):**
   - its own handoff, and a normal release;
   - verified with a Flatpak test build, built from the owner's manifest once
     it exists, or before that with the architect's private test manifest.
3. **Stage 3, Flathub (§1, the owner):**
   - the owner's manifest and the app ID verification;
   - the PR to `flathub/flathub`'s `new-pr` branch, with the required video
     of the app running as a Flatpak;
   - the review.

**After acceptance on Flathub:**
- **Updates are PRs to `flathub/io.github.forskscope.forskscope`.**
- **The update bot does not regenerate `cargo-sources.json`,** so the owner
  regenerates it each time `Cargo.lock` changes.
- **Not every patch release needs to reach Flathub.** The cadence is the
  owner's (§Open questions).

## Acceptance criteria

1. **Stage 1 passes (§2),** with the AT-SPI checks quoted.
   - **Done in the test build (§2, Result).**
   - **Repeated on stage 2's build,** where §4.7 makes the change in code,
     not through an environment variable or a renamed launch.
2. **Without network permission,** the app renders. A Flatpak test build is
   shown with no `--share=network`.
3. **Without the home grant,** the Explorer shows §4.3's message.
   - **With the grant,** it opens on the real home.
   - **Comparisons by forwarded paths open in both cases.**
4. **Forwarded files and folders show host paths everywhere** (§4.4).
5. **A save over a forwarded file with backups on:**
   - the host file changes;
   - no backup is claimed, and none exists (§4.5).
6. **No update request is ever made in a Flatpak:**
   - shown by the `LD_PRELOAD` socket observer from RFC-094's proofs;
   - and *Check for updates* is absent.
7. **Outside a Flatpak, nothing changes.** The existing tests pass, and
   `platform::flatpak()` returns *not in a Flatpak*.
8. **The metainfo passes `flatpak-builder-lint appstream`** in CI.
   - **The desktop file and icon** install for the AUR package too.
9. **The guide's Flatpak commands are the ones tested,** quoted from the test
   runs.

## Non-goals

- **A self-hosted Flatpak repository** (§Alternatives).
- **A mode without the home folder** beyond §4.3's message. The owner decided
  that home access is a requirement.
- **Bundling WebKitGTK,** or moving to the GTK 4 WebKit, before GNOME 51 ends
  (§Risks).
- **Snap and AppImage.**

## Alternatives considered

- **A self-hosted Flatpak repository,** on GitHub Pages, signed by us.
  **Rejected:**
  - it is unusual;
  - it gives no listing in software centres;
  - users would have to add a remote by hand;
  - we would hold a signing key and host a growing repository.
- **Portal-only file access,** where the Explorer browses only folders chosen
  in the system file picker. **Rejected by the owner** as too complicated: two
  behaviours, for one tool.
- **`--filesystem=host`.** Broader than the app needs. Home is the honest
  minimum.
- **`--share=network`.** It makes the app render, but grants a capability the
  app does not use, and contradicts *Local only*. §4.2 removes the need.

## Risks

- **Flathub refuses the home permission.** Then the decision returns to the
  owner: a portal-based Explorer, or no Flathub.
- **The AI disclosure.** Reviewers may reject based on the extent of generated
  material. Only the owner can weigh this.
- **The GNOME 51 runtime is supported until September 2027.**
  - The next GNOME runtime drops WebKitGTK 4.1 (gnome-build-meta MR !4734).
  - **Before then,** the Dioxus web view layer (wry) must move to the GTK 4
    WebKit (tauri-apps/wry PR #1767, open), or the manifest must build
    WebKitGTK 4.1 itself.
  - **Registered in the ROADMAP,** with that date.
- **Stage 1 may depend on an upstream fix,** with no date.

## Open questions (the owner's)

1. **The icon:** made by the owner, commissioned, or drafted here and
   disclosed?
   - **Decided (owner, 2026-10-25): drafted here, and disclosed.** The
     architect traced the owner's logo into an SVG, with opaque shapes on a
     light tile, so it reads on dark backgrounds too. The owner reviewed and
     approved it.
   - **The file:**
     `packaging/linux/icons/hicolor/scalable/apps/io.github.forskscope.forskscope.svg`.
   - **The Flathub submission discloses it** as AI-made.
2. **The Flathub release cadence:**
   - every release, or minor releases only?
   - ForskScope releases every few days, and Flathub discourages
     high-frequency updates.
