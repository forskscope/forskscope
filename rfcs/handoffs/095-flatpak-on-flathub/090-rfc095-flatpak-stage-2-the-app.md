> **Updated 2026-10-22: RFC-096 (one program) comes first in `0.196.0`.**
> `forskscope-cli` is being replaced by `forskscope compare` and `forskscope mcp`
> in the one `forskscope` executable. Its own handoff follows once the owner
> accepts it.
>
> - **Until then, do not write any `forskscope-cli` instruction** for Flatpak:
>   no `--command=forskscope-cli` in `scripts-and-agents.md`. Under RFC-096 it is
>   `flatpak run io.github.forskscope.forskscope mcp …`.
> - **In `main`, RFC-096's dispatch comes before RFC-095 §4.2 and §4.7.** A
>   subcommand must never run the Flatpak start-up changes. If you reach
>   `main` first, leave the very first lines for the dispatch.
> - **RFC-096 is done (reviews 187, 188).** Start this handoff now. Its first
>   commit carries review 188's test-side retry for the `ExecutableFileBusy`
>   flake.
> - **Everything else in this handoff stands.**

# Handoff 090 — 0.196.0: RFC-095 stage 2, the app ready for Flatpak

**From:** architect. **Date:** 2026-10-22. **Release:** `0.196.0`.
**Implements:** RFC-095 §4, §5 and §6
(`rfcs/accepted/095-flatpak-on-flathub.md`). It was accepted on 2026-10-21 and
amended with stage 1's result. The owner confirmed the Flatpak 1.15.11
minimum.

**The RFC is the authority.** Where this document and the RFC differ, the RFC
wins. If the work shows the RFC is wrong anywhere, **stop and tell me.** Do
not change the design, and do not edit any RFC.

## Read first

- **RFC-095, all of it,** especially:
  - §1, who does what;
  - §2 *Result*;
  - §4.7.
- **The test-build evidence:**
  `.git-exclude/specs/flatpak-research-2026-10-21.md`.
- **Review 185 §2:** how stage 1 was made to pass, with the exact commands.

## Rules for this work

**Flathub's AI policy (RFC-095 §1):**
- **Never write, draft or edit a Flathub manifest,** `cargo-sources.json`, or
  anything for the Flathub pull request.
- **For test builds,** use the architect's private manifest,
  `.git-exclude/tmp/flatpak-spike/spike.yml` (app ID
  `io.github.forskscope.forskscope.Spike`).
  - You may change it for testing. Point its git source at your commit.
  - **It never leaves `.git-exclude/`.**
- **The files in §5 are part of the application.** Write them as usual. The
  owner discloses AI-written material in the submission.

**Desktop safety, for every live check:**
- **AT-SPI actions only, never keystrokes.**
- **A sandbox `HOME`** for native runs. The Flatpak test app's own data
  folder for Flatpak runs.
- **Do not grant the test app `home`.** For the with-home case, grant a
  sandbox folder: `--filesystem=<sandbox path>`, together with a `HOME`
  override where needed. Say how you did it.
- **Stop only the test app,** with `flatpak kill …Spike` or by pid. Never
  `pkill -f`.
- **No Orca.**

**Installations:** use only the runtimes already installed. Ask before
installing anything else, and give its size.

**WebKit's sandbox is turned on here (§4.7).** It is never weakened, not even
for diagnosis, without asking me.

**Research online freely.** A handoff's "no network" rule constrains the app
under test, never your research.

## 1. App changes (RFC-095 §4)

Implement **§4.1 to §4.7** as written. In particular:

- **§4.1 `platform::flatpak()`, in `forskscope-desktop`:**
  - it reads `/.flatpak-info` once;
  - it returns: in a Flatpak or not, the app ID (`[Application] name=`), and
    whether home is granted (`[Context] filesystems=` containing `home` or
    `host`, with or without `:rw` or `:ro`);
  - **unit-test the parser** on planned inputs: no file, no `[Context]`,
    `filesystems=` empty, `home`, `home:ro`, `host`, `xdg-documents` only,
    and a `filesystems=` line in a section other than `[Context]`.
  - **Every behaviour below calls it,** and nothing else checks for itself.
- **§4.2 and §4.7: the start of `main`,** in this order, and only in a
  Flatpak:
  - the program name is the app ID (`g_set_prgname`);
  - `GIO_USE_PROXY_RESOLVER=dummy` is set, unless the user already set it;
  - WebKit's web-process sandbox is turned on:
    - **if the app can reach the web context** before the first web process
      starts, through `webkit2gtk`'s `set_sandbox_enabled(true)`;
    - **otherwise** through `WEBKIT_FORCE_SANDBOX=1`, unless the user already
      set it;
    - **say which, and why.**
  - **Not a GTK application ID:** tao would make the app single-instance,
    and `git difftool`'s second launch would hand over to the first. Comment
    this beside the code.
  - Environment changes happen **before any thread starts.** Say how that is
    guaranteed.
