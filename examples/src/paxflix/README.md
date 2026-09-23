# Paxflix

A fictional cinema library built in Pax for PAX-1000. It contains **35 original
ImageGen films, 100 real cards, ten horizontal Scrollers, and one vertical page
Scroller**. Clicking a card opens its full-resolution still, synopsis and local
Play/Download feedback. It has no media backend or real download action.

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

`FilmArtwork` places the card thumbnail beneath its full-size still, keeping
the selected artwork visible while the larger image loads. A multi-stop linear
gradient fades into the active theme's surface at the bottom of both the hero
and detail image: black in dark mode, ivory in light mode. The surrounding
surface continues the fade without a hard edge. Short landscape details also
fade at the right edge. The original film images retain their colors.

```text
Fixed masthead: Paxflix / profile mark
  Profile popover: Dark/light mode / Manage profiles / Account / Help center
Vertical page Scroller
  Half-viewport hero: full-size still + title + summary + two actions
  Category title and previous/next controls
    Edge-to-edge horizontal Scroller: ten cards with text over a lower image fade
  …nine more independent shelves…
  Small closing credit
Root overlay, when selected
  Detail panel: same film, full-size still, synopsis, Play / Download / close
  Native EventBlocker backdrop, behind the panel and above the page
```

Desktop cards are 292px wide with 16px gaps and 48px page gutters. Below 700px,
cards are 236px wide and gutters become 20px. The 4:3 card frames crop the stills
to fill; a 124px lower gradient reaches near-opaque black or ivory behind the
title and metadata. Titles have room for two lines, and the hover indicator sits
in the upper-right corner. Shelf Scrollers span the viewport. Their content has
leading and trailing gutter space: the first card aligns with the heading at
zero scroll, while cards can cross either gutter as the row moves.

The logo's transparent source margin is cropped by a Pax Frame so the visible
lettering aligns with the same gutter. Profile controls and row arrows use an
equal right gutter. Native iPad headers reserve 32px above the usual 68px bar;
native iPhone headers reserve 64px in portrait and 12px in landscape. Landscape
iPhone fixed content uses symmetric 64px side gutters. These are explicit
example layout margins, not measured safe-area insets, which Pax does not yet
expose. Web header geometry is unchanged. The menu, page viewport and detail
placement share the native top spacing so their controls remain below it.

The hero is 52% of viewport height
within a 360–520px desktop range, and 354px on compact screens. Short landscape
windows use a side-by-side detail layout; shorter portrait windows allocate
more height to the detail copy. The whole detail component fades in over 320ms
and out over 220ms with `InOutQuad` opacity, alongside its short vertical motion.
The backdrop uses matching fade durations. Hover adds a fine accent outline and
detail indicator without changing the shelf geometry.

`catalog.json` owns stable film IDs, titles, years, runtimes, ratings, genres,
synopses and image paths. `src/catalog.rs` deterministically places every film
two or three times, with ten different films per shelf. This is a curated fixture,
not a recommendation system. Shelf arrow controls ease their bound native
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
3. Use `dev inspect tree --session <id>` to verify 100 MovieCard components,
   103 Image nodes including the logo and both hero layers, and 11 ScrollerHost nodes with the
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

## Validation and remaining checks

- Catalog tests verify unique identities, all asset files, 100 placements,
  ten unique films per row, and two or three references to each film.
- Debug and release-baked web builds pass. Browser verification covers desktop,
  compact portrait, short landscape, horizontal/vertical scrolling, repeated
  modal selection and dismissal, backdrop input blocking, profile menu feedback
  and dismissal, and animated row paging.
- The iPadOS app builds, installs and launches on the iPad Pro 11-inch (M5)
  simulator running iOS 26.4. The initial catalog and logo were visually checked.
  The revised card overlays and native header spacing were rebuilt and launched;
  their final native visual/interaction check was interrupted by the Mac lock
  screen. Native iPhone, real touch and diagonal trackpad gestures remain
  unverified. The earlier macOS build passed, but its initial blank window still
  needs a separate native inspection.
- A scrolling detail panel was investigated for short windows. On web, a root
  EventBlocker intercepted wheel input above the panel's nested Scroller even
  though that Scroller rendered above it. The delivered panel uses responsive
  geometry and contains no Scroller. No engine workaround or engine changes
  are included; the layering finding is recorded in the authoring pain points.

Run focused tests with:

```sh
cargo test --manifest-path examples/src/paxflix/Cargo.toml --lib
```

No recording, publication, commits or merge are part of this example task.
