# Paxflix

A fictional cinema library built in Pax for PAX-1000. It contains **35 original
ImageGen films, 210 real cards, twenty-one horizontal Scrollers, and one vertical page
Scroller**. Clicking a card opens its full-resolution still, synopsis and local
Play/Download feedback. It has no media backend or real download action.
The package title is `Paxflix`, also used for the native home-screen label.

## Run

From this repository root, build the current CLI and run the example:

```sh
cargo build -p pax-cli
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli run \
  --path examples/src/paxflix --target web --libdev
```

Use the URL printed by the CLI. On this workstation, source `~/.zshrc` and run
`nvm use` first so the Node version in the repository's `.nvmrc` is active.
Alternatively, from this example directory, use `./pax run --target web`.
The default debug reload lane updates `.pax` templates. Rebuild after changing
Rust or `catalog.json`, which Rust includes at compile time.

For macOS, change the target to `macos`. To launch the native iPad simulator app:

```sh
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli run \
  --path examples/src/paxflix --target ipados --libdev
```

Select a particular installed iPad with `--ios-device "simulator:DEVICE_UDID"`.
Build the release-baked web cartridge:

```sh
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli build \
  --path examples/src/paxflix --target web --release --libdev
```

The output is `.pax/build/release/web` inside this example. Serve that directory
with a local HTTP server; opening its HTML as a file does not load Wasm correctly.

## Design and structure

The visual identity pairs Georgia film titles with neutral Arial labels and a
peach ImageGen wordmark. Dark mode starts by default; the profile menu switches to
warm ivory surfaces with dark text. All typography, metadata, gradients,
controls and rounded clips are authored in Pax; none is baked into the stills.
The navigation logo is a transparent PNG with the same peach lettering in both
modes.

Movie cards use viewport proximity events to request a 480 ms entrance timeline
at the first sampled positive viewport overlap, with no visible-area threshold.
The thumbnail stays fully opaque in its resting position beneath the stationary
bottom gradient. Only the text animates: the title starts 48 ms into the timeline
and the metadata line starts at about 125 ms; each slides 32px from right to left
while fading from 0 to 1 with `OutQuad`. The card itself does not fade or scale.
Its resting surface matches the page in both themes. A fixed outer wrapper
supplies the observation bounds independently of the text's motion.

Each shelf schedules its text entrances 80 ms apart, with at most 240 ms of
additional delay. Cards arriving in the same sampled frame are ordered by their
horizontal index. Later arrivals use the next available start, so crossings at
0/30/60 ms start at 0/80/160 ms, while crossings at 0/250/500 ms start immediately.
The delay is relative to recent arrivals, never the tile's absolute index. A
large burst shares the capped start time rather than building a long backlog.
The title/metadata stagger above is relative to each card's scheduled start.

The shelf passes an explicit `entrances` binding to its cards. Viewport handlers
enqueue arrivals; a shelf subscription orders the settled batch and publishes
start times before drawing. Each card then queues a hold followed by its existing
timeline, without per-card tick handlers or a waiting window to detect a burst.
Departing proximity or unmounting removes that card's reservation and cancels
its queued animation. Rows schedule independently.

The same entrance works for vertical page scrolling and horizontal shelves.
Cards have no exit transition; leaving the expanded proximity region resets
the playhead while offscreen. Reversing near a viewport edge keeps a revealed
card visible; returning after a full departure replays it. Artwork initialization
and caching remain independent of animation. Events are sampled once per engine
tick, so fast native scrolling may skip the exact first-pixel position. The
timeline authors the initial text position and opacity before any handler
runs; the handler advances its shared playhead linearly while each track supplies
its own easing and stagger.

Detail popups use the same 32px `OutQuad` slide-and-fade for their copy: the
eyebrow/title begin at 48 ms, metadata/genres at 128 ms, and synopsis at 208 ms.
Each stage takes 336 ms. The existing panel fade/slide finishes at 320 ms,
while the full entrance timeline finishes at 560 ms. Buttons stay together
with their labels. All buttons and clickable movie cards request the pointer
cursor on hover and restore the automatic cursor when the pointer leaves.

Every visual component imports `CinemaTheme` through `<ImportSettings>`, passing
`is_dark={!light_mode}`. The root owns the `light_mode` Property; only the profile
menu receives a two-way binding to change it. The provider owns typography,
surface and control colors, outlines, backdrop opacity, and all gradient stops.
This includes cards, shelves, detail panels, artwork and the profile menu;
imports do not automatically reach inside child component templates. Switching
the provider's reactive property leaves the catalog and its scroll state mounted.
The selection lasts for the current app session and resets to dark on reload.

Each import opts into `SettingsTransition::Ease(300ms, TransitionCurve::InOutQuad)`.
The toggle is the PAX-1001 testbed: its thumb position, text colors, surfaces, outlines, and the
artwork's transparent gradients transition together. A subtle hero highlight
crossfades an offset cool radial gradient into a concentric warm one; their
210/260 radii are local logical pixels on both GPU and Piet. A second toggle during the
fade continues from the visible mixture. New cards or a newly opened panel use
the current theme immediately. The explicit menu/detail entrance and exit
timelines retain ownership of their animated properties. Gradient crossfading
is a paint operation and does not change the group-opacity limits below.

