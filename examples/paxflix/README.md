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
  --path examples/paxflix --target web --libdev
```

Use the URL printed by the CLI. On this workstation, source `~/.zshrc` and run
`nvm use` first so the Node version in the repository's `.nvmrc` is active.
Alternatively, from this example directory, use `./pax run --target web`.
The default debug reload lane updates `.pax` templates. Rebuild after changing
Rust or `catalog.json`, which Rust includes at compile time.

For macOS, change the target to `macos`. To launch the native iPad simulator app:

```sh
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli run \
  --path examples/paxflix --target ipados --libdev
```

Select a particular installed iPad with `--ios-device "simulator:DEVICE_UDID"`.
Build the release-baked web cartridge:

```sh
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli build \
  --path examples/paxflix --target web --release --libdev
```

The output is `.pax/build/release/web` inside this example. Serve that directory
with a local HTTP server; opening its HTML as a file does not load Wasm correctly.

## Design and structure

Titles use Georgia and UI text uses Arial. These system font families preserve
the demo's typography without web-font downloads; platforms without them use
their system fallback.

### Embedding the live app

`Paxflix` can also be imported as a component through a Cargo path dependency.
Its responsive layout, theme typography, and detail dialogs use its own logical
bounds, not the outer browser window. Each instance owns its catalog selection,
scroll positions, open dialogs, and appearance independently.

```pax
<Paxflix width=100% height=100% embedded=true top_inset=48 safe_bottom=24
    disable_keyboard=true disable_card_entrances=true/>
```

`embedded=true` leaves safe-area values to the host instead of mounting native
`DynamicIslandSpacer`s. Set `disable_keyboard=true` when multiple instances share
a page: Pax keyboard events are global, so this leaves Escape and other keys to
the host; each instance's close and backdrop controls still work. Standalone
defaults and native safe-area behavior are unchanged.

`disable_card_entrances=true` shows titles and metadata immediately at their final
position and opacity. The optional per-card proximity observers and per-shelf
entrance scheduling subscriptions are not mounted in this mode. It does not
disable scrolling, hover, dialogs, or image preparation. The default is `false`,
so the standalone example retains its staggered text entrances. Changing the
option dynamically mounts/unmounts only those animation helpers, not the cards.

Each instance mounts all 210 card trees with its shelves. Scroll preparation
only governs rendering resources, not application lifecycle; there is no
first-visit row-mount gate. Two embedded instances therefore mount 420 cards.
This intentionally separates startup cost from tile and image readiness during
scrolling. In animated mode, helper components own the mount subscriptions and
proximity bindings for scheduled text entrances.

The website mounts one live instance in a scaled tablet frame on desktop, with
card entrances disabled. The wing devices and all compact-layout devices use
linked screenshots. The live instance stays mounted as the page scrolls; it does
not defer application lifecycle work until viewport approach. This adds startup
work and is not a runtime lazy-init implementation. See the
[website showcase](../../pax-website/README.md#paxflix-showcase) for its current layout.
The website imports the source directly and links
`pax-website/assets` to this example's assets, so artwork is not duplicated in
source control. The built web output contains ordinary copied assets.

### Visual treatment

The visual identity pairs Georgia film titles with neutral Arial labels and a
peach ImageGen wordmark. Dark mode starts by default; the profile menu switches to
warm ivory surfaces with dark text. All typography, metadata, gradients,
controls and rounded clips are authored in Pax; none is baked into the stills.
The navigation logo is a transparent PNG with the same peach lettering in both
modes.

By default, movie cards use viewport proximity events to request a 480 ms entrance timeline
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
210/260 radii are local logical pixels on both GPU and Vello CPU. A second toggle during the
fade continues from the visible mixture. New cards or a newly opened panel use
the current theme immediately. The explicit menu/detail entrance and exit
timelines retain ownership of their animated properties. Gradient crossfading
is a paint operation and does not change the group-opacity limits below.

Ordinary opacity now composes each canvas portion of the detail subtree before
fading it, on WGPU and browser Vello CPU. Native text/controls and separate Scroller
surfaces still fade independently, so the mixed-content result remains an
approximation. See [subtree opacity](../../pax-docs/book/src/compositing-effects.md#opacity-through-a-subtree)
for current behavior and backend costs.

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
2. Read `target/debug/pax-cli dev status --path examples/paxflix` and
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
