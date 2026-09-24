# Foundation verification — 22 September 2026

This verifies the local foundation and stub, not publication or final release copy.

24 September update: the selected deployment plan now shares the existing
`www.pax.dev` bucket under `blog/`. Terraform and the separate private-origin
rewrite below are historical checks of a superseded draft. The existing website
publisher was located and the preservation/integration requirements handed to
PAX-869; publication itself remains unimplemented and untested. See the current
[hosting notes](infra/README.md).

- Compiler suite: **193 tests passed**. Includes metadata validation, safe bootstrap
  JSON, propagation into nested route entries, and local server fallback boundaries.
- Web navigation/location/metadata suites: **15 tests passed**. Covers same-origin
  delegation, path boundaries, relative destinations, queries, fragments, new tabs,
  and query-backed embeds. CloudFront rewrite suite: **1 test passed**.
- Pax website: debug and release builds passed. Both emit the same policy and the
  same web-interface JS bytes. Release baking was audited: this setting is emitted
  during web interface preparation, before cartridge generation, and is not an
  execution field in the program IR or binary format. No Wasm format change is needed.
- Browser integration: clicked the real website's **PAX NOTES** link, opened the
  article, used Back/Forward, and reloaded it. The resulting static article has no
  script tags or Pax mount. The shared navigation path runs through `Link`'s Rust
  `NodeContext::navigate_to` call and the web chassis handler.
- Desktop and 390px mobile article layouts inspected; mobile page width equals the
  viewport width. Index, article, archive, and tag pages return HTML with working
  stylesheet URLs. Missing blog documents/assets return 404. `/blogger` receives
  application HTML when requested with the browser's HTML Accept header.
- Zola's actual dev server serves `/blog/` and rebuilds both a temporary Markdown
  edit and its restoration. The custom output path requires `--force` during
  watch rebuilds; the runner includes it. The temporary probe was removed.
- Production-base Zola build and `zola check` passed. The announcement stub is
  excluded from production output; index/feed/sitemap contain no localhost URLs
  or stub references. Preview Atom and sitemap XML parse successfully.
- API docs regenerated from source comments; the example-source bundle was synced;
  `mdbook build pax-docs/book` passed.
- Terraform formatting and schema validation passed with the locked AWS provider.
  AWS inspection used only CloudFront listing/configuration reads and a website
  bucket configuration read. No plan, import, apply, upload, or invalidation ran.

The repository-wide TypeScript check still reports **24 existing diagnostics**.
An isolated copy using the original navigation handler produces the identical
diagnostics after normalizing source locations. None originates in the new
navigation module. The production bundler and focused executable tests pass.

The local Zola preview is at `http://127.0.0.1:8095/blog/pax-0-39-0/`; the integrated
Pax preview used `http://127.0.0.1:59881/`. These addresses are workstation-local
and last only while their processes run. The integrated preview uses generated
draft files in `examples/src/pax-website/public/blog/`; run `blog.py unstage`
before any later website release build intended for publication. The debug and
release artifacts verified here contain no bundled draft blog.

Final announcement copy, archive migration, real embedded Pax demos, production
deployment, and production acceptance remain outside this completed foundation step.