Ordinary opacity now composes each canvas portion of the detail subtree before
fading it, on WGPU and browser Piet. Native text/controls and separate Scroller
surfaces still fade independently, so the mixed-content result remains an
approximation. The historical measurements below predate this group-compositing
change. See [subtree opacity](../../../pax-docs/book/src/compositing-effects.md#opacity-through-a-subtree)
for current behavior and backend costs.

The September 25 release comparison on Molino's M4 iPad Pro measures roughly
57–58% less CPU rasterization time for the large native occlusion masks with
alpha-only bitmaps (about 6.4 ms versus 15.1 ms per mask). Bitmap storage is one
quarter of RGBA. Cold artwork/rendering stalls remain; this is not a displayed
frame-rate claim. The [controlled catalog measurements](../../../pax-docs/book/src/design/isolated-compositing.md#catalog-scale-native-mask-measurements)
describe the workload, repeated runs, and limits.

To check it, open the profile mark, switch Dark/Light mode in both directions,
then click again before the 300 ms fade finishes. Check the hero and card fades
against their surrounding surfaces, and open a film after switching to confirm
it begins in the selected theme. Repeat after scrolling a shelf and verify the
scroll position stays put.

`FilmArtwork` places the card thumbnail beneath its full-size still, keeping
the selected artwork visible while the larger image loads. A multi-stop linear
gradient fades into the active theme's surface at the bottom of both the hero
and detail image: black in dark mode, ivory in light mode. The hero keeps its
upper 68% free of the bottom fade and reaches solid surface at 98%, before the
image edge. Its horizontal text scrim ends 720px beyond the left gutter (capped
at the viewport width), preserving clear artwork on wide screens. Compact hero
copy stays within the 282px action row and uses darker light-theme text so it
does not run across the exposed artwork. The detail variant
limits the fade to the bottom 30% of the artwork and reaches full opacity before
the image edge and heading. Portrait copy and actions use the bottom 320px
with 20px side insets. The surrounding surface continues the fade without a
hard edge. Short landscape details give artwork 40% of the panel, fade at its
right edge, and vertically center the copy/action block in the remaining space.
The original film images retain their colors.

The shorter hero fades were checked in the release web build at tablet and
phone widths in both themes, including the compact synopsis contrast. This
visual pass uses the existing still; it does not add a larger hero asset.

```text
Fixed masthead: Paxflix / profile mark
  Profile popover: Dark/light mode / Manage profiles / Account / Help center
Vertical page Scroller
  Half-viewport hero: full-size still + title + summary + two actions
  Category title and previous/next controls
    Edge-to-edge horizontal Scroller: ten cards with text over a lower image fade
  …twenty more independent shelves…
  Small closing credit
Root overlay, when selected
  Detail panel: same film, full-size still, synopsis, Play / Download / close
  Native EventBlocker backdrop, behind the panel and above the page
```

Desktop cards are 292px wide with 16px gaps and 48px page gutters. Below 700px,
cards are 236px wide and gutters become 20px. Card height is three quarters of
its width plus 36px, retaining more artwork above the text. A lower scrim fades
to fully opaque black or ivory over 39px, then stays solid behind the title and
metadata. Titles have room for two lines, and the hover indicator sits
in the upper-right corner. Shelf Scrollers span the viewport. Their content has
leading and trailing gutter space: the first card aligns with the heading at
zero scroll, while cards can cross either gutter as the row moves.

The logo's transparent source margin is cropped by a Pax Frame so the visible
lettering aligns with the same gutter. Profile controls and row arrows use an
equal right gutter. `DynamicIslandSpacer` binds the native safe-area insets;
the header adds its usual 68px below the measured top inset. Both side gutters
include the larger of the left/right insets, keeping fixed content symmetric
in landscape. The menu, page viewport and detail placement share these measured
margins; details and the end of the catalog also reserve the bottom safe area.
Web insets remain zero. Category rows still bleed to the viewport edges.

The profile menu measures its vertical stack with `autosize`, including twelve
pixels of padding. Demo feedback is mounted only when nonempty and wraps at its
intrinsic height, so the surface grows with the message. Its background uses
`LayoutRole::Breakout` to fill the measured bounds without influencing them.

The hero is 52% of viewport height
within a 360–520px desktop range, and 354px on compact screens. Short landscape
windows use a side-by-side detail layout; shorter portrait windows allocate
more height to the detail copy. The whole detail component fades in over 320ms
and out over 220ms with `InOutQuad` opacity, alongside its short vertical motion.
The backdrop uses matching fade durations. Hover adds a fine accent outline and
detail indicator without changing the shelf geometry.

`catalog.json` owns stable film IDs, titles, years, runtimes, ratings, genres,
synopses and image paths. `src/catalog.rs` retains ten evenly distributed shelves
and adds eleven thematic collections with ten different films each. The latest
five are Turn up the volume, Far from familiar, Small moments, big stories,
Under cover of night, and Take the scenic route. Every film appears at least
twice, and the curated collections have distinct selections. Page height and footer placement use the
actual shelf count. This is a curated fixture, not a recommendation system.
Shelf arrow controls ease their bound native
scroll position by two cards over 360ms with `OutQuad`, clamped at either end.
A new arrow press retargets from the current position; wheel or touch input
cancels the pending animation. Opening a modal changes only selection and modal
state; the page and shelves remain mounted. Close, Escape and backdrop clicks
dismiss it. Keyboard defaults are cancelled while the modal is open.
The profile popover also closes with Escape, its close control, another profile
icon click, or an outside click. The appearance switch changes the live theme;
the three other entries only update local feedback.

## Asset inventory

All 35 stills were generated separately with the built-in ImageGen tool and
inspected individually/as a contact-sheet overview. Each film has:

- `assets/stills/<slug>.jpg`: the full generated 1672×941 image, saved as a
  quality-90 JPEG without resizing.
- `assets/thumbs/<slug>-thumb.jpg`: a 640×360 quality-82 JPEG derivative.
- An entry in `assets/prompts.json` with its generation prompt (catalog branding
  normalized to Paxflix), and in
  `catalog.json` with the consuming paths and fictional metadata.

`assets/paxflix-logo.png` is the 2172×724 ImageGen wordmark, retaining its alpha
channel and peach color. Its original and refinement prompts are also recorded
in `assets/prompts.json`.

The complete asset directory is approximately 24.5 MB. All repeated appearances
use the identical thumbnail path. Only the hero and selected modal request
full-size images. Engine caches and tiled surfaces still determine actual GPU
residency; a shared filename does not imply a single texture across all surfaces.

Thumbnail basenames deliberately differ from full-size basenames. Apple's
Swift resource processing flattens names and rejects `stills/film.jpg` plus
`thumbs/film.jpg` as duplicate resources. No external stock art, remote fonts,
API credentials or image-generation service is needed to run the finished app.

## Reproducible inspection scenario

1. Start a debug web session and open its URL with `?pax_log=trace` for a bounded
   profiling pass. Wait for asynchronous image loading to finish.
2. Read `target/debug/pax-cli dev status --path examples/src/paxflix` and
   copy the **web** session ID. Use `--session <id>` for subsequent commands,
   especially if a native session is also running.
3. Use `dev inspect tree --session <id>` to verify 210 MovieCard components,
   213 Image nodes including the logo and both hero layers, and 22 ScrollerHost nodes with the
   modal closed. `for` expands the full collection; it is not list virtualization.
4. Move the first shelf from start to end and back, then jump to the bottom of
   the page and return. Repeat three times, alternating axes quickly. Test real
   diagonal trackpad gestures separately; sequential automation is not that test.
5. Open several films after moving their row offscreen and back. Check identity,
   the larger still, Play/Download feedback, close button, Escape, and backdrop.
   Try wheel input and Page Down while open; compare every scroll offset before
   opening and after closing. Only the intended shelf should move horizontally.
   Check that row arrows produce intermediate offsets in both directions, and
   that the profile menu's demo entries and all dismissal controls work.
   Switch themes with nonzero page and shelf offsets, confirm those offsets
   stay unchanged, then inspect detail text, fades, controls and hover states
   in both modes. Observe intermediate opacity on popup entry and dismissal.
6. During the bounded run collect `dev logs --session <id> --follow --limit 2000`.
   Inspect `[pax-tile-cull]`, `[pax-render-filter]`, and `[pax-gpu-resources]` rather
   than inferring performance from appearance. Turn trace logging off afterward.
7. Repeat at 390×844 and 844×390. Capture stills with
   `dev look --session <id> --output-dir /tmp/paxflix-captures`.

All cards remain in the expanded tree, so initialization and property work
scale with the fixture. Culling bounds drawing and surface work; this example
makes no universal FPS or total GPU-memory claim.

### Recorded web inspection

Before the theme toggle was added on September 23, 2026, the example contained
1,103 expanded nodes,
including 102 Images (100 cards plus two hero layers), 100 Frames and 11
ScrollerHosts. After portrait/landscape checks, three start→end→start shelf/page
scroll cycles at 1280×720 each returned to 64 canvas surfaces and 15,608,829
backing pixels. These are DOM canvas measurements, not total GPU memory or
decoded image residency.

Representative warm shelf samples considered 26–27 retained nodes and drew 24
across two surfaces, with zero texture creates/upload bytes, zero vector
resource creates and zero geometry cache misses. Startup allocation is larger,
and images can occupy multiple surfaces. The resize/modal stress trace also
reported `pax-tile-window-escape` when viewport bounds changed and
`pax-render-filter fallback=missing_dirty_node` afterward. No blank tiles were
seen in the checked final views, but these diagnostics need a separate renderer
investigation; the bounded surface counts do not establish that every culling
path is optimal. No engine changes are included here.

Input verification preserved both page and shelf offsets through modal
interaction and dismissal. The updated release was also checked by opening
North of Nowhere, reading demo profile feedback, dismissing the menu through
Escape/outside/profile clicks, and sampling intermediate scroll offsets in
both arrow directions. Use a fresh session for reproduction rather than a long
template-editing session with accumulated hot-reload state.

### Recorded Argus transition profiling

On September 24, 2026, a release build on the physical iPhone 17 Pro Max
returned to roughly 120 display-link callbacks per second when idle, with
about 4ms of engine/native/render CPU work. Repeated detail opening and closing
produced real callback gaps around 60–140ms, in addition to the visible
per-descendant opacity compositing effect. These are callback/CPU measurements,
not GPU timestamps or a count of displayed frames. The one-second windows
include idle time and should not be read as transition-only FPS.

A temporary probe around native mask rasterization recorded 139 calls above
4ms at each of two sizes: 1320×2478 and 1320×2868 pixels, with eight cutout paths
each. Their median raster times were 18.50ms and 18.55ms. During that pass the
native-update phase peaked at 65.3ms and the render call at 48.1ms, excluding
startup. The mask pair alone regularly exceeds a 60Hz frame budget. Changing
opacity and geometry changes mask signatures, so the existing raster cache
cannot reuse those intermediate images.

The temporary probe was removed afterward. Optimizing native mask generation
while preserving synchronous presentation is a separate follow-up from
[PAX-1004's isolated group compositing](https://linear.app/paxdev/issue/PAX-1004/add-isolated-group-compositing-for-mixed-nativecanvas-content).
A translation-only transition would avoid translucent descendant stacking,
but moving occlusion can still regenerate masks; it is not yet a measured
performance fix. The example keeps the existing fade/motion for comparison.

### Molino CPU baseline after automatic theme transitions

On September 25, 2026, the release at `38c996c0a` was installed on Molino (2),
an iPad Pro 13-inch (M4) running iPadOS 26.6.2. A five-minute Time Profiler
capture, with `PAX_IOS_FRAME_INSTRUMENTATION=1`, was predominantly idle. Settled
windows reported 120 display-link callbacks per second and approximately 5.2ms
of CPU frame work, including about 4.6ms in `pax_tick`.

`sync_imported_settings` accounted for approximately 86% of sampled main-thread
CPU time in that capture. It collects and deep-clones provider settings before
checking whether the provider signature changed, then discards those temporary
settings on unchanged frames. This is an independent source of allocation and
CPU overhead; the capture does not establish when that overhead was introduced
or how much a proposed optimization would save.

A brief interaction burst produced a 115.11ms maximum callback gap, with native
update and render-call maxima of 61.22ms and 32.59ms respectively. In the
five-second region around it, native mask rasterization accounted for 301ms of
452ms sampled native-update CPU time. The interaction has not yet been labeled
as a theme change, popup, or scroll, so these numbers must not be attributed to
one of those actions. This Time Profiler configuration excludes waiting-thread
samples and cannot explain render-call wall time spent waiting on the GPU or
driver. Separate labeled captures of theme changes, detail entry/exit, and
scrolling remain necessary; scrolling flashes also require visual correlation.

A subsequent three-minute capture on the same release separated user-confirmed
slow/rapid theme toggles from repeated detail opening and closing:

| Phase | Maximum callback gap | Maximum engine tick | Maximum native update | Maximum render call |
| --- | ---: | ---: | ---: | ---: |
| Theme toggles | 94.95ms | 23.43ms | 25.61ms | 35.15ms |
| Detail open/close | 110.37ms | 19.06ms | 62.04ms | 27.35ms |

These are individual phase maxima, not simultaneous values to add together.
The theme phase includes opening Profile; both windows include idle time.
Mask rasterization represented about 17% of sampled native-update CPU time in
the theme window, versus 69% in the detail window. Native text application and
content-signature calculation were larger contributors during theme changes.
`collect_scene_lighting_for_layer` represented about 35–38% of sampled
render-call CPU time in these windows. These samples identify CPU work, not the
unmeasured waiting portion of render-call wall time. A separate scrolling pass
had uncertain start timing. Its retry, synchronized with Instruments' recording-
started notification, was invalidated by a concurrent PAX-1004 deployment to
Molino: a different Paxflix bundle relaunched and ran an automated popup cycle
while the profiler remained attached to the original process. Neither scrolling
pass establishes the cause of the reported flashing. Future hardware captures
need exclusive device use across worktrees and confirmation of the foreground
bundle/process before attributing input to a trace.

Imported settings discovery now collects provider identities and lightweight
references first, compares the ordered signature, and materializes settings/scope
layers only when it changes. Provider-property changes continue through existing
reactive bindings. A regression test that previously made 101 settings copies
across initial discovery and 100 unchanged passes now makes just the initial
copy. Coverage also checks provider order, replacement/removal, transition-policy
changes, and receiver rebinding with a retained provider. Physical frame timing
has not been remeasured after this fix.

After the settings fix, all 182 runtime tests pass in both debug and release,
and the designtime library check passes. The optional designtime unit-test suite
cannot compile because existing fixtures omit the feature's extra globals and
context arguments. No manifest or baked-program format changes are involved.
The documentation book builds.

The updated release-baked web build passes palette switching, quick reversal,
and detail creation in the selected theme, with no browser errors. Earlier web
validation also checked retention of the visible page position through a theme
switch. These are functional checks, not a web frame-rate measurement. Argus was
unavailable during the profiling pass.

### September 25 scrolling check with 21 shelves

The release with 210 cards, the autosized profile menu, the settings-copy fix,
and visible-tile replay grouping ran on Molino (2). Frame-phase logging was
enabled only for this manual pass; there were no automated device gestures.
The user confirmed vertical scrolling, then horizontal row swipes separately,
and described the horizontal pass as qualitatively good. Startup was excluded.
The intervals between cues and replies also contain idle time, so their average
callback rates are not scroll-only FPS.

| Confirmed pass | Largest callback gap | Largest CPU frame | Render step peak | Native-update peak |
| --- | ---: | ---: | ---: | ---: |
| Vertical | 76.20ms | 60.44ms | 33.49ms | 21.33ms |
| Horizontal | 65.30ms | 50.20ms | 23.23ms | 16.94ms |

Settled callbacks returned to approximately 120/sec, with an 8.33ms budget.
These measurements establish real frame-production stalls, not displayed-frame
counts or proof that all visual tearing is gone. Phase maxima can belong to
different frames and must not be summed. Time in the render call includes any
GPU waits; attributing it to tile preparation or a particular draw operation
requires a deeper trace. The previous copy reduction and tile-ordering fix do
not eliminate this remaining work. Molino was returned to a normal launch with
frame instrumentation disabled after saving both passes.

### September 25 lighting cost reduction on the iPad simulator

The 21-row release was compared before and after two shared-renderer fixes on
the iPad Pro 13-inch (M5), iOS 26.4 simulator. `PAX_RENDER_TIMINGS=1` separates
lighting collection/delivery from retained preparation, encoding, submission,
presentation, and layer initialization. Startup windows were excluded. Theme
toggles and row paging were exercised in both builds; details were also checked
in the optimized build. These interaction samples have different operation
counts and are not a controlled comparison of overall frame rate.

| Operation | Before, mean per layer | After, mean per layer |
| --- | ---: | ---: |
| Lighting collection | 0.378ms | 0.00014ms |
| Lighting delivery to renderer | 0.072ms | 0.00060ms |

After the rebase, the collector uses main's registered lighting contributors and
cached scope memberships, preserving live enabled/property/scope resolution and
storing only nonzero light memberships.
Paxflix has no lighting providers, so its collection path avoids scene scans.
Separately, each GPU renderer skips identical lighting values, avoiding both a
uniform upload and unnecessary retained-scene invalidation. New renderers still
receive their first lighting value. Custom Rust lighting adapters must opt in
with `InstanceNode::has_scene_lighting()`; templates and the baked program
format are unchanged.

The remaining simulator interaction time is largely encoding and presentation
waits, with retained preparation also measurable. No layer initialization was
recorded in the sampled theme/paging interactions; tile-set churn still needs a
confirmed new-row scrolling capture. Simulator gesture driving did not produce
a usable vertical pass, so these numbers do not establish an improvement to
Molino's vertical-scroll judder. Transient native-text visibility issues were
also visible in the baseline; this pass does not fix group compositing.

Validation: 187 runtime tests and 17 ordinary GPU tests pass in debug and
release. The new focused Metal test passes in both modes and fails against the
old unconditional redraw behavior; other opt-in hardware tests were not run.
The no-light collector regression also fails against the old scene scanner.
The designtime library check, native release build, web release build, API
regeneration, and documentation build pass. Web theme switching, details, and
scrolling were checked without browser errors. The optimized simulator app is
left running with both profiling flags disabled. Molino was not used.

### September 25 tile eviction probe on the iPad simulator

A temporary, opt-in `on_tick` probe drove the existing `scroll_y` binding through
prepared content (0–160px and back), then 0–5800px, back to zero, and down again.
The long passes lasted 12 seconds each, with idle gaps. Screenshots confirmed
vertical movement and populated later rows. This exercises programmatic native
scrolling and surface lifetime; it does not reproduce a finger gesture or establish
Molino's frame rate. Automated simulator drags did not produce usable scrolling.
The temporary handler was removed from source and the final installed build.

Native `PAX_RENDER_TIMINGS` now includes tile creation, matching-tile discard,
origin/size reset, and survivor-reuse counters. The control and final probe used
the same app, simulator, and trajectory. Counts below cover the whole post-startup
sequence, including idle gaps; they are not frames-per-second measurements.

| Operation | Control | Preserve survivors on eviction |
| --- | ---: | ---: |
| Physical renderers created | 66 | 62 |
| Matching key/host renderers discarded | 12 | 8 |
| Renderers explicitly reused | 0 | 5 |
| Layer initialization calls (including empty targets) | 102 | 54 |
| Total layer initialization wall time | 692ms | 554ms |
| Largest layer initialization span | 35.5ms | 25.5ms |

Threshold crossings can differ slightly with frame cadence (102 versus 103
surface-set changes). Timing variation and simulator presentation waits prevent
interpreting these samples as a general percentage speedup. Prepared-content
scrolling created no renderers and encoded no GPU frames, although the bound
property path still spent roughly 2ms per active render on retained preparation.

The GPU renderer now preserves surviving key/host renderers when the tile set
shrinks or reorders, evicts removed resources immediately, and invalidates old
index-based replay/dirty scopes. Survivors replay together, including any pending
warm work; origin/size changes retain their existing invalidation behavior. Adding
a surface or replacing its host still initializes a new logical-layer target.
Incremental creation on growth remains the next resource-lifetime opportunity.
The memory retention region, synchronous visible-tile policy, and app image
initialization behavior are unchanged.

A separate experiment shared one GPU device/pipeline context across all native
layers. Setup became modestly cheaper, but presentation waits remained noisy and
sometimes longer. A repeat control did not justify keeping it, so that code and
its experiment-specific test were removed. No context-sharing optimization ships
in this iteration.

Validation: the final runtime tests cover key/host matching, rejection of new or
rebound surfaces, survivor resource identity/order, and release of evicted resources.
All 190 runtime and 17 ordinary GPU tests pass in debug and release (9 opt-in
GPU hardware tests are excluded from those suites). Release simulator/web builds
and the docs build pass. The normal simulator build was checked for profile/theme
interaction, row paging, and detail open/close; web scrolling and detail opening
produced no console warnings/errors. The final simulator launch has profiling
disabled. No public API or baked program format changed; no API reference
regeneration was required for the eviction fix. Molino was not used.

### September 27 incremental tile creation

The GPU chassis factory now creates only missing tile surfaces, using the
surviving logical layer's GPU context. Existing renderers and their scenes stay
ready during creation; native view and browser canvas generations prevent reuse
across physical surface replacement. Pending replay and dirty indices follow
survivors through reordering. New/retargeted/resized tiles receive targeted replay,
including current lighting, and clean neighbors are not encoded simply because a
tile was added. Web surface notifications no longer force a whole-layer redraw.
Failures leave survivors usable and retry when the layout changes. Obsolete
asynchronous completions and recycled layer slots are rejected.

The September 25 scripted trajectory was rebuilt and rerun on the same dedicated
13-inch iPad simulator for both the eviction-only control and incremental path.
Both builds used the same per-view generation registration. The table sums 51
one-second diagnostic windows after startup through the completed trajectory,
including its idle gaps; this is programmatic scrolling, not finger-scroll FPS.

| Operation | Eviction-only control | Incremental additions |
| --- | ---: | ---: |
| Physical renderers created | 62 | 54 |
| Matching key/host renderers discarded | 8 | 0 |
| Tile encodes | 67 | 54 |
| Layer creation calls | 54 | 54 |
| Total layer creation wall time | 567ms | 509ms |
| Largest layer creation span | 32.2ms | 12.5ms |
| Total retained preparation time | 4253ms | 1881ms |
| Total encoding time | 633ms | 614ms |
| Total presentation time | 633ms | 533ms |

This eliminates the measured redundant tile replacements and reduces replay
work. It does not eliminate first-draw/upload costs or prove a physical-device
frame-rate improvement. Presentation timing remains noisy. The count of
initialization calls stays the same because additions still call the factory,
now for fewer surfaces. Set-change/reuse counters have a narrower reconciliation
boundary than in the control and are not compared as equivalent event counts.
The incremental path also records scene resets on reactivation of previously empty
layers; those counts include fresh surfaces and do not imply lost survivor scenes.

Validation includes a real Metal regression for `[A, B] → [B, C]`: B is not
encoded/reset/reuploaded, C shares its context and paints the expected pixel.
It also covers reorder, targeted DPR/size invalidation, failed additions, stale
creation across surface replacement, and release of the shared context when the
layer empties. Ordinary tests cover layer-slot lifetime reuse and pending warm
replay remapping. Both debug and release suites pass: 193 runtime tests and 17
ordinary GPU tests, plus the separately enabled Metal regression in both modes.
The wasm check and release simulator/web builds pass. Web checks cover vertical
scrolling to the final shelves and back, horizontal travel in both directions,
detail open/close, and newly hydrated tiles after switching to light mode, without
console warnings/errors. Simulator checks cover populated lower shelves, profile
and theme controls, and row paging. The final simulator build has no scroll probe
and profiling is disabled. Molino was not used.

Canonical scrolling/profiling guidance and the internal API references are
updated, and the documentation book builds. There are no template, manifest, or
release-baked format changes. The internal GPU factory now accepts a `LayerCreationRequest`; chassis integrations
must filter surviving identities and create additions with the supplied context.
The Piet fallback retains its existing tile-set rebuild behavior.

### September 27 physical Molino scrolling capture after incremental creation

The same release was deployed to Molino (2), then relaunched with
`PAX_IOS_FRAME_INSTRUMENTATION=1` and `PAX_RENDER_TIMINGS=1`. The user manually
performed mixed short and full-page vertical scrolls, with at least one horizontal
swipe. No automated gestures or application probes ran. The full recording contains
a roughly 31-second activity burst, identified by tile churn, between settled
intervals. The separately written cue marker lagged that activity, so analysis
uses the full capture rather than treating the marker as its start. Individual
directions and first/return trips cannot be assigned from this sample.

| Measurement in the activity burst | Observed |
| --- | ---: |
| Largest display-link timestamp gap | 68.88ms |
| Largest measured frame body | 55.76ms |
| Frame bodies exceeding the reported 8.33ms budget | 227 / 3,528 callbacks |
| Native message processing peak | 19.84ms |
| Render-call peak | 35.54ms |
| Individual tile initialization peak | 4.08ms |
| Individual tile encode peak | 3.47ms |
| Individual tile presentation peak | 12.79ms |

These are CPU wall times, including waits, and callback measurements, not GPU
timestamps or a count of missed displayed frames. The two instrumentation streams
have independent one-second windows; phase maxima must not be added or assumed
to describe the same frame. Startup and the long settled tail are excluded.

The burst created 188 physical tile renderers, with zero matching-survivor
discards. Across 423 tile presentation calls, presentation consumed 1,402ms of
1,973ms measured flush time (71%); encoding consumed 471ms. This makes presentation
synchronization a stronger immediate investigation target than assuming image
upload or tile initialization is the dominant stall.

The native surface uses `presentsWithTransaction = true` to keep GPU artwork and
native text in the same Core Animation commit. The installed wgpu-hal 28.0.1 Metal
backend commits a presentation command buffer and calls `wait_until_scheduled`
for each such presentation. The timing includes this path but does not isolate
the wait from drawable presentation. Investigate batching synchronization while
preserving the shared commit, and reducing optional offscreen presentation bursts;
disabling transaction synchronization would risk reintroducing visual judder.
Native-message spikes need finer attribution as a separate cost. The settled
tail also spends about 1.7ms per render in retained preparation despite no tile
encodes, a secondary invalidation/preparation opportunity.

Raw capture and analysis were saved under `/tmp/paxflix-molino-scroll-20260927*`.
Molino was returned to a normal launch with both timing flags disabled. This pass
changes no renderer behavior and is not a controlled before/after comparison.

### September 28 shared image textures across sibling tiles

GPU contexts now weakly index immutable image textures by identity, version, and
pixel dimensions. Sibling tiles reuse those pixels while retaining independent
uniform bindings, transforms, clipping, and opacity. Unreferenced textures are
not kept warm by this cache. Image replacement can leave old and new versions
alive independently until their tiles release them. Switching a tile from images
to the vector-only batching path also releases its old image bindings.

The incremental renderer's vertical probe was repeated before/after on the
dedicated iPad Pro 13-inch (M5) simulator, with upload counters added to both
builds. Over the same 51 post-startup one-second windows, both created 54 tiles
and uploaded 324 image textures (303,970,208 bytes), with zero shared-cache hits.
That path does not expose simultaneous image reuse within a shared GPU context;
different shelves have separate contexts. Timing differences are not evidence
of an upload-related improvement. This change has not demonstrated a reduction
in Paxflix's vertical scrolling stalls.

A separate horizontal probe moved the first shelf 0–2,000px and back. Its one
new tile uploaded three textures and reused one already resident image. The hit
counter establishes one avoided upload; this was not a paired timing comparison.
The Metal regression also verifies that a new tile reuses a surviving sibling's
image without uploading again. Broader sharing across logical-layer contexts and
retention of unused images are deliberately outside this change.

Validation: 193 runtime and 17 ordinary GPU tests pass in debug and release. The
new Metal test and updated incremental-tile Metal test pass separately in both
modes, covering distinct bindings, clipping/opacity, image versions and extents,
context isolation, eviction, and final-reference release. Native simulator and
web release builds are checked, along with regenerated API documentation and the
book. Temporary probes are removed from the final app; profiling is disabled.
The simulator launch is verified. Visual UI verification is blocked by the
locked Mac, so this pass does not claim a visual or physical-device frame-rate
improvement. Molino was not used. No template/manifest/baked-format changes are
needed for this renderer resource-ownership change.

Capture files and analysis are in `/tmp/paxflix-texture-sharing/`. Native-update
profiling is deferred until the performance work incorporating PAX-1002 lands.
Presentation scheduling is unchanged; these results still leave its repeated
synchronization as a separate investigation.

## Validation and remaining checks

- The latest 21-row release is installed and launched on Molino (2). Catalog
  tests verify all 210 placements. Web checks cover the autosized menu in both
  themes, growth around wrapped feedback, dismissal, and the new final shelf
  and footer. Release web/iPadOS builds and the docs build pass.

- A September 25 scroll investigation reproduced a shared GPU scheduling defect:
  retargeted visible tiles were split into directional replay batches across
  frames. Visible tiles now repaint together; offscreen warm tiles keep their
  deferred ordering. All 184 runtime tests pass in debug and release, and the
  release web build and docs book build pass. Web checks exercise both scroll
  axes and return travel without console errors, but one immediate capture
  still shows transient missing native text before settling. This is not a
  claim that all scrolling artifacts are resolved. The updated release is
  installed and running on Molino (2), with its new process verified; physical
  scroll comparison followed in the 21-row pass described above. Attribution
  within the render step remains pending. No device taps or scrolling were automated.
- The September 25 release rebuild with the imported-settings copy fix, 300ms
  theme transitions, and shorter hero fades is installed and launched on
  Molino (2). The running process matches the newly installed app container;
  its home-screen name is `Paxflix`. This verifies deployment and startup, not
  a new frame-timing measurement. No automated taps accompany this launch.
- Catalog tests verify unique identities, all asset files, 210 placements,
  ten unique films per row, complete catalog coverage, contiguous row positions,
  and distinct selections for the eleven curated collections.
- Debug and release-baked web builds pass. Browser verification covers desktop,
  compact portrait, short landscape, horizontal/vertical scrolling, repeated
  modal selection and dismissal, backdrop input blocking, profile menu feedback
  and dismissal, and animated row paging.
- The iPadOS app builds, installs and launches on the iPad Pro 11-inch (M5)
  simulator running iOS 26.4. The September 24 rebuild visually confirms the
  card overlays, measured header spacing, and restored Georgia/Arial font
  selection. Profile theme toggles in both directions, close and outside
  dismissal, detail opening/closing, and row-arrow paging were checked after
  fixing the shared native EventBlocker touch forwarding. Automated drags and
  wheel input did not move the page even in a fresh session before opening any
  menu, so swipe scrolling still needs manual verification. Hardware touch and
  diagonal trackpad gestures remain unverified. The earlier
  macOS build passed, but its initial blank window needs separate inspection.
- The later September 24 pass removes the emoji-selecting Play glyph, moves
  the detail heading below a shorter opaque-ended image fade, and checks both
  themes in the iPad simulator and release web preview, including compact and
  short-landscape layouts. Static native text refuses implicit Core Animation
  actions. iOS native-tree publication is synchronous with the engine frame and
  Metal presentation joins that frame's Core Animation transaction. Native leaf
  and Scroller occlusion masks now resolve synchronously in that update, using
  the raster cache and unchanged-mask fast path. The example retains its 320ms
  entrance and 220ms exit timelines. Raster cache misses now contribute to the
  frame update; this pass does not make a frame-rate or profiling claim.
- The synchronous-mask release, including all sixteen shelves, is installed and
  running on Molino (2), with its new process verified after launch. Physical
  popup transitions are ready for visual evaluation; Argus deployment is deferred.
- A subsequent shared-renderer fix makes Image honor inherited opacity and
  invalidate on opacity-only changes, without reuploading its texture. Primitive
  regression, Metal pixel tests (source alpha, clipping and retained fades),
  shader validation and retained-transition tests pass. Debug and release web
  builds passed; the release is also installed and launched on Molino (2).
  The popup retains its normal 320ms entrance and 220ms dismissal.
  Slowed web captures over shelf rows still expose a separate native/canvas
  blending limitation: artwork fades differently over native Scroller surfaces
  than over their gutters. Isolated group compositing is deferred to
  [PAX-1004](https://linear.app/paxdev/issue/PAX-1004/add-isolated-group-compositing-for-mixed-nativecanvas-content),
  a low-priority Pax Core backlog item; Paxflix keeps the current transitions
  and accepts the remaining optical staggering. Synchronous masks and correct
  Image opacity alone do not provide isolated group compositing. See
  [coverage limits](../../../pax-docs/book/src/compositing-effects.md#coverage-has-limits).
- Updated standalone release builds installed and launched on Argus (iPhone)
  and Molino (2) (iPad). Argus's running process was verified; Molino became
  unavailable before a separate process check. Interaction and animation checks
  above were performed on the simulator/web.
- The subsequent card pass shortens the fade and lowers the text, verified in
  both themes on the iPhone 17 Pro Max simulator. That simulator survived both
  landscape orientations and returned to portrait. Argus's earlier console
  reports termination by signal 9, but the device's crash-report listing has no
  corresponding Pax or recent jetsam report. The physical rotation failure's
  cause is not established; simulator survival does not verify that hardware bug.
- All 17 Swift package tests pass, including six native touch-sequence
  regressions, presentation/action regressions and the incoming family-only font tests. The docs book builds.
- The later Argus startup failure is confirmed by device Console logs as
  `jetsam / per-process-limit`. The shared iOS surface allocator now releases
  distant GPU backing surfaces using presented ancestor clips, preserving all
  sixteen shelves and their native state. A diagnostic release measured about
  773 MiB after settling, compared with 3,242 MiB during the failing startup.
  The iPhone simulator starts and survives rotation; all 21 Swift tests pass.
  This bounds backing-surface allocation, not image decoding or application
  lifecycle work. PAX-1002 remains the separate path to deferred image loading.
- A scrolling detail panel was investigated for short windows. On web, a root
  EventBlocker intercepted wheel input above the panel's nested Scroller even
  though that Scroller rendered above it. The delivered panel uses responsive
  geometry and contains no Scroller. That web layering finding is recorded in
  the authoring pain points; the iOS touch-forwarding fix addresses a separate
  native input problem.

Run focused tests with:

```sh
cargo test --manifest-path examples/src/paxflix/Cargo.toml --lib
```

Validation recordings are temporary files outside the repository. No publication
or new commits are part of this iteration.

### September 28–29 presentation scheduling experiments

The mask-pipeline-sharing experiment was reverted: its matched simulator run
showed no useful reduction in tile initialization or overall render time. Keeping
empty logical-layer GPU contexts was also rejected earlier because its settled
simulator footprint grew from about 336MB to 493MB without reducing initialization
time. Neither experiment remains in the renderer.

A second trial submitted every dirty logical layer before presenting any tiles,
while keeping presentation inside the same native Core Animation transaction.
Its Metal regression verified submission ordering, both tiles of two independent
layers, captured pixel colors, immediate single-layer flush, and clean/inactive/
failed/pending layers. All 234 ordinary and hardware tests passed in both debug
and release before this trial was reverted.

Four simulator runs traversed the same catalog down/up twice with a temporary
application probe, using control–trial–trial–control order. Each created 78 tiles
and uploaded 466 image textures (440,209,216 cumulative bytes). Presentation wall
time was 4.66s and 3.71s in the controls, versus 6.53s and 6.23s in the trials.
Large intermittent host stalls, unequal callback counts, and adjacent one-second
instrumentation windows prevent a clean estimate of the regression; there is no
evidence of a benefit. These are simulator CPU wall times, not device frame-drop
counts. One recorder hit its startup allowance; its flushed log contains the
completed 48-second probe sequence. The final control waited for probe completion.

The submit-first trial is reverted too. wgpu-hal 28.0.1 still creates and waits
for a new presentation command buffer per transactional drawable. Eliminating
those repeated waits requires explicit batched presentation in the dependency,
with correct acquired-texture ownership and one scheduling barrier per queue
batch. Disabling `presentsWithTransaction` would undo the native/GPU synchronization
fix. No dependency fork is introduced by these experiments.

Logs, measurements, the reverted trial patch, and a scoped dependency proposal
are under `/tmp/paxflix-presentation-20260928/`. The ordinary Paxflix source has
no automatic scrolling probe. These trials used the simulator, not Molino.

Further dependency work is deferred to [PAX-1009](https://linear.app/paxdev/issue/PAX-1009/reduce-repeated-transactional-metal-presentation-waits-in-wgpu) in the Pax Core backlog. Both experiments remain reverted. The normal web and iPadOS simulator release builds and the documentation book build pass. No public API or release-baked format change remains.
