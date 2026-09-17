# Store listing content

Tracked, reviewable source for the Microsoft Store listing — RFC-079 §9 Q5's
precondition. **Publishing this to Partner Center stays a manual step**;
nothing here is read by `store-submit.ps1` or any workflow. This exists so
that manual step has something version-controlled to copy from, instead of
text that only ever existed inside Partner Center.

- `en-us/listing.toml`, `ja-jp/listing.toml` — title, short/long
  description, search terms, and which screenshots represent the listing,
  one per market.
- `en-us/identity.toml` — the fixed `Identity`/`Publisher`/
  `PublisherDisplayName` triple Partner Center has on file, so
  `store-validate.ps1` can catch a manifest edit that drifts from it (see
  that file's own comment for what this check can and cannot catch). This
  is language-neutral package identity, not listing content — it stays in
  `en-us/` only; `store-submit.ps1` and `store-validate.ps1` read it from
  that exact path, and a second market does not get its own copy.
- `en-us/screenshots/`, `ja-jp/screenshots/` — committed PNGs, not a
  promise to regenerate them. `packaging/render_check.py` walks the AT-SPI
  tree and asserts geometry; it captures no images, so there is no
  machinery to generate these from. They were captured by hand against the
  real, running application.

Two markets exist today, matching `AppxManifest.xml`'s two `<Resource>`
entries (`en-US`, `ja-JP`, F108). A further market gets its own `<lang>/`
sibling directory holding `listing.toml` and `screenshots/` — not
`identity.toml`, which is not per-market. Every market's `listing.toml`
must make the same claims about the product; a difference in wording is
expected, a difference in what the app can do is not.

## Manifest versus listing: which wins

`AppxManifest.xml`'s `Description`, `DisplayName`, and `PublisherDisplayName`
are **OS-facing** strings — what Windows shows in the Start menu, Task
Manager, and package properties. They come from the manifest and nowhere
else; Partner Center never sees them.

`listing.toml`'s `short_description` and `long_description` are
**shopper-facing** marketing copy — what appears on the Store product page.
They come from Partner Center (seeded from this file, by hand) and the
manifest never sees them.

These are different surfaces, not the same field under two names — but both
describe the same product, and can drift into disagreeing about what it
does. **`listing.toml` is the source.** When wording changes, it changes
here first; the manifest's `Description` attribute is a compressed
derivative of `short_description`, not authored independently of it. The
manifest schema takes literal text, not a pointer into this file, so nothing
enforces this mechanically — it is enforced by this document and by review,
the same way `packaging/linux/PKGBUILD`'s description field has no automated
tie to this project's own README either.
