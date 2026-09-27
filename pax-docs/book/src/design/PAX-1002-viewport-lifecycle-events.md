# PAX-1002 - Viewport lifecycle events

Implementation checkpoint, 2026-09-25. The rendering foundation and opt-in event
trio are implemented. Public usage is taught in
[Scrolling and Viewports](../scrolling-viewports.md#observe-viewport-proximity).

## Current contract

- `@viewport_proximity_enter`: initially nearby content and each reentry.
- `@viewport_proximity_change`: initial sample, changes during a visit, and a terminal outside sample.
- `@viewport_proximity_exit`: cleanup following the terminal change.

Enter precedes initial change and uses the same frozen current sample. Any
subset can be bound independently. The explicit enter event avoids requiring
coordination between handlers for ordinary preparation work.

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

Payload shape (schematic types; public fields use `kurbo::Rect`):

```rust
struct ViewportProximityEnter { current: ViewportProximitySnapshot }
struct ViewportProximityExit {
    previous: ViewportProximitySnapshot,
    current: ViewportProximitySnapshot,
}

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
| Initially inside proximity, whether visible or not | enter(current), then change(None, current) after usable layout |
| Initially outside proximity | None |
| Enters or reenters proximity | enter(current), then change(None, current) |
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

The approved MVP uses one global viewport width on each horizontal side and one
global viewport height on each vertical side. Apply those same margins to scroll
clips; do not resolve them against each nested owner's dimensions. No axis or
margin configuration is exposed in this pass.

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

The example compares sampled heights directly. Entering through the
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

For lazy data, proximity enter can request data through an application service
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

The rendering validation below predates the event implementation. Event validation
is recorded in the subsequent milestone; book validation alone does not establish
runtime/performance behavior.


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


## Event milestone, 2026-09-25

The three explicit bindings use the existing typed handler generation and local
component/inline routing. Registration is deferred until mount/layout has settled:
component mount can hold its property-scope borrow while mounting descendants.
Only registered targets and their shared ancestors acquire observation effects.
Full snapshots live in the active set, not every cold registration. Template
replacement revokes the old registration generation and rebuilds dependency chains;
reparenting retains instance history while rebinding scroll domains.

Observation consumes the renderer's prepared geometry records and the same sparse
spatial-index implementation. Non-canvas targets/ancestors get records only on
demand, without entering the paint-coverage index. Layout bounds have separate
scroll-domain indices because they are not padded canvas paint coverage. Nested
queries prune cold scroll owners; unclippable and unclipped-scroll branches remain
conservative candidates. Clip-envelope intersection is still consumer-specific;
this does not claim every rectangle comparison is shared with rasterization.

Fixed margins use global viewport dimensions on both axes, independent of warm
tiles. Enter/change and terminal change/exit use frozen pairs. Suspension clears
history, unmount cancels delivery, and handler mutations classify next tick. The
example observes a wrapper, eases a named timeline's local playhead, restores on
reversal, requests a shared page of field notes on enter, and releases a consumer
lease on exit or unmount. Web uses a bundled HTTP fixture; native uses a worker
thread. Completion returns through one root tick drain, with lease and data-ID
checks before touching card properties. This is application-owned example code,
not an engine async-loading framework.

The enabled/disabled runtime measurement also exposed an existing handler-context
cost: removing a temporary reactive dependency scanned all upstream siblings.
Dependency teardown now removes one reciprocal edge per inbound occurrence by
searching backward, retaining the remaining effect order and duplicate-edge
semantics. Short-lived contexts normally own the newest edge. This avoids the
full sibling scan on this dispatch path without caching contexts or changing their
reactive semantics. A regression test checks duplicate edges and remaining order.

### Measurements

Run `cargo run -p pax-runtime --example scene_geometry_bench --release` for the
renderer geometry service and the ignored `observation_workload_measurement`
integration test for full mounted-runtime scrolling (set `PAX_BENCH_N=1000` or
`10000`, and `PAX_BENCH_OBSERVE=0` or `1`, in a fresh process). Timings are local
measurements, not FPS guarantees; GPU raster/compositing is excluded. Allocation
counts include application-handler dispatch, and live bytes exclude allocator
metadata/RSS. Initial mounting and whole-list relayout remain linear work.

The final flat-list run with fixed nearby density measured:

| Mounted targets | Bindings | Live requested bytes after mount | Scroll microseconds/tick | Samples/tick | Index updates during 200 scroll ticks |
| --- | --- | --- | --- | --- | --- |
| 1,000 | none | 35,469,882 | 0.53 | 0 | 0 |
| 1,000 | trio | 36,776,442 | 23.86 | 5.47 | 0 |
| 10,000 | none | 326,754,666 | 0.54 | 0 | 0 |
| 10,000 | trio | 339,759,482 | 24.76 | 5.47 | 0 |

The observed delta is about 1.3KB per registered target, including registry,
dependencies, index, and handlers. Native scrolling sampled the same nearby set
at both sizes, with roughly 135 allocations/tick including handler contexts.
Initial timings varied with concurrent builds and are not used for a speedup
claim. The separate nested-list regression also checks identical domain,
candidate, and sample counts at 1k/10k, with fewer than 60 detailed samples.

### Validation

Twelve runtime integration tests cover initial/repeated admission, first-pixel
intersection, the 80px decrease (the example now uses 40px), terminal pairs, large jumps, idle/no-subscription
work, frozen callbacks, teardown, suspension, replacement, reparenting, nested
native scroll, page-backed visual viewport, clipping/escapes, zero-area transforms,
resize, and 1k/10k query scaling. They pass in debug and release. Runtime, runtime
API, and standard-library tests pass (316 tests, plus the ignored manual benchmark).
Rich-manifest and ProgramIR binary roundtrips retain all three inline/component
bindings; generated Rust retains the handler names. No binary schema change is
needed: only existing event-name/handler representations cross that boundary.

The wider manifest suite has three existing failures, reproduced using the
unmodified HEAD version of pax-manifest in a temporary crate: two route-metadata
panic expectations and the type-qualified object-constructor expectation. Its
other 49 integration tests and all eight current manifest unit tests pass.

Browser checks cover initial display, early departure, reversal, cold cleanup,
and fresh visits after returning. Public articles and generated API reference
explain the fixed region and approximation. Web debug and baked release builds
pass, and the macOS debug application builds through the shared native bridge
and Xcode host. The book builds and 468 local links/anchors on affected pages
resolve. The baked release browser reports no warning/error from its own origin.
The native interaction/profiling follow-up below supersedes the build-only macOS
check. iOS/iPadOS interaction and an end-to-end frame-rate guarantee remain unverified.

### Native profiling follow-up, 2026-09-25

Time Profiler captures of the 270-card macOS debug example identified several
distinct costs. In a five-second scrolling segment, main-thread samples included
655ms in SwiftUI/AppKit intrinsic scene measurement and 320ms in native text
content signatures. Viewport-proximity dispatch itself accounted for 6ms, about
0.2% of the 2542ms main-thread total. This does not include downstream rendering
work triggered by handlers, and roughly 497ms of the total included accessibility
inspection stacks from the test driver.

The native scene now accepts SwiftUI's viewport proposal on macOS 13+, avoiding
intrinsic-size traversal through the native descendants. Font/alignment signatures
use typed hashing instead of reflection; color signatures hash their values rather
than converting them through AppKit/ColorSync. In a later five-second scrolling
segment, intrinsic measurement was absent and content signatures accounted for
53ms. These are sampled CPU costs, not equal-frame benchmarks or FPS measurements.
Native layout, view-tree rebuilding/sync, occlusion, and GPU work still have costs.
macOS 12 retains the older representable-sizing fallback.

The runtime also performed occlusion twice when clock-driven animation changed
geometry before observation and proximity handlers changed visuals afterward.
A regression test first failed with coverage sampled at both the pre-handler and
post-handler positions. Occlusion now runs once after handlers settle, while the
observation batch stays frozen. The test verifies final paint geometry and initial
scroller layer assignment as well as the next tick's terminal events. Presentation
setters explicitly invalidate observations; renderer-plan invalidation alone no
longer schedules geometry sampling.

After these changes, 290 runtime/runtime-API/standard-library unit tests and all
12 proximity integration tests pass; the integration suite also passes in release.
Six shared Swift tests pass, including font identity across family, typography,
and font sources. Web release and macOS debug/release builds pass. The documentation
example was regenerated through the repository tooling; its embedded app loads
the asynchronous field notes. The book builds and all 119 local article links
and anchors checked across the three affected learning articles resolve.

The final release build was launched and visually judged smoother, but still near
60fps during scrolling. A temporary host-only cadence probe measured a 120Hz
display-link source and approximately 120 engine ticks/second at rest. During
automated sustained scrolling, complete two-second windows measured 61.6, 69.5,
and 70.2 ticks/second. Median tick execution was 3.8–4.5ms; median main-queue delay
was 2.6–4.1ms, with much longer tail delays. These are engine tick measurements,
not displayed-frame or GPU completion counts. The instrumentation was removed
after capture.

A release Time Profiler capture during the same twelve-scroll workload sampled
6813ms of main-thread CPU over the active eight seconds, including 1134ms in
accessibility inspection stacks. Of the remaining 5679ms, occlusion accounted for
1068ms, AppKit layout for 990ms, native scene sync for 778ms, and native render-tree
construction for 315ms. These inclusive categories can overlap and must not be
summed as separate frame phases. The evidence identifies remaining scene-wide
CPU work and does not establish a 60Hz timer cap or guaranteed 120fps presentation.
Further work should reuse prepared geometry for bounded occlusion/mask queries
and avoid rebuilding/synchronizing unchanged native scene branches. A human-input
capture is still useful to separate real scrolling from automation overhead.


### Incremental native compositing, 2026-09-25

Native bounds now join canvas coverage in the runtime-owned geometry service.
Structural reconciliation retains layer/order/clip metadata and exact coverage;
leaf invalidation refreshes only the changed record and queries native bounds
under both its old and new coverage. Native mask evaluation queries the shared
canvas index, then applies exact paths, clips, opacity, and stacking order.
Exact empty queries are distinct from canvas replay's full-replay fallback.
Native render consumers retain geometry independently of proximity bindings.
Removed nodes release compositing records, and root unmount clears the cache.

Scrolling a leaf-only island queries native bounds in the old and new viewport
regions without rewriting content-coordinate indices. Inherited clip/container
changes, unclippable escape layers, and scroll domains that mix independently
scrolling content without islands retain complete reconciliation. Unclipped scrolling and nested scroll
presentation containers also keep the complete path. These are correctness
boundaries for the initial incremental migration.

The Swift bridge preserves changed element IDs in a bounded, non-destructive
journal. Native hosts update retained leaves directly; new hosts, missed
history, structural/order/container changes, and glass-container updates reconcile
from current models. Fonts and reset retain global invalidation. Existing view-backed transforms
continue to control native hit testing; native containers already disable child
autoresizing.

Regression coverage compares incremental native-mask hashes with full
reconciliation for movement, opacity, clipping, native rotation, ordering,
removal, scrolling, nested domains, and disabled islands. A 2,000-node fixture
checks bounded work for one animated shape and nearby scroll queries with no
scroll-induced index updates. Swift tests check coalesced independent journal
readers, history overflow, retained native controls, selection during scaling,
and ordering fallback.

An instrumented release run measured
approximately 1.7–2.0ms median tick work in fully active two-second scroll windows,
versus the earlier 3.8–4.5ms. Main-queue delays still limited those windows to
roughly 66–70 ticks/second. These are engine tick measurements, not presented
frame counts. In an eight-second active CPU interval, occlusion accounted for
31ms (earlier 1,068ms), whole-tree native construction/sync was absent from the
sample, and direct native updates accounted for 436ms. AppKit layout remained
1,444ms and Core Animation commit 2,937ms inclusive; these categories overlap.
The runs use accessibility-driven scrolling and are diagnostic samples, not a
normalized throughput benchmark. AppKit layout remains a follow-up bottleneck.
The clean macOS release passed a visual check on 2026-09-26: early exit at 26px
of remaining overlap, restoration on reverse scrolling, a jump to the final
cards, and return to the beginning. This check did not measure frame cadence.
Structural layer classification uses a single pass,
and hot scroll queries use cached layer ownership rather than scanning layers.
Scenes without native mask targets retain their scroll-only presentation path,
including nested scroll grids.


Validation: 290 runtime/runtime-API/standard-library unit tests passed; all 17
viewport/compositing integration tests passed in debug and release (one manual
benchmark remains ignored). All eight Swift tests passed. The shared Swift
renderer compiled for the iOS simulator, and clean macOS and web release builds
completed. Generated API references and the mdBook build succeeded; 82 local
links/anchors in the affected compositing and viewport articles were checked.
The refreshed web preview was exercised through the final cards and returned
to the beginning. No compiler/program-IR or baked representation change is
needed for this runtime-owned cache and chassis reconciliation change.

### Native property application and controlled scrolling, 2026-09-27

Native geometry now compares individual fields. Opacity and stacking patches do
not write frame/bounds/rotation; translation uses AppKit's origin setter. Scaling
still repairs logical bounds when AppKit changes them with the frame, and the
shear/reflection fallback restores view-backed rotation when switching modes.
Unchanged mask/snapshot state avoids repeated clears and visibility assignments.
Updating snapshot pixels also reapplies an unchanged punch-through mask.

Ten Swift regressions pass in debug and release, including coordinate conversion
through scale/rotation and shear-to-rotation transitions, selection retention,
zero geometry writes for opacity/stacking patches, and pixel assertions for
changed text under a stable mask followed by unmasking.

A temporary release probe compared 18 and 270 otherwise-identical cards with no
animation, opacity only, and the current scale/translation/opacity animation.
All modes retained the same proximity handlers, text updates, and loading logic.
The 900×900-point window followed the same native scroll path from y=400 to 2800
and back at 1,200 points/second. Each 15-second phase had three warm-up seconds,
ten scrolling seconds, and two settling seconds. The measurements below retain
whole one-second windows ending between seconds 4 and 12. Each case ran twice,
in forward then reverse order, without Instruments or accessibility polling.

| Cards | Animation | Callback rate | Inner text-scroll layouts per callback |
| --- | --- | ---: | ---: |
| 18 | None | 107.8/s | 47.5 |
| 18 | Opacity | 112.5/s | 49.4 |
| 18 | Current | 101.7/s | 44.0 |
| 270 | None | 23.8/s | 682.7 |
| 270 | Opacity | 25.2/s | 647.6 |
| 270 | Current | 23.9/s | 655.5 |

Both non-scaling cases recorded zero native geometry writes during the measured
windows. The current animation updated about six native leaf geometries per
callback in either list size, while inner NSScrollView layout work grew with the
mounted scene. The strong size dependence persists without card animation;
bounding the attached native hierarchy is the next architectural candidate.

These are display-link callback rates, not presented FPS. The probe includes
programmatic scrolling and native layout counters, so its callback durations and
rates are not directly comparable to earlier accessibility-driven measurements.
An initial long Animation Hitches capture exhausted temporary disk space and was
discarded; its stalled recorder/service were stopped to reclaim the open trace.
The complete table comes from the subsequent lightweight run.

A separate Time Profiler capture confirmed the source of the cost. In an
eight-second 270-card opacity-only interval, native scrolling's
`NSClipView.setBoundsOrigin` occupied 2,388ms, AppKit layout 2,011ms, and tracking
area updates 870ms of sampled main-thread CPU. Engine tick work accounted for
109ms, incremental native leaf updates 9ms, and occlusion 14ms. Core Animation
commit accounted for 4,795ms inclusive of overlapping AppKit work; the categories
must not be added together. These stacks point to native hierarchy maintenance,
not repeated scene reconstruction or animation geometry writes.

A later eight-second Animation Hitches capture completed after the active matrix
and had no app-scoped update rows; it does not establish presented-frame cadence.
The temporary example properties, native scroll driver, and release counters were
removed. The normal macOS release build and iOS simulator compilation passed.


### Native culling follow-up (2026-09-27)

macOS now uses the existing native layer index and scroller prewarm policy to
issue advisory `NativeCullUpdate` membership deltas. Only the clipped leaf-only
islands already classified by compositing participate. Structural reconciliation
initializes membership; scroll and leaf-animation ticks query nearby geometry
and compare warm ID sets. No second spatial index or per-scroll cold-node scan
was added. User-facing viewport proximity margins remain unchanged.

The Swift host detaches eligible Text and NativeImage views, retaining their
models and native instances. Cold views receive patches and regain their original
subview order on re-entry. Editable/focused/selected text, overflowing text, glass
content, and other controls stay attached. Focus/selection pins are reconsidered
on subsequent scene updates. VoiceOver and Switch Control restore all views via
the existing scene invalidation path; accessibility virtualization is outside
this change. Live assistive-technology navigation has not been exercised; the
fallback state transition is covered in the native host regression test.

The new protocol message is serialized by the existing native message queue in
both debug and release. It adds no manifest fields or baked-program semantics.
Web/iOS/iPadOS do not emit this message or use the macOS detachment path. The
public compositing and native-control articles and generated protocol reference
are updated.


A temporary release-only probe compared the unchanged 270-card example with
culling disabled/enabled/enabled/disabled in four 15-second phases. Each phase
used the same 900×900-point window, a 1,200-point/sec triangular native scroll
between y=400 and y=2,800, three seconds of warmup and ten seconds of motion.
One-second windows ending 5–12 seconds into each phase were analyzed. There
were no concurrent builds, Instruments recording, or accessibility polling
during these measured windows. Disabling culling used only the probe's internal
host fallback; it did not change system accessibility settings or the example.

| Culling | Updates/sec | Attached native text views in scroller | Inner scroll layouts/update | Median update work (ms) |
| --- | ---: | ---: | ---: | ---: |
| Disabled, first pass | 20.8 | 810 | 691.2 | 16.44 |
| Enabled, first pass | 117.8 | 18 | 13.2 | 3.13 |
| Enabled, second pass | 115.7 | 18 | 12.8 | 3.12 |
| Disabled, second pass | 21.1 | 810 | 682.1 | 16.10 |

These are main-thread app update callbacks, not presented-frame measurements.
The work timing includes the injected scroll driver; it does not include all
subsequent Core Animation work. Both paths retained identical models, original
card animations, geometry/mask updates, and proximity handlers. The probe was
removed before rebuilding the normal example. Raw windows are retained locally
at `/private/tmp/pax-native-culling-profile.json`.

Validation: 170 runtime unit tests, 19 runtime/proximity integration tests in
debug and release (one manual measurement ignored), and 11 Swift tests in debug
and release pass. The shared Swift package compiles for the iOS simulator. The
culling tests cover large-scene query work, scroll jumps, cold-to-warm geometry
changes, conservative fallback, real FlexBuffers decoding, detached content
updates, retained identity, stacking, focus, selection, overflow, accessibility
fallback, and deletion. Native visual checks include fast scroll jumps and
re-entry. API references regenerate and the documentation book builds.

### Post-culling profile and next candidates (2026-09-27)

A temporary release probe measured the unchanged 270-card example in a
900×900-point window. Each 60-second cycle contained 12 seconds idle, 18 seconds
of native triangular scrolling from y=400 to 2,800 at 1,200 points/sec, 18 seconds
at 4,800 points/sec, then 12 seconds idle. The first four seconds of each phase
were excluded. Timings include the synthetic scroll driver; deferred AppKit/Core
Animation work is outside the timed update. The probe also counted attached text
views, full native rebuilds, layout calls, and time in each update stage.

Concurrent builds and other applications materially affected cadence: the first
cycle delivered only 65 updates/sec at normal speed and 55 at fast speed. Later
complete cycles 8–10, after our stack sampling ended and before rebuilding,
produced the following results. Other workstation activity was not controlled;
these are repeat observations, not an isolated benchmark or presented FPS.

| Phase | Updates/sec across three cycles | Median update work (ms) | Mean runtime tick (ms) | Mean native bridge (ms) | Mean render call (ms) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Idle | 120 | 0.23–0.27 | 0.08–0.10 | 0.01–0.02 | <0.01 |
| 1,200 points/sec | 113.3–118.1 | 2.87–3.03 | 0.52–0.60 | 0.09 | 2.15–2.22 |
| 4,800 points/sec | 107.0–108.6 | 2.94–3.01 | 0.78 | 0.12 | 2.05–2.10 |

The median-work column is the median of each phase's one-second-window medians;
stage means are weighted by update count. About 17–19 text views were attached,
and full native rebuilds remained at the two startup rebuilds. Normal-speed gaps
over 12.5ms occurred in 1.1–3.3% of updates; fast-speed gaps in 8.4–8.8%. This
confirms that culling remains effective while leaving less headroom at fast
scroll speeds. Idle updates perform no native layout in these windows.

An eight-second `sample` capture at 12:48:07 local time covered normal-speed
scrolling in cycle 7. Of 5,404 main-thread stack observations:

- `pax_render` accounted for 1,190; lighting collection for 545 of those
  (45.8% of the render observations, 10.1% of all main-thread observations).
- Core Animation transaction commit accounted for 2,135 (39.5%). AppKit subtree
  layout accounted for 1,276 (23.6%), mostly within those commits; NSTextView
  layout accounted for 771 (14.3%). These inclusive categories overlap.
- Runtime engine tick accounted for 361 (6.7%); native message handling for 63
  (1.2%). `CAMetalLayer.nextDrawable` accounted for 144 (2.7%); only one of those
  observations was in its semaphore wait, so this sample does not point to
  drawable starvation as the primary problem.

These are stack-occupancy samples, including waits, not GPU timings or a
CPU-only Instruments trace. The attempted Time Profiler recording stalled while
saving and was discarded. Raw timing windows and the independent stack sample
are retained locally in `/private/tmp/pax-native-followup-all-windows.json` and
`/private/tmp/pax-native-followup-active-sample.txt`.

The first recommended follow-up is lighting collection. Source inspection shows
that each dirty canvas layer scans the node cache twice and builds light-frame
ancestry even when no lights are authored, as in this example. A proposed bounded
change is light/ambient membership maintained through existing node lifecycle
hooks, a no-light fast path, and invalidation of cached membership masks when
light scope or scene structure changes. Correctness must include removing the
last light, authored ambient-only scenes, animated light properties, ancestor
lights across scroller layers, and reparenting. Visibility culling must not
exclude an offscreen light that affects onscreen content. This is a proposal,
not an implemented cache or a measured speedup.

The second follow-up is a controlled text-layout experiment: hold the example's
live diagnostic labels constant, then separately remove animation geometry, to
distinguish content invalidation from scroll/transform-driven TextKit viewport
maintenance. Use the result to narrow changes to the existing native text host;
preserve selection, editing, accessibility, and visual behavior. Replacing the
native text architecture is not justified by this profile alone. Presented-frame
pacing and GPU timing still need a successful short capture on a quieter system
before changing display-link or Metal presentation policy.

No public behavior changed during this profiling pass. Temporary probes were
confined to generated chassis files and removed by rebuilding the normal macOS
example; the example source and canonical renderer were unchanged.

### Lighting contributor registry and mask reuse (2026-09-27)

The runtime now records lighting contributors through the existing node cache
mount, unmount, and replacement hooks. `InstanceNode::has_scene_lighting()` is a
stable capability, independent of a light's current enabled state. Both standard
light primitives opt in. Custom primitives implementing either light resolver
must add the same opt-in; the primitive-authoring article explains this change.
There are no new manifest fields or serialized/baked program semantics: the
capability is part of the Rust primitive implementation in debug and release.

Rendering resolves values only for registered contributors. Per-layer selected
light IDs and lexical owners key the retained direct-light masks; ordinary
geometry changes, native scrolling, and animated intensity/color/position do not
rescan recipients. Existing structural compositing invalidation discards those
keys for reparenting, layer changes, and node replacement. A changed selection or
structure can still require a recipient scan in a lit scene. Unlit scenes take
an immediate empty-light path, and zero masks are implicit. Removing the last
light clears prior retained masks and dirties affected drawing. Offscreen lights
remain eligible; no spatial visibility test was introduced for contributors.

The broader retained-rendering tests exposed an old incidental dependency:
inserting initial zero lighting masks also scheduled a canvas node's first draw.
Mount now explicitly marks canvas nodes dirty. Idle conditional branch handoffs
therefore remain correct without either lights or an unrelated animated sibling.

Validation: all 265 runtime/standard-library unit and integration tests passed in
debug; all 205 runtime tests passed in release, with one manual benchmark ignored
in each configuration. Six new lighting regressions cover a 1,024-node unlit
scene with zero scan visits, mask reuse under geometry/value changes, scope
changes, disabled/enabled and removed lights, ambient-only scenes, replacement,
and ancestor lights across scrolling/layer moves. Existing scope and overflow
tests remain passing. The designtime runtime configuration also compiles.

The same temporary native probe ran unchanged 270-card animations at 900×900
points, using the preceding profile's 60-second workload. Complete unprofiled
cycles 0 and 2 were measured; cycle 1 contained an eight-second stack capture
and was excluded. Whole one-second windows ending at least four seconds into
each scrolling phase were retained, fourteen windows per phase per cycle.

| Phase | First cycle updates/sec | Repeat updates/sec | Gaps over 12.5ms, first / repeat |
| --- | ---: | ---: | ---: |
| 1,200 points/sec | 119.5 | 119.6 | 0.48% / 0.24% |
| 4,800 points/sec | 116.1 | 119.5 | 0.98% / 0.24% |

Only 18–19 text views remained attached and the native host recorded no full
rebuilds beyond its two startup rebuilds. Prior observations were 113–118 and
107–109 updates/sec respectively. These are app callback rates, not presented
FPS or an isolated A/B speedup: workstation load differed between sessions.
Median callback work was about 3.6ms at normal speed and 2.1–2.5ms at fast speed;
render-call wall time includes graphics acquisition and is not a CPU-work metric.

The new eight-second stack sample contained zero lighting-collection observations
among 4,768 main-thread observations, versus 545 among 5,404 previously. Rendering
accounted for 688 observations, of which 349 were in `CAMetalLayer.nextDrawable`.
Core Animation commit accounted for 2,151 and AppKit subtree layout for 1,292;
these are overlapping inclusive stacks. The removed lighting work is no longer
a sampled hotspot, while native layout and presentation remain the next areas
to investigate. No changes to either were made in this step.

Raw windows and the sample are retained locally at
`/private/tmp/pax-lighting-profile.json` and
`/private/tmp/pax-lighting-active-sample.txt`. Temporary probes are removed from
the generated host before delivery. The canonical example source is unchanged.
Clean macOS and web release builds passed and were exercised through scrolling
and return to the beginning; the web check also reached the final cards. API
references regenerated, the documentation book built, and `git diff --check`
passed. The normal macOS release is left running.
