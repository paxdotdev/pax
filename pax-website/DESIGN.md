# Pax website design notes

## October 1 — Feature cards as small demonstrations

The rail should show a behavior, not just decorate its title. Keep the existing
43-card editorial set, order, summaries, qualifications, and destinations. Keep
the layered-stroke and Handwriter mask studies; replace 28 flat illustrations
with bounded, grayscale studies. The remaining diagrams receive a foreground-
first layer-order repair and lose the cyan placeholder palette.

Artboard contract: a centered 288×140 logical-pixel scene, proportionally
scaled down for compact cards, inside a clipped 174px media Frame. Keep captions
outside the artboard at an explicit 146px Y with a zero anchor. Foreground text
and details precede their opaque surfaces. Leave stroke bleed inside the frame;
the rail's edge-to-edge layout and native scrolling behavior are unchanged.

Motion vocabulary: slow eight-second round trips, endpoint holds, finite enter/
exit reversals, and no flashing loop resets. Only visible study clocks advance;
static cap/join comparisons have no advancing clock. Existing rail virtualization
is preserved. This is application-level animation gating, not engine subtree
suspension. The new source is included in the ExampleHost drawer.

The actual path, smoothing, gradient, clip, light/material, dynamic-class,
Stacker reflow, autosize, and Rust computed-property primitives power those
studies. Tooling, route, and scrolling sequences are illustrative diagrams:
they do not repeatedly recompile the site, navigate its Router, invoke a native
picker, or nest interactive scrollers inside the already draggable rail.
This keeps whole-card documentation links and page scrolling unambiguous.

Validation: 24 website tests, the debug web build, mdbook, Rust formatting,
and whitespace checks pass. Desktop and 390px browser checks cover responsive
containment, captions, native rail turnover, and source-drawer tabs. A temporary
phase probe verified visible clocks advance while offscreen studies retain their
phase; the probe was removed. Unsupported transition curve names found during
QA were corrected to the documented Quad curves. Existing startup reactive-drain
and large-jump tile-window diagnostics remain. Baked release, touch-device QA,
and sustained performance benchmarking were not completed for this pass.
CLI screenshot export showed misplaced native text not present in the browser
capture; use the browser image for visual review pending capture-path diagnosis.

## September 30 — Screen-density scroller rendering

The fidelity follow-up below is now implemented. The April 12 `0283907ff3`
scrolling change introduced the unconditional 1x tiled-scroller override; it
was not introduced by the latest rebase. Remove that override for both WebGPU
and Piet, and require a pane to fit at requested screen density before choosing
one surface. Otherwise tile sooner within the existing backing-pixel budgets.
Hard dimension clamps remain a last-resort allocation guard, with a once-per-
changed-fallback warning and an inspectable `data-pax-resolution-fallback`
attribute. No adaptive resolution reduction or card-art workaround is added.

Debug and baked-release browser checks on a 2x display now show 2496×1116
backing for each 1248×558 CSS-pixel gallery tile, with no fallback markers.
Layered Paint edges are sharper; horizontal/vertical scrolling and tile turnover
show no visible gaps in the checked views. A 390px viewport smoke test also
passes; the browser's viewport override uses 1x, so this is a responsive check,
not a physical high-density phone test. Real iOS/Piet runtime QA remains open.

Five-second debug spot checks at 576×930/DPR 2 showed roughly 2–3ms CPU frame
cost before and 4–6ms afterward. Mean frame cadence remained around 8–9ms,
with occasional 16.6ms frames and no >33ms frames in the settled samples. These
are not controlled benchmarks: gallery content varied and a concurrent release
build competed for CPU. Four times the backing pixels has a real cost; retain
fidelity and investigate rendering work rather than silently reducing density.

Validation: 222 runtime tests, three surface-density tests, three browser-policy
tests, two native-surface tests, debug/release website builds, Piet WASM compile,
mdbook, and whitespace checks pass. Repairing the stale image test renderer's
required opacity method exposed an unrelated existing image-unmount test panic
(`properties not of expected type`); the broader web suite is not fully green.
Existing startup reactive-drain and large-jump tile-window diagnostics remain.

