# PAX-1002 - Viewport lifecycle events

Design checkpoint, 2026-09-24. **Proximity APIs remain proposed, not implemented.**
Zack authorized the rendering-only foundation migration after this design discussion.
That migration is implemented and validated below; event/configuration syntax is still a proposal.

## Current recommendation

Zack proposed restricting detailed geometry-change delivery to an active proximity
region, with previous/current snapshots supplied by the runtime. This addresses
the custom 80px departure policy while avoiding global fine-grained observation.
Recommend this as the core MVP:

- `@viewport_proximity_change`: initial admission, subsequent effective geometry changes
  while nearby, and one final change when leaving proximity.
- `@viewport_proximity_exit`: a convenient cleanup notification following that final change.

Separate viewport_proximity_enter is redundant because admission is explicit in the first
change. Viewport enter/exit can be derived from the snapshots' intersection state;
standalone bindings remain possible convenience filters, not required core
machinery. This updates the prior four-event recommendation in response to the
latest design discussion, without committing to final syntax yet.

Use `change`, matching existing `textbox_change`, `slider_change`, and
`checkbox_change`, not `changed`. Both leave and exit are uninflected forms; prefer
`viewport_proximity_exit` to preserve the naming used throughout this discussion. There is
no claim that every existing event follows one universal naming convention.
Keep the explicit `viewport_` prefix: it identifies the reference for proximity
at the binding site. Recommend these as the canonical names, without short aliases.

The ImportSettings/theme topic was intended for another conversation. Provider
routing, selector-event exports, and theme layering are entirely out of scope.
Local handler ownership fits the existing dispatcher. Continue using existing
named timeline resolution and explicit local playhead controls; no theme-owned
playback system or declarative threshold language is proposed.

## Performance is part of the MVP

A callback gate alone is insufficient. Recomputing 10,000 detailed intersections
and then suppressing 9,950 callbacks would retain the expensive part of the work.
The previous unrestricted reactive-observation proposal did not adequately bound
that cost. Design candidate discovery and active observation together.

Use a cheap bounds-selection pass before detailed geometry, with observations
indexed by cached bounds in their owning scroll-content coordinates. On pure
scroll, move the viewport query rather than rewrite every descendant's bounds.
Process the new candidates plus the previous active set (needed to deliver exits),
then compute the full presented/ancestor-clipped snapshots only where needed.
Index membership must not depend on the target having previously rendered.

For nested Scrollers, query hierarchically: locate nearby outer rows/scroll owners,
then query their inner content. A far-away row must not cause all its horizontal
tiles to be evaluated independently. Preserve conservative coverage for transformed,
oversized, unclippable, and unknown-bound targets; a pruning shortcut must never
lose an entering node. New mounts, data/layout changes, and reparenting update the
appropriate bounds entries even when outside the active region.

An axis interval index or spatial buckets may be sufficient for the common list
case; choose the smallest correct structure after a focused implementation spike.
Do not assume an ordered-list-only algorithm is sufficient for arbitrary Pax
layout. Avoid a full-tree walk on every scroll and avoid a new property/effect
per descendant tied to the scroll offset. Targets without bindings have no
observation-specific detailed state.

Expected scaling target: ordinary scrolling should depend primarily on queried
candidates/nearby targets plus exits, rather than total collection length. This
is a design objective to measure, not a universal O(log N) or FPS promise. Initial
registration/index construction and a genuine whole-list relayout can still cost
O(N). If 10,000 targets actually overlap the region, 10,000 callbacks may be valid.
Cheap index records still consume O(N) memory for N registered targets; richer
last-sample state is needed only for the active set.

Current renderer support is not a drop-in solution:
`SurfaceReplayCoordinator::spatial_replay_node_ids_for_surface_bounds` linearly
filters a `HashMap` of canvas-node coverage. That coverage is populated by render
recording and does not describe every observable component/native element. Reuse
the underlying geometry/cache and query machinery through a shared runtime layer;
renderer residency does not determine lifecycle membership.
This task bounds its additional observation cost; it does not virtualize the
existing 10,000 mounted components or solve PAX-839 startup hydration.

