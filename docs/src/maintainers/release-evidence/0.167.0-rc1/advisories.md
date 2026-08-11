# Security Advisory Dispositions

**Source commit:** see this evidence directory's `README.md` / `artifacts.md`
once a real release candidate is cut against this plan.
**Review date:** 2026-08-11
**Reviewer:** implementer (M4-C1 handoff), pending owner/architect sign-off.

`cargo audit` currently exits 0 while reporting **14 warnings** — 12
`unmaintained`, 2 `unsound` — plus 2 further advisories suppressed outright in
`.cargo/audit.toml`. Per Gate C: *"`cargo audit` exit success is not
sufficient by itself. Every unsoundness advisory must have a reachability
statement, owner, review date, and upgrade trigger in the release evidence."*
Per advisory N5: *"keep policy-pass distinct from a clean advisory set."* This
document is that distinction, made explicit. A green `cargo audit` here means
*reviewed and accepted*, not *clean*.

This directory name (`0.167.0-rc1`) is a **placeholder** chosen only so this
evidence has somewhere to live before a real release candidate exists — see
this slice's review request for the explicit question to the owner about the
actual version/RC identifier. Nothing in this document depends on the
placeholder being correct; the dispositions below are tied to dependency
versions and code paths, not to a release number.

---

## 1. Unsoundness advisories — full disposition

### RUSTSEC-2024-0429 — `glib` 0.18.5

**Title:** Unsoundness in `Iterator`/`DoubleEndedIterator` impls for
`glib::VariantStrIter`

**What the advisory says:** `VariantStrIter::impl_get` (the internal method
backing `next`/`nth`/`last`/`next_back`/`nth_back`) passes an immutable
reference to a raw pointer into a C function that mutates it in place as an
out-argument. Under optimization, the write is disregarded, and every call
violates `CStr::from_ptr`'s safety requirement of pointing at a valid C
string — producing crashes from NULL-pointer dereferences. Fixed in `glib`
0.20.0 by passing the pointer as `&mut` instead of `&`.

**Dependency path.** `glib` is transitive through the Linux GTK/WebKitGTK
desktop backend, reached via `dioxus-desktop` → (`tao`, `wry`, `muda`,
`tray-icon`) → `gtk`/`gdk`/`webkit2gtk` → `glib`. ForskScope's own crates
(`forskscope-core`, `forskscope-ui-logic`, `forskscope-ui`) never depend on
`glib` directly — confirmed by `grep -rn "glib::\|use glib" crates/` finding
no matches.

**Reachability — not reachable, established by full-source search.**
`VariantStrIter` has exactly one public constructor:
`glib::Variant::array_iter_str(&self) -> Result<VariantStrIter, ...>`
(`glib-0.18.5/src/variant.rs:843`). It is the *only* way to obtain the
unsound iterator anywhere in the crate. Searched every locally cached
version of every crate in the dependency chain that could reach `glib`
(`atk`, `atk-sys`, `cairo-rs`, `gdk`, `gdk-pixbuf`, `gdk-sys`, `gio`,
`glib-macros`, `gtk`, `gtk-sys`, `gtk3-macros`, `muda`, `tao`, `tray-icon`,
`webkit2gtk`, `webkit2gtk-sys`, `wry`) for any call to `.array_iter_str(`:

```text
$ grep -rln "array_iter_str" ~/.cargo/registry/src/*/*/src
glib-0.18.5/src/variant.rs        # the definition itself
glib-0.18.5/src/variant_iter.rs   # the Iterator/DoubleEndedIterator impls
```

No other crate's source — including every crate actually in this project's
resolved dependency chain to `glib` — calls it. `glib::Variant`'s
string-array iteration API exists to support GVariant/D-Bus interop (menu
definitions, application actions with string-list parameters); nothing in
ForskScope's window-toolkit usage (a single top-level application window, no
D-Bus service, no GMenu/GAction string-array construction) exercises that
surface.

**Owner:** nabbisen.

**Review date:** 2026-08-11.

**Upgrade trigger:** `atk`/`gdk`/`gtk`/`webkit2gtk` (the gtk-rs 0.18 family)
publish a release depending on `glib >= 0.20`, or `dioxus-desktop`/`tao`/
`wry`/`muda`/`tray-icon` bump their own gtk-rs dependency past that line —
whichever lands first pulls a fixed `glib` in automatically. Revisit
immediately if any future ForskScope code (or a dependency bump) begins
constructing/iterating `glib::Variant` string arrays, which would change this
disposition's premise regardless of the trigger above.

**Release decision:** Accept. Not reachable; no ForskScope code path, and no
code in the resolved dependency graph, exercises the unsound API.

---

### RUSTSEC-2026-0097 — `rand` 0.7.3

**Title:** Rand is unsound with a custom logger using `rand::rng()`

