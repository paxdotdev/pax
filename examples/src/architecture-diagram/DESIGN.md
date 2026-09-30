# First implementation / drawing sheet 01

The approved topology is the semantic reference. The first implementation is a
3120 × 2160 logical-pixel drawing sheet, rendered entirely with Pax Text, Path
and Rectangle. The fixed header hosts the existing animated Pax logo.

## Textual wireframe

One fixed header: current logo followed by “Architecture”. The logo starts at
its finished pose and replays on click. No repeated title, subtitle, logo, rule
or schematic/revision label appears inside the scrolling sheet. Controls move
below the title at narrower widths; sheet view retains the logo, title and a
visible Exit sheet view button. Lower the logo by 6px for optical alignment of
its main signboard with the title. No floating annotations accompany the cards.

Five vertical assemblies, left to right:

1. Compiler: authored sources, Parser, cartridge generator, cartridge.
2. Scene graph: traverser, instance behavior, expander and the live scene graph.
3. Reactive runtime: handlers, Dirty DAG, control flow, layout, scene geometry,
   opacity scopes and primitive hooks.
4. Rendering engine: compositor, tile/replay/dirty planning, lights, native patches,
   retained renderer, vector/texture/stencil paths and shared subtree capture.
5. Platform Chassis: scheduling, input, native bridge/elements/masks, surfaces and host
   composition.

Feedback routes run below the assemblies and return to their actual consumers.
No footer legend or caveat copy; support limits remain in the relevant inspectors.

## Spatial and visual specification

Warm drafting paper, dark blue-black ink, thin ruled assembly boundaries and
chamfered parts. Connections share one muted blue stroke and an open arrowhead.
All component cards use the same muted blue-gray outline across every column.
Muted sage assembly fills have enough contrast to group their cards; pale
connector halos preserve line separation across these darker panes.
Relationship kinds and causal captions live in the inspectors, keeping the sheet
focused on the parts and their composition.
Assembly numbers and small interface names help orient the drawing and code.
No paragraph is printed inside a component. Long explanations belong to the
inspector. Hierarchy uses line weight, whitespace, enclosure and type size.

Use three discrete card widths: compact 200px, single 300px, double 640px.
All cards are 88px high, including Resource loader. A 10px chamfer cuts the
top-right and bottom-left corners; the inspector uses the same shape at 14px.
Align all band headings at y=35; reserve the region below y=75 for routing.
Rendering engine rows use a 220px pitch (132px clear between cards), matching
the Compiler's spacing. Let this denser assembly extend below the other columns
instead of compressing its lower renderer rows; feedback routes run beneath it.

Columns have different widths: the rendering assembly needs the most room.
Orthogonal routes use gaps between nodes and reserved inter-column channels.
Shared sides distribute their ports symmetrically around the center, with a
28px preferred spacing, ordered toward the actual approach after the endpoint
stub. An initialization pass packs horizontal and vertical segments into compact
24px-gutter buses, reusing lanes for disjoint spans. Keep the input and measurement
feedback in adjacent channels rather than routing them around the sheet's bottom.
Arrowheads stop two screen pixels
short of the target card, adjusted when zoom or window size changes. Crossings
have no junction dot. Direction stays explicit without printed edge labels.

## Interaction and export

Initial fit-to-view includes every part. Native two-dimensional
Scroller navigation supplies wheel/trackpad/touch panning after zoom. Native
buttons supply zoom, fit and an explicit exit from sheet view. Clicking/tapping a
part opens the same inspector. Connection buttons select their destination card.
The new inspector enters from the right above a frozen prior card, using a
320ms OutQuad slide. The prior card starts its 240ms OutQuad exit 80ms into that
entrance; closing starts the same exit immediately. Each serial-keyed wrapper
holds an inner conditional inspector, preserving the deck's stacking order even
while Pax retains its exit. The inspector's unmount retires the empty wrapper.
Keyboard shortcuts supplement the native buttons.
Cards show a pointer cursor on hover. Each inspector includes three curated
prose/API docs links, exposed through keyboard-accessible native controls.
Hovering an edge highlights the entire route in copper above other connectors;
clicking or tapping pins that highlight. Picking compares distance to actual
segments, so neighboring lanes stay individually traceable. No hover-only information. The inspector can be dismissed and never contains
the only explanation of a principal connection.

A sheet view hides the zoom controls, closes the inspector and fits the same
sheet below the fixed header, with Exit sheet view still available. Its untouched logo uses the deterministic
finished pose; clicking can replay it. A composed browser screenshot
must include native text and canvas geometry. Document final export commands,
sizes and validation in README.md after verifying the actual build.

## Source ownership

`content.rs` owns part IDs, labels, explanations and source anchors. `layout.rs`
owns positions, assembly enclosures and routed relationships. `routing.rs` spaces
ports and packs corridor tracks at mount; `docs.rs` owns curated documentation references. `lib.pax` and
`part.pax` own rendering and interaction presentation. Relationships carry
endpoint IDs and topology references; validation checks endpoints and geometry.
This is one authored schematic, not a diagram editor or an auto-layout engine.
The pure geometry lane allocator provides a useful extraction point for a
future graph viewer. A reusable viewer would also need a node/port/channel data
contract, measured-card bounds and policies for dynamic topology; those are not
required to evaluate this routing pass.

The replay assembly remains decomposed into replay planner, scene geometry
index, dirty render planner and surface culler. Its inspector notes describe the
confirmed responsibility split; this first pass does not assert code debt that
the audit has not established.