## Share geometry work with rendering

The renderer already performs overlapping work, so a separate observer geometry
pipeline would duplicate both computation and invalidation logic. Strengthen the
recommendation from sharing helpers to sharing maintained geometry and spatial
query infrastructure, with rendering and viewport observation as consumers.

Retain local/content-space bounds, scroll-owner relationships, and clip/transform
dependencies in the common runtime. Update those facts on relevant changes. On
pure scrolling, update the owner viewport/presentation state and query its index;
do not rewrite every descendant's indexed bounds. Resolve presented transforms
and clip intersections on demand for candidates, caching common results by their
input revisions. Both consumers should reuse an identical current calculation.
This must remain demand-driven: sharing does not justify eagerly resolving rich
geometry for every mounted node.

Use the same spatial query infrastructure for render candidates and observed
targets, retaining distinct layout and paint-coverage bounds where necessary.
The current `pax-std/src/common.rs` canvas coverage adds a 64-unit culling pad in
canvas coordinates, and Path coverage can extend outside layout bounds. Those
coverage rectangles cannot become the public target bounding box. Queries may
share candidate results when regions, bounds semantics, and revisions match;
different regions can query the same index and perform their own final tests.
Renderer tile padding/budgets and application proximity margins remain separate
policies. A shared conservative candidate envelope must contain both kinds of
bounds, so paint outside a layout rectangle is never incorrectly culled.

Register observable component/native targets before their first drawing, alongside
the records needed by rendering. Cached render coverage alone has neither that
coverage nor that timing guarantee. Observation must also run on native scrolling
that changes visibility while retained surfaces need no redraw. Shared geometry
therefore belongs upstream of drawing and must not force occlusion or canvas
replay solely to obtain a sample.

Freeze event payloads for dispatch. If a handler changes geometry, rendering may
need a newer revision in the same tick; that is a legitimate new calculation,
not duplicate work. Keep the existing one-observation-batch-per-tick contract.

Implement the common foundation and integrate the overlapping render path as part
of this feature, without a wholesale rendering rewrite. Measure shared cache/query
reuse and total frame cost with observation enabled and disabled. The expected
savings come principally from avoiding repeated ancestor walks, transformed-bound
construction, and full scans; the magnitude is unmeasured. A few policy-specific
rectangle comparisons may reasonably remain separate.

## Evidence from the pre-migration checkout

| Area | Current implementation and design implication |
| --- | --- |
| Tile preparation | `pax-runtime/src/engine/layer_tiling.rs` has `ScrollerTilingPolicy.prewarm_viewport_pad_*`. Defaults are max(viewport width × 1, 512) and max(viewport height × 1.5, 512), on scrollable axes, within a tile/surface planning policy. Single-surface paths can bypass this calculation. It is not a node membership contract. |
| Warm rendering | `pax-runtime/src/engine/layer_surface.rs::replay_batches_by_priority` deliberately separates visible work from warm rings. Neither surface residency nor warm replay completion should dispatch application entry. |
| Node geometry | `ExpandedNode.transform_and_bounds` is reactive content-space layout. `RuntimeContext::presentation_scroll_transform_for_node` composes render-ancestor scroll transforms for presented coordinates. Reading layout alone misses native scrolling. |
| Scroll invalidation | `RuntimeContext::update_scroller_surface_scroll` dirties layer plans without requiring occlusion recomputation. Both web and the shared Apple bridge call it for `ScrollerPosition`. `ScrollerHostInstance::property_requires_occlusion_recompute` explicitly excludes scroll offsets. Observe this path directly. |
| Clips | `InstanceNode::resolve_effect_clip_path`, `clips_content`, and `scrolls_content`, plus render ancestry, expose the relevant geometry. `engine/occlusion.rs` already carries presentation transforms and clip bounds, but its cached compositing state is not an authoritative visibility observer. In particular, its disjoint-clip fallback retains prior bounds; a membership classifier must preserve an empty intersection. |
| Mount and frames | `ExpandedNode::recurse_mount` calls application mount before primitive mount/effect setup finishes. The default TransformAndBounds is 100×100, so mere existence of bounds is not readiness. `PaxEngine::tick` settles effects around tick/pre_render, advances clocks, flushes custom events, then settles effects and occlusion before returning native messages. |
| Playback | `animation-motion.md` and existing timelines establish explicit playhead control. A timeline name organizes tracks; there is no general `ctx.play_timeline(name)` API. Structural `@in/@out` mean mounted-tree insertion/removal. |
| Event integration | The grammar already accepts event identifiers. `pax-manifest/src/cartridge_generation/mod.rs::event_to_args_map` determines typed handler arguments; generated cartridge macros dispatch them. Implicit lifecycle discovery in `parsing.rs` currently requires two arguments, so typed viewport lifecycle discovery would need deliberate extension. |
| Idle behavior | Render work already exits when no canvas work is dirty. The web loop still schedules display frames. `design/demand-driven-frame-scheduling.md` is an exploration, not an implemented idle/vsync handshake. New observation must add no recurring clock demand. |

