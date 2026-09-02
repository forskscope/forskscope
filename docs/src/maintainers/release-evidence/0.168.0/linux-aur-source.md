# Platform Evidence — Arch/CachyOS via the AUR source package

**Scope.** Not a Gate D row. This records the **first evidence for the
`PKGBUILD` install path**, which F81 noted `matrix-plan.md` has no row for at
all — *"a documented install method that nothing tests."* It covers dynamic
linkage and process start only; see Limitations.

**Package:** `forskscope 0.168.0-1`, built by `makepkg` from the **AUR
repository as published** (cloned from `https://aur.archlinux.org/forskscope.git`
at commit `2baae0d`), not from the in-repo `packaging/linux/PKGBUILD`.
**Source SHA-256:** `3be2e5afe0f73806d100f94211de2edd328d858543bf4a72ad8f726e836f978e`
— validated by `makepkg --verifysource` against the tag tarball before building.
**Test date (UTC):** 2026-09-02
**Tester role:** architect
**Host:** CachyOS, kernel 7.2.2-1-cachyos, x86_64 — a **libxdo-4** distribution,
i.e. exactly the family F44 describes.
**Install source:** `makepkg --nodeps` (system `cargo` comes from rustup rather
than the `cargo` package; every other declared dependency was present).

## What this establishes

**F44's closure argument was reasoning; this is measurement.** F44 was closed as
a Gate D blocker on the basis that libxdo-4 distributions keep a working install
path through the AUR **source** package, which compiles against the host's own
library. That had never been executed. It now has:

```text
$ readelf -d usr/bin/forskscope | grep -i xdo
 0x0000000000000001 (NEEDED)  Shared library: [libxdo.so.4]

$ ldd usr/bin/forskscope | grep xdo
 libxdo.so.4 => /usr/lib/libxdo.so.4
```

The published tarball records `libxdo.so.3` and cannot start here; **the AUR
build of the same tag records `libxdo.so.4` and resolves.** The binary was then
executed and returned its own CLI usage message, so the dynamic loader resolved
every dependency, not merely `libxdo`.

## Limitations — deliberately not overstated

- **This is not P01.** No GUI cold launch, no Explorer render, no diagnostics
  check. It establishes that the artifact links correctly and the process
  starts.
- `--nodeps` was used, so this does **not** prove the declared `makedepends` are
  complete for a clean Arch host — only that the declared `depends` resolve for
  the built binary.
- One host, one distribution. It is evidence about the *mechanism* (source build
  links the host's `libxdo`), which is what F44's closure turned on.

## Related

F44 (closure now evidenced), F81 (this install path had no test row), RFC-081
(AUR publication), and the `cargo install` question the owner raised the same
day — that path shares this mechanism, so this record is partial evidence for it
too.