**What the advisory says:** unsound only when *all* of: the `log` and
`thread_rng` features are enabled, a custom `log::Log` implementation is
installed as the global logger, that logger itself calls `RngCore` methods on
`ThreadRng`, `ThreadRng` reseeds while inside that logger call (every ~64kB
of generated data), and trace-level logging is active (or warn-level with
`getrandom` failing). None of these are inherent to merely depending on
`rand` 0.7.3 — the unsound path requires an application to wire a custom
logger into `rand`'s internals.

**Dependency path — established via `cargo tree -i rand@0.7.3 --target all`:**

```text
rand v0.7.3
└── phf_generator v0.8.0
    └── phf_codegen v0.8.0
        [build-dependencies]
        └── selectors v0.24.0
            └── kuchikiki v0.8.8-speedreader
                └── wry v0.53.5
                    └── dioxus-desktop v0.7.9
                        └── forskscope-ui v0.166.1
```

**Reachability — not reachable, on two independent grounds.**

1. **It is a build-time-only dependency.** The entire path to `rand` passes
   through a `[build-dependencies]` edge (`phf_codegen`, used by `selectors`'
   own build script to generate a perfect-hash-function lookup table at
   compile time). `rand` 0.7.3 is never a runtime dependency of the shipped
   `forskscope`/`forskscope-ui` binary — it does not link into the artifact
   users run, only into the build process that produces it. This also
   answers the handoff's specific question of *why* a `rand` 0.7 is still in
   the graph at all: it's an old, stable, unmaintained-but-inert build tool
   nobody has had reason to touch, not a live runtime choice.
2. **Even within that build-time context, the unsound preconditions aren't
   met.** `phf_generator`'s own source (a small, self-contained codegen
   crate) defines no custom `log::Log` implementation, and nothing in
   ForskScope's own build (`build.rs`, `xtask`, or the workspace `Cargo.toml`)
   installs a global logger during compilation. Absent a custom logger
   calling back into `ThreadRng` during a reseed, the specific aliasing bug
   cannot occur.

**Owner:** nabbisen.

**Review date:** 2026-08-11.

**Upgrade trigger:** `kuchikiki`, `selectors`, or `phf_codegen` publish a
release moving off `rand` 0.7, or `wry` moves off `kuchikiki`/`selectors`
entirely (both are plausible independent of any advisory pressure — this is
old, likely-legacy tooling in that dependency's own build chain). Revisit if
any future build script in this workspace installs a custom global logger,
which is not expected but would change this disposition's second ground.

**Release decision:** Accept. Not reachable at runtime (build-dependency
only), and the unsound preconditions are not met even during the build
itself.

---

## 2. Unmaintained advisories — policy statement

**Policy.** An `unmaintained` RustSec advisory means the crate's authors have
stated they will not publish further updates — it is a statement about
*future support*, not a report of a specific defect in the code as it
exists today. For this project's risk posture:

- An unmaintained crate is accepted to remain in the dependency graph when
  (a) it has no separately-reported vulnerability of its own, and (b) it is
  reached through a version-pinned, actively-maintained parent crate that
  controls when and whether to move off it.
- **A pass under this policy is explicitly not the same as a clean advisory
  set.** `cargo audit`'s exit-0 with warnings is a passive default, not an
  evaluated decision (N5) — this section is that decision, recorded once
  rather than re-litigated per advisory.
- An unmaintained crate becomes a blocker — requiring an upgrade path or a
  documented mitigation before the next release — the moment RustSec (or any
  other source) attaches an actual vulnerability advisory to it, or the
  parent crate that pulls it in stops updating in turn.

**The twelve, by dependent family:**

| Crate | Version | Advisory | Pulled in via |
|---|---|---|---|
| `atk` | 0.18.2 | RUSTSEC-2024-0413 | gtk-rs 0.18 (GTK3 bindings), via `dioxus-desktop`'s Linux backend |
| `atk-sys` | 0.18.2 | RUSTSEC-2024-0416 | same |
| `gdk` | 0.18.2 | RUSTSEC-2024-0412 | same |
| `gdk-sys` | 0.18.2 | RUSTSEC-2024-0418 | same |
| `gdkwayland-sys` | 0.18.2 | RUSTSEC-2024-0411 | same |
| `gdkx11-sys` | 0.18.2 | RUSTSEC-2024-0414 | same |
| `gtk` | 0.18.2 | RUSTSEC-2024-0415 | same |
| `gtk-sys` | 0.18.2 | RUSTSEC-2024-0420 | same |
| `gtk3-macros` | 0.18.2 | RUSTSEC-2024-0419 | same |
| `fxhash` | 0.2.1 | RUSTSEC-2025-0057 | transitive, hashing utility |
| `paste` | 1.0.15 | RUSTSEC-2024-0436 | transitive, proc-macro token-pasting helper |
| `proc-macro-error` | 1.0.4 | RUSTSEC-2024-0370 | transitive, proc-macro error-reporting helper |

Nine of the twelve are the gtk-rs 0.18 GTK3-binding family (the same
`unsound`-advisory `glib` 0.18.5 belongs to, above) — the entire family moves
together whenever `dioxus-desktop`'s Linux stack bumps its GTK3 bindings, so
they share one upgrade trigger rather than needing nine separate ones.
`fxhash`, `paste`, and `proc-macro-error` are unrelated small utility crates
pulled in transitively; each is a leaf dependency of some other actively
maintained crate in the graph, not a direct ForskScope dependency.

---

## 3. Suppressed advisories — restated from `.cargo/audit.toml`

`.cargo/audit.toml` carries this rationale as a code comment:

> quick-xml 0.39 remains only through wayland-scanner, a proc-macro/codegen
> dependency in the GTK/Dioxus Wayland stack. The runtime XLSX parser path
> through sheets-diff/calamine has been removed, so user-supplied workbook
> XML no longer reaches this vulnerable quick-xml version. `cargo xtask
> audit-deps` enforces that this exception remains limited to the reviewed
> path.

That rationale is accurate, verified independently below, and restated here
in the same four-field form as the unsoundness advisories — a comment in a
config file is not release evidence.

### RUSTSEC-2026-0194 — `quick-xml` 0.39.4 — quadratic duplicate-attribute check

**What it is:** `BytesStart::attributes()`'s default duplicate-name check is
`O(N²)` in the number of attributes on one start tag — a crafted tag with a
large, unbounded attribute count can pin a CPU core parsing untrusted XML.
Fixed in `quick-xml` 0.41.0.

**Dependency path — `cargo tree -i quick-xml --target all`:**
```text
quick-xml v0.39.4
└── wayland-scanner v0.31.10 (proc-macro)
    └── wayland-client / wayland-protocols → rfd → dioxus-desktop → forskscope-ui
```

**Reachability:** not reachable. `wayland-scanner` is a `proc-macro`,
build-time-only crate that parses the Wayland protocol's own bundled XML
specification files (fixed, project-controlled inputs shipped inside the
`wayland-protocols`/`wayland-client` crate sources) to generate Rust bindings
at compile time. It never parses attacker-supplied or user-supplied XML at
runtime — ForskScope's own XLSX/XML-adjacent code path (`sheets-diff`/
`calamine`) has been removed entirely (security-disabled per an earlier
decision), so there is no runtime XML-parsing surface in this project at all
that this advisory's DoS could reach.