Read issue descriptions/status/labels/relations for PAX-1002 and PAX-839, all
their current comments, the Pax Core project, and PAX-1000/PAX-996 as references.
PAX-839 remains Backlog; its existing blocker PAX-851 is Done. PAX-1000 is a
consumer, not a dependency. No changes to either worktree or their scope are
needed. PAX-996 remains parked.

## Event and payload contract

Candidate payload shape (schematic types):

```rust
struct ViewportProximityChange {
    previous: Option<ViewportProximitySnapshot>,
    current: ViewportProximitySnapshot,
}

struct ViewportProximitySnapshot {
    in_proximity: bool,
    bounds: Rect,                    // Presented target AABB, logical window units.
    viewport_intersection: Option<Rect>, // Positive-area common intersection.
}
```

`is_in_viewport()` can derive from the nonempty intersection. A helper can derive
an intersection ratio from target/intersection areas, but do not require callers
to use percentages or store redundant percentage fields in the payload. The 80px
case needs intersection width/height, not a ratio. Ratios would describe bounding
geometry under the chosen approximation, not actual visible/opaque pixels.

`previous` is the last delivered snapshot in the current continuous proximity
visit, not an unobserved intermediate native scroll position. On first admission
and re-admission it is None, because cold geometry is not sampled continuously.
Authors can retain longer histories across visits themselves. This avoids
computing a derivative across a long unobserved gap. Cold index bounds are not
silently presented as a fresh previous observation.

| Situation | Delivery |
| --- | --- |
| Initially inside proximity, whether visible or not | change(None, current) after usable layout |
| Initially outside proximity | None |
| Enters or reenters proximity | change(None, current) |
| Effective geometry changes while nearby | change(previous, current) |
| No effective change | None |
| Leaves proximity | final change(previous, outside), then viewport_proximity_exit with the same snapshots |
| Outside remains outside, including a jump over the region | None; no swept visit |
| Node unmounts | Cancel registration; normal unmount cleanup, no synthetic geometric exit |

The final change is a deliberate terminal notification even though current is
outside. Without it, a jump directly from visible to beyond proximity could hide
the viewport exit from a change handler. It also makes previous/current semantics
consistent for users who prefer a single handler; viewport_proximity_exit is convenience
for explicit cleanup. Do not duplicate business side effects in both handlers.
Only registered handlers run.

A change includes target bounds changes OR changes to effective intersection caused
by viewport/clip/presentation changes. Raw TransformAndBounds changes alone are
insufficient because native scroll presentation can bypass layout. Coalesce to one
settled sample per target per engine tick; this is not a callback for each property
write. Equal snapshots produce no event, and idle scenes perform no observation
work. A membership edge must not be lost to coarse numeric rounding.

Local events do not bubble or have cancellable default actions. Keep current
component/inline scope and stable delivery order. Include no history beyond the
previous/current pair; timestamps or additional metrics should require a concrete
consumer before expanding the payload.

## Proximity configuration and geometry

