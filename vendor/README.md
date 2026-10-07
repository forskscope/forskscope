# `vendor/`

Patched copies of published crates live here, each one temporary. A crate
belongs in `vendor/` only when: it is a verbatim copy of a specific published
version, with a stated, minimal diff, and a stated condition under which the
copy is deleted and the patch removed. Nothing else goes here.

The directory and this file are the whole of `vendor/`.

## `muda` — F179

**Why it exists.** Our Linux release binary links `libxdo.so.3` through
`muda`, the native-menu crate `dioxus-desktop` depends on, and that library
is missing on Arch (which ships `libxdo.so.4`), so the binary fails to start
there. `muda`'s only use of `libxdo`, in
`src/platform_impl/gtk/mod.rs`, fakes a keystroke for the predefined
*Copy*/*Cut*/*Paste*/*Select All* menu items. ForskScope runs with
`.with_menu(None)` (`crates/forskscope-ui/src/main.rs`), so it never has a
native menu and that code path never runs — `libxdo` is dead weight we can't
drop from our own manifest, because Cargo features only add and
`dioxus-desktop` 0.7.9 enables `muda`'s (and `tray-icon`'s) default features.

**Provenance.** Copied from `muda` **0.17.2** exactly as published on
crates.io — the version already pinned in our `Cargo.lock` — verified
against that lockfile's checksum:

```
sha256 7c9fec5a4e89860383d778d10563a605838f8f0b2f9303868937e5ff32e86177
```

**The diff** (nothing else changes):

- `Cargo.toml`: the `libxdo` target dependency and `"libxdo"` from `default`
  are dropped. The `libxdo` feature **name is kept**, as `libxdo = []`:
  `tray-icon` 0.21.3's own `libxdo` feature forwards to `muda/libxdo`
  (`libxdo = ["muda/libxdo"]`), so removing the name would make Cargo fail to
  resolve `tray-icon`, not just drop a capability.
- `src/platform_impl/gtk/mod.rs`: the two `#[cfg(feature = "libxdo")]` blocks
  (the keystroke call in the predefined-item `connect_activate` closure, and
  `fn xdo_keys`) are removed — with the feature now empty, they would try to
  compile against a crate that's gone. With the feature disabled, this is
  exactly the code that already ran (nothing): the behaviour is unchanged.

This backports two fixes already made upstream, neither yet on our
dependency line:

- `muda` 0.21.0 (2026-09-30) removed the `libxdo` feature entirely (its
  default is now `gtk3` alone).
- Dioxus PR [DioxusLabs/dioxus#5749](https://github.com/DioxusLabs/dioxus/pull/5749)
  (merged to `main`, not yet on the `v0.7` branch we depend on) declares
  `muda`/`tray-icon` with `default-features = false` and makes `libxdo`
  opt-in as a new `linux-libxdo` feature.

**Condition for deleting it.** Once `dioxus-desktop` resolves to a version
whose manifest defines `linux-libxdo` (i.e. #5749 has reached a release we
depend on), `libxdo` is opt-in upstream and this patch is no longer needed:
delete `vendor/muda/` and the `[patch.crates-io]` entry in the workspace
`Cargo.toml`. `cargo xtask audit-deps` checks for this condition on every run
and fails, naming it, once it is met — see `xtask/src/main.rs`.
