# Pax website

The first-party Pax application lives at root-level `pax-website/`, beside the
static Zola site in [`pax-blog/`](../pax-blog/README.md). It is not a reusable CLI
example or a publishable crate. The monorepo explicitly excludes it from its
Cargo workspace so the site's standalone profiles remain effective: development
dependencies use optimization level 1; the website application uses level 0.

Web previews enable the experimental loading screen. Install the qualified
Wasm splitter and matching wasm-bindgen CLI described in
[loading-context components](../pax-docs/book/src/components-composition.md#loading-context-components-experimental)
before running; both the bootstrap and application Wasm are emitted by the same build.
The website sets `loader_exit = "first_viewport"` under `[package.metadata.pax.web]`:
the loading screen exits once the initial viewport's native measurements and visible
surfaces are ready. Remaining content mounts cooperatively after the loader disappears;
ordinary animations and input still wait for full mounting.

## Local development

From the monorepo root on this workstation:

```sh
source ~/.zshrc
nvm use
./pax-website/pax run --target web
```

The launcher builds this checkout's CLI, selects the website by its own location
(not the invoking working directory), and uses `--libdev`. The equivalent direct
command is:

```sh
cargo run -p pax-cli -- run --path pax-website --target web --libdev
```

For an optimized web build without serving or publishing:

```sh
./pax-website/pax build --target web --release
```

To preview the same hero on Apple targets, run one of these commands:

```sh
./pax-website/pax run --target macos
./pax-website/pax run --target ios --ios-device simulator
./pax-website/pax run --target ipados --ios-device simulator
```

Stop the previous CLI preview before switching targets: one project shares its
generated cartridge and interface staging directory across targets.

Debug and release outputs live in `.pax/build/debug/web/` and
`.pax/build/release/web/` beneath this directory. Generated `.pax/`, `target/`,
`Cargo.lock`, AI-primer HTML, and staged blog previews stay untracked.

The hero uses `AnimatedPaxLogo` from `pax-std` through the `pax_kit` re-export.
Its website-local `SiteLogo` wrapper carries the new radiance treatment and fits
the artwork into its hero home or the navigation's smaller container. View Source
includes the wrapper and canonical files in `pax-std/src/drawing/pax_logo/`.
The navigation uses main's fixed `Overlay` and mounted-node transfer mechanism
(described below), with an opaque vertical slide instead of group opacity.
The controls and background slide down over 500ms when the logo docks and back
up over 450ms when it returns. The hidden offset includes the top safe-area inset;
reversals ease from the current position. The logo's destination remains mounted
so transfers have a stable target, while fully hidden controls are unmounted.
Source inspection hides the nav. The homepage starts without a reserved nav gap;
other routes keep their nav-sized inset and static logo.

The opening sequence uses the outer native scroller as a shared clock.
`scroll_story.rs` maps its offset into overlapping chapter-local clocks for the
hero, the paired Paxflix/calculator showcase, and creative examples. The hero starts fully composed;
its logo and heading separate slightly on departure, while screenshot-backed
device frames assemble, hold their poses, and depart. Named component timelines sample those ranges in either
scroll direction. Measured content and viewport minima determine section heights,
with additional breathing room capped so tall screens do not introduce empty
screenfuls. Animated child transforms do not change those heights. This first slice
does not pin scenes. On compact layouts the Paxflix and calculator fanouts each
become a single screenshot link, while the creative studies stack vertically.

`GlassAction` shares opal/obsidian paint and material helpers, with 5⅓px corners, from
`SiteTheme`, without outline strokes. `SiteShell` owns one stationary `StripLight`
from `(0, 100% viewport height)` to `(100% viewport width, 33% viewport height)`, outside the scroller
and safe-area insets. Its length and angle respond only to viewport resizing;
scrolling surfaces move underneath it. Opal has a soft response; obsidian has a
narrow specular response. Buttons do not own lights or a separate lighting clock.
`SiteShell` also adds a viewport-space point light following mouse movement and touch
start/move, outside section light frames, without capturing input or cancelling
native scrolling. A unit ambient contribution preserves ordinary matte paints;
native labels stay unlit. Base paints remain visible on CPU renderers, but dynamic
lighting requires the GPU backend. These are opaque glass-like surfaces, not
refractive glass. The GitHub icon's official source is
recorded alongside the asset in `assets/branding/`.
The nav is borderless and has no gradient overlay. Foreground surface ordering
for a future soft edge is tracked in
[PAX-1041](https://linear.app/paxdev/issue/PAX-1041/support-independently-ordered-foreground-compositor-surfaces).
Living Quilt remains independently runnable in `examples/living-quilt/`; neither
it nor the `pax-logo` harness is a website dependency. Cargo dependency paths
and favicon paths are relative to this manifest; Rust `include_str!` paths are
relative to their source file.

The navigation contributes an `Overlay` above scrolling content. As the top of
the logo's reserved home reaches the nav's lower edge, the same mounted
`SiteLogo` moves into `#site_nav`'s `#logo_container` Group. A subscription follows
the home's layout, the page's scroll position, and the responsive nav boundary;
returning requires 24px of clearance to prevent jitter at that edge. If the
compact home already overlaps the future nav, departure waits for 32px of scroll
so the initial page stays at home and can return through that hysteresis. Both moves
use a 320ms ease-out, preserving the logo playhead and leaving the home footprint
intact. The return flight stays above the nav until it settles home.
The destination is a plain Group, separate from the fallback brand's Link,
so the visiting logo's own hit area receives its clicks.

The shell retains its 64px/80px responsive nav and safe-area insets. Pages without
a hero show the static `NavLogo`; removing the hero also ends the transferred
instance's lifetime. The shell returns visitors in `on_pre_unmount` when their
original owners still exist. The hero logo replays on click at home; in the nav,
it acts as the shell's home link and scrolls back to the top. See [node transfers](../pax-docs/book/src/event-handling-rust.md#find-and-move-existing-nodes)
for the runtime ownership and recovery contract.

The creative-freedom section uses screenshot-backed links to the oscilloscope
and Space Game. Their live components mount only on their standalone routes;
the homepage keeps the device-frame choreography and View Source without
running the demos. System reduced-motion integration is tracked in PAX-1048.

The HTML background uses `package.metadata.pax.web.background_color` to match
the scene's dark ink before the cartridge starts. The hero logo holds its
initial pose until frames and hero height have stayed stable for 100 ms, then
runs the usual reveal. Long startup frames restart that settling interval;
a five-second ceiling prevents a persistently slow frame rate from suppressing
the logo indefinitely. There is no fixed delay added to every page load.

The website enables `package.metadata.pax.loader`,
`package.metadata.pax.web.yield_mount`, and `loader_exit = "first_viewport"` in
the web metadata. The loading screen stays visible while the main application's
initial viewport mounts in a hidden, inert region. Once its required content,
native measurements, and visible surface submissions are ready, the page appears
beneath the loader and the loader begins its exit. Remaining content mounts
cooperatively after the loader disappears. The main animation clock stays held
until mounting finishes; input is enabled once both mounting and loader exit
complete. This handoff does not reduce the total mounting cost. See
[loading screens](../pax-docs/book/src/components-composition.md#loading-context-components-experimental)
for the build requirements and publication contract.

Resize and native measurement feedback temporarily pause background mounting
to settle and present the current viewport, then resume the remaining queue.
This uses the same autosize and Mask requirements as the first reveal, so those
dependencies can require work beyond the visible box. Already-admitted reactive
work still drains through the shared graph. Background admission pauses at an
internal queue threshold and drains a batch before mounting more content, reducing
the backlog inherited by late feedback. This is a soft limit: a single subtree or
resize can still enqueue enough work to delay presentation across several slices.

A configured loader always uses yielded preparation, including with
`?pax_mount_yield=0`. To compare synchronous startup, first disable the loader in
Cargo metadata and rebuild; that query override then applies. Without the loader,
yielded startup can reveal the initial viewport and progressively present the
remainder, with native text appearing before artwork. Keep the loader setting,
startup mode, and viewport dimensions the same when comparing measurements;
the standalone Paxflix desktop route mounts the full 21-shelf catalogue.
Homepage scrolling never mounts the four showcased apps; opening their routes
mounts a single example. See
[resumable startup](../pax-docs/book/src/how-pax-runs.md#experimental-resumable-startup-on-web).

Ordinary URLs use cooperative rendering, passive wheel input, and feature-detected
CSS box clipping for rectangular Frames. No scrolling query switches are required.
`pax_surface_trace` / `pax_scroll_perf` remain opt-in diagnostics, not rendering
policy. Native chassis retain their own scheduling; cross-platform integration
qualification is separate from this website's web smoke tests.

The feature gallery uses its own native-scroller-backed marquee. Card geometry
has a valid initial value and is published through the local property view before
descendants mount. All 43 authored cards are projected at startup, with one keyed
slot each, rather than mounting on first encounter during horizontal scrolling.
The shared yielded-startup scheduler handles their initialization. Each card
remains mounted until the gallery or that authored child is removed. Looping,
scroll reversal, and viewport resizing reposition the same card rather than
recycling its slot into another card. Cards keep their positions outside
the requested window; they wrap only when another occurrence is needed near
the viewport. Leaving overscan does not immediately move a trailing card to the
far end of the loop, avoiding needless geometry churn on quick reversals.
On web, a loop recenter changes the finite native pane's origin and scroll
position together; the content-space viewport, cards, and painted tile coordinates
stay put. This avoids replacing the whole visible band at a loop boundary.
The internal `ScrollerHost` origin integration is web-only, not a public
cross-platform infinite-scroller API. Resize and authored content changes remain
real layout changes and may require repainting.
This preserves local demo state; the demo clocks pause while their artboards
are offscreen. This is a bounded gallery policy, not a general-purpose
virtualization or memory-eviction policy.

Card allocation waits for positive, finite card and viewport dimensions. The
mounted set changes only when authored children change, not when scrolling.
The viewport plus two cards of overscan on each side determines which loop
occurrences must be positioned nearby, not which components exist. Rails shorter
than the viewport display once without looping or projecting the same child into
multiple slots. Eager mounting does not eagerly rasterize the entire rail:
offscreen drawing and surface preparation remain the renderer's responsibility.

The gallery's automatic march pauses while the outer page's bound scroll
position changes, including momentum, and resumes after 150 ms without a new
position. Hover, touch, and manual-scroll pauses still apply independently.
Manual horizontal scrolling stays available while the page moves, and resuming
the march does not catch up for time spent paused.

Carousel vector fills and outlines explicitly use `Material::unlit()`; empty
hit surfaces have no paint. The material and scoped-lighting cards are labeled
illustrations, using authored grayscale gradients and a moving position marker
instead of live lights. This avoids lighting-dependent tile invalidation from
the shell's strip/pointer lights and removes the cards' local light emitters;
it does not eliminate the rail's animation, layout, or compositing work.

The full-bleed rail fades into the page ink over twice each content gutter (12% on
desktop, 48px below 760px). Two non-interactive `NativeImage` overlays reuse
`assets/branding/carousel-edge-fade.svg`, mirrored on the right, outside the
horizontal scroll host. On web this preserves per-pixel alpha over both native
text and rendered scroller content; a canvas gradient's native coverage estimate
would flatten that fade. The image uses `SiteTheme::ink()`'s `#11110f`; keep the
asset aligned if the page background changes. These decorations do not change
scroll capture, card hit targets, or the marquee's mounting policy.

The favicon and social-preview logo use the current signboard artwork from
`pax-std/src/drawing/pax_logo/pax-logo.svg`, in the site's paper-on-ink palette.
Their checked-in exports live in `assets/branding/`; the ICO contains 16, 32,
48, and 64px sizes. Regenerate both exports with ImageMagick after changing the
canonical SVG or site palette:

```sh
bash pax-website/tools/render-brand-icons.sh
```

Ordinary website builds need no image conversion tools. The favicon is emitted
as `/pax-favicon.ico`, independently of the legacy macOS app icon.

## Website and blog boundary

`SiteHeader` in `src/site_header.{rs,pax}` owns the shared navigation visuals
and links. `SiteShell` retains the homepage's logo-transfer destination and
scroll-driven positioning around it. The static blog's separate
`pax-blog/header` runnable imports this exact component through a local Cargo
dependency on the website; there is no copied Pax header or separate component
library. Its dependency disables the default `app` feature, excluding the website
runnable, split-Wasm loader, non-header modules, optional demo dependencies, and
`pax-kit/shaders`. Default website builds still enable all of those. Keep
header-only dependencies in `SiteHeader`, `NavLogo`, and `SiteTheme`; new
website-only modules and dependencies belong behind `app`. Rebuild both bundles after
shared-header source changes. The blog uses
an ordinary HTML article with an independently mounted header and HTML navigation
fallback; see the [blog workflow](../pax-blog/README.md#shared-pax-header).

The website declares `/blog`, `/ai`, `/ai.md`, `/paxflix`, `/calculator`,
`/oscilloscope`, and `/space-game` as `server_owned_prefixes`.
Navigating to these paths loads a server document instead of a Pax client route.
The `/blog/*` Router placeholder must not be restored. `build.rs` generates
`public/ai/index.html` from `public/ai.md` only when its contents change.

To check the separate blog locally, follow its setup instructions, start the
website, and use the website's printed port:

```sh
python3 pax-blog/blog.py stage --port <website-port>
```

This is a draft preview, generated into `pax-website/public/blog/`. It is not
production output. Remove it before a website release intended for publication:

```sh
python3 pax-blog/blog.py unstage
python3 pax-blog/blog.py build
```

The second command independently builds production blog files, excluding drafts,
into `pax-blog/public/`. Neither command deploys anything. Website publication
must exclude `blog` and `blog/*` from root uploads, including deletion, while blog
uploads remain scoped to `s3://www.pax.dev/blog/`. The publisher and production
acceptance are still pending; see the [hosting handoff](../pax-blog/infra/README.md).

The website's source is inspected in the running site, not through the CLI's
bundled `examples/` catalog. After moving or changing reusable examples, refresh
that separate snapshot with `python3 scripts/sync-docs-examples.py`.

## Paxflix showcase

At a showcase width of 760px or more, the homepage shows three linked captures:
a tablet in front, with macOS and iPhone wings. Below that width it shows only the
tappable iPhone capture. No homepage layout mounts Paxflix. The
fixed device footprint scales with the section width, and sizing subscribes to
reactive bounds so it settles during initial mounting. All three desktop frames
have scroll-sampled arrival, hold, and departure poses. There are no hidden
preview apps, screenshot polling, or deferred live mounts.

Every device and the full-screen action open `/paxflix/`.
The generated `/paxflix/index.html` is a standalone document entry into the
website's demo route. Its server-owned navigation policy deliberately reloads
the document so the demo participates in the configured yielded startup and
initial presentation gate. It uses the same website bundle, with one Paxflix
instance and no homepage tree. This reduces mounting work, not Wasm download
size. The old homepage modal, which created another app instance, is removed.
Browser Back can restore or reload the homepage; the demo's Back to Pax link
uses ordinary in-app navigation. Each new demo session starts with fresh state.

The captures use `NativeImage` and live in `assets/paxflix/`; their README records
their native/browser origins and regeneration commands. Device switching is not
implemented in this version.

## Calculator showcase

The native-and-web section pairs Paxflix with a calculator three-device fanout: a captured,
upright phone in front, with a smaller tilted macOS-style window on the left and
a tablet on the right. The window uses `DeviceFrame kind="window"`, with a dark
title bar and traffic-light controls. Its narrow capture fills the content area
without the calculator's wide-window background gutters.
All three screens use `NativeImage`; none mounts a calculator. The frame poses
respond to the shared runtime chapter clock without resizing their captured content.

Above the section's 1000px breakpoint, the showcases sit side by side in a 62/38
row. Below it, they stack and each fanout becomes one tappable phone capture.
The section chooses both presentation modes, rather than each narrow column
mistaking itself for a mobile viewport. Resizing only switches presentation. The
fixed-aspect stage scales to its assigned column without stretching the devices.

The compact preview, captured wings, and full-screen action open `/calculator/`.
Like `/paxflix/`, this is a generated document entry with server-owned navigation:
one live example, no homepage tree, and safe-area-aware full-screen layout with a
Back to Pax link. Each new demo session starts with fresh calculator state.
The runtime section's View Source includes both showcases and the calculator full-screen wrapper;
application logic remains in the canonical `examples/calculator` Cargo dependency.
Capture provenance and refresh commands live in `assets/calculator/README.md`.

## Linked creative examples

`/oscilloscope/` and `/space-game/` are generated document entries, with the same
server-owned navigation policy as the other examples. They mount only the chosen
app, provide View Source, and have a Back to Pax link. Their homepage captures
live in `assets/playground/`; that directory's README describes how to refresh
them. The website asset namespace symlinks Space Game's canonical sprite files,
just as it does for Paxflix media.

All four live examples still share the website's Wasm bundle. Screenshot-only
presentation removes their homepage node trees, timers, nested scrollers, and
rendering work; it does **not** split the initial download. Independent example
bundles would be a separate packaging improvement. Returning to the homepage
does not retain the abandoned example's application state, except where the
browser itself restores a document from its back/forward cache.

## Typography

The website, calculator controls, and `ExampleHost` UI use bundled
Manrope. Source code and terminal commands use a system monospace; the calculator
LCD retains its Courier-family lettering. Paxflix keeps its system Arial UI
and Georgia titles, which require no downloads. The website makes no Google Fonts
requests for these families. Manrope is also bundled in `pax-std/assets/fonts/`
for standalone ExampleHost and demo use; the identical website asset merges into
one output file. Keep these copies and their license aligned.

The shared `ExampleHost` drawer offers canonical Solarized Dark and Light through
a moon/sun pill switch beside the close control. Solarized applies only to the
code pane; the surrounding chrome stays black and white. Light is the default.
Every website host and mobile viewer explicitly binds to HomePage's single
`source_light` preference, so toggling any viewer updates them all. The choice
survives closing/reopening drawers but is not persisted across page reloads.
Source selections and open states remain independent. Syntax accent colors stay consistent across modes.
The title shares a top-aligned row with the controls and wraps to its measured
height; the source tabs follow below it. Filenames appear only in the tabs.
The 160px source tabs stay single-line and scroll horizontally; very long labels
clip within the tab.

Below 600 logical pixels, HomePage mounts the shared `ExampleSourceViewer`
above the page Scroller at full viewport size, rather than inheriting a section's
height. `MobileSourceSheet` uses lifecycle timelines to slide it up from the
bottom in 350ms and back down in 300ms. Its opaque background moves with the
panel; a separate transparent input blocker leaves the page visible underneath.
The page and its scroll position remain mounted. Fixed navigation stays hidden
and the input blocker stays mounted through the retained exit, not just until
the open flag changes. Each section explicitly
binds its open, source-index, and theme state to both presentations, so resizing
or closing/reopening does not reset the selection or palette.
Only the selected file is syntax-highlighted; opening the viewer does not
prepare every hidden tab's source. This remains local UI state, not a URL or
browser-history entry.

Content and visual design history are archived in
[PAX-869](https://linear.app/paxdev/issue/PAX-869).

## Shared-definition section

`AuthoringSection` is parked, not mounted on the homepage; its source remains
available for a future editorial pass. Its
“A playground for your mind. And artificial minds.” headline introduces an
expressive, compact UI definition as a shared, inspectable contract for people
and coding agents. Compactness refers to directly declaring UI relationships,
not a measured token-efficiency claim. The visual-scratchpad vision remains
explicitly aspirational. Copy and layout remain open to editorial review.

`SharedDefinitionDemo` pairs the actual `relationship_preview.pax` source
(embedded with `include_str!`) with a live instance of that component. The 25%
and 75% buttons change one state value, updating both the label and bar; they do
not simulate source editing or an agent interaction. The panels stack below
900px. No animation clock or additional dependency is introduced. The homepage
does not allocate source-viewer state or source strings for this parked section.

## Hero lettering

The three Manrope words in `RotatingHeadline` use checked-in Bézier outlines at
weight 800 of the [bundled font](assets/fonts/README.md). `HeroLetter` animates
ordinary Pax paths: high-performance accelerates and brakes, native assembles along
short arcs, and portable presses and rebounds. Native and portable freeze their
painted fields and fade in place on exit, reusing the assembled pixels rather
than resizing shader captures. Creative keeps its EMSLeague handwriting and CMY
mesh treatment, with boundary control points moving up to 26 percentage points
and the center up to 30, while keeping the cells convex. The mesh loops every
two seconds, twice the previous morph speed.
Its stroke width is `max(1.6, font_px * 0.038) + 1.5` logical pixels.
The source drawer includes the path loader, generated geometry, and letter component.

High-performance uses a cool silver gradient and a moving GPU light scoped by
`LightFrame`. Its ambient response is 0.91 under the shell's unit ambient light,
preserving contrast in the chrome bands and headroom for highlights. The frame
contains its own light but does not exclude the shell's lighting.
Its sweep begins during the reveal and lasts 1.04 seconds.
Portable uses `ImpressionistMode::Gauguin` on a field of 18 white star
trails sweeping diagonally upward. Staggered speeds, lengths, and thicknesses
create bright heads and long translucent tails; overlapping trails accumulate
alpha over a 33%-alpha white ground. The smaller brush scale preserves the
streaks, while the outer mask keeps each letter crisp.

Native uses the built-in `ImpressionistMode::Signac` treatment on six moving
radial-gradient dots (two each of cyan, magenta, and yellow) over a paper-colored
ground. The animated letter paths provide an outer alpha mask, so the filtered
colors move inside crisp letter contours. The shader is the Mask's content;
the letter group is its separate source. Brush scale follows the font size.
Both native and portable use this crisp outer letter mask. One phase drives
their paint and source motion while either word enters or holds; it stops for
the exit fade. The native selection Text stays outside both the shader and mask.

The website enables `pax-kit/shaders`, including the generic effect runtime and
WGSL validator in Wasm. This is separate from the offline font tracing; no
runtime font outlining dependency has returned. These effects require WGPU and
use one color-field capture per word, rather than one per letter. Native's radial
centers move inside fixed rectangles; portable's gradient endpoints travel
through fixed diagonal strips. Each trail clears the strip before its phase
wraps. This keeps capture dimensions stable while their source pixels change.
Letter outlines remain pretraced path geometry.

Hero timings are 80% of the original durations: 3.36-second holds, 0.72-second
handoffs, 1.28-second creative writing, and proportionally shorter letter
entrances/staggers and logo reveal. Creative's mesh uses the faster two-second
period described above. The word's clipping Frame has
`layout_role=LayoutRole::Breakout`: animated descendant bounds cannot change
the ancestor Stacker's measured height. The fixed three-line signboard and its
normal Text nodes determine the paragraph's position.

## Free and open source / kinetic acrostic

`OpenSourceSection` follows the feature gallery and precedes the final CTA.
It connects Pax's free-and-open-source identity with the project's
creative mission; the MIT / Apache-2.0 license and inline GitHub link live here rather than repeating in the
footer. The maturity and accessibility qualifications remain in the footer.
The section and its lettering are inspectable through their own ExampleHost.

`Acrostic` uses four retained cyclic tapes containing MAKE, PLAY, OPEN, FREE,
and CODE. A deterministic solver chooses matching letter occurrences for each
row's direction, then aligns the word vertically at one of five horizontal
columns. Rows ease independently over at most 2.3 seconds; the aligned word
brightens to warm white during a 3.2-second reading hold and dims before travel.
The field has seven visible cells per row on desktop, six on narrow layouts,
and 80 retained individual Path letters,
including clipped wrap overscan. Letter geometry does not change during playback.

The graphic pauses its clock offscreen. There is no visible vocabulary legend
or playback control; these are decorative outlines, not selectable vector text
or a complete accessibility audit.
System reduced-motion integration remains tracked separately in PAX-1048.
The grayscale-to-white, unlit paints work on GPU and CPU. On GPU, the four
selected letters also receive a restrained `Radiance` treatment: emitted-light
opacity follows the same illumination envelope with a 36% strength multiplier
(the actual rays are dimmer after attenuation), with a reach of 36% of a cell
and at most 48 logical pixels. Linear falloff keeps the outer light legible.
During each illuminated hold, one backlight eases from the bottom of the column
to the top, translated into each letter's local coordinates. It resets only
while the light is off; the existing offscreen clock pause applies to the sweep.
Transmission and polish are disabled
so the letter faces remain unchanged. Four small, normalized letter artboards
keep the effect captures local; they sit outside the tape's clipping Frame so
the first and last rows can cast light beyond its edges. The selected tape paths
remain retained but hidden while those artboards draw them, avoiding doubled
edges. During travel only the ordinary tape paths draw; CPU retains the same
letter/color choreography without the shader.
Desktop places the field beside the mission copy; narrow layouts stack it above.

The uppercase alphabet uses the same Manrope 800 font and offline extraction as
the hero (800 is the font's maximum supported weight). The decorative grid fits
each outline's full control bounds into equal ink boxes, including rounded-letter
overshoot above cap height. A matching 0.025-cell outline adds weight; after that
stroke, both horizontal and vertical gutters are 0.055 cells. This optical grid
treatment is separate from the hero's unmodified, proportionally shaped letters.
The commands below also regenerate/check `src/acrostic_outlines_data.rs`;
commit that generated source when changing the alphabet. The tape and word list
are website-local constants in `src/acrostic.rs`, not a general text-rendering API.

## Offline outline extraction

The private offline extractor in `tools/trace-hero` uses HarfRust + Skrifa to
shape each complete phrase before splitting it into clusters. It is a standalone
tool, outside the library workspace and the website's dependency graph (including
tests). Ordinary website builds run no tracing step and embed no font bytes.

From the monorepo root, explicitly regenerate after changing the font, weight,
or the phrase list in `tools/trace-hero/src/main.rs`:

```sh
cargo run --manifest-path pax-website/tools/trace-hero/Cargo.toml
cargo run --manifest-path pax-website/tools/trace-hero/Cargo.toml -- --check
cargo test --manifest-path pax-website/tools/trace-hero/Cargo.toml
```

Commit `src/hero_outlines_data.rs` with the source change. It records the font
checksum, weight, and shaping versions, shares 17 unique cluster outlines
across the three phrases, and retains source ranges, glyph IDs, advances and
baseline metrics. `--check` compares the entire generated source, including every
coordinate and cluster placement, against fresh extraction. No manual curve
fitting or raster tracing is involved. The existing `build.rs` still only
generates the AI primer.

This is fixed, single-line Latin LTR copy, with no runtime wrapping, bidi layout,
font fallback or arbitrary text API. `hero_outlines::hero_runs()` expands the
baked data into the same `HeroCluster`/`PathElement` values once at mount.
Coordinates stay in em units; resizing changes bounds, and playback changes
scalar transforms without reshaping or replacing path elements.

Reusable runtime text and `TextGpu` API design are deferred to
[PAX-1036](https://linear.app/paxdev/issue/PAX-1036/design-textgpu-api-with-zack-and-resume-runtime-text-outlining),
starting with Zack-led API design. The full unpublished `pax-text` experiment is
preserved at `79a522edbc29614cf39aae70d30fba680f3c4b86`; it is not part of the launch
library. Neither a `TextGpu` component nor a `Text` rendering flag is implemented.

At that snapshot, the optimized web build measured 1,758,864 bytes with gzip
level 9, versus 2,208,106 bytes for the runtime-shaped predecessor: about 439 KiB
recovered.
Brotli level 11 recovered about 329 KiB. These are whole-Wasm measurements,
including the source drawer, and exclude the separate native font asset; they
do not establish the production server's compression settings.

Each vector word has one clipped native Text counterpart with transparent ink and
the same packaged font. Its selection rectangles follow the resting layout, not
the animated letters. Select during the reading hold for the closest alignment; copying
uses the actual phrase. This is approximate selection, not a complete accessibility
implementation or a public replacement for `Text`. The font remains a separate
asset for native text and selection; it is no longer embedded in the cartridge
for shaping.
Creative also has a clipped, transparent-ink Manrope counterpart outside its
handwriting mask. Its selection is approximate because the visible strokes use
EMS League. Keep the Text node itself opaque so the native selection highlight
remains visible; hide only its ink.
All website Manrope styles use the same `assets/fonts/Manrope.ttf` source and
actual family name. Web loads the file by URL; Apple registers the bundled file
at startup, without downloading another Manrope face. The existing Apple
font resolver selects its ExtraBold named instance (weight 800).
The selection box is clipped to enable the existing iOS/iPadOS text-view path;
the independently animated glyphs retain their original clipping behavior.

Headline motion stops while offscreen. The website has no pause button or
automatic OS reduced-motion handling yet. The logo plays once on mount and
replays when clicked or tapped at home; replay is disabled while visiting the nav.
The hero owns one playhead for the 1440ms reveal and Radiance lighting, including
click replay at home. The light returns to −20%, dims to 1.70, and idles between
−20% and 120% every 12 seconds. The same Radiance effect and light playhead
continue through the move into navigation and while docked, with the light
volume scaling down alongside the logo.
Set `HeroSection.radiance_enabled=false` for the plain logo;
CPU also renders the plain children.

The manifest pins the website's Pest parser family to 2.8.7. A fresh resolution
to Pest 2.9.3 introduced `stacker`/`psm` into the development Wasm and left an
unresolved `env.rust_psm_stack_pointer` import, preventing browser startup. Keep
the pin until that upstream Wasm path has been verified with Pax.

The website owns its `assets/` directory. Paxflix media entries remain symlinks
to the example assets, so adding website fonts does not modify that example.

Apple builds also pin `libc` to 0.2.189: `backtrace` 0.3.76 still imports dyld
bindings that 0.2.190 restricts to macOS, breaking iOS simulator compilation.
Remove the pin after verifying the upstream combination on Apple mobile targets.

The site shell uses `DynamicIslandSpacer` insets on Apple mobile targets so the
header and scrolling content avoid the status bar, cutout, and home indicator,
while the background remains edge to edge.

The runtime-shaped predecessor was checked in debug builds on an Apple Silicon
Mac and in the iPhone 17 Pro and iPad Pro 11-inch simulators, including iPad rotation.
The paused native word can be selected and copied on macOS; iPhone selection
shows the platform's handles and Copy menu. These checks do not cover physical
devices or optimized Apple apps. The full macOS release build was stopped during
lengthy LLVM optimization of the application crate, which includes this baseline's
large generated Rust manifest. That release runtime check remains open. The
pre-traced replacement passes exact geometry/placement comparisons, an optimized
web build, and desktop/mobile browser checks including paused selection and
copying. The updated Apple application has not been rerun.

The web build also extracts source-known copy and links into each concrete route's
HTML as a semantic article for assistive technology and browser reading modes.
There is no visible reading-mode button. Reader availability is browser-dependent;
the article is also exposed for printing, without JavaScript, and on link focus.
Website
`///#h1`–`///#h3` comments declare heading levels; `///#ignore` excludes interactive
demos and source overlays. The gallery's document contract visits its authored
cards once, retaining descriptions and documentation links without initializing
its live carousel. Check `Static document [...]` build diagnostics for unresolved
copy before publishing. See the [template guide](../pax-docs/book/src/template-language.md#static-page-text-on-the-web).
