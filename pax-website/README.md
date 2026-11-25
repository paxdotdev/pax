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
The website sets `loader_exit = "full_mount"` under `[package.metadata.pax.web]`:
the loading screen stays over the prepared hero until mounting finishes and the
main animation clock can run. Its 250 ms exit overlaps the hero's startup settling
interval, instead of exposing a stationary logo while the rest of the site mounts.

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

Keep this debug session running for visual iteration: `.pax` edits hot-reload
by default (`--hot-reload=pax`), including changes to existing layout and effect
components. Rust logic or new component types require a rebuild and restart.
Use release previews for performance checks, not routine template iteration:
release builds disable hot reload. The extracted reader HTML is also a build-time
snapshot; rebuild to refresh it after changing copy.

For an optimized web build without serving or publishing:

```sh
./pax-website/pax build --target web --release
```

### Publish the website to staging

After the release build above, run from the monorepo root:

```sh
./pax-website/deploy.staging.sh --dry-run  # offline: validate and print the runtime URL and routes
./pax-website/deploy.staging.sh --publish  # requires the AWS CLI and the pax profile
```

Both commands use the existing `pax-website/.pax/build/release/web/` directory;
neither rebuilds the site. To promote a release built in another checkout, pass
`--web-dir /path/to/release/web`. The target is the **root** of
`s3://staging.pax.dev/`, not the retired website's `/www` prefix. The blog,
including any local draft preview under `blog/`, is not published by this command.

Every generated Pax route links its stylesheet, script and cartridge to the same
`/_pax/releases/<content-hash>/` directory. The publisher uploads that runtime
first, including both Wasm modules and their JS imports; old runtime URLs remain
available. It then uploads site content and revalidatable route HTML (homepage
last), and waits for CloudFront to invalidate the mutable paths and index aliases.
To smoke-test an upgrade, reload an existing staging tab normally and check
that the hero actually paints; a fresh tab or hero text in the DOM alone can
miss a stale stylesheet.

### Website and blog analytics

The ordinary local web build does not include analytics. Staging and production
use the same first-party consent control and the same GA4 stream.
To prepare production website files **without deploying**, build the release
above and run the following with a new output directory:

```sh
python3 pax-website/tools/prepare_production.py /path/to/new-website-package
```

This packages the same versioned runtime and standalone route documents as
staging, with one analytics consent control on each HTML page (including
the static AI guide). The blog is excluded; its production Zola template uses
the same script from `pax-blog/static/site-analytics/`. Neither the loader,
embedded examples, native apps, nor the blog's Pax header configure GA.

Only top-level documents on **https://www.pax.dev** and **https://staging.pax.dev** offer analytics. The
visitor can allow or decline, and reopen **Analytics settings** at any time.
The choice is stored per origin: granting it on staging does not grant it
on production. The website and blog on the same origin share it. Until granted,
no Google tag is fetched and no actions are queued. With consent, stream
`G-MG3GLRDHYR` receives its normal pageview for each document load; URL queries
and fragments are omitted. Ads and Google Signals are disabled. Withdrawing
disables the tag and removes this site's host-only GA cookies.