## September 30 — Calculator lighting parity

The canonical calculator now has one LightFrame and pointer-following direct
light shared by the desktop body and edge-to-edge background/keypad. Pointer
conversion happens in calculator-viewport coordinates, outside its Scroller.
The website retains host scaling, safe insets, lazy mounting, pause gating,
and the omission of layer-wide ambient light. No renderer changes were needed.

Validation: debug website and standalone web builds pass, alongside 38
calculator tests, 21 website tests, mdbook, source-bundle synchronization, and
whitespace checks. Browser screenshots and live LightSource scene inspection
verify pointer tracking in desktop and 390px layouts, including the embedded
phone's 0.877 scale. Standalone keypad input still evaluates 7 × 6 to 42.
Native targets and baked release were not rechecked for this application-only
change. CLI screenshot capture timed out; browser captures provided visual QA.

Separately, the Layered Paint card's rough edges match the web chassis's explicit
1x tiled-scroller backing policy: carousel tiles were 1248×534 pixels at the same
CSS size on a 2x display, while the root canvas used 2x backing. The demo remains
vector geometry. Leave the quality/performance policy unchanged pending a
focused fidelity and allocation-budget pass; do not hide it by changing card art.

## September 30 — The example supports the footer CTA

Remove the separate “Do the math” section and its calculator explainer. The
existing “Your next interface starts here” CTA now sits beside the live phone:
setup commands and the primary get-started link on the left, an uncaptioned
calculator on the right. At widths below 1000px, stack the CTA before the phone.
Project-maturity/accessibility/license notes and resource links follow, then
the built-in-Pax credit. Preserve those qualifications and destinations.

`CalculatorPreview` is only the reusable phone/example host. Its parent owns
gutters; it reports its aspect-ratio height from the assigned column width.
`ResourceSection` publishes its measured content height to one ExampleHost,
replacing both the old fixed footer envelope and separate calculator section.
The shared source drawer retains the actual calculator and device-frame files.
Lazy first mount, offscreen update gating, session retention, safe insets, and
keyboard policy are unchanged. No framework API or canonical example changes.

Validation: 21 website tests, the debug web build, mdbook, and whitespace checks
pass. Browser checks at 1440px and 390px cover the responsive composition,
7 × 6 = 42, retained calculation state through resize, and the shared source
drawer. The CLI screenshot request timed out; visual checks used the browser.
No release rebuild or native-device checks were run for this composition pass.

## September 29 — Morphing CMY mesh in the hero

With PAX-1011 on main, “creative” now reveals real mesh paint rather than the
cycling linear-gradient placeholder below. One unlit 3×3 field spans the entire
Handwriter mask, including an 8px paint bleed for round caps. Cyan, magenta, and
yellow anchors retain their colors while their positions morph: corners stay
fixed, edge midpoints slide along their edges, and the center drifts out of
phase. The ten-second loop has no reset jump or travelling stripe geometry.

The paint clock remains independent from the 1.6-second handwriting reveal.
Pause finishes the writing and freezes the current field; offscreen frames and
other adjectives do not advance or regenerate the mesh. Resume and subsequent
visits continue from the retained field. Text alternatives, sizing, signboard
transitions, and View Source remain unchanged. This uses the documented WGPU
mesh/alpha-mask implementation; no renderer or public API changes are needed.

Validation: 21 website tests pass, including mesh coverage/anchor ordering,
periodicity, and pause/resume clock behavior. Debug and baked-release web builds,
Rust formatting, mdbook, and whitespace checks pass. Browser checks at 1440px
and 390px show the continuous masked field, handwriting, and motion controls;
the release cartridge also renders the mesh reveal with no logged warnings or
errors on its origin. Debug retains the previously noted startup reactive-drain
and large-resize tile-window diagnostics; no mesh-specific diagnostic appeared.