A useful candidate proximity range is one viewport length on each side of the
relevant scroll axis: for a vertical viewport of height H, the region spans 3H
in total. Use logical units/percentages, e.g. an explicit y margin of 100% for a
vertical list. A horizontal Scroller uses its width; nested owners resolve against
their own viewport dimensions. The exact default and whether axis choice is
inferred or explicit still need alignment. Explicit x/y margins keep it predictable.

This is stable application policy, independent of tile size, renderer prewarm
multipliers, surface budgets, backend caps, and scroll velocity. Enlarging it
increases the active working set and callback cost. Proximity includes the real
viewport, so all sampled viewport crossings can be represented by the stream.

Compute in logical window coordinates after transforms and presentation scroll
offsets. For viewport membership, intersect the target's presented AABB with the
root and the common intersection of all applicable render-ancestor clips. A tile
inside its horizontal row is not visible when that row is outside the vertical
ancestor/root viewport. Handle web visual viewport/page scroll without double
translation. Template/event ancestry is not the geometry ancestry.

For proximity, expand root/Scroller regions before composing the region; ordinary
Frame/hard-mask clips remain strict in the current proposal. Tight wrappers can
therefore suppress nearby preparation; fixture and confirm this policy instead
of silently bypassing permanent clips. Honor unclippable escapes while retaining
presentation transforms; LayoutRole::Breakout does not escape clipping.

Recommend bounding envelopes for transformed/rounded/path clips initially, with
the approximation explicit. Ordinary rectangular axis-aligned clipping is exact
under this contract. Opacity zero, transparent texture pixels, alpha-mask coverage,
sibling occlusion, and external window coverage do not affect membership. This is
not an impression/viewability API.

Require positive area: contact alone and zero/singularly collapsed bounds are
outside; partial intersection of an oversized node is inside. Keep unbound layout
separate from known zero size. Preserve empty intersections. Classify sampled
endpoints, not swept movement; a fast jump provides no guaranteed animation time.

## Local threshold and animation policy

The handler can infer initial proximity admission from previous=None, viewport
entry from outside/absent previous intersection to a current intersection, and
viewport exit from a previous intersection to none. This includes initial visible
content without a mount workaround. On re-admission, a current visible snapshot
is a fresh visible visit; do not infer unobserved motion through the cold interval.

For the demonstrated vertical list, compare intersection heights in previous and
current. After a baseline is available, the application can apply:

```text
first visible sample: reveal
if 0 < current_height <= 80 and current_height < previous_height and not departing:
    departing = true
    start pop-out
if current_height > previous_height and departing:
    departing = false
    restore/take over toward reveal
actual viewport exit: reset the visit's animation policy
proximity exit or unmount: release proximity-owned application work
```

Use a small deliberate numerical tolerance in the example. Entering through the
80px band does not pop out; holding still does not retrigger or restore. A short
initially clipped tile establishes a baseline before direction is inferred.
Reversal can restore while still visible without fabricating a physical entry.
For other geometries, the handler can use intersection rectangles and select the
relevant edge/axis. This sample policy is not built into the engine.

Observe a stable wrapper and animate its contents to avoid animation changing its
own observation geometry. If the observed target itself moves, those changes are
real and handled by the normal next-pass feedback contract, not hidden debounce.

Named timelines with a component-owned Property playhead already provide imperative
playback from local handlers. Reuse existing timeline symbols/resolution and
Property easing; each component instance owns its state. Direct string lookup via
`ctx.play_timeline(name)` is not an existing public API and is not assumed here.
There is no theme-owned controller, declarative threshold DSL, or engine-owned
animation reversal policy in this MVP.

For lazy data, first proximity change requests data through an application service
keyed by data ID. Deduplicate ready/in-flight requests; previous/current samples
must not repeatedly fetch. Completion returns on the runtime thread and checks
target lifetime, current data ID, and request generation. Stale results can still
populate a cache. Proximity exit releases this target's request lease; cancel a
shared request only when remaining consumers allow it. Unmount also cleans up.
Data ID changes while active need their own application path. No networking
framework or cross-thread Property ownership is introduced.

## Delivery, identity, and readiness

