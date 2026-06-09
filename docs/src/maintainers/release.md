# Release Process

1. Update `CHANGELOG.md` with the new version and date.
2. Bump `version` in `Cargo.toml` (workspace `[workspace.package]`).
3. Run `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`.
4. Move completed RFCs from `rfcs/proposed/` to `rfcs/done/` and update
   `rfcs/README.md`.
5. Build the release archive (see below).

## Archive format

The archive unpacks to a top-level `forskscope-vX.Y.Z/` directory
containing the Cargo project files directly — no nested subdirectory.

```sh
VER=0.23.0
git archive --prefix="forskscope-v${VER}/" HEAD \
  | gzip > "forskscope-v${VER}.tar.gz"
```

Or, to build from a working tree (excluding `target/`):

```sh
VER=0.23.0
tar --exclude='./target' --exclude='./.git' \
    -czf "forskscope-v${VER}.tar.gz" \
    --transform "s,^\.,forskscope-v${VER}," .
```