**To test a staging cut:** open the published staging page, choose **Allow
analytics**, then open **View Source** or copy the install command. In GA4 →
Admin → Data display → [DebugView](https://support.google.com/analytics/answer/7201382),
look for the staging `page_view` and `view_source` or `copy_install`. Staging
sets `debug_mode: true` on the tag; production omits it. This flag alone does
not exclude staging traffic from regular reports. To keep developer traffic
out of those reports but visible in DebugView, test and then activate GA4’s
[Developer Traffic exclusion filter](https://support.google.com/analytics/answer/13296662).
The publisher does not change GA settings or publish the independently owned blog.

| Website event | When it fires | `placement` |
| --- | --- | --- |
| `copy_install` / `copy_ai_prompt` | Clipboard write **succeeded** | `hero` or `footer` |
| `open_in_codex` | Visitor requested the handoff (not proof the app opened) | `hero` or `footer` |
| `view_source` | A closed drawer opened; not a mount, resize, or default-open cube | Section identifier |

No clipboard contents, prompts, deep links, or per-frame/scroll events are passed
by the app. **Before the first analytics-enabled staging or production cut**, in GA Admin → Data streams,
turn off Enhanced Measurement (especially outbound clicks, form interactions,
and history-based pageviews) so it does not add events or capture link URLs
outside this list. Register `placement` as an event-scoped custom dimension if
you want to break down reports by hero/footer/section. Verify a consenting
homepage → demo → blog visit in DebugView; expect three pageviews. See
[Google's pageview guide](https://developers.google.com/analytics/devguides/collection/ga4/views)
and [basic consent mode](https://developers.google.com/tag-platform/security/concepts/consent-mode).

Offline acceptance: build the production blog (`python3 pax-blog/blog.py build`),
then run `PAX_TEST_SITE=/path/to/new-website-package node --test pax-website/tests/analytics-smoke.cjs`
with Playwright and Chrome available. It serves the actual packaged documents
to Chrome by request interception; test visits never reach Google or production.

To preview the same hero on Apple targets, run one of these commands:

```sh
./pax-website/pax run --target macos
./pax-website/pax run --target ios --ios-device simulator
./pax-website/pax run --target ipados --ios-device simulator
```

Stop the previous CLI preview before switching targets: one project shares its
generated cartridge and interface staging directory across targets.

## Native desktop robustness workload

The complete website is the application-level acceptance workload for the alpha
Windows and Linux hosts. Build on the corresponding operating system:

```sh
pax-cli build --path pax-website --target linux --release
pax-cli build --path pax-website --target windows --release
```

Use this checkout's CLI and keep each complete `.pax/build/release/<target>/`
directory together. The optional `standalone` feature enables `bin/native.rs`,
which initializes the website directly through `pax-kit`. Native CLI builds
select this binary; Cargo's default `run` binary remains the development wrapper.
A successful build is not full-site acceptance. Qualification
includes the rotating hero and native text, wide/narrow resizing, every page
section, source drawers and selection, calculator controls, embedded/full-screen
Paxflix, gallery motion, and independent parent/nested scrolling. Source drawers
require selectable styled code; replacing them with plain text would not qualify
the intended experience. The desktop cartridge normalizes that markup into styled
runs for the native host; both parts of the package must support the
[desktop rich-text subset](../pax-docs/book/src/text-fonts-images.md#markdown-content).

The website and its runnable calculator/Paxflix dependencies use automatic
[package-qualified native exports](../pax-docs/book/src/targets-build-deploy.md#entry-and-mounting).
The embedded examples remain in the workload; native packaging selects the
website's entry points without changing those libraries.

Measure process startup separately from compilation, then stationary animation,
first traversal and repeat traversal at a recorded viewport size. Paxflix retains
visited rows, and the gallery retains its largest valid projection window, so
first-visit allocation growth needs a repeat traversal before interpreting it as
a leak. Record CPU time, host timing, memory and normal shutdown independently;
remote-desktop stream FPS is not application presentation FPS. The
[Linux probe](../tests/src/native-platform-probes/linux/README.md#website-application-workload)
records this initial baseline. The focused native-control, IME and snapping
fixtures remain necessary: the website's gallery cards illustrate some features
without exercising their underlying platform integration.

Debug and release outputs live in `.pax/build/debug/web/` and
`.pax/build/release/web/` beneath this directory. Generated `.pax/`, `target/`,
`Cargo.lock`, AI-primer HTML, and staged blog previews stay untracked.

The hero uses `AnimatedPaxLogo` from `pax-std` through the `pax_kit` re-export.
Its website-local `SiteLogo` wrapper carries the new radiance treatment and fits
the artwork into its hero home or the navigation's smaller container. View Source
includes the wrapper and canonical files in `pax-std/src/drawing/pax_logo/`.
The navigation uses a fixed `Overlay` with an opaque vertical slide instead of
group opacity. A centered, island-sized Scroller with no scroll range clips the
slide at the island’s top edge and keeps the logo, pill-shaped Get Started action,
and native links together. The page stays visible and interactive in the gutters.
Only the top safe-area inset receives a separate native backplate for Safari’s status icons.
Hero and nav each retain a separate `SiteLogo`, coordinated by
one reversible 800ms portal clock (described below). The hidden clip is parked
outside the viewport so it cannot intercept clicks; reversals continue from the current pose. Both logos
stay mounted in their own rendering contexts, while fully hidden controls unmount.
Only a full-screen source sheet hides the nav, including during its exit animation.
With an inline drawer open, the nav still appears as the hero scrolls away.
The homepage starts without a reserved nav gap;
other routes keep their nav-sized inset and static logo.

## Homepage content

The page opens with the two-line heading “Build creative/native/portable/eye-catching
user interfaces,” followed by the Pax/Rust introduction and the install snippet.
The Get started section lists individual targets. Windows, Linux, and Android carry inline “(alpha)”
labels; the other targets are unmarked. As platform support evolves,
check the release's target availability before publishing. The shared CLI-first
snippet appears in both the hero and closing CTA, with AI onboarding in a
secondary tab.

“A declarative language for interactive graphics” follows the hero, pairing a real
template with its live result and explaining the Rust/Pax boundary. Paxflix's
full-width fanout comes next, without a second runtime manifesto. The toolkit
carousel precedes the reusable-component section. The live oscilloscope
follows the Get started CTA at every width;
the calculator is no longer embedded on the homepage.
“A love letter to the pixel” pairs the acrostic with the project's open-source
mission and design influences. The closing CTA links to setup and
examples, followed by the oscilloscope, then docs, GitHub, blog, community, and platform support.

The [black sateen curtain](../examples/fixtures/curtain-panel/README.md) fills
the hero behind the logo, headline, actions and install snippet, down to the
bottom rule. The space below the install snippet matches the 62px gap from
the CTA buttons to the top of the snippet box. The hero's `LightFrame` contains
the cloth and the glass buttons.
A neutral strip light follows the logo's reveal and idle sweep across the hero.
The 900px strip sits 340px above the stage with a 2300px falloff radius so
several cloth folds catch it at once. The button rim focuses its own response
along the edge (`light_span=0.22`), showing about one cyan–magenta–yellow triplet
per glint (`hue_cycles=1`) without narrowing the light cast on the curtain.
The triplet rolls twice per button width of travel (`hue_travel=2`), so
different colors take the lead as the glint moves. The pointer light has a 640px
falloff radius; its motion shifts the bands even when the strip is still.
The `.hero_sweep` settings bind the same artwork-space position and brightness
to both components. The curtain maps that position onto its own width and height,
while the logo uses it directly for its `Radiance` projection. The GPU cloth pass
runs at half resolution and samples its declared solid color. Its clock pauses
offscreen and resumes from the held pose. CPU rendering uses the plain cloth color.

The language introduction and Increment Cube share a solid black background.
Copy retains the page gutters. Desktop opens the cube's source drawer by default;
compact layouts stack the cube and inline source with an expand action.
“Plug, Extend, and Play” follows “Simple meets powerful,” introducing reactive composition
and linking to the standard library and component-authoring guide. Its copy lives
on the plain themed background in `ModularitySection`. At widths of
1,000px and above, its copy and circuit board share a row, with the board on the
right. Below that breakpoint the board sits between the large introductory paragraph
and the smaller paragraphs about wiring and authoring components. The section's
ExampleHost lists the wrapper, board, package, pin, and logo templates/Rust sources,
plus the board layout, motion, and signal-routing modules.
Its `CircuitBoard` illustration assembles six named packages from reusable
`CircuitPackage` and `CircuitPin` templates. The packages form a reduced tree of
the illustration's own template: `CircuitBoard` → `For` (chips) → `CircuitPackage`,
which branches to `Text` and `For` (pins) → `CircuitPin`. Every package has leads
on its left and right sides: three per side on most packages, five on
`CircuitPackage` to give its branches separate pin banks. Both layouts use four
physical rows: `CircuitBoard`, `For`, `CircuitPackage` / `CircuitPin`, and
sibling `Text` / `For`. CircuitPin sits slightly above CircuitPackage, aligning
its lower-left pad with the normal through the nearby copper bend.
`CircuitPin` deliberately sits above its parent `For`
to reclaim the otherwise empty last row; the trace still shows their relationship.
The packages align in two sparse columns: `CircuitBoard`, the first `For`,
`CircuitPackage`, and `Text` on the left; the final `For` and `CircuitPin` on the
right. Short side-pin routes keep the rows close together on a 580px-tall PCB;
only the branch to the pin subtree crosses between columns. The stage occupies
540px on desktop and 370px below a 760px viewport width.
Lighting, layout Groups, primitives, and repeated instances
are folded away from the depicted hierarchy. Closely spaced octilinear copper
bundles show parent–child composition, with no cross-links. Each bundle follows
one shared route, with 4px perpendicular lane spacing through nested 45° bends
and fan-outs at its pads. The GPU view uses lit cuboids and
face-mounted native text in an orthographic view. While visible, the board rocks
about its Y axis between −20° and +20° over a ten-second cycle, centered face-on.
Click or touch the board and drag horizontally to rotate freely through full
turns (one stage-width of travel is one revolution). On release or cancellation,
it eases to the nearest equivalent idle endpoint in 450–850 ms, then rocks inward
again. Re-grabbing interrupts that return at its current pose. Touch input waits
for horizontal intent; vertical gestures remain available for page scrolling.
A still Pax logo fills the upper-right clearing above CircuitPin, using the same
copper paint and material as the traces. `CircuitLogo` contains four filled paths
ejected from the canonical `pax-std/src/drawing/pax_logo/pax-logo.svg`; its lettering
is cut out to reveal the PCB, and the logo adds no animation or alpha mask.
The board's sixteen traces connect pin pads, including three parallel runs from the
left-edge connector into CircuitBoard's inputs. The connector aligns with the
root package and has two banks of three soldered through-board leads. The top,
middle, and bottom Ethernet pins emit staggered cyan, magenta, and yellow pulses.
The short input traces run at about 153 board pixels per second. At each package, a
pulse keeps its color and takes a randomly chosen connected output until it
reaches a leaf. Eight to ten pulses travel at once. Alternate pulses cast a
23-board-pixel-radius light onto the copper. Board and signal clocks pause
offscreen and skip pauses of a second or longer
instead of catching up after a hidden-tab pause. Below 600 logical pixels of
available board width, the PCB and column spacing narrow while preserving the
same four-row arrangement. CPU rendering uses
an unrotated, static plan view without drag rotation.
Back-facing native labels stay mounted but hide while the GPU board
is flipped, preserving pointer capture without reversed print.
Each lead passes through the 12px PCB and projects 12px beyond its back, with
copper pads and rounded solder beads on both sides. The back face prints
“Πάντες ἄνθρωποι τοῦ εἰδέναι ὀρέγονται φύσει” in 24px italic lettering, vertically
centered beside the first For's three-bead banks. A `CuboidFace` keeps the lettering
upright from the reverse view; its ink hides from the front. The bundled Circuit
Greek font is a small Inter Italic subset covering the exact phrase, including
its polytonic Greek accents (see
[font provenance and regeneration](assets/fonts/README.md#circuit-greek)).
The GPU assembly contains
106 cuboids and 92 ellipsoids (rounded solder beads); there are no decorative LEDs.
The ellipsoids share retained sphere detail levels selected by their transformed
size in this orthographic view, up to 3,968 triangles per instance. Lower levels
use a conservative 0.25-logical-pixel surface-error bound and hysteresis;
rendering, picking, and native masks share the selected surface. The illustration
is a composition analogy, not an electrical simulator. Its
connector is a folded cuboid shell; package notches and matching front/rear
mounting-hole marks are surface details, without geometric bore cut-outs.
It has no imported mesh, cast shadows, or texture maps, and
the solder beads are ellipsoidal approximations, not concave fillets.
The board's opaque solid-color canvas paint uses the GPU depth path automatically,
sharing depth between its planar faces and curved solder beads. Topology stays
retained through rotation; depth and transform parameters change. Transparency,
alpha masks, images, gradients, effects, and captures retain the ordered
compositor. Native label masking and canvas/native publication are separate;
this optimization does not fix cross-layer drift when frames drop. See
[spatial depth composition](../pax-docs/book/src/layout-responsiveness.md#rotating-flat-surfaces-in-3d).

“An alternative to HTML, CSS, and JavaScript” and the Paxflix showcase share one full-width pressed-glass wall
in `RuntimeSection`. The text, live tablet, device links, and source drawer sit
in front of the glass. The dark and light themes use the fixture's dark and
white backing paints. The wall uses 175% brick size, 0.70 thickness, 0.31
surface ripple, and inner and outer bezel strengths of 0.25× and 0.40×.
Three rear point lights at −52 px sweep right to left at 3.5× their individual
speeds. Cyan (0, 225, 255), magenta (255, 7, 116), and amber (255, 162, 6)
each have intensity 1.40 and a radius of 89% of the wall width. A light picks
a new height and speed once its glow clears the wall. The route's white
pointer at +95 px reflects off the front.

The `ModularityRain` backdrop is available for reuse but is not mounted.
Its wet-pane shader, animation clock, and three searchlights do not run behind
the circuit board.

The opening sequence uses the outer native scroller as a shared clock.
`scroll_story.rs` maps its offset into overlapping chapter-local clocks for the
hero, the Paxflix showcase, and the toolkit examples. Paxflix's clock includes the
language section's measured height, including reflow when its drawer opens.
The hero starts fully composed;
its heading shifts slightly on departure, while
device frames assemble, hold their poses, and depart. Named component timelines sample those ranges in either
scroll direction. Paxflix's chapter ends just below the measured showcase,
including its link on compact layouts; tall viewports do not add a scroll tail.
Animated child transforms do not change chapter heights. This first slice
does not pin scenes. Paxflix keeps three devices at every width, with a live
center tablet on desktop and linked screenshots on compact layouts. The
calculator retains its single-phone layout at every width; only compact layouts
use the screenshot link. The oscilloscope occupies its own centered study.

White hero, footer and navigation buttons use the pool-floor shader over
`#EEEEEE` paint, with wave density 1.50, amplitude 1.00 and speed 1.25.
The hero swaps the white and obsidian faces in light mode. White View Source
tabs use density 1.70, including when the tip folds into the close tab.
The lit CMY contour extends 2 logical pixels into white faces and 1 pixel
into dark faces.

The website supplies the pointer light and one shared 30 Hz wave clock.
Visible water faces follow that clock. Reduced motion freezes the waves
while the pointer light can still move. Each white face shades the water and
its contour in one half-resolution GPU pass. The projected rectangle or path
supplies the shape, with labels and hit targets above it. CPU rendering uses
the painted face. The standalone blog header keeps its painted fallback.
The GitHub icon's official source is
recorded alongside the asset in `assets/branding/`.
The island has a thin themed outline and no gradient overlay. Foreground surface ordering
for a future soft edge is tracked in
[PAX-1041](https://linear.app/paxdev/issue/PAX-1041/support-independently-ordered-foreground-compositor-surfaces).
Living Quilt remains independently runnable in `examples/living-quilt/`; neither
it nor the `pax-logo` harness is a website dependency. Cargo dependency paths
and favicon paths are relative to this manifest; Rust `include_str!` paths are
relative to their source file.

The navigation contributes an `Overlay` above scrolling content. As the logo's
reserved home reaches the nav boundary, the hero logo shrinks and rocks upward
offscreen. The nav slides down, and its finished-logo twin enters from the left,
rocking level and shrinking to nav size. The two edge crossings overlap slightly;
scrolling back reverses the same choreography. `logo_portal.rs` defines the shared
phase ranges. No node or shader capture transfers between canvas contexts.
A subscription follows the stationary home's layout, page scroll, and responsive
nav boundary; returning requires 24px of clearance to avoid threshold jitter.
If the compact home already overlaps the future nav, departure waits for 32px of
scroll. Animated transforms leave the reserved home and section layout unchanged.

The island measures 552 × 52px on desktop, 14px below the safe top. Below 760px
it is at most 312 × 48px, with an 8px top gap and at least 10px on either side.
Pages without
a hero show the static `NavLogo`. On first reveal, the logo starts unfurling
after its layout height has been stable for 100 ms, including on slower displays.
If the layout keeps changing, the intro starts after five seconds.
The hero logo replays on a scroll
return from the nav. A return holds the intro's initial pose during the portal
trip, then replays the full unfurl and lighting sequence once the hero home is
reached. Departures into the nav and hiding it for a full-screen source sheet do not trigger the
intro; reversing an unfinished return cancels it. Clicking or tapping the hero logo
replays only the unfurl and light sweep. Switching appearance keeps the page in place
and rewinds the hero logo. After 100 ms of stable frames
(or 900 ms on a persistently slow device), the logo unfurls, the curtain light sweeps, and
the headline writes “creative” again. A switch from the docked nav or embedded Paxflix
waits until the hero is back in view; repeated switches queue only one replay. The nav twin uses the completed logo and the
same slow 12-second Radiance sweep as the hero, without replaying its intro.
It samples the runtime clock independently of the offscreen hero's paused
playhead; its light updates run only while the nav twin is entering, docked, or
leaving on GPU. Hidden navigation and CPU rendering do not run this light tick.
This is a website
motion-design workaround, not a fix for the deferred foreground-compositing
limitation in PAX-1041.

The footer embeds a live oscilloscope. `FramedStudy` spans the content gutters,
with no rotation or simulated window chrome. A wide host gives the matrix more
columns, up to 1600×600 before scaling; a phone gets a taller cabinet with
upright letters marching upward and the control pairs below the screen.
The homepage mounts it on first approach and pauses its animation offscreen
without discarding the visitor's settings. System reduced-motion integration is tracked
in PAX-1048.

The HTML background uses `package.metadata.pax.web.background_color` to match
the loader's black backdrop, including under translucent mobile browser chrome.
The hero logo holds its
initial pose until frames and hero height have stayed stable for 100 ms, then
runs the usual reveal. Long startup frames restart that settling interval;
a five-second ceiling prevents a persistently slow frame rate from suppressing
the logo indefinitely. There is no fixed delay added to every page load.

The website enables `package.metadata.pax.loader`,
`package.metadata.pax.web.yield_mount`, and `loader_exit = "full_mount"` in
the web metadata. The loading screen stays visible while the main application's
initial viewport mounts in a hidden, inert region. Once its required content,
native measurements, and visible surface submissions are ready, the page is prepared
beneath the opaque loader. Remaining content mounts cooperatively before the loader
begins its exit. Full mounting releases the main animation clock, allowing the
hero's 100 ms settling interval and logo reveal to begin during the loader's
250 ms fade. Slow frames can still extend that settling interval; this is not a
guarantee of a particular first animation frame. Input is enabled once the loader
exit completes. This handoff trades a longer loading screen for avoiding a frozen
hero; it does not reduce the total mounting cost. See
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
The homepage includes desktop Paxflix (a capture on compact layouts) and,
on first approach, a responsive live oscilloscope. Opening an example's route mounts a
single example without the homepage. See
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
illustrations, using authored gradients (silver and sea-glass tones on paper) and a moving position marker
instead of live lights. This avoids lighting-dependent tile invalidation from
the shell's strip/pointer lights and removes the cards' local light emitters;
it does not eliminate the rail's animation, layout, or compositing work.

The full-bleed rail fades into the page ink over twice each content gutter (12% on
desktop, 48px below 760px). Two non-interactive `NativeImage` overlays reuse
`assets/branding/carousel-edge-fade.svg`, mirrored on the right, outside the
horizontal scroll host. On web this preserves per-pixel alpha over both native
text and rendered scroller content; a canvas gradient's native coverage estimate
would flatten that fade. The image follows `SiteTheme::ink(light_mode)` in both palettes; keep the
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
and links. `SiteShell` retains the homepage's portal-logo twin and
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
and `/oscilloscope` as `server_owned_prefixes`.
Navigating to these paths loads a server document instead of a Pax client route.
The `/blog/*` Router placeholder must not be restored. `build.rs` generates
`public/ai/index.html` from `public/ai.md` only when its contents change.
The Markdown is the canonical AI onboarding guide. Its HTML version uses
semantic headings, links, lists, and code blocks, works without JavaScript,
and links to the raw `/ai.md` source. Keep commands and target caveats aligned
with the version-matched CLI documentation; do not duplicate a separate guide
in the website component.

The hero and bottom Get started section use the same compact `GetStarted`
component. Both default to the CLI commands; a secondary **With AI** tab shows
one read-only, selectable prompt, with inset clipboard and Open in Codex actions.
The clipboard control shows a tooltip on hover or while touched. Each instance
draws one of 25 credential-free app ideas on mount; resizing, scrolling, and
switching tabs preserve the choice. **Another idea** never repeats the current
preset. `ai_prompt.rs` owns the briefs and exact prompt used by both actions.
The bottom section also links to `/ai/` in the live page and reader article.

Copy uses the shared `NodeContext::write_clipboard_text` API for either tab and only
reports success after confirmation. Browser-denied writes leave the prompt
selectable. Open in Codex uses the documented
`codex://threads/new?prompt=…` deep link to prefill a draft, never auto-send it
or select a local project directory. It requires the installed Codex app;
Copy prompt is also usable with Claude Code and other agents. The handoff
points to the production guide, so deploy `/ai/` and `/ai.md` with the website
before using the new onboarding flow publicly.

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

The homepage shows a gutter-to-gutter fan of three devices: a tablet in front,
with macOS and iPhone screenshot wings. The section's 760px breakpoint chooses
the center screen: one live Paxflix instance on desktop, a linked tablet capture
on compact layouts. All three devices remain visible in either layout.

The live tablet uses `embedded=true`, `disable_keyboard=true`, and
`disable_card_entrances=true`: it omits standalone safe-area UI and global keyboard
handling, and skips shelf-card entrance observers while retaining interaction and
nested scrolling. Its viewport stays at 1024×768 logical pixels. Only the outer
device stage scales to the available width, without a maximum-width cap; the
scroll-sampled arrival, hold, and departure poses transform those frames without
resizing the app. The center tablet stays upright throughout scrolling; only its
vertical position changes, while the captured wings retain their rotations.
It stays mounted as the page scrolls. Crossing the compact
breakpoint replaces the app with a capture, so returning to desktop starts fresh.

The screenshot devices and launch links open `/paxflix/`. On desktop,
**Full Window** sits inside the tablet's bottom-right corner, outside the live
app's scroller. It travels with the tablet but keeps a screen-sized hit area.
On compact layouts, **Try Paxflix** sits just below the fan.
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

## Homepage composition and standalone calculator

The homepage flows from the hero to the declarative-language introduction, cube,
and **An alternative to HTML, CSS, and JavaScript**, followed directly by its example, Paxflix.
**Simple meets powerful** and the feature carousel precede **Plug, Extend, and Play**.
`ModularitySection` owns the modularity copy and the circuit-board visual's integration
location; it is a standalone section, not part of `AuthoringSection`. Keep the board
with that copy when integrating PAX-1080.
The mission/acrostic section follows modularity. **Get started** contains the
shared CLI-first tabbed snippet and guide actions, then the oscilloscope, then
the footer links. The gap below the last line mirrors the gap above “Build” in
the hero, including the logo row when the hero stacks on smaller screens.

The calculator no longer mounts on the homepage or contributes source tabs to
its drawers. Its existing `/calculator/` URL remains available. Like `/paxflix/`,
this is a generated document entry with server-owned navigation:
one live example, no homepage tree, and safe-area-aware full-screen layout with a
Back to Pax link. Each new demo session starts with fresh calculator state.
Application logic remains in the canonical `examples/calculator` Cargo dependency.
Capture provenance and refresh commands live in `assets/calculator/README.md`.

## Live creative example

The oscilloscope fills the Get started content gutter below the CTA actions and
above the footer links. It is playable inline on phones and desktops. The caption
opens the full-screen route. The Get started source drawer includes its wrapper and application sources.

`/oscilloscope/` is a generated document entry, with the same server-owned
navigation policy as the other examples. It mounts only the oscilloscope,
provides View Source, and has a Back to Pax link.
Space Game is no longer included in the website's routes, assets, or Cargo
dependencies; its canonical standalone fixture remains in `examples/fixtures/space-game`.

All three routed examples still share the website's Wasm bundle. Using a
screenshot for compact Paxflix removes its homepage nodes, timers and nested
scrollers; it does **not** split the initial download.
Desktop Paxflix and the oscilloscope retain one live homepage instance each. Independent example
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

## Appearance

The overlay nav has a keyboard-operable moon/sun switch just before Blog, on desktop
and mobile. The hero logo replays its animation without changing appearance. Dark is
the initial appearance. The standalone blog header retains its logo theme shortcut.
A root-owned Boolean drives the page’s `SiteTheme` settings imports, both hero
materials, source drawers, and the live Paxflix profile switch (in either direction).
On web, the same-origin preference is restored on reload and on the blog and demo routes.
Paxflix swaps its device captures and bezels without recoloring movie photography. The circuit board keeps its black packages; a warmer ambient fill and green solder mask distinguish its daylight presentation. The oscilloscope keeps its original colors.

With a local build running and Playwright available, run
`PAX_SITE_URL=http://127.0.0.1:<port> node --test pax-website/tests/theme-smoke.cjs`
from the repository root to exercise hero replay, the nav switch, the drawer, Paxflix, and preference persistence.
With WebGPU available, run `pax-website/tests/button-rim-smoke.cjs` the same
way to check that one CMY triplet rolls through the moving glint at both wide and narrow widths.

Website hosts omit their own moon/sun switch (`show_theme_toggle=false`).
Both the Solarized code palette and the source chrome follow the page. Other
ExampleHost consumers still default to light code, dark chrome and a visible switch.
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

## Increment Cube and language section

`AuthoringSection` is the mounted “A declarative language for interactive graphics”
introduction and cube stage. “An alternative to HTML, CSS, and JavaScript” follows in `RuntimeSection`, with
one glass wall continuing behind Paxflix. The dedicated cube
viewer contains only `increment.pax` and `lib.rs`, selected from the canonical
example's source list. Compact inline and expanded presentations share the
cube selection; the desktop drawer starts open with half the panel for code and
half for the live cube. The divider remains draggable. Source changes reset the code
pane's horizontal and vertical offsets, so the new file opens at its beginning.

Oscilloscope sources live in the Get started section's View Source panel;
the cube viewer shares the page's Solarized theme.

`IncrementCube` mounts the canonical `examples/increment-cube` component.
Clicking or tapping the cube accumulates
randomized three-axis turns and settles one second after the latest activation.
The centered front face reads CLICK on desktop and TAP on mobile. The first
activation rolls to 1, the second to 2; from 7 onward the new number replaces
the oldest. The cube remains centered across responsive remounts. Pressing
compresses it and releasing briefly expands it. The six GPU faces each have
a translucent surface with a refractive rim and a separate native label. The
website's pointer light reveals the rim locally; the cube adds no fixed light to
the page. CPU uses a flat counter with the same invitation and input behavior.
Compact layouts stack below 900px; an explicit
source expansion owns the full-screen sheet independently of the desktop drawer.

## Hero lettering

The three Manrope words in `RotatingHeadline` use checked-in Bézier outlines at
weight 800 of the [bundled font](assets/fonts/README.md). `HeroLetter` animates
one path per shaped cluster. Each path pivots around the center of its cap height.
Native assembles along short arcs, and portable presses and rebounds.
The static “Build” and “user interfaces” lines sit above the animated artwork
so its effect bounds can cross their line boxes without masking the text.
Native and portable freeze their
painted fields and fade in place on exit, reusing the assembled pixels rather
than resizing shader captures. Letter poses derive reactively from the word, ordinal, and clock, including
the retained native → portable handoff; they do not wait for a per-letter
pre-render callback. Creative keeps its EMSLeague handwriting and CMY
mesh treatment, with boundary control points moving up to 26 percentage points
and the center up to 30, while keeping the cells convex. The mesh loops every
two seconds, twice the previous morph speed.
Its stroke width is `max(1.6, font_px * 0.038) + 1.5` logical pixels.
The source drawer includes the path loader, generated geometry, and letter component.
The selectable “Build” prefix occupies only its 2.75em slot, leaving the
transparent native text over the animated word available for direct selection.

Eye-catching composes frosted gradient letters with the reusable `Flashy` shader. On dark, a neutral key lights cool glass at frost strength 0.8. On light, the same key sweeps from cyan through magenta to gold over blue-violet glass at strength 0.42. The hero and docked logo use Radiance’s CMY prism in light mode. One scoped scene `LightSource` drives
both the material sheen and the shader’s narrow facet reflections; the long
starburst rays retain a shared orientation. Light depth and radius scale with
font size because scene lights measure those distances in world pixels. The facet phase stays zero, so the moving light alone triggers the
effect. The light begins moving with the letter entrance and crosses the word
in 2.2 seconds, finishing during the reading hold. Its clock freezes on pause
and while offscreen.
One 100px/em word artboard keeps spacing and capture padding proportional to
type size. Its 10px reflector spacing and seed 40 define a stable facet field;
`flare=5.0` extends the atmospheric rays and haze around those glints.
Recheck the full sweep when changing the font, word, or light path.
Native selection text stays outside the effect. The source drawer
includes Flashy's Rust and WGSL. This is a frosted surface treatment without
backdrop sampling or refraction. CPU retains the unlit gradient letters.
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
the ancestor Stacker's measured height. The fixed two-line signboard and its
normal Text nodes determine the paragraph's position.

## Free and open source / kinetic acrostic

`OpenSourceSection` follows the modularity section and precedes the final CTA.
It connects Pax's free-and-open-source identity with the project's
creative mission and closes with a link to the Pax repository. The footer links
to platform support and notes that APIs and capabilities evolve.
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
In light mode, successive words cycle through cyan, magenta, and amber. The colored face is retained on CPU; on GPU a single column-sized instance of the oscilloscope’s two-pass `LedGlow` softens a brighter rim during each hold. No Radiance pass runs in this mode.

In dark mode, the grayscale-to-white, unlit paints work on GPU and CPU. On GPU, the four
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
tests). Ordinary website builds run no tracing step; the generated outline data
contains paths, not font bytes. Native text still uses the
[bundled font assets](assets/fonts/README.md).

From the monorepo root, explicitly regenerate after changing the font, weight,
or the phrase list in `tools/trace-hero/src/main.rs`:

```sh
cargo run --manifest-path pax-website/tools/trace-hero/Cargo.toml
cargo run --manifest-path pax-website/tools/trace-hero/Cargo.toml -- --check
cargo test --manifest-path pax-website/tools/trace-hero/Cargo.toml
```

Commit `src/hero_outlines_data.rs` with the source change. It records the font
checksum, weight, and shaping versions, shares 19 unique cluster outlines
across the four phrases, and retains source ranges, glyph IDs, advances and
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
replays when it returns home from the nav.
The hero owns one playhead for the 1440ms reveal and Radiance lighting, including
the return replay. At 2100ms the light returns to (28, 233) in the
920.72 × 463.95 artwork and dims to 1.70. Its 12-second idle sweep reaches
(952, 177) halfway through the cycle. The nav twin uses the same path and brightness,
but its own visibility-gated clock, so it keeps panning while the hero is paused
offscreen. Each logo keeps its own Radiance capture through the portal transition,
with the light volume scaling alongside that instance.
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
