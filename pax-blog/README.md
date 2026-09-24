# Pax blog

The first-party blog lives in this monorepo at `pax-blog/`, independently of the
Cargo workspace. Keeping its Markdown, theme overlay, website links, and hosting
configuration together makes URL and publishing changes reviewable in one diff.
Zola produces static files for **https://www.pax.dev/blog/**. No production
application server or database is needed.

The selected baseline is **Zola 0.23.6 + Tabi 5.0.0**, pinned in
`dependencies.json`. The small local theme overlay uses the website's warm dark
palette and yellow accent. Theme sources, tool binaries, and generated output
are ignored; authored content, configuration, and overrides are versioned.

## Local setup and preview

From the monorepo root, with Python 3 and curl installed:

```sh
python3 pax-blog/blog.py setup
python3 pax-blog/blog.py serve
```

Open **http://127.0.0.1:8095/blog/pax-0-39-0/**. Zola watches content, templates,
and styles and reloads the browser. `--port 8100` selects another port and updates
the generated base URL. The server binds to loopback only.

Setup downloads and checks the pinned theme archive. On Apple Silicon macOS it
also installs the pinned Zola release into `.tools/`, without replacing system
tools. On another platform, install Zola 0.23.6 from its
[official releases](https://github.com/getzola/zola/releases/tag/v0.23.6), then run
setup with `ZOLA_BIN=/absolute/path/to/zola`. The same override can be used on
macOS. Setup refuses unexpected archive checksums; dependency updates require
reviewing the version, source, digest, and compatibility together.

The announcement is an explicitly marked draft stub. Its date and byline are
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
Search, analytics, theme-switching scripts, and comments are disabled. Built
article HTML has no JavaScript; Zola's development server adds only live reload.

For future interactive examples, use a same-origin iframe containing a complete,
versioned Pax web bundle, loaded after an explicit activation. Keep article text,
poster, caption, and links in ordinary HTML. The existing embedding research is
in `pax-docs/research/pax-984/README.md`; no multi-mount capability is assumed.

## Verify navigation from the Pax website

Build the CLI from this checkout so it includes `server_owned_prefixes`:

```sh
source ~/.zshrc
nvm use
cargo run -p pax-cli -- run --path examples/src/pax-website --target web --hot-reload=off
```

In another terminal, use the port printed by Pax (shown here as `59881`):

```sh
python3 pax-blog/blog.py stage --port 59881
```

This generates a **local draft preview** in the website's ignored `public/blog/`.
The running Pax server sees the new files without restarting. Click **PAX NOTES**
near the bottom of the homepage, then the announcement. Check Back, Forward,
refresh, and direct article links. `/blog/missing` should return 404; `/blogger`
should remain an application path. No root-app script or Wasm should appear in
the article's initial document.

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

Keep `pax-blog/` independent of the Rust workspace. The proposed move of the
website to root-level `pax-website/` belongs to PAX-869. Sibling projects can share
a publishing command that builds both production outputs before uploading either,
while retaining fast blog-only builds and publication. Nesting the blog is not
required for that workflow. Until the move lands, local staging still uses
`examples/src/pax-website/public/blog/`.

[Verification results](VERIFICATION.md) record the checks and known limits of this foundation.

Focused checks from the monorepo root:

```sh
source ~/.zshrc
nvm use
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
