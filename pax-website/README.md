# Pax website

The first-party Pax application lives at root-level `pax-website/`, beside the
static Zola site in [`pax-blog/`](../pax-blog/README.md). It is not a reusable CLI
example or a publishable crate. The monorepo explicitly excludes it from its
Cargo workspace so the site's standalone profiles remain effective: development
dependencies use optimization level 1; the website application uses level 0.

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
View Source includes the canonical files in `pax-std/src/drawing/pax_logo/`.
The animated hero logo remains inside the native page scroller, with its own
playhead and click replay. `LogoHome` measures its bounds; explicit bindings
carry its bottom edge and the observed page scroll offset into `SiteShell`.
The entire nav—static logo, links, CTA, and background—shares one fade playhead.
Its separate logo instance holds `progress=1` with replay disabled. The nav fades
in over 500ms once the hero logo is 128px past the nav's bottom edge, and fades out
over 450ms when that clearance returns to 96px. Both fades ease gently
at their start and finish; the earlier exit leaves time for the longer fade.
Hysteresis prevents
small scroll reversals from repeatedly restarting the fade. Neither instance
is translated or scaled in response to scrolling. The homepage scroller fills
the safe area from the start, with no reserved nav gap or scroll-driven reflow.
The hero logo aligns with View Source at a 24px top inset; on narrow screens its
width leaves room for that button. Fully hidden nav contents are unmounted, so
invisible links cannot intercept input. Hero source inspection hides the nav;
the standalone Paxflix route has no site shell. Other routes without a hero
retain a nav-sized content inset and show it directly.
This replaces the frame-sampled moving-overlay prototype, which
showed visible jumps while following native scrolling.
The nav is borderless and has no gradient overlay. Foreground surface ordering
for a future soft edge is tracked in
[PAX-1041](https://linear.app/paxdev/issue/PAX-1041/support-independently-ordered-foreground-compositor-surfaces).
Living Quilt remains independently runnable in `examples/living-quilt/`; neither
it nor the `pax-logo` harness is a website dependency. Cargo dependency paths
and favicon paths are relative to this manifest; Rust `include_str!` paths are
relative to their source file.

The creative-freedom headline embeds `LedOscilloscope` from
[`examples/led-oscilloscope`](../examples/led-oscilloscope/README.md), with the
message “DREAM IT. BUILD IT.” The standalone example and website share the cell
buffer, bitmap font, cabinet, and reusable knobs. The section publishes the
component's suggested responsive height; the cabinet fills that panel without
adding outer spacing. Its source is included in View Source. Motion
stops offscreen; system reduced-motion integration is tracked in PAX-1048.

The HTML background uses `package.metadata.pax.web.background_color` to match
the scene's dark ink before the cartridge starts. The hero logo holds its
initial pose until frames and hero height have stayed stable for 100 ms, then
runs the usual reveal. Long startup frames restart that settling interval;
a five-second ceiling prevents a persistently slow frame rate from suppressing
the logo indefinitely. There is no fixed delay added to every page load.

The website enables `package.metadata.pax.web.yield_mount`, so ordinary homepage
and standalone demo URLs use yielded startup without query flags. Use
`?pax_mount_yield=0` only to compare the synchronous path. The plain URL waits for
the initial viewport's native font and layout measurements, then eagerly mounts
and progressively presents the remainder. Text can appear before artwork;
canvas drawing still waits for engine settlement. Keep the startup mode and viewport
dimensions the same when comparing measurements; desktop layouts mount the full
21-shelf Paxflix catalogue. See
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

The website declares `/blog`, `/ai`, `/ai.md`, `/paxflix`, and `/calculator` as `server_owned_prefixes`.
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

At a showcase width of 760px or more, the homepage mounts one live tablet demo
with macOS and iPhone captures behind it. Below that width it shows only the
tappable iPhone capture; a fresh mobile homepage mounts no Paxflix app. The
fixed device footprint scales with the section width, and sizing subscribes to
reactive bounds so it settles during initial mounting. Crossing this breakpoint
removes or creates the tablet session; ordinary scrolling keeps it mounted.

Both the phone preview and the full-screen action open `/paxflix/`.
The generated `/paxflix/index.html` is a standalone document entry into the
website's demo route. Its server-owned navigation policy deliberately reloads
the document so the demo participates in the configured yielded startup and
initial presentation gate. It uses the same website bundle, with one Paxflix
instance and no homepage tree. This reduces mounting work, not Wasm download
size. The old homepage modal, which created another app instance, is removed.
Browser Back can restore or reload the homepage; the demo's Back to Pax link
uses ordinary in-app navigation. Each new demo session starts with fresh state.

The captures use `NativeImage` and live in `assets/paxflix/`; their README records their native origin
and regeneration commands. Device switching is not implemented in this version.

## Calculator showcase

The footer keeps its getting-started copy alongside a three-device fanout: a live,
upright phone in front, with a smaller tilted macOS-style window on the left and
a tablet on the right. The window uses `DeviceFrame kind="window"`, with a dark
title bar and traffic-light controls. Its narrow capture fills the content area
without the calculator's wide-window background gutters.
The desktop phone mounts and updates with the page, keeping its session while
scrolling without an application-level visibility gate. The wings use
`NativeImage`, not additional calculator instances.

At the footer's single-column breakpoint (section width below 1000px), the fanout
becomes one tappable phone capture. A fresh compact homepage mounts no calculator.
Resizing across that breakpoint removes or creates the live phone session. The
fixed-aspect stage scales to its assigned column without stretching the devices.

The compact preview, captured wings, and full-screen action open `/calculator/`.
Like `/paxflix/`, this is a generated document entry with server-owned navigation:
one live example, no homepage tree, and safe-area-aware full-screen layout with a
Back to Pax link. Each new demo session starts with fresh calculator state.
The footer's View Source includes both the showcase and full-screen wrapper;
application logic remains in the canonical `examples/calculator` Cargo dependency.
Capture provenance and refresh commands live in `assets/calculator/README.md`.

## Typography

The website, calculator controls, and `ExampleHost` UI use bundled
Manrope. Source code and terminal commands use a system monospace; the calculator
LCD retains its Courier-family lettering. Paxflix keeps its system Arial UI
and Georgia titles, which require no downloads. The website makes no Google Fonts
requests for these families. Manrope is also bundled in `pax-std/assets/fonts/`
for standalone ExampleHost and demo use; the identical website asset merges into
one output file. Keep these copies and their license aligned.

The shared `ExampleHost` drawer uses dark grayscale surfaces, white active tabs,
and CMY plus lightened RGB syntax colors. Its 160px source tabs stay single-line
and scroll horizontally; very long labels clip within the tab, with the selected
path also shown above the strip.

Content and visual design history are archived in
[PAX-869](https://linear.app/paxdev/issue/PAX-869).

## Shared-definition section

`AuthoringSection` sits between creative freedom and the feature gallery. It
introduces the declarative UI definition as a shared, inspectable contract for
people and coding agents, while keeping the visual-scratchpad vision explicitly
aspirational. Its copy and layout are a working placeholder for editorial review.

`SharedDefinitionDemo` pairs the actual `relationship_preview.pax` source
(embedded with `include_str!`) with a live instance of that component. The 25%
and 75% buttons change one state value, updating both the label and bar; they do
not simulate source editing or an agent interaction. The panels stack below
900px. No animation clock or additional dependency is introduced. View Source
includes the section, the demo wrapper, and the preview template.

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
`LightFrame`. Its sweep begins during the reveal and lasts 1.04 seconds.
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
replays when clicked or tapped: `AnimatedPaxLogo.click_to_replay` already defaults
to true. Its `progress=bind:logo_progress` shares one playhead with the website's
initial 1440ms reveal; there is no competing per-frame progress writer.

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
