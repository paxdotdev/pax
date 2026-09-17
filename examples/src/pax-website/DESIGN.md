# Living Quilt website pass

## Concept and layout

The website begins with the actual Living Quilt: an edge-to-edge, interactive
geometric field, with its original logo animation, lighting, slides, and color
rings. The content below is its quiet counterpart: warm-black surfaces, warm
white headlines, gray body text, square-edged cards, and fine rules. Color is
reserved for the primary action and small, vivid demonstrations.

The page sequence is quilt → introduction and CLI → creative freedom → language
and logic → native/web and performance → horizontal feature gallery → start
building. The content rationale and claim boundaries are in `CONTENT.md`.
The provisional authoring animation and explanatory graphics were removed in
the September 15 editorial pass; intentional new demonstrations come later.
Desktop introduction uses
two columns; mobile stacks the same content. The quilt retains its own 3/5-column
responsive behavior. Its interactions stay inside the hero's clipping frame.

`SiteTheme` owns the shared palette, sampled from Living Quilt's grays and vivid
color sets (the violet is lightened for small-text contrast). Existing typography
remains Manrope with IBM Plex Mono labels and code. Feature-card copy and
qualifications are unchanged. The gallery keeps its
local native-scroller marquee; `pax-std::Carousel` is not involved.

## One source of truth for the hero

`Cargo.toml` imports `living-quilt` using `path = "../living-quilt"`, relative to
the manifest. `hero_section.rs` imports its public `Example` as `LivingQuilt`.
The website mounts this component directly: no iframe, source copy, symlink,
or separate embedded app session. The example remains independently runnable.

Pax resolves templates relative to the component's declaring crate. The source
drawer uses Rust `include_str!` paths relative to `home_page.rs`, so those paths
also do not depend on the invoking shell's working directory. These source
strings are the existing ExampleHost inspection mechanism, not a code fork.
There are no external media assets to remap in the current quilt.

Keep this composed example out of `pax-std`. If it later needs distribution
outside this monorepo, publish or package the example as its own component
crate; do not turn a branded demo into a standard-library dependency.

## Validation

Build with the worktree's CLI and `--libdev` so the rebased renderer and compiler
are used. Check desktop and 390px mobile, quilt interaction and source selection,
the CLI link, gallery clipping and motion, and the existing blog/default routes.
Also build the release cartridge to verify dependency discovery and palette
functions through the baked-program path.

The website follows dev's selective debug profile: runtime dependencies use
optimization level 1, while `pax-website` stays at level 0 for fast application
rebuilds. Living Quilt is a dependency here and therefore uses level 1; its own
standalone profile is not inherited. Build scripts/proc macros retain Cargo's
defaults. Use `CARGO_PROFILE_DEV_OPT_LEVEL=0` for fully unoptimized debugging.

### September 14 smoke-test notes

- Debug and optimized web cartridges build with the sibling dependency.
- Desktop (1440px and 1800px) and 390px browser layouts checked. Quilt tiling,
  color ripples, primary CTA, gallery edge clipping, horizontal navigation,
  automatic advance, and vertical page scrolling over the gallery checked.
- Resolved the separate first-paint gap at x=1248 on the normal 1800px / DPR 2
  viewport. The small-scene retained renderer queued sibling draws but applied
  their stencil transitions before submitting them. Runs now submit before a
  stencil replacement/pop; scissor/alpha-only changes still batch. Debug and
  optimized builds paint across both native tiles, including after scrolling
  down the page and back. The 390px debug viewport also remains correct.
  No quilt source, layout width, or DPR workaround was needed.
- The source drawer opens and contains both the website and original quilt
  sources. Selecting a different source tab was unreliable in the automated
  mobile check; this needs a focused follow-up rather than a claimed pass.
- A fresh optimized browser session logged no warnings/errors. Hot-reloading
  the complete page after opening/closing drawers did produce missing-layer
  warnings in the debug session; reloading cleared that session state.
- Existing entity-escaped gallery titles (for example `&amp;`) remain literal
  on screen. The authored card content was deliberately preserved in this pass.
- `/`, `/ai/`, and `/ai.md` return HTTP 200 from the local optimized static build.

Focused validation: `cargo test -p pax-macro --offline`,
`cargo test -p pax-compiler --lib --offline`,
`cargo test -p pax-std --lib --offline`, and the website's library tests
(244 tests total). `git diff --check` passes.