Keep existing mount/tick/pre_render/clock/custom-event order. After normal binding
and layout effects settle, capture one frozen observation batch, commit internal
state, dispatch local handlers, settle their effects, then finalize native messages
and Pax rendering. Do not sample constructor-default 100x100 bounds. Later async
measurement can invalidate valid earlier geometry; readiness is not resource
completion. Stable registration order must not depend on HashMap iteration.

At most one batch per target per tick. Callback-driven geometry changes render
from settled properties but reclassify next pass; retain pending work and request
that pass without requiring another user input. Do not borrow registries/geometry
across callbacks. Newly created targets observe next pass. Author initial entrance
visuals from construction, because native compositors can reveal old pixels before
Pax receives scroll notifications. No before-first-physical-pixel promise is made.

Use mounted target identity plus registration generation. Stable keyed reuse keeps
history only when the instance stays active; reparenting updates index/clip-owner
dependencies. Drop stale callbacks after unmount/reload. Exit-retained instances
remain mounted; proximity does not start structural @out or retain them. Suspension
suppresses delivery and must not produce a derivative over a long unsampled gap
on resume; resume establishes a fresh baseline. Registration cleanup must not read
stale repeat-item expressions during teardown.

## Seam for PAX-839

Share geometry classification/cheap candidate-selection concepts with separate
lifecycle and future materialization consumers. PAX-1002 indexes already mounted
observed nodes and does not defer mount, tick, pre_render, or property setup.

Unmaterialized subtrees still need independent identities, scroll extents, order,
conservative bounds, and ancestor wake dependencies before full initialization.
Fixed-size rows are a plausible first case; autosized text, mount-dependent sizing,
and unknown overflow need conservative eager fallback or an estimate contract.
Unknown cannot mean safely cold. A renderer coverage cache populated after drawing
cannot solve that bootstrap problem. Do not implement PAX-839 or reopen PAX-996.

## Implementation and validation after alignment

1. Establish geometry/readiness and candidate-selection fixtures before claiming
   scalability. Cover initial states, partial overlap/contact, empty/nested clips,
   transformed/oversized/unknown bounds, escapes, visual viewport/page scrolling,
   cold-to-hot layout changes, and whole offscreen horizontal rows.
2. Establish shared revision-aware geometry/cache and indexed queries, integrating
   the overlapping render candidate path. Implement active-set observation and
   local change/exit delivery. Test
   admission baseline, previous/current immutability, terminal change before exit,
   direct visible-to-cold jumps, outside-to-outside silence, reentry, callback
   feedback, suspension/resume, teardown during dispatch, keyed identity,
   reparenting, projection, retained exits, and reload generations.
3. Audit event maps, handler/config descriptors, rich manifests, program IR,
   binary/rust_manifest roundtrips, cartridge/templates, and debug/release parity.
   Keep runtime-only observation history out of baked program metadata.
4. Validate the manual 80px policy, named timeline/Rust easing, and delayed
   deduplicated/stale data in a dedicated example. Provide web debug/release
   preview plus representative Apple checks using Pax dev event traces and
   screenshot sequences. Do not modify Paxflix without coordination.
5. Measure 1k versus 10k+ items with fixed viewport/nearby density. Count candidate
   visits, detailed geometry evaluations, emitted callbacks, bounds-index updates,
   allocations, and frame cost separately from initial expansion/index cost.
   Compare observation enabled/disabled and verify reuse of common geometry and
   query results; include rendering cost in the measurements.
   At idle, added observation work must be zero. Pure scrolling must not silently
   turn into a full scan; genuine bulk relayout cost is measured separately.
   Include nested lists, fast jumps, resize, deletion, and all-items-overlapping
   worst cases. Data volume alone does not establish an FPS guarantee.
6. Update scrolling-viewports.md, event-handling-rust.md, animation-motion.md, and
   public Rust/API docs. Keep lifecycle observation distinct from virtualization.
   Document proximity budget and previous=None after cold gaps. Record concrete
   authoring pain points and build the book. No theme documentation changes.

No proximity APIs have been implemented. Rendering migration validation is tracked
separately below; book validation does not establish runtime/performance behavior.


