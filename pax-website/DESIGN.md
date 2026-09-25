# Living Quilt + chromatic contour website

## Current baseline — September 25

Decorative CMY contours are unplugged while the content and live demos take
priority. The user observed performance hiccups; this is a deliberate removal
of that workload, not a claim that the underlying bottleneck has been profiled
or fixed. No contour instances, scroll-motion stores, pointer tracking, or
contour control are mounted in the website. Source drawers show active content
and demos only. Device frames, Living Quilt, the feature gallery, and the demos'
own playback/scrubbing controls remain active.

`src/chromatic_contour.rs` and `src/chromatic_contour.pax`, including their tests,
remain intact as a parked experiment. The design and validation notes below
record that experiment rather than the current mounted scene.

Before bringing it back, measure the content/demo baseline at desktop and mobile
sizes, then add one contour and profile path generation, reactive updates,
tessellation, and compositing separately. Scale to multiple visible cards only
after establishing a frame-time budget. The eventual integration needs:

- A `ChromaticContour` underlay after the opaque device/card surface, sharing
  its bounds, with a suitable `radius`, `strength`, and stable per-instance seed.
- A page `ScrollMotion` store observing native vertical scroll, with gallery
  scope supplying horizontal scroll while preserving the vertical coordinate.
- A separate page `PointerMotion` store sampled by a bubbled mouse-move handler.
- An explicit decoration-only opt-out; do not couple demo playback to it.

Do not re-enable the effect by merely hiding its paths: removing scene instances
is what removes their per-frame handlers and geometry work from this baseline.

Validation: the debug web build, all eleven library tests (including the parked
effect tests), Rust formatting, docs book build, and whitespace checks pass.
Live scene inspection reports zero `ChromaticContour` nodes, four `DeviceFrame`
nodes, and two `PathStudy` nodes. Desktop and 390px checks cover the plain frames,
gallery layout and horizontal navigation; the path demo's pause/resume still
works. This pass is not a frame-time benchmark or a release-build verification.
The existing debug initialization reactive-budget warnings and a transient
viewport-resize tile diagnostic still appear; they are not claimed fixed here.

## Concept and layout

The website begins with the actual Living Quilt: an edge-to-edge, interactive
geometric field, with its original logo animation, lighting, slides, and color
rings. The content below is its quiet counterpart: warm-black surfaces, warm
white headlines, gray body text, square-edged cards, and fine rules. Broad
actions stay neutral; cyan, magenta, and yellow live in fine contour motion
and small demonstrations, not split or blurred typography.

The page sequence is quilt → introduction and CLI → creative freedom → language
and logic → native/web and performance → horizontal feature gallery → start
building. The content rationale and claim boundaries are in `CONTENT.md`.
The September 24 pass adds live, inspectable path-drawing, material, and native
scrolling studies in restrained device silhouettes, following the C1–C4 comps.
Desktop introduction uses
two columns; mobile stacks the same content. The quilt retains its own 3/5-column
responsive behavior. Its interactions stay inside the hero's clipping frame.

`SiteTheme` owns the shared grayscale + CMY palette. Existing typography
remains Manrope with IBM Plex Mono labels and code. Feature-card copy and
qualifications are unchanged. The gallery keeps its
local native-scroller marquee; `pax-std::Carousel` is not involved.

## Contour Drift — first implementation

Textual wireframe: unchanged quilt/intro → editorial copy beside a path-drawing
phone → language/logic copy beside a drawing laptop → tablet material study
with a smaller native-scrolling phone → full-bleed feature rail → CLI/resources.
Narrow screens stack these pairs. Text remains intrinsically measured, with
content-sized stacks publishing their height to the bounded ExampleHost shell.
The gallery title is “Meet your creative toolkit”; its title, description, rail,
and footer are one measured vertical flow rather than fixed y offsets.

Visuospatial rules: warm-black page, graphite hardware, thin gray bevels,
generous gutters, crisp warm-white titles. CMY lives behind the opaque device
or card; it can briefly stretch past the silhouette but does not cover the copy.
The phone/tablet/laptop are Pax geometry and their screens are live Pax—not
images of fictitious finished applications. Material appearance is backend
dependent, as qualified in the runtime copy.

`ChromaticContour` accepts `radius`, `strength`, and `seed`. Give it the same
position and dimensions as a rounded rectangular surface, and place it after
that surface in source order. It is a non-raycastable, breakout underlay with
three closed, stroked cubic paths. No effect mask or duplicate content layer is
involved. This first version targets axis-aligned rounded rectangles.

Motion uses one continuous contour, not separate scroll/settle/idle phases:

1. At rest, three thin CMY outlines carry low-amplitude, spatially noisy waves.
   Periodic noise samples are smoothly interpolated in space and time, with
   independent channel seeds. This is a synthetic microphone-waveform feel,
   not actual audio analysis; no microphone access is requested.
2. Passive, bubbled mouse movement adds a short-lived scratch disturbance near
   the pointer. Both amplitude and evolution speed increase, then settle when
   the pointer stops. Phase is integrated so acceleration cannot reset the wave.