- **§4.3, the home message:**
  - in both languages, through the i18n gate;
  - **comparisons started with paths still open** without home.
- **§4.4, host paths for display only.** The attribute
  `user.document-portal.host-path` is read once per path. **I/O keeps the
  sandbox path.** Test the display mapping with a planned set:
  - the attribute present;
  - the attribute absent;
  - a non-UTF-8 value;
  - a path that is not a document-portal path.
- **§4.5, the backup:**
  - the save-target line says no backup can be kept for this file;
  - **no backup is attempted, and none is claimed.**
- **§4.6:** no *Check for updates* entry or setting in a Flatpak, and no
  request ever made.
- **Outside a Flatpak, nothing changes.** Every existing test passes
  unchanged.

## 2. Listing material (RFC-095 §5)

**The metainfo:**
- **Where:** `packaging/linux/io.github.forskscope.forskscope.metainfo.xml`,
  with everything §5 lists.
- **A CI gate runs `flatpak-builder-lint appstream` on it,** where warnings
  are fatal.
  - Use Flathub's own tooling, in a container or image **pinned by digest**.
  - Falsify the gate: remove the OARS rating, and it fails.
- **Report what the linter does with screenshot URLs.** They point at the
  `0.196.0` tag, which does not exist until the release. If the linter
  fetches them, propose how the gate stays meaningful before the tag.
  **Do not weaken it silently.**

**The desktop file:**
- Renamed `io.github.forskscope.forskscope.desktop`, with
  `StartupWMClass=forskscope`.
- **Update every place that installs it:** the PKGBUILD, `install.sh`, and
  any others you find.

**The icon: waiting on the owner.**
- Whether the icon is made by the owner, commissioned, or drafted here is an
  open question (RFC-095 *Open questions* 1).
- **Prepare everything around it:**
  - the install paths in the PKGBUILD and `install.sh`;
  - the metainfo;
  - a placeholder path the lint gate knows about.
- **Do not draw or generate an icon.** I tell you when the file exists.

**The screenshots:**
- Window only, at most 1000×700, default settings, English, with one-sentence
  captions.
- **Made with the sandbox-home recipe,** so nothing private shows.
- **Look at each one before committing it, and describe it** in the request.
- Suggested set:
  - the Explorer;
  - a file comparison;
  - a folder comparison;
  - the HTML report.

## 3. Documentation (RFC-095 §6)

**`installation.md`, `git-integration.md`, `scripts-and-agents.md`,
`threat-model.md`:**
- Write them as §6 says.
- **Every command quoted in the guide is one you ran** in the test build, and
  the request quotes the run.

## 4. Verification (RFC-095 acceptance criteria)

**Build the test Flatpak from your commit,** with the private manifest
changed only in its git source. **Remove every test-only workaround from it:**
- no `exec -a` launch;
- no `--env=` variables.

The app's own code must now do all of that.

1. **Criterion 1:** in that build, with no network permission, repeat stage
   1's checks through AT-SPI, and quote them:
   - the toolbar buttons with their names;
   - an Explorer row's action;
   - *Save merge result* in a forwarded file comparison, **pressed**, with
     the host file changed.
2. **Criterion 2:** the app renders with no `--share=network`. Take a
   screenshot, and describe it.
3. **Criterion 3:** without the home grant, §4.3's message. With a grant
   (§Rules), the Explorer on the granted folder. Forwarded comparisons work in
   both.
4. **Criterion 4:** a forwarded folder pair shows host paths in the tab, the
   header and the save target.
5. **Criterion 5:** a save over a forwarded file with backups on:
   - the host file changes;
   - no backup is claimed, and no `.bak` exists on the host.
6. **Criterion 6:** in the Flatpak build, with *Check for updates* absent, no
   socket is opened. Use the `LD_PRELOAD` observer from RFC-094's proofs,
   inside the sandbox if it can be loaded there. If it cannot, say so and
   propose an equivalent.
7. **Criterion 7:** outside a Flatpak, nothing changes:
   - the existing tests pass;
   - a native live check shows the Explorer and a comparison as before.
8. **Criterion 8:** the metainfo gate in CI, and the AUR package installing
   the desktop file. The icon follows once it exists.
9. **Criterion 9:** the guide's commands, as run.

**Test names, not sums,** across the whole handoff.

## Scope

- **In:** RFC-095 §4, §5 (except the icon file itself) and §6, with their
  tests and gates.
- **Out:**
  - the Flathub manifest and submission;
  - the icon's design;
  - CHANGELOG, ROADMAP, the version;
  - enabling WebKit's sandbox outside a Flatpak.

## Gates

- **The standard set,** including Windows GNU clippy, and the new metainfo
  gate.
- **A green CI run** with its ID.
- **A Windows Check dispatch** with its ID: the startup code changes.

Reply in `.git-exclude/review-request/168-flatpak-stage-2.md`.