Renderer follow-up: `cargo test -p pax-gpu --lib --offline` passes 17 tests
(three native-GPU tests remain ignored by default), and all 55 `pax-std` library
tests pass. The new sibling/nested-clip command-order regression fails with the
old ordering restored and passes with the fix. A second test protects batching
across scissor/alpha-only changes. This changes no compiler/runtime wire format.
The CLI scene selector worked; `dev look` timed out during this follow-up, so
visual verification used browser captures rather than claiming CLI captures.

Living Quilt also builds independently with `pax-cli build --path
examples/src/living-quilt --target web --libdev`, confirming that its own app
entrypoint is preserved.

### Native text flicker follow-up

The four-line title and paragraph exposed false native occlusion from animated
quilt paths outside their clipping Frames. Shared coverage bounds now intersect
ancestor clips before testing native overlap; exact geometry is retained when
overlap remains possible. No website-specific compositor exception was added.
The title and paragraph also omit fixed heights so native text measurement
tracks wrapping. At 764px the resulting boxes match their 348px/147px text; at
390px they measure 180px/88px and remain separated. A 100-observation narrow
desktop sample saw no mask activations or height mismatches. The runtime-only
fix was first checked against the old fixed boxes to isolate the cause.
The optimized web build also passes the 764px repro (100 observations, no
spurious masks); the debug browser was checked at 390px, 764px, and 1440px.
The CLI scene selector confirms that the measured height reaches runtime layout,
not just the browser's CSS box. The release preview remains on port 8069.

All 162 `pax-runtime` and 55 `pax-std` library tests pass. The four new coverage
tests protect animated, nested/disjoint/empty, transformed, and unclipped cases;
three fail when clip-aware culling is bypassed. No message, manifest, or baked
program format changes are needed.

The authoritative WIP launch docs are in the PAX-975 worktree,
`/Users/zack/.codex/worktrees/f8d8/pax/pax-docs/book/src/`. Its Getting Started
guide and table of contents were checked; the public `docs.pax.dev` site is
still stale. Keep public links stable for that upcoming publication.

### September 15 editorial pass

The README and PAX-975 `what-is-pax.md` inform the facts and progression, but the
website prose is newly written. Creative freedom comes first, then the source
model, runtime, concrete feature inventory, and invitation. The final section
includes the full CLI sequence, pre-1.0 status, accessibility limitations, and
PAX-973's accepted self-contained OSS boundary.

The four new prose layouts use measured Text heights and autosized Stackers,
with shared editorial typography in SiteTheme. ExampleHost still supplies a
fixed preview envelope; narrow-desktop heights allow for additional wrapping.
Section source inspection now points at the new AuthoringSection; obsolete
BuilderProofSection files and imports are removed. Living Quilt and all 41
authored gallery cards are preserved. Only the gallery's introductory sentence
changed. New guide links follow the README's clean-URL convention.

Debug and optimized web builds and the three website library tests pass. Browser checks covered
1440px, 764px, and 390px layouts. A subsequent in-app browser connection loss
interrupted the final blog-navigation check and visual recheck of the last
spacing adjustments; those are not claimed as passes.

### September 17 intro messaging

The hero now leads with "A declarative language for native and web UI." The
authoring section uses "Describe your interface. Power it with Rust." Both
supporting paragraphs make the template/application-logic boundary explicit.
Homepage route metadata and package titles use the same category-first message.
Living Quilt and the feature gallery are unchanged.

The hero uses measured Text inside autosized Stackers rather than fixed copy
offsets, following the September 15 authoring notes in `pain-points.md`. Desktop
keeps the 54%/40% split with a 6% gutter; mobile stacks the title and copy with
a 24px gap. The fixed ExampleHost envelope allows additional wrapping on mobile
and narrow desktop. Browser checks covered 1280px, 764px, and 390px; the live
scene inspector confirmed the 764px headline measures 435px high. The debug web
build passes and generated title, description, Open Graph, and Twitter metadata
contain the new messaging.

The authoring section's existing responsive handler was missing its template
lifecycle bindings. Wiring mount/pre-render restores its intended mobile stack;
the revised heading, lead, and source-layer explanations were checked at 390px.
