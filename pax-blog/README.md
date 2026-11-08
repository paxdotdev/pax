# Pax blog

The first-party blog lives in this monorepo at `pax-blog/`, independently of the
Cargo workspace. Keeping its Markdown, theme overlay, website links, and hosting
configuration together makes URL and publishing changes reviewable in one diff.
Zola produces static files for **https://www.pax.dev/blog/**. No production
application server or database is needed.

The selected baseline is **Zola 0.23.6 + Tabi 5.0.0**, pinned in
`dependencies.json`. The small local theme overlay uses the website's warm dark
palette, Manrope typography, and white accents. Theme sources, tool binaries, and generated output
are ignored; authored content, configuration, and overrides are versioned.

## Local setup and preview

From the monorepo root, with Python 3 and curl installed:

```sh
python3 pax-blog/blog.py setup
python3 pax-blog/blog.py serve
```

Open **http://127.0.0.1:8095/blog/a-place-for-notes/**. Zola watches content, templates,
and styles and reloads the browser. `--port 8100` selects another port and updates
the generated base URL. The server binds to loopback only.

Setup downloads and checks the pinned theme archive. On Apple Silicon macOS it
also installs the pinned Zola release into `.tools/`, without replacing system
tools. On another platform, install Zola 0.23.6 from its
[official releases](https://github.com/getzola/zola/releases/tag/v0.23.6), then run
setup with `ZOLA_BIN=/absolute/path/to/zola`. The same override can be used on
macOS. Setup refuses unexpected archive checksums; dependency updates require
reviewing the version, source, digest, and compatibility together.

### Shared Pax header

The navigation's single source is `pax-website/src/site_header.{rs,pax}`.
The website's `SiteShell` composes it alongside the homepage logo-transfer
destination. `pax-blog/header` is only a runnable host with a local Cargo path
dependency on `pax-website`; it does not contain another header implementation.
Changing the shared component requires rebuilding both applications.
The dependency disables the website's default `app` feature, omitting its
application entry point, split-Wasm loader, non-header modules, four demo
dependencies, and custom shader support. `SiteHeader`, `NavLogo`, and `SiteTheme`
remain available from the same website source files. The ordinary renderer and
software fallback remain enabled; this is not a reduced-compatibility build.

Build and stage the header before previewing the blog:

```sh
source ~/.zshrc
nvm use
cargo build -p pax-cli
./target/debug/pax-cli build --path pax-blog/header --target web --libdev
python3 pax-blog/blog.py stage-header
python3 pax-blog/blog.py serve
```

The staged bundle lives in ignored `static/_pax/header/`, then Zola includes it
at `/blog/_pax/header/`. Staging preserves the matching JS, Wasm, CSS, and snippets,
but packages only the assets explicitly listed in `blog.py`'s `HEADER_ASSETS`:
Manrope and its license/provenance. Update that list when the header acquires
another local asset. This is an explicit header deployment closure, not automatic
asset tree-shaking: the underlying Pax build still copies component-package assets.
Staging relocates website-root font preloads to the header's deployment prefix
and rejects preloaded local fonts missing from the asset list. The original
build and website assets are never pruned.

Release staging also minifies the interface and cartridge JS with the existing
Pax interface toolchain's pinned esbuild. Its classic `window.Pax` entry and ESM
exports/import URLs are preserved, as are legal comments. Debug staging leaves
JS readable. Source NVM in the staging shell too; if dependencies are missing,
run `npm ci --prefix pax-compiler/files/interfaces/web` from the repository root.
Staging validates assets and completes minification before replacing the previous
marked bundle, so a failed build/minifier leaves the previous preview intact.
The debug bundle also attempts a development connection to its serving origin;
on the website's server this belongs to a different cartridge, so its manifest
is rejected. Stage a release header for a clean integrated preview without that
development connection.

The blog's ordinary HTML remains readable and scrollable throughout startup.
The mount uses `init_web` without page-scroll/history ownership and handles
`pax-navigate` as document navigation. The HTML header is the startup/error
fallback and remains the keyboard/screen-reader navigation after enhancement;
keyboard focus restores its visual rendering. No blog Router or iframe is used.
An embedded instance is retained during browser back/forward-cache suspension.

For publication, build the header with `--release`, then run (from a shell with
the repository's Node version selected)
`python3 pax-blog/blog.py stage-header --profile release`. Production blog builds
reject a staged debug header. This copies local artifacts; it does not deploy.

`content/a-place-for-notes.md` is a lorem ipsum reading-layout draft, available at
`/blog/a-place-for-notes/` in draft previews. It is not launch copy.
The old 0.39.0 announcement is an explicitly marked draft stub. Its date and byline are
layout placeholders, not a publication decision. Replace its body with the
approved Markdown from PAX-978, confirm metadata, and only then remove
`draft = true`. PAX-985 is a different article. Historical imports and any
embedded Pax demos remain follow-up content work; this project makes no new
release claims and imports no subscriber data.

## Build and content

```sh
python3 pax-blog/blog.py build
python3 pax-blog/blog.py check
```

`build` writes production-base output to `pax-blog/public/` and **excludes drafts**.
Until the announcement is approved, the production index and Atom feed have no
posts. `check` also checks external links and needs network access. For a static
local draft build, use `python3 pax-blog/blog.py preview-build`; it writes to
`preview/`, which is also used for the dev server's assets. Avoid building into
that directory while its server is running.

Posts live under `content/` with explicit title, description, date, authors, stable
slug, and optional tags. Index pagination, archive, tags, Atom feed, sitemap,
syntax highlighting, and social metadata come from Zola/Tabi. We use Atom rather
than adding an RSS template that would need a separate author/email convention.
The local override supplies self-canonical URLs. Drafts and loopback previews
receive `noindex`; an unapproved draft must never be published with `--drafts`.

Edit `static/pax.css`, `templates/partials/nav.html`, or
`templates/tabi/extend_head.html` for the Pax overlay. Do not modify downloaded
theme files. The header deliberately links to `/` on the current origin, allowing
the same navigation in production and an integrated local preview. The standalone
Zola server has no Pax homepage at `/`; use the integrated preview for that link.
Search, analytics, theme-switching scripts, and comments are disabled. Article
content is static HTML; the optional header enhancement loads a Pax bundle.
The CSP allows same-origin scripts, Wasm compilation, and Pax's dynamic styles.
Zola's development server additionally uses live reload.

For future interactive examples, use a same-origin iframe containing a complete,
versioned Pax web bundle, loaded after an explicit activation. Keep article text,
poster, caption, and links in ordinary HTML. The existing embedding research is
archived in [PAX-984](https://linear.app/paxdev/issue/PAX-984). The header now uses
the embedded-mount API directly; article examples remain a separate future step.

## Verify navigation from the Pax website

Build the CLI from this checkout so it includes `server_owned_prefixes`:

```sh
source ~/.zshrc
nvm use
cargo run -p pax-cli -- run --path pax-website --target web --libdev --hot-reload=off
```

In another terminal, use the port printed by Pax (shown here as `59881`):

```sh
python3 pax-blog/blog.py stage --port 59881
```

This generates a **local draft preview** in the website's ignored `public/blog/`.
The running Pax server sees the new files without restarting. Click **PAX NOTES**
near the bottom of the homepage, then the announcement. Check Back, Forward,
refresh, and direct article links. `/blog/missing` should return 404; `/blogger`
should remain an application path. Header script/Wasm requests must use the
dedicated `/blog/_pax/header/` bundle, never bootstrap the full website root.

`stage` is a snapshot; rerun it after edits. Use the separate Zola server for
automatic rebuilding. The website declares `/blog`, `/ai`, and `/ai.md` in
`[package.metadata.pax.web].server_owned_prefixes`; the old generated blog Router
placeholder has been removed. Metadata changes require rebuilding/restarting Pax.

Remove the staged preview **before building a website release for publication**:

```sh
python3 pax-blog/blog.py unstage
```

The command only removes a directory marked as generated by `stage`. Production
publication will upload the blog separately to `s3://www.pax.dev/blog/`; a website
release must contain no local draft preview. Website-root uploads must exclude
`blog/*`, including when deleting stale website files.
Neither this helper nor `build` uploads anything or invokes AWS/Terraform.

## Hosting and checks

[Hosting and publication coordination](infra/README.md) describes the selected
shared-bucket plan, the existing website publisher, and the PAX-869 handoff.
Terraform is deferred; the earlier private-origin draft is superseded. The
publication commands are not implemented by `blog.py`; running a local build
does not deploy anything.

Keep `pax-blog/` independent of the Rust workspace. The website lives beside it
at root-level [`pax-website/`](../pax-website/README.md), explicitly excluded from
the Cargo workspace to preserve its standalone profiles. Sibling projects can share
a publishing command that builds both production outputs before uploading either,
while retaining fast blog-only builds and publication. Nesting the blog is not
required for that workflow. Local staging uses `pax-website/public/blog/`.

[PAX-984](https://linear.app/paxdev/issue/PAX-984) preserves the foundation's
verification results and known limits.

Focused checks from the monorepo root:

```sh
python3 -B -m unittest discover -s pax-blog -p 'test_*.py' -v
source ~/.zshrc
nvm use
node --test pax-blog/pax-header.test.mjs
node --test pax-blog/minify-header.test.mjs pax-blog/header-boundary.test.mjs
node --test pax-compiler/files/interfaces/web/tests/navigation.test.mjs
node --test pax-compiler/files/interfaces/web/tests/route-location.test.mjs
node --test pax-compiler/files/interfaces/web/tests/route-metadata.test.mjs
cargo test -p pax-compiler --lib
mdbook build pax-docs/book
```

## Third-party sources

- [Zola 0.23.6](https://github.com/getzola/zola/tree/v0.23.6): EUPL-1.2. Used as a build tool; not shipped as an executable in the site.
- [Tabi 5.0.0](https://github.com/welpo/tabi/tree/v5.0.0): MIT. Its original license and asset licenses remain in the downloaded theme. The footer retains theme credit.
- Tabi's unused optional assets remain in its build output for now. Page transfer is much smaller than the output directory; trim optional assets only after checking references and licenses.