## Authorized rendering-only migration

The first implementation milestone moves the existing rendering workstream into
runtime-owned geometry preparation and indexed selection before adding events.

- `InstanceNode::prepare_canvas_geometry` supplies primitive-local paint bounds.
  The default is unbounded for custom primitives. Standard bounded primitives use
  layout bounds; Path supplies overflow-aware bounds and a prepared command path.
- `RuntimeContext` owns prepared records and a sparse, size-tiered spatial grid
  partitioned by logical canvas layer. Each rectangle occupies at most four cells;
  unknown/extreme bounds use conservative handling. Ordered sparse coordinates
  avoid walking empty cells in long scroll ranges.
- Existing canvas dirty signals invalidate preparation; mount registers canvas
  targets and unmount removes records. Geometry settles after normal effects and
  occlusion/layer assignment. Replay-only dirtification does not reprepare geometry.
- Primitive drawing obtains the same prepared record used by candidate selection.
  Native scrolling moves content-coordinate queries without rewriting indexed
  descendant bounds. Layout bounds remain distinct from padded paint coverage.
- `ReplayCanvasLayerUpdate` now carries optional canvas-content regions. Web and
  Apple chassis both resolve those regions through the runtime. GPU and Piet keep
  surface scheduling and tile culling, with no backend scene-coverage map or scan.
- Geometry work counters cover preparations, changed index entries, queries, and
  candidate tests. Tests compare queries against a linear oracle, exercise updates
  and teardown, verify record reuse before first draw and at rest, and compare
  fixed-density 1k/10k collections.

This milestone does not introduce subscriptions, public proximity margins, or
viewport event dispatch. Existing layout, native clipping, paint padding, tile
budgets, and lifecycle behavior remain the rendering contract. In particular,
canvas paint coverage is not yet a public clipped viewport observation. Future
observation can add a consumer to the shared preparation/index path without a
second renderer coverage registry.

Prepared records and the index are runtime state, not baked program metadata.
The primitive trait is implemented by the same standard-library code in debug and
release. No manifest/program-IR/schema change is needed for this milestone.

The public template articles were reviewed (Scrolling & Viewports, How Pax Runs,
Drawing & Styling). This internal migration adds no author-facing syntax or
capability; maintainer API comments and generated reference describe the changed
primitive/backend contract.

### Rendering migration validation (2026-09-24)

- `cargo test -p pax-runtime -p pax-std --lib --tests`: 240 tests passed.
  After the final dirty-queue change and strengthened nested-owner fixture, all
  five spatial unit tests and three geometry integration tests passed again.
- The randomized spatial oracle covers mixed sizes, layers, movement, deletion,
  negative coordinates, and unknown/extreme bounds. Horizontal and vertical
  fixed-density queries return six hits and test fewer than twelve candidates
  at both 1k and 10k indexed records. This measures candidate selection, not
  frame time, allocation cost, or total memory footprint.
- Integration tests verify preparation before first draw, shared record identity
  through replay/drawing, zero added idle geometry work, layout/transform changes,
  unmount cleanup, and content-space stability under nested native scrolling.
- `cargo check -p pax-chassis-common` passed. The macOS debug
  `rounded-scroller-tiles` app built and launched; scrolling preserved gradient
  rendering and rounded clipping. iOS/device validation was not performed.
- The web debug `scroll-matrix` app built and ran. Visual checks covered nested
  vertical/horizontal scrolling, previously offscreen tiles, distant two-axis
  tile replay, and viewport resize. No browser console errors were observed.
- The web release `path-drawing` app built through the baked-program path and
  rendered animated paths without browser console errors. No binary format
  migration was required.
- API reference was regenerated through `gen_api_docs`; `mdbook build` passed,
  and 163 local link targets in changed reference/navigation pages resolved.

Initial registration/preparation still scales with mounted canvas nodes. Custom
primitives with unknown coverage remain conservative candidates, and empty/unknown
queries preserve the existing full-replay fallback. These checks do not establish
a universal query bound or an end-to-end 10k-element performance guarantee.
