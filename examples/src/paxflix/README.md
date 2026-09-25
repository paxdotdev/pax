# Paxflix

A fictional cinema library built in Pax for PAX-1000. It contains **35 original
ImageGen films, 160 real cards, sixteen horizontal Scrollers, and one vertical page
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

Every visual component imports `CinemaTheme` through `<ImportSettings>`, passing
`is_dark={!light_mode}`. The root owns the `light_mode` Property; only the profile
menu receives a two-way binding to change it. The provider owns typography,
surface and control colors, outlines, backdrop opacity, and all gradient stops.
This includes cards, shelves, detail panels, artwork and the profile menu;
imports do not automatically reach inside child component templates. Switching
the provider's reactive property leaves the catalog and its scroll state mounted.
The selection lasts for the current app session and resets to dark on reload.

Each import opts into `SettingsTransition::Ease(400ms, TransitionCurve::InOutQuad)`.
The toggle is the PAX-1001 testbed: its thumb position, text colors, surfaces, outlines, and the
artwork's transparent gradients transition together. A subtle hero highlight
crossfades an offset cool radial gradient into a concentric warm one; their
210/260 radii are local logical pixels on both GPU and Piet. A second toggle during the
fade continues from the visible mixture. New cards or a newly opened panel use
the current theme immediately. The explicit menu/detail entrance and exit
timelines retain ownership of their animated properties. Gradient crossfading
is a paint operation and does not change the group-opacity limits below.

To check it, open the profile mark, switch Dark/Light mode in both directions,
then click again before the 400 ms fade finishes. Check the hero and card fades
against their surrounding surfaces, and open a film after switching to confirm
it begins in the selected theme. Repeat after scrolling a shelf and verify the
scroll position stays put.

`FilmArtwork` places the card thumbnail beneath its full-size still, keeping
the selected artwork visible while the larger image loads. A multi-stop linear
gradient fades into the active theme's surface at the bottom of both the hero
and detail image: black in dark mode, ivory in light mode. The detail variant
limits the fade to the bottom 30% of the artwork and reaches full opacity before
the image edge and heading. Portrait copy and actions use the bottom 320px
with 20px side insets. The surrounding surface continues the fade without a
hard edge. Short landscape details give artwork 40% of the panel, fade at its
right edge, and vertically center the copy/action block in the remaining space.
The original film images retain their colors.

```text
Fixed masthead: Paxflix / profile mark
  Profile popover: Dark/light mode / Manage profiles / Account / Help center
Vertical page Scroller
  Half-viewport hero: full-size still + title + summary + two actions
  Category title and previous/next controls
    Edge-to-edge horizontal Scroller: ten cards with text over a lower image fade
  …fifteen more independent shelves…
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

The hero is 52% of viewport height
within a 360–520px desktop range, and 354px on compact screens. Short landscape
windows use a side-by-side detail layout; shorter portrait windows allocate
more height to the detail copy. The whole detail component fades in over 320ms
and out over 220ms with `InOutQuad` opacity, alongside its short vertical motion.
The backdrop uses matching fade durations. Hover adds a fine accent outline and
detail indicator without changing the shelf geometry.

`catalog.json` owns stable film IDs, titles, years, runtimes, ratings, genres,
synopses and image paths. `src/catalog.rs` retains ten evenly distributed shelves
and adds six thematic collections with ten different films each: Worlds beyond
ours, Love and other detours, Keep you guessing, The great wide open, One more
chance, and Together, somehow. Every film appears at least twice, and the new
collections have distinct selections. Page height and footer placement use the
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
3. Use `dev inspect tree --session <id>` to verify 160 MovieCard components,
   163 Image nodes including the logo and both hero layers, and 17 ScrollerHost nodes with the
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

## Validation and remaining checks

- Catalog tests verify unique identities, all asset files, 160 placements,
  ten unique films per row, complete catalog coverage, contiguous row positions,
  and distinct selections for the six added collections.
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