## September 29 — A live calculator in the footer

The second hero adjective is now “high-performance.” The footer adds a quieter
technical proof after the get-started/resources section: short copy beside a
390×844 logical-pixel phone, stacked on compact screens and uniformly scaled
to fit its available width. The application fills the rounded screen up to
the inner bevel; a floating island and home indicator sit above it. Host-owned
48px/24px safe insets keep controls clear without introducing a blank bezel.

Import `calculator::Calculator` through a Cargo path to the canonical example;
do not fork the calculator source. The example now has a thin standalone root
and explicit embedding/update/keyboard controls. The footer's ExampleHost
contains both website wrapper files, the device frame, and the actual calculator
template, keypad, model, parser, sampler, and layout source.

The empty phone host observes viewport proximity and mounts the app only on
approach. It stays mounted afterward to preserve calculations and plot state,
but its frame updates and local point lights stop when the phone is out of view.
This is explicit application-level gating, not engine-wide lazy initialization
or subtree suspension. Embedded keyboard input is disabled so global calculator
shortcuts cannot hijack the surrounding page. On-screen keys and graph gestures
remain interactive. Layer-wide AmbientLight is omitted in embedded mode so
the calculator cannot relight other website content.

For now, choose live composition for this one bounded example. A raster poster
would avoid initialization cost but lose its interaction; multiple heavier
examples (such as two Paxflix views) still need their own measurements. Consider
poster-to-live activation, retained offscreen suspension, or a single active
instance based on those measurements rather than automatically mounting all
examples at landing-page startup. No new general renderer mechanism is added here.

Validation: 21 website tests and 38 calculator-core tests pass. Website debug
and baked-release web builds pass, as does a separate standalone-calculator web
build. Desktop and 390px browser checks cover calculation (7 × 6 = 42), mode
switching, rounded screen clipping, source-tab switching, and session retention
after scrolling away and resizing. A scaled release graph drag pans correctly;
keyboard input remains with the page. The long hero adjective fits at 390px.
Scene inspection confirmed zero calculator instances before approaching its
host and one after. The current release JS/Wasm bundle is 1.6 MB gzip (assets
excluded); this is not a calculator-only size delta or a sustained performance
profile. Native-device touch behavior and multiple heavy demos remain untested.
Canonical source bundles, mdbook, and whitespace checks also pass.

Remaining diagnostic: at the default-width debug startup, eight transient
`reactive drain hit 100000 evaluations` warnings occur while the page settles,
even with zero mounted Calculator instances. Large automated scroll/resize
jumps also reported tile-window escapes. Neither persisted in the settled
footer, but these are not a clean-console/performance sign-off. The dev-look
capture request timed out; visual checks used the browser instead.

## September 29 — Cycling CMY placeholder; mesh-gradient design lane

“Creative” now reveals the repeating CMY linear gradient from the `mask-strokes`
proving fixture. Two identical periods cover a 200%-wide rectangle, translating
one period every four seconds for seamless cycling. This is not mesh-gradient
paint: genuine mesh support is being designed separately in PAX-1011. The word's
aspect ratio, heavier stroke, 1.6-second handwriting reveal, and signboard
choreography stay unchanged. A small paint bleed preserves the round stroke caps.

The paint and source are unlit. No hero LightFrame, LightSource, or metallic
material remains. The visible headline owns separate writing and paint clocks:
pausing finishes the handwriting but freezes the colors without a jump. Both
stop outside the viewport; paint also stops when another adjective is mounted.
A transparent native Text outside the mask keeps the word browser-readable,
since native leaves inside a mask source are suppressed. This preserves the
existing text-equivalent boundary, not a new ARIA/heading claim. Alpha masking
still requires the GPU backend. View Source includes the live template and logic.

