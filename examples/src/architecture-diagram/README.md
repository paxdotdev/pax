# Pax architecture schematic

A working architectural drawing made with Pax text and vector geometry. The
3120 × 2160 sheet names components; routed arrows explain their cooperation.
One fixed header contains the animated Pax logo and “Architecture”. The logo
starts at its finished pose (`progress=1.0`); click it to replay its animation.
There are no external image or font downloads.

The implementation follows the proposal in [TOPOLOGY.md](TOPOLOGY.md) and the
layout specification in [DESIGN.md](DESIGN.md). Its engine reference is
`3c46cd13d`, including isolated opacity and shared mask-source subtree capture.

## Run and build

From the repository root, use the CLI built from this checkout. An older
globally installed CLI may not understand the current language/runtime.

```sh
cargo build -p pax-cli
# On this workstation, initialize the repository's Node toolchain:
source ~/.zshrc
nvm use
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli run \
  --path examples/src/architecture-diagram --target web --libdev
```

The CLI prints the local preview URL. Template changes hot-reload; after changes
to Rust content or coordinates, rebuild/restart the default run session.

```sh
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli build \
  --path examples/src/architecture-diagram --target web --libdev --release
python3 -m http.server 8080 --bind 127.0.0.1 \
  --directory examples/src/architecture-diagram/.pax/build/release/web
```

## Reading and navigation

- **Fit** shows the whole sheet. **+ / −** adjust magnification; the native
  Scroller supports two-dimensional wheel/trackpad/touch navigation.
- Hover an edge to trace its complete path in copper. Click/tap to pin the
  highlight; select it again, click blank sheet space, or press Escape to clear it.
- Click or tap a part to open its responsibility, connections and source
  reference. Each connection is a native button that selects
  the related card. **Close** dismisses the inspector.
- Inspectors enter over the previous card in 320ms and exit in 240ms, using
  OutQuad easing. The old card begins exiting 80ms into the next entrance, so
  their motion overlaps for 240ms. Stable keyed wrapper layers keep the new card
  in front, including during the old card's exit. The wrapper is retired by the
  inspector's actual unmount, preserving its content until the motion finishes.
- With focus on the sheet, `+`, `−`, `0`, `N`, `P` and `Escape` control zoom, fit,
  inspection and dismissal. The web chassis reserves keyboard events for focused
  native controls; use those controls directly while they have focus.
- **Sheet view** hides the zoom controls and inspector and fits the complete
  drawing, keeping the single logo/title header and an **Exit sheet view** button.
  Use that button or Escape to return to the controls. Narrow viewports place
  one row of controls with 44px-high touch targets below the header title.

All connections use the same solid blue line and open arrowhead. Edge captions,
arrow categories and the footer legend are omitted. The inspector retains each
relationship's direction and explanation, plus three relevant links to the
published prose/API docs. Links open separately and support native keyboard
activation. Cards use a pointer cursor on hover.

Every card is 88px tall, with 200px compact, 300px single and 640px double
widths. Cards and inspectors share top-right/bottom-left chamfers. All numbered
band headings share a baseline and all routes stay below that heading row.
Assembly panes use a muted sage fill and a darker outline; pale edge halos keep
the intersecting blue connectors distinct against those panes.
Rendering engine rows are 220px apart, leaving 132px of vertical space between
cards. The sheet extends to accommodate the taller assembly and feedback routes.

At initialization, shared card sides receive separate ports centered as a group,
with a 28px preferred gutter. Ports follow the direction of the approaching
trace. Horizontal and vertical routing channels pack parallel runs into compact
buses with a 24px preferred gutter; disjoint runs reuse a lane. A channel only
compresses the gutter if it cannot hold that many concurrent tracks. Input and
measurement feedback use direct channels rather than enclosing the entire sheet.
Each connection remains separate and individually traceable; sharing a channel
does not imply a junction. The geometry scales with the sheet; arrow tips keep
two screen pixels of clearance at the current zoom. Card positions and channel
choices remain authored. The lane allocator is computed at mount, without
running a placement planner every frame or adding a general diagram editor.

The replay area deliberately exposes four responsibilities: surface scheduling,
spatial node selection, dirty traversal and per-surface culling. The inspector
records their existing contracts. It does not label those contracts as debt
without evidence.

## Composed raster export

The checked-in [full-sheet PNG](../../../pax-docs/book/src/images/pax-architecture.png)
is an offline overview for readers and agents. Pair it with [TOPOLOGY.md](TOPOLOGY.md)
and the code anchors in [content.rs](src/content.rs); the still image cannot show
the inspector's explanations or documentation links.

Capture provenance:

- Captured on **2026-10-01** from diagram revision
  `cbba60910e289ef86e274d10c8f221653c5f5c47`.
- Audited engine reference: `3c46cd13dfdc7fb4ac5992aa6c44672e4fc1f916`.
- Image: **3120 × 2288 pixels**, composed Chrome browser capture in Sheet view,
  including native text and GPU drawing, saved as PNG without resizing.

To refresh it, rebuild and serve the release app, export the complete Sheet view,
and replace the PNG above. The exporter below produces a 3120px reading asset
that can be copied to that path. Review the labels, arrows, all five columns and
the logo, and update the capture date/revision here with the image. Keep generated
browser captures and build output outside the repository; this reference PNG is
the deliberate documentation asset.

