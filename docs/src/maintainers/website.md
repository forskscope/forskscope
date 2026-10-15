# The Website

<https://forskscope.github.io/forskscope/> serves two things:

| What | Source | Published at |
|---|---|---|
| The download page | `docs/site/index.html` (one file, no build step) | the site root |
| The user guide | `docs/src/`, built by `mdbook` with `docs/book.toml` | `/guide/` |

The page's images are the guide's own (`guide/assets/…`), so each image exists
once, in `docs/src/assets/`.

## How it is published

`.github/workflows/pages.yml` runs on every push to `main` that touches `docs/**`
or the workflow itself, and on `workflow_dispatch`. It:

1. Installs `mdbook`, pinned to an exact version and checked against a SHA-256
   recorded in the workflow.
2. Builds the guide (`mdbook build docs`).
3. Assembles `_site/`:
   - `docs/site/` at the root;
   - the built guide under `guide/`;
   - the guide's `404.html` copied to the root, so any missing path gets a
     styled page.
4. Runs the two gates below. A failure stops the deploy.
5. Deploys with GitHub's Pages actions.

**Permissions:**

- `contents: read`, `pages: write`, `id-token: write`, and nothing more.
- Concurrency group `pages`, never cancelled midway.
- The security notes cover this surface (*Threat Model*, §8a).

**The site is not versioned and has no tag.** It follows `main`'s docs. The
download page links to `releases/latest`, never to a named version, so it
cannot go stale. Work that changes only the site is delivered without a release
(*Release Process*).

## The two gates

| Gate | What it checks |
|---|---|
| `cargo xtask pages-origin-check _site/index.html` | The download page loads nothing from another origin. It checks `src`, `srcset`, `poster`, `<object data>`, `<link href>`, `@import` and `url()`, case-insensitively. Links a person clicks (`<a href>`) are allowed. |
| `cargo xtask pages-link-check _site` | Every relative link and image in the site resolves to a file in it. |

The page's footer promises *"This page loads nothing from other sites and
collects nothing."* **The origin gate is what holds that promise.** A web font, an
analytics snippet or a CDN script fails the deploy.

**To run both locally,** assemble the site the way the workflow does:

```sh
mdbook build docs
rm -rf _site && mkdir -p _site/guide
cp -r docs/site/. _site/ && cp -r docs/book/. _site/guide/
cp _site/guide/404.html _site/404.html
cargo xtask pages-origin-check _site/index.html
cargo xtask pages-link-check _site
```

## Changing the download page

**Its wording is the owner's.** The page is the project's public statement, and
every sentence on it was approved by the owner, including the
platform notes and the privacy line. A change to what the page *says* needs the
owner's approval before it is published. A change to markup or style that leaves
the text unchanged does not.

**Screenshots of the app** in the page and the README come from a sandboxed run:

- the real home folder is hidden, and the app sees a made-up `/home/you`;
- the files shown are invented;
- only the app's own window is captured;
- image metadata is stripped;
- the files are checked for the maintainer's user name and real paths before
  committing.

**Never take a public screenshot from a real working session.** The Explorer
shows full paths.