The previous pinstripe treatment is preserved, unmounted, in `cmy_ribbon.*`.
It is reusable content for a Mask, taking externally driven `progress` from zero
to one and following its container bounds. Three touching 2px CMY stripes share
one hard-stop gradient rectangle at 45 degrees. They begin fully off-left,
expand across the narrow axis, accelerate right, and leave white underpaint.
To reproduce the prior pipeline, drive progress with
`clamp((writing_ms - 1100) / 500, 0, 1)`. The extraction deliberately removes
the material and light; there is no mounted clock/resource cost. Tests retain
the mask-slice clearance checks across mobile and desktop lettering sizes.

This iteration changes only website composition, not a library API or capability.
Canonical gradient/masking articles already cover the APIs used here.

Validation: 20 website tests, debug and baked-release web builds, mdbook, Rust
formatting, and `git diff --check` pass. Browser checks cover the writing reveal,
moving colors, pause/resume, and layouts at 390px and 1440px; a release runtime
smoke check also shows the cycling paint. Both current-origin warning/error
logs were empty. Scene inspection confirms zero mounted CmyRibbon instances.
The known Pax formatter trailing-space issue still requires trimming wrapped
lines after formatting. No Apple-native or sustained performance audit is claimed.

## September 29 — Paint and mask feature studies

The first two gallery cards remain Hot Reload and Path Drawing. Two new cards
follow: Layered Paint and Dynamic Vector Masks. These are small real renderer
studies. Keep their treatment grayscale: a three-stroke ribbon on one Bézier
path, then handwritten “Pax” revealing a moving silver gradient. Each lives in
the existing card media box, with the same full-card documentation link and
unchanged native marquee.

`FeaturePaintDemo` shares a seven-second draw/hold/rewind clock. Viewport-proximity
notifications gate updates using the clipped global viewport, including the
outer page and horizontal rail. The visible component owns that clock; detached
mask sources deliberately receive no viewport notifications. The gallery's
existing structural culling unmounts distant instances. Both templates and
Rust logic are exposed by View Source. No new global lights or decorative
page effects are mounted.

The mask card explicitly names GPU alpha masking and supplies its text
equivalent outside the source. It does not imply native controls or images can
be captured as mask sources. The paint card links to the canonical Paint Layers
guide; the mask card links to Painted Alpha Masks. The older general clipping
card and its mixed-native composition claims are unchanged.

Validation: 218 runtime and 78 standard-library tests pass both with and without
designtime; 38 calculator and 17 website tests pass. Debug and baked-release web
builds, mdbook, both source-bundle checks, and whitespace checks pass. Browser
checks cover desktop, 390px mobile, and an 1800px wide layout at DPR 1; both new
docs destinations and the gallery's Pax/Rust source tabs work. Scene inspection
confirms the two demo instances leave the mounted window and remount on return.
Debug resize/large-scroll checks emit existing tile-window-escape diagnostics;
the release-origin warning/error log was empty in this smoke test. This is not
a Retina, Apple-native, touch-device, or sustained frame-time/resource audit.

## September 25 — Inverse logo, handwriting, and supporting copy

The hero now selects `LogoBackgroundMode::Dark` in the canonical `pax-logo`
component: light signboard/post with dark letters/counters against the dark
page. The component's default Light mode preserves existing consumers; its
two configurable fills are swapped consistently by Dark mode. The standalone
logo example has a light/dark background toggle for inspection.

“Creative” uses the bundled EMS League single-stroke font through Handwriter,
with a 1.6-second draw-range reveal and an em-relative box preserving the
source word's 4060.8:815.55 aspect ratio. The stroke is 1px heavier than the
initial treatment at every viewport size. Its paths are
generated on mounting, not retraced per frame; only draw progress changes.
The existing clock drives writing, pauses offscreen, and finishes the writing
when the user pauses. Other words retain their native-text treatment pending
art direction for traced typography. Handwriter already supplies `alt_text`
via a transparent native Text layer; no duplicate ordinary Text is mounted
for creative. This is a useful text alternative, not a complete heading or
screen-reader contract.