Run [scripts/export.cjs](scripts/export.cjs) against the served app. It requires
Playwright and Chromium, or an existing Chrome executable. For example, on macOS
with Playwright available through an existing Node installation:

```sh
PAX_EXPORT_BROWSER='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' \
node examples/src/architecture-diagram/scripts/export.cjs \
  http://127.0.0.1:8080 /tmp/pax-architecture-exports
```

If Playwright is installed outside normal module resolution, set `NODE_PATH`
to that installation's `node_modules`. The script uses an isolated browser,
activates Sheet view through the real UI, waits for fonts and repeated identical
composed frames, and writes:

- `pax-architecture-1560.png` — 1560 × 1144 overview.
- `pax-architecture-3120.png` — 3120 × 2288 reading/print asset.

Both use a 1560 × 1144 CSS viewport including the fixed 64px header, at device
scale 1 or 2. A browser screenshot
includes native text and GPU canvases; exporting a GPU canvas alone omits text.
Application state is deterministic. Pixel hashes can vary with browser, fonts,
OS and GPU; the script reports the actual SHA-256 of each settled output.

## Content and maintenance

| Source | Owns |
| --- | --- |
| [content.rs](src/content.rs) | Stable part IDs, component names, interfaces, explanations and code anchors |
| [layout.rs](src/layout.rs) | Part placement, assemblies, endpoint ports, routes and topology edge references |
| [routing.rs](src/routing.rs) | Ordered ports, horizontal/vertical bus packing and target clearance |
| [docs.rs](src/docs.rs) | Curated prose/API references per component |
| [lib.pax](src/lib.pax) | Drawing sheet, navigation and inspector deck |
| [inspector.pax](src/inspector.pax) | Chamfered inspector and enter/exit motion |
| [connection_link.rs](src/connection_link.rs) | Keyboard-accessible navigation to related cards |
| [edge_layer.rs](src/edge_layer.rs) | Nearest-path hover and pinned tracing |
| [part.pax](src/part.pax) | One labeled, selectable component |
| [lib.rs](src/lib.rs) | Data preparation, scale/selection state and graph validation |

Keep labels noun-based, move detailed causal prose to explainers, and keep
connections explicit. Update the topology and evidence when the architecture
changes. New relationships must name real endpoint IDs and cite the relevant
topology connection. Do not merge distinct code responsibilities merely to
obtain a tidier box or route.

```sh
cargo test --manifest-path examples/src/architecture-diagram/Cargo.toml --lib
cargo fmt --manifest-path examples/src/architecture-diagram/Cargo.toml --check
git diff --check
```

The graph check validates unique IDs, placed/connected parts, existing endpoints,
orthogonal routed segments, distinct ports, card intersections, sheet bounds and
2–4 existing documentation references per part. Inspect actual screenshots after
layout changes: graph validity cannot establish legibility or correct stacking.

The public docs embed lives in **How Pax Runs**, after its compiler/runtime/chassis
overview, with a link from the Maintainer Reference. Its `<pax-example>` entry
registers the app with the docs release-build generator. Source tabs are hidden
in that reference embed; Restart and Open standalone remain available. Publishing
the generated docs is a separate step from maintaining this source.

## Validation

On 2026-09-29, the web release build, Rust formatting, graph checks, edge-picker
checks and documentation book build passed. Geometry checks cover the three
allowed widths, one height, routes below the headings, distinct attachment
ports, no routes through cards, and existing docs references for every part.
The edge-picker checks distinguish nearby lanes and follow bends rather than
using an edge's bounding box.

Browser checks confirmed connection navigation selects the destination card,
new cards enter above frozen outgoing content, the deck settles to one card,
and Close retains the panel during its slide out. Published docs links returned
HTTP 200 and opened in a separate tab. Composed exports were inspected for card
sizing, heading alignment and routing.
Pointer checks confirmed card selection and pointer cursors, complete copper
highlighting across long edge routes, and pinned highlights surviving selection
of a different card.
Escape returns from sheet view to the controls.
Frame-by-frame DOM inspection confirmed the old and new titles remain distinct
during overlapping motion. Rapid forward/back navigation settled to one card;
Close removed it after its exit. Verify rebuilt release previews from a fresh
browser origin when cached Wasm assets leave an older build running.

On 2026-09-30, the consolidated header was checked at 390px, 800px and 1280px
viewport widths. Sheet view retains one header. Clicking the header logo
replayed its animation in the release web
build; clicking the standalone logo updated its bound scrubber in a debug web
build. Both builds passed, along with the two diagram tests, all 25 logo tests,
formatting checks and the documentation book build. Logo tests cover replay
during an existing animation and preserving external playback when disabled.
The export script's dimensions account for the fixed header. The current
reference PNG and its capture provenance are recorded above.

The next pass on 2026-09-30 removes the floating annotations and previous/next
toolbar buttons, optically lowers the logo against the title, and adds a visible
sheet exit. The routing pass packs compact buses, reuses disjoint tracks and
shortens feedback detours. Additional checks cover fixed bus gutters, endpoint
preservation, separate parallel traces, label clearance and feedback path length.
The release web build, all four diagram tests and formatting checks passed.
Source-derived geometry was inspected; browser verification of this pass was blocked by the
browser tool's localhost access policy.

Native Apple targets, the Piet backend and physical touch input have not been
exercised.
The animation article and logo component documentation describe the shared
`click_to_replay` flag; the diagram's layout change adds no engine API.