**Owner:** nabbisen. **Review date:** 2026-08-11. **Upgrade trigger:**
`wayland-scanner` publishes a release depending on `quick-xml >= 0.41`, or
this project's Wayland dependency chain changes. **Enforcement:** `cargo
xtask audit-deps`'s `assert_quick_xml_path_is_reviewed` fails the gate if
`quick-xml`'s only immediate dependent stops being `wayland-scanner` —
i.e., if this exception's premise ever stops holding, the build breaks
before a silent scope-widening could happen.

**Release decision:** Accept, suppressed via `.cargo/audit.toml`, enforced
by `cargo xtask audit-deps`.

### RUSTSEC-2026-0195 — `quick-xml` 0.39.4 — unbounded namespace-declaration allocation

**What it is:** `NsReader` allocates an internal `NamespaceBinding` per
`xmlns`/`xmlns:*` attribute on every start tag, with no upper bound, before
the caller ever sees the event — a crafted tag can force large heap
allocations independent of any input-size limit the caller enforces. Fixed
in `quick-xml` 0.41.0 (configurable cap, default 256).

**Dependency path:** identical to RUSTSEC-2026-0194 above —
`wayland-scanner`'s build-time-only use of `quick-xml`.

**Reachability:** not reachable, for the same reason as RUSTSEC-2026-0194:
`wayland-scanner` parses fixed, non-adversarial protocol XML at build time
only; there is no runtime XML-parsing surface in this project reachable by
an attacker.

**Owner:** nabbisen. **Review date:** 2026-08-11. **Upgrade trigger:** same
as RUSTSEC-2026-0194. **Enforcement:** same — `cargo xtask audit-deps`.

**Release decision:** Accept, suppressed via `.cargo/audit.toml`, enforced
by `cargo xtask audit-deps`.

---

## 4. Summary table

| ID | Crate | Class | Reachable? | Decision |
|---|---|---|---|---|
| RUSTSEC-2024-0429 | glib 0.18.5 | unsound | No — no call site to the only constructor anywhere in the resolved dependency source | Accept |
| RUSTSEC-2026-0097 | rand 0.7.3 | unsound | No — build-dependency only, never linked into the shipped binary; unsound preconditions also unmet at build time | Accept |
| RUSTSEC-2024-0411…0420, 0413, 0436, 0370, 2025-0057 (×12) | see §2 table | unmaintained | N/A (informational; policy-governed, not a specific defect) | Accept under §2 policy |
| RUSTSEC-2026-0194 | quick-xml 0.39.4 | DoS (informational, suppressed) | No — build-time-only, non-adversarial input | Accept, suppressed |
| RUSTSEC-2026-0195 | quick-xml 0.39.4 | DoS (informational, suppressed) | No — same | Accept, suppressed |

No dependency was added, removed, or version-changed to produce this
document, per this slice's constraints.