At this September 25 checkpoint, CMY banding was pending a mask-component lifecycle fix:
an alpha-mask prototype rendered blank because the off-tree Handwriter never
runs its mount handler or expands its component template. Alpha sources already
trim primitive Path strokes, but this does not establish support for a component
that generates those paths at mount. The prototype was removed; no extra color
animation is active. Once component mask sources are supported, use moving CMY
strips behind the revealed stroke with the same pause/offscreen clock, retaining
a native text alternative outside the off-tree mask source.

The logo's left edge shares the content gutter on desktop and mobile. Two light,
intrinsically measured paragraphs read: “Write application logic in Rust,
alongside Pax: a declarative language designed from scratch for expressive user
interfaces.” and “Ship to native macOS, iOS, iPadOS, and the web. This website is
built in Pax.”

Rust Hot Reload leads the feature carousel, followed by Path Drawing Animations.
Automatic advance pauses offscreen so the lead card is still first when the
visitor reaches the section. The template uses the 30px/s default directly.

Gutter/copy/carousel follow-up validation: debug web build, 15 website tests,
docs book build, and whitespace checks pass. Browser checks cover the hero at
1280px and 390px and confirm Hot Reload is first on reaching the mobile rail.
The attempted Handwriter mask was blank and removed, as described above; the
current preview retains white handwriting. This follow-up did not rerun release.

Earlier hero validation: debug and release web builds pass, as do 15 website and 23 logo
tests. Debug browser checks cover 1280px and 390px; the release smoke test
confirms the inverse palette, handwriting, paragraph, and pause control with
no warning/error logs in that fresh session. One `creative` text alternative
is present, but the current web reading tree places the conditionally mounted
Handwriter text after other hero content; semantic heading/reading order still
needs follow-up. No screen-reader or native Apple audit is claimed. The docs
book, bundled example snapshot check, formatting, and whitespace checks pass.

The notes below describe earlier stages of this iteration.

## September 25 — Logo and signboard hero

Concept: the mark and typography take the lead, without the quilt or the long
introductory paragraph. Desktop wireframe: animated logo in the left 28% column,
6% gutter, three-line headline and compact CLI actions in the remaining 66%.
Below 900px the logo sits above the headline. The content hull sizes ExampleHost
and its source drawer; the logo's natural-coordinate drawing is a breakout child
inside an explicitly reserved layout footprint.

The fixed headline grid reads “A declarative language” / “for [adjective]” /
“user interfaces”. Its font size follows the available column width. Each word
has a 4.2-second reading hold and a 0.9-second change: horizontal slide to
high-performance, flattened signboard flip to native, vertical roll to portable, and
tilt/scale recovery to creative. One native Text changes at the invisible
midpoint, avoiding duplicate words or per-letter reading order. Geometry stays
fixed as the words change; offscreen playback stops advancing. The logo plays
once from the existing `pax-logo` crate, with no copied animation source.

A native browser Button pauses/resumes both hero motions. Pausing resolves any
in-between word to a legible resting pose and completes the logo. This is not
automatic OS reduced-motion support or a full accessibility audit: Pax currently
has no unified reduced-motion query, general heading/ARIA authoring contract,
or image alt-text property. Keep the words as real native text, not an image
with an assumed alternative. Rich traced-letter experiments should wait for a
deliberate semantic counterpart.

The carousel's default speed is 30px/s (1.25× the previous 24px/s), with unchanged
native scrolling, pause, and culling behavior. The removed gallery footer stays
removed. The notes below are historical and do not imply Living Quilt or the
contour experiment is currently mounted.

Validation: the debug web build and all 14 website tests pass, including word
order/looping, hidden handoffs, legible paused poses, and distinct transforms.
Browser checks cover 1440px, 1280px, the normal 816px panel, and 390px mobile;
the native pause button responds to Enter, and the source drawer includes the
headline and original logo sources. The docs book and whitespace checks pass.
CLI scene inspection and timed captures were exercised; the scaled CLI captures
omit some GPU background/chrome, so browser screenshots are the visual layout
reference. This is not a release-build, screen-reader, or frame-time audit.

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