3. Native scroll coordinates drive a bounded, time-based lag with the opposite
   sign: scrolling down pulls the lines upward; scrolling right pulls them left.
   Only that resisting side stretches; the channels pull at different distances. The same
   geometry relaxes into the quiet waveform when scrolling stops, with gentle
   stroke-width and opacity changes rather than an on/off transition. Large
   jumps and marquee recentering do not create huge impulses.

The resting baseline stays outside the opaque surface so ordinary deformation
does not repeatedly hide/reveal the strokes. Duplicate engine timestamps retain
the previous picture, and long frames use a bounded integration step rather than
blanking paths. These replace authored sources of the earlier rapid flicker:
idle eruptions, a discontinuous phase switch, and explicit slow-frame blanking.

`ScrollMotion` is scoped to the page; the feature gallery supplies its horizontal
coordinate while retaining the page's vertical coordinate. Nothing in this
treatment writes scroll position or consumes wheel events. `PointerMotion` is
a separate page store, so gallery scroll scoping retains the pointer input.
Offscreen contours skip path generation without changing opacity. The “Contour
motion” control below the rail pauses these
decorations and the new drawing studies, not the unchanged Living Quilt hero
or the existing marquee. It is a manual opt-out, not an OS reduced-motion query.

The reusable frame, contour, and demo source files are included in the relevant
section source drawers. The pre-existing source-tab interaction issue remains
a separate ExampleHost follow-up.

## One source of truth for the hero

`Cargo.toml` imports `living-quilt` using `path = "../examples/src/living-quilt"`, relative to
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
the CLI link, gallery clipping and motion, server-owned blog navigation, and the
default application route.
Also build the release cartridge to verify dependency discovery and palette
functions through the baked-program path.

The website follows dev's selective debug profile: runtime dependencies use
optimization level 1, while `pax-website` stays at level 0 for fast application
rebuilds. Living Quilt is a dependency here and therefore uses level 1; its own
standalone profile is not inherited. Build scripts/proc macros retain Cargo's
defaults. Use `CARGO_PROFILE_DEV_OPT_LEVEL=0` for fully unoptimized debugging.

### September 24 contour pass checks

Waveform revision (following the initial pass below):

- Eleven website tests pass, including periodic noise continuity, continuously
  visible channels, duplicate/slow frame handling, finite exterior geometry,
  scroll stops/reversals and opposing axis signs, and pointer-energy decay.
- The revised debug web build, Rust format check, docs book build, and
  whitespace checks pass. An optimized web build was started but was still
  compiling at this handoff; the previous optimized preview is not evidence
  for the waveform revision until that build finishes and is checked.
- Desktop (1280px) and 390px browser checks cover device and card contours,
  pointer movement, scrolling, horizontal gallery navigation, and idle frames.
  Sampled frames retained the contours without the old authored blanking.
  This is a visual spot-check, not a high-frame-rate renderer benchmark;
  it does not establish that every possible rendering issue is ruled out.
- Pax scene selection and ray-casting worked. The CLI screenshot sequence timed
  out in this revision, so visual sampling used browser screenshots instead.
  The existing debug initialization reactive-budget warnings remain; viewport
  resizing also produced transient tile-window escape diagnostics.
- The animation change is website-local. Living Quilt, shared rendering code,
  native scrolling, the gallery content, and standard-library Carousel are
  unchanged. The source design notes and authoring pain points are updated;
  no public library API documentation change is needed.

Initial contour pass (before the waveform revision):

- Debug web builds and seven website library tests pass. Tests cover live
  scroll/pause observation, time-based directional response, overlapping idle
  channels, the settling boundary, existing marquee normalization, and contrast.
- The final optimized web build also passes. Its baked cartridge was opened at
  390px and 1280px: the measured gallery flow and animated device contours work,
  with no warnings/errors observed in that fresh release browser session.
- Browser checks at 1280px, 800px, and 390px cover the responsive compositions,
  idle CMY overlap, vertical and horizontal scrolling, path scrubbing, moving
  material lighting, the nested native scroller, and the motion pause control.
  Device interiors are breakout scenes inside explicitly reserved flow bounds;
  a long inner scroller no longer inflates the surrounding article.
- At 390px the gallery headline measures 342 × 92px; the 342 × 54px description
  begins 18px below it. Desktop and intermediate widths also keep a measured
  gap. The rail remains full-bleed, with 18px vertical contour clearance that
  does not reduce the original card height.
- The gallery remains windowed (8–9 mounted cards in the checked sessions,
  including retained overscan). Its authored 41-card content and docs targets
  were compared with the original source and are unchanged.
- Pax scene inspection and screenshot sequences were used alongside browser
  captures. No touch hardware or native Apple build was exercised in this pass.
  This is not a frame-time benchmark or an accessibility audit.
- Debug fresh loads/hot reloads still emit a reactive-drain budget warning
  during initialization. It did not continue during the checked scrolling and
  idle sessions; its cause has not been isolated. The pre-existing source-tab
  selection issue is also not claimed fixed by this styling pass.
- `mdbook build pax-docs/book`, Rust formatting checks, and `git diff --check`
  pass. Pax sources were formatted and formatter-emitted trailing spaces removed.
  Documentation changes are website design notes and authoring pain points;
  no shared library API or behavior changed.

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
