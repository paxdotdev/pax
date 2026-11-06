# How Pax Runs
<!-- summary: Follow a Pax program from source to a running scene, and understand where update and rendering work happens. -->
<!-- tags: runtime, compiler, cartridge, chassis, rendering, performance, debug, release -->

A Pax application has a reactive scene behind the interface you see. Its
properties carry values, its component instances organize the scene, and its
platform integration brings together drawn content and native controls.
Understanding those pieces helps you explain an update, investigate a slow
interaction, and choose what to measure before shipping.

The same core model runs on web, macOS, iOS, and iPadOS. This chapter follows
it from source to the screen. It builds on [Properties](state-properties.md),
[Events](event-handling-rust.md), and [Layout](layout-responsiveness.md);
[What is Pax?](what-is-pax.md) introduces the authoring model.

## Compiler, runtime, and chassis

Three parts work together to run your application:

| Part | Responsibility |
| --- | --- |
| Compiler | Analyzes the application and its templates, prepares the declarative program, and generates the connections to Rust component types and handlers |
| Runtime | Instantiates the scene, maintains reactive relationships, handles events, computes layout, and coordinates rendering and native-element updates |
| Chassis | Connects the runtime to a target's window or browser, graphics surfaces, input, native elements, and platform services |

Cargo compiles Rust application logic and expands Pax's macros for the selected
target: WebAssembly for the browser, or native code for Apple targets. The macros
analyze the Rust source and templates selected by that target's configuration,
then generate program data describing templates, bindings, events, and timelines.
`pax-cli` handles asset packaging, web serving, hot reload, and Apple deployment.
On macOS, ordinary `cargo run` runs the native application directly, without a
preceding CLI build or a development server. `init!` initializes within that
process; it never selects or launches another development target.

Reusable component libraries supply the source configuration selected by their
own compilation, including dependency features and custom `cfg` values. A library
can live in a different Cargo workspace: macro expansion does not resolve or
fetch that workspace's unrelated dependencies. This works with ordinary Cargo
path dependencies and does not require an application `build.rs` or CLI setup.

You may encounter the word **cartridge** in build output or engine code. It
names the application-specific part that connects your program to the shared
runtime. `runnable!` generates this connection during compilation; you do not
maintain a cartridge file by hand. On Apple platforms it remains a separately
linked library, which a Rust launcher or an existing host can mount. See
[entry and mounting](targets-build-deploy.md#entry-and-mounting).
Build for each destination target—there is no single executable
cartridge file that you can interchange between a browser and an iOS app.

At a high level:

```text
 .pax templates + Rust application + asset references
                         │
                   compiler / build
                         │
          target application + packaged assets
                         │
             runtime mounts the main component
                         │
             reactive scene + platform chassis
                         │
             drawn surfaces + native elements
```

Local assets are included in the target's output as appropriate; external
URLs may still be loaded at runtime. An image or font referenced by the
program is not necessarily embedded inside its Wasm or native executable.

### Architecture schematic

Explore how the compiler, scene graph, reactive runtime, rendering engine and
platform chassis connect. Select a part for its role, connections and related
docs; select a connection in the inspector to follow it to another part.
Use **+ / −** to zoom and **Fit** to restore the overview. **Open standalone**
gives the drawing more room, and **Exit sheet view** restores its controls.

For an offline overview, [download the full-sheet PNG](images/pax-architecture.png).

<pax-example
  path="architecture-diagram"
  title="Pax architecture"
  height="760"
  source="false"
  files="src/lib.pax,src/lib.rs,src/layout.rs,src/routing.rs">
</pax-example>

## From templates to running instances

A template is a definition. The runtime creates instances from it: component
state, element properties, and nodes with resolved layout and parent-child
relationships. This running structure is called the **expanded tree** in
inspection and runtime code.

One repeated element in a template can produce many nodes in that tree. A
conditional can mount or remove a branch as its condition changes. Slots
connect supplied content to the place where a component presents it. These
are useful distinctions when the visible scene contains more—or fewer—nodes
than you expect from counting tags in a file.

Instances share immutable settings definitions and prepared selectors from
their runtime template. Property definitions are prepared on the first selector
match or inline binding, then reused for that template. Unmatched selectors and
unused template bindings do not prepare property columns. Selector matches,
conditional values, and reactive bindings remain specific to each node; imported
expressions still use their provider's scope. This sharing applies to both
development and release builds. Template replacement, including hot reload,
starts with fresh definition caches while
preserving the [settings precedence rules](template-language.md#imported-settings).

Before mounting an ordinary template node, the runtime allocates stable property
slots and binds its settings. For fresh common-property slots, omitted settings
leave the unset defaults in place without a second binding operation. Conditional
settings still receive reactive bindings, even when initially inactive. Later
settings changes or reloads still clear removed values. This optimization applies in
both development and release builds and does not defer lifecycle handlers.

Compiler-created `if`, repeat, slot, and router nodes also allocate their property
slots before binding structural expressions. Initial mounting binds those inputs
once. Imported-settings refresh does not recreate these transport nodes or their
parent-layout bindings; their expressions continue to observe changes in the
containing component. Explicit template replacement still installs new structural
expressions. This applies to both development manifests and release-baked programs.

When imported settings change on an ordinary node, the runtime compares ordered
shared definitions and their provider scopes, then applies a sparse `SettingsPatch`.
Omitted properties keep their existing bindings; removed definitions receive an
explicit reset. An empty Patch avoids rebuilding the node's scope and listeners
or invalidating its layout and drawing. This compares binding definitions, not
current values, so conditions, `@base`, and reactive provider updates remain live.
Full template reloads, custom initialization, store-provider alias changes, and
local-scope timelines retain their existing full binding or recreation paths.
Dynamically assembled timeline-selector definitions are conservatively rebound.

The tree and property graph answer different questions. The tree says where
an instance belongs; the graph records which values depend on which inputs.
A child's width can depend on its parent's bounds, while its text depends
on application state shared from elsewhere. Those dependencies do not have
to follow the same shape as the tree.

State also has a lifetime. Removing and recreating a component can recreate
its local state. Keep state that must outlive a particular view in an
appropriate owner, and persist it explicitly when it must survive an
application restart. A `Property` is a reactive value, not persistent storage.
See [Components and Composition](components-composition.md) for ownership and
shared state.

## From a property change to the screen

Consider a handler that increments a counter used in a Text element. The
handler sets the source property. Pax marks dependent values **dirty**:
their cached values may no longer be current. When a dependent computed
value is needed, the graph evaluates it using the current inputs.

The runtime also drains registered reactive effects that need to push an
update into the scene. Multiple writes can settle into one downstream
evaluation when that value has not been read or its effect drained between
the writes. This does not make a sequence of arbitrary property writes an
atomic transaction.

The consequences depend on what changed. New text can require a native text
update and measurement. A different width can change descendant layout. A
new fill can mark drawn content for rendering. Changing an `if` condition
can alter the mounted tree and its event registrations.

Pax tracks dirty nodes and layers so unchanged drawing can be reused or
skipped. The GPU path retains rendering data, and the runtime can avoid a
canvas rendering pass when there is no canvas work pending. Native elements
receive their own updates through the chassis.

The runtime's GPU renderer admits at most two unfinished application frames per shared
device. Layers, tiles, and cooperative rendering slices of the same frame share
one allowance, retained until its latest submission completes. If both frames
are still running, drawing waits without blocking the host thread
or consuming pending scene changes. Input, application ticks, and native updates
continue. The next admitted frame reads the latest settled scene instead of
replaying a backlog of intermediate animation states. This applies in debug and
release GPU builds; the CPU renderer does not use this GPU limit. It bounds queued
work, not the cost of a shader or the time needed to finish one frame, so an
expensive effect can still reduce the displayed frame rate. Elapsed-time
animations keep their duration even when intermediate visual states are skipped.

The runtime retains scene geometry independently of canvas residency. Paint
bounds and occlusion bounds have separate indexes: a primitive's stable occlusion
shape can exceed the portion it currently paints. Native
occlusion uses those indexes after layer and clip membership have settled;
a structural rebuild constructs each native mask from that completed geometry,
using the same query path as an incremental update. Ancestor scroll placement
is a reactive value shared by descendants, including native presentation offsets
and page-scroll delegation. Reading it after a property write observes current
placement without waiting for a frame or traversing the ancestor chain again.
Clip owners retain local paths separately from their placement, so moving a Frame
or changing its opacity does not rebuild its local clip shape.

Transfers between existing clipping and presentation domains repair the moved
subtree's layer/container membership and both its former and current overlaps.
Unrelated coverage remains retained. Overlay entry, return, and animation-flight
handoffs preserve the normal camera and clipping rules; reuse does not bypass
invalidation. Container presentation follows native Scroller ancestry
even when that Scroller has no vector clip path. Changes to scroller-layer
allocation, escaping content, or unsupported structural shapes retain the
conservative full reconciliation path.

With `3d` enabled, perspective picking, local-coordinate conversion and native masking also
reuse per-instance projections, inverses and depth planes. The runtime compares
retained world geometry, camera and scroll inputs before projecting. These records
retain the camera's original clipping equations across coordinate changes and
retire when the instance unmounts. Ordinary affine queries do not allocate those
perspective records. Builds without `3d` use affine placement and omit the
spatial projection records. Input eligibility and paint opacity remain
separate policies.

Screen-point queries use a separate input index, including layout-only nodes.
Primitives prepare immutable rectangles, paths, or posed meshes; exact tests
consume those values without live-node callbacks. Unbounded coverage remains a
candidate in the same index. Settled queries visit candidates and their clipping
ancestors, and unchanged queries reuse hit geometry. During startup or pending
reactive work, the facade prepares shapes on demand through an uncached traversal;
it does not drain effects or advance a frame. See
[primitive input geometry](primitives.md#input-geometry) for the author contract.

Affine input bounds are indexed in unscrolled coordinates, grouped by their
enclosing scroller. Scrolling transforms the pointer query into each group's
coordinates rather than rebuilding every descendant's bounds. Nested scrollers
compose their transforms; exact clipping and hit tests retain their normal
semantics. Layout, shape changes, and reparenting still invalidate the affected
geometry or membership. Perspective input remains projection-aware: scrolling
before projection can change a descendant's screen footprint and requires
refreshing it. This optimization belongs to the shared runtime, not a web-only
input path, and does not change event frequency or cancellation policy.

Native masks retain dependency plans before mapping paths into a control or
serializing SVG. An unchanged plan performs neither operation; an opacity-only
change reuses mapped geometry, although the chassis may still rerasterize the
mask. The existing native-mask message format is unchanged. With `3d` enabled,
2D placements inherit a certified support plane through the render-parent
chain. A local tilt, depth offset, or camera starts a separate support. This lets
several controls on one face share an opaque solid's front coverage without
merging their bounds, clips, opacity, or mounted identity. Geometric relationships
retain the camera's original clipping equations and computational viewport.
Native plans and solid/support coverage have a combined ceiling of 4,096 entries
and 8 MiB of charged retained data, split equally between the two caches. An
oversized result is used without retention; eviction changes work, not coverage.
GPU solid/plane coverage uses that same runtime-owned relation budget. Tile
charts keep their computational viewports and original camera clipping distinct;
dropping a tile retires its relations. The GPU retains only weak coverage
dependencies beside its stencil resources, so unchanged geometric plans avoid
retessellation during paint and opacity updates. Plane partitioning, stencil
execution, and bounded fade composition remain backend responsibilities.
On web, opacity patches update existing SVG paths and clip groups; Apple reuses
parsed mask geometry while rerasterizing changed pixels. This resource reuse
does not alter the native transparency approximation.

Custom `InstanceNode` implementations can provide conservative local occlusion
bounds through `resolve_local_occlusion_bounds`. The default is unknown, so
custom shapes remain candidates and their exact occlusion hooks still run.
An implementation that supplies bounds must invalidate coverage when its shape
can grow beyond them. Paint bounds and hit shapes do not supply this contract.
The independent `local_occlusion_path_is_retained` opt-in permits retaining the
exact local path until coverage is invalidated. It requires invalidation on
**every** path change, including changes inside unchanged bounds. Standard
Rectangle, Ellipse, Line, and Path implement this contract; other custom hooks
remain live by default.

The GPU renderer retains independent transforms and opacity values for scenes
with more than 480 transformed drawing operations. It stores transforms and
geometric clips in small uniform-buffer pages that grow with scene demand,
including on the browser's conservative GPU limits. The page size is an
internal batching detail, not a limit on the number of elements you can author.
Moving or fading retained vectors can reuse their geometry; a draw spanning
multiple transform pages is split while preserving paint order. Memory and
rendering cost still grow with the scene, and ordinary visibility culling does
not unmount nodes or release their retained identities. Removing nodes makes
their slots available for reuse. This behavior is shared by debug and release
GPU builds; it does not change the Vello CPU renderer.

Surfaces using the same GPU context share immutable mask and mesh-gradient
pipelines and neutral fallback textures. Each surface still owns its mask
results, gradient pages, and mutable drawing state, so resizing or animating
one surface does not overwrite another's pixels. Creating a new scroll tile
therefore reuses that setup, but still requires its own backing storage and
rendering work before its content is ready.

The GPU backend explicitly releases retired multisampled color and stencil
attachments when a surface is resized, evicted, or shut down. Attachments still
used by encoded commands remain valid until those commands are submitted or
discarded; submitted GPU work does not require a CPU completion wait. Reusable
tiles and capture targets keep their attachments. This policy is shared by
native GPU and WebGPU builds and avoids relying on browser garbage collection
to reclaim retired attachment storage. It is not a cap on total application or
browser memory, nor does it change pixel density, antialiasing, or warm regions.

Low-level renderer integrations receive a `CommandBatch` from
`take_pending_command_buffers()`, rather than a raw command-buffer vector.
Combine batches with `CommandBatch::append`, submit through the renderer, and
perform the existing submission cleanup. The batch carries the attachment
leases with the commands, including when several surfaces share a submission.
Custom runtime backends can implement `RenderContext::begin_render_turn` and
`end_render_turn` to participate in nonblocking admission; the defaults always
admit work. A successful begin must have a matching end, and nested rendering
must share the outer admission. Cooperative slices pass the same application
identity and tick epoch so they can finish all layers of an admitted frame.
Direct `pax-gpu` integrations can check
`GpuContext::render_queue_ready` before encoding and call `finish_render_turn`
after submitting each slice, using the same `RenderFrame` identity for every
slice of that application frame. Standalone low-level submissions
do not automatically acquire a runtime frame budget.

The amount of work is therefore related to the dependencies and surfaces
affected by a change. A single property write can still have a broad effect:
resizing a container, removing an overlapping element, or changing a clip
can require work beyond the element you edited. Dirty tracking does not
guarantee that every update redraws only the smallest possible pixel region.

[Properties](state-properties.md) covers computed values, subscriptions, and
avoiding unnecessary writes. [Layout](layout-responsiveness.md) explains the
bounds and transforms that these updates can affect.

### Work that continues between interactions

The runtime advances clocks and invokes registered lifecycle handlers such
as `@tick` and `@pre_render`. A running transition or timeline produces new
values as time advances. These are legitimate sources of work even when
the person using the application is not touching anything.

The current web chassis continues to schedule animation-frame callbacks
while running. Dirty tracking reduces work inside those callbacks; it does
not establish zero CPU usage at rest or suspend the frame loop altogether.
An otherwise static page can still have clock, lifecycle, or platform work.

Web uses cooperative rendering by default, placing browser task boundaries
between complete rendering units. It does not split application
ticks, reactive settlement, or individual inseparable drawing units. The shared
runtime owns work selection and correctness; the chassis decides when to resume.
Between application frames, the host observes native scroll movement to refresh
surface demand without publishing reactive scroll state. The next application
frame delivers the latest position before ticking, so preparing pixels does not
itself invoke scroll handlers and invalidate the scene it is about to draw.
See [cooperative Web rendering](scrolling-viewports.md#experimental-cooperative-web-rendering)
for accounting boundaries and limitations.

Keep expensive data processing and repeated no-op state writes out of
per-frame handlers when they are not needed. Use a reactive expression for
a value derived from other properties, and use the motion APIs for the
transitions they describe. Neither choice makes computation free, but it
gives the runtime explicit dependencies to work with. See
[Animation and Motion](animation-motion.md).

### Experimental resumable startup on web

Set `yield_mount = true` under `[package.metadata.pax.web]` to use incremental
initial mounting on ordinary URLs. It defaults to false; custom JavaScript hosts
can override the generated setting with `init_web({..., yieldMount: true})`.
Without a loading screen, `?pax_mount_yield=1` forces it on and
`?pax_mount_yield=0` forces it off, taking precedence over both settings.
A configured loading screen always uses incremental mounting for its hidden
application, even with `?pax_mount_yield=0`. Other query values are ignored.
This remains experimental for development and release web cartridges; Apple
chassis do not enable it. The host resolves the option once and passes it into
either cartridge's initializer. It adds no template syntax or baked manifest
fields; rebuild the host after changing Cargo metadata.

The web host services mount continuations, reactive effects, and native patches
in regular MessageChannel tasks, sharing a 4 ms soft budget per call. Native
patches retain their order, and each batch drains before the engine produces
another. Ordinary tasks leave opportunities for font downloads and resize events
to deliver feedback during mounting. Browser animation frames publish
completed scene work. A service call does not itself advance the application
frame clock or repeat `@mount`, `@tick`, or `@pre_render` handlers. Wall time still
passes during ordinary initialization, so this does not pause elapsed-time animations.
When a [loading screen](components-composition.md#loading-context-components-experimental)
is configured, the loader and main application instead have independent clocks
and reactive graphs. The main clock stays paused until mounting completes.
By default the loader remains active over the prepared viewport until then.
`loader_exit = "first_viewport"` under `[package.metadata.pax.web]` starts its
exit as soon as that viewport settles, leaving the prepared scene visible while
remaining mounts complete. Scrolling, application input, and accessibility are
released when the loader's exit finishes, even while the main clock remains
paused. Their rendering scenes are independent while sharing
the runtime's GPU device. Cooperative preparation and rendering share a 4 ms
allowance per animation-frame opportunity across these hosted applications;
an indivisible callback can exceed this soft budget.
The `init_web` Promise waits for mounting and the initial presentation gate;
aborting its signal cancels pending slices and rejects initialization while
cleanup proceeds.

Native patches can reach the browser between calls so hidden text can load fonts
and return measurements. Text responses retain their request-generation checks.
Viewport and initialized native scroll state also continue updating. A newly
created scroller begins reporting browser scroll feedback only after its initial
layout and authored scroll position have been applied. A yield between creation
and that update must not replace a nonzero starting position with the DOM's
temporary `(0, 0)` default. After initialization, normal browser scrolling remains
authoritative. Responsive layout should
derive from reactive bounds rather than copy them in `@pre_render`: startup
feedback does not rerun application lifecycle handlers. Application input is
queued with its existing recipients' mount lifetimes; removed or remounted
recipients are skipped. Browser default actions for queued input are suppressed
at capture time, and clicks on the unpublished background do not later target
new controls. Once input is released, the queue drains in order, one event per
host frame, including during background mounting; consecutive pointer moves may
be coalesced. Replaying input does not advance the paused animation clock.

The first preparation checkpoint prioritizes the initial viewport. Without a
loading screen, this checkpoint permits the first reveal described below. A
configured loading screen additionally waits for the initial visible surfaces
to be submitted before revealing that viewport underneath the active loader.
The rest of the scene mounts cooperatively with ordinary ticks still held.
Input waits for loader exit, or full mounting when there is no loader.
Loader exit follows the configured `loader_exit` policy; see its
[publication contract](components-composition.md#loading-context-components-experimental).
A node with a known
outer box entirely outside that viewport can defer its whole subtree; this
coarse heuristic does not search descendants for transformed visual overflow.
Unknown dimensions remain candidates for traversal. Mounted `Overlay`
contributions use their viewport bounds. Their pending
mounts remain tied to their authored owners even though they have no ordinary
render parent. An Overlay beneath an entirely deferred ancestor is discovered
when that ancestor mounts.
Child reconciliation invalidates layout membership at the affected parent;
changes to descendant hull values then propagate through their reactive
dependencies. Overlay and animated-move ordering use a separate viewport-order
signal, so the presence of an Overlay does not make ordinary child reconciliation
invalidate layout listeners throughout the application. Moving content updates
its old and new parents and its projected-content consumers. A deferred child
finishing its mount also notifies its parent explicitly.
Selection continues recursively through visible containers: an autosized outer container does not by itself
require mounting its entire content. A start-aligned Stacker with a fixed cross
axis resolves a visible prefix, accepting native measurements before deciding
whether another child is needed. A child that already covers the viewport can
keep its trailing content deferred. If a following child is needed, the preceding
child's exact main-axis extent becomes a dependency and is resolved first.
Centered, rotated, reversed, or cross-axis-measured flows conservatively require
complete content. Scroller's total scroll extent can grow as deferred content
mounts; it is separate from its viewport and child layout frame.

When a custom container feeds a child's size back into an ancestor, subscribe to
`NodeInterface::measured_size_property()` and wait for `Some(size)`. Layout bounds
can still be provisional and inherited from the parent while the child is
unmeasured. Treating those bounds as content measurements can create a feedback
loop, especially when adding padding. An autosize measurement can grow as more
content mounts; it does not by itself certify that the whole subtree is complete.

Components describe these dependencies through the experimental
`InstanceNode::plan_startup` hook. A `StartupPlan` combines a visible region,
exact measurement axes, and complete-subtree requirements. The scheduler owns
continuations, native measurement checkpoints, and mount-lifetime checks; it does
not identify component types. Mask forwards visibility into its content subtree
and requires its complete detached source subtree before first presentation.
A resolved empty alpha source hides the content rather than exposing it unmasked.

Rust components can register a planner in their mount handler with
`NodeContext::set_startup_planner`. For example, a component whose children must
all be available together can conservatively request them with:

```rust,ignore
use pax_runtime::engine::startup::StartupDemand;

ctx.set_startup_planner(|_node, plan| {
    plan.require_children(StartupDemand::complete());
});
```

More selective planners can forward the inherited request with
`plan.forward_children()` and add requirements for individual children with
`plan.require(child, demand)`. Requirements accumulate, and a complete-subtree
request cannot be weakened by a descendant. Planners describe work; they must
not mount nodes or invoke application handlers. Callbacks are scoped to the
current mount and retained for viewport feedback checkpoints until startup
mounting completes; use weak node references
in captures. This is an experimental runtime API, not a new template property.

Before this first reveal, native Text fonts and their latest measurements must
settle, followed by a browser layout opportunity and another readiness check.
Feedback can promote deferred work if layout or viewport size changes. Font
failures permit fallback; slow fonts can delay the first reveal. Native controls
remain measurable behind the visibility gate.

After the first reveal, all remaining branches mount eagerly in bounded slices.
Background admission pauses when the queued effects and cutoff propagation reach
an internal threshold. The runtime drains that batch in its existing dependency
order before admitting more mounts, preserving coalescing within the batch. This
keeps background mounting from accumulating the whole page's reactive backlog;
it can perform additional evaluations compared with mounting everything first.
Web layout and asset feedback (including viewport resize and native text
measurement) pauses background admission for a new viewport checkpoint. The
mounted graph settles, the same planners resolve newly required content, and
the host waits for native measurement and visible surface submission before
resuming the parked mounts. Requests coalesce while a checkpoint is pending.
Optional offscreen rendering can remain pending at publication; visible drawing
and its browser submission acknowledgement must both finish. After full startup,
ordinary frames likewise wait for required visible drawing, while optional
offscreen work stays queued without holding the application frame clock.
Renderer allocation acknowledgements do not pause mounting. This prioritizes
settlement over unrelated mounts; it does not isolate a separate reactive graph
for the visible subtree, and required autosize or mask dependencies can still
extend outside the viewport. Reactive work admitted before the feedback also
has to drain. The admission threshold is soft: one mount, resize, or indivisible
handler can enqueue a large amount of work, and the initial required viewport is
not constrained by it. A late resize can therefore still take multiple slices.

Native leaves appear as their own readiness permits; the canvas keeps its last
presentation until each feedback checkpoint settles. The whole-scene visibility
gate stays open.
The final readiness check holds application input and the `init_web` Promise,
without hiding already presented content. These checkpoints do not advance an
application tick for each slice or repeat the initial handlers on existing nodes.
Animations driven by those handlers can remain in their initial pose until
ordinary ticks resume; early presentation does not imply that their entrance
animation has completed.
This is startup prioritization, not ongoing viewport virtualization: every
selected branch eventually mounts and remains subject to ordinary lifecycle.

Individual callbacks, structural dependency groups, scene reconciliation, a
single native patch, and rendering can exceed the soft budget; subsequent
ordinary ticks remain synchronous. Use this to investigate responsiveness, not
as a strict per-frame time bound or a promise of unchanged visible layout under
arbitrary mount-handler side effects. A large required measurement subtree can
still delay the first presentation. Timing-dependent application initialization
can change the amount of work performed between comparison runs.

## One scene, multiple surfaces

Drawn geometry and native elements share Pax layout, transforms, and ordering.
Their presentation is divided across graphics surfaces and platform-owned
elements. The compositor coordinates those pieces, including the places where
clipping, scrolling, and overlap cross surface boundaries.

The runtime retains node-local placement and unprojected world geometry before
deriving the current 2D presentation. Prepared geometry records carry that
world geometry alongside their canvas-space coverage. This keeps plane
inspection independent of surface ownership and opacity scopes. The experimental
orthographic spatial path projects flat surfaces for native presentation and
partitions GPU composition where their geometric depth order changes. Existing
2D trees retain the affine path. See
[Rotating flat surfaces in 3D](layout-responsiveness.md#rotating-flat-surfaces-in-3d)
for authoring and current backend/compositing limits.

For example, a Rectangle can be drawn into a graphics layer while a Textbox
is a browser control or Apple-native view. The runtime sends the native
element its required updates; user input returns through the chassis to the
runtime and application handlers. The platform still handles native concerns
such as editing interactions and input-method behavior.

On macOS, native scene updates are applied during the chassis frame cycle,
before that frame's GPU submission. Animated native controls therefore receive
the current geometry without waiting for a later SwiftUI view update.

This division preserves useful native behavior, while making backend and
element type important when choosing an effect. An effect supported on drawn
geometry does not automatically support a native control or every form of
text and image rendering. The detailed boundaries belong in
[Compositing and Effects](compositing-effects.md) and
[Accessibility and Native Controls](accessibility-native-controls.md).

### Rendering backends

On the web, Pax selects WebGPU when browser policy permits and initialization
succeeds. **Vello CPU** is the portable software fallback when WebGPU is absent,
adapter/device initialization fails, or the conservative iOS Safari policy
selects CPU. Selection finishes before application initialization and template
mounting. CPU rasterizes into reusable pixel buffers; Canvas2D presents the
finished pixels. Native browser text and controls remain native on either path.
GPU startup acquires one adapter and device for the mounted application;
its canvas surfaces share that context. It does not acquire a separate adapter
just to detect support. A pending browser request remains pending rather than
triggering a timed CPU fallback.

Use `?pax_renderer=cpu` to exercise software rendering, `?pax_renderer=gpu` to
require WebGPU, or `?pax_renderer=auto` for normal selection. Explicit GPU
selection rejects startup if initialization fails. The `cpu` Cargo feature
on `pax-kit` also selects CPU by default for web builds; a query override takes
precedence. The former `piet` feature and Piet adapter have been removed.

`@gpu` and `@cpu` identify this session's selected renderer. There is no live
backend switch after mount; later device loss does not automatically remount
the application on CPU. Check renderer logs when diagnosing startup failures.
CPU scroller tiles use a 2048-pixel backing-dimension budget; the root viewport
surface is sized separately.
CLI web builds enable WebAssembly SIMD and optimize the software rasterizer
for speed; see [web build profiles](developer-workflow.md#web-build-profiles)
for browser requirements and custom compiler flags.

The WebGPU renderer stages ordered buffer updates in pooled, unmapped upload
buffers. Copies stay ordered with their draws, including when several draws
use different versions of one destination buffer in a single submission.
This avoids a Safari validation failure with repeated copies from mapped
staging buffers; it does not provide recovery from graphics-device loss.
Native GPU builds retain their mapped staging-belt upload path.

Recycled WebGPU canvases stay hidden until their new surface owner submits its
first frame. This prevents old pixels from a previous tile appearing during
asynchronous initialization, including on browsers that retain a displayed image
after canvas reset. Already-presented tiles keep their coverage while updating;
this gate adds no GPU wait or extra clear pass, and does not change tile budgets
or the Vello CPU rendering path.

Mesh-paint preparation and alpha-mask rendering allocate a command encoder
only when they record GPU work. Empty mesh inputs and unchanged cached mesh
or mask content do not allocate one. This matters on browsers where dropping
an unused encoder can leave native command resources waiting for JavaScript
garbage collection. Changed content still follows the normal ordered
submission path; this does not change rendering quality or tile budgets.


The Apple-native chassis use Pax's GPU rendering integration alongside native
views. Running a web build in Safari on an iPhone is a different target path
from running a native iOS build on the same phone. Test the destination your
users will actually use.

Graphics effects, surface limits, and rendering costs can differ between
backends. There is no universal frame-rate guarantee for a Pax application.
Scene complexity, assets, device capabilities, and the chosen renderer all
contribute to the result.

### Viewports and large collections

Clipping and culling can reduce the drawing submitted for content outside a
viewport. Scroller surfaces can also be tiled to limit the backing surfaces
needed for a large scrollable area. These are rendering optimizations.

The repeated component instances, properties, and application data still
have their own cost. Putting a large `for` loop inside a Scroller does not
automatically turn it into a virtualized list. Check node count and state
work as well as what is visible. [Scrolling and Viewports](scrolling-viewports.md)
covers content measurement and practical collection limits.

## Debug and release

Debug builds support the development loop, including richer program metadata
and the configured hot-reload lanes. Release builds optimize the compiled
application and bake its declarative program into the artifact. The build
paths differ internally, so a successful debug run is only one part of
checking an application intended for release.

Release applications disable the live `.pax` and application-logic reload
lanes. Hot reload is a development capability; keeping it available does not
require shipping it in the release application. See
[Developer Workflow](developer-workflow.md#hot-reloading) for the current reload choices.

From your project directory, build the web release with:

```sh
pax-cli build --target web --release
```

Test the resulting application in addition to the debug session. Use the
release output when measuring shipping performance or download size. The
CLI reports bundle statistics, but those figures do not describe every
asset or network request your application may load. Include images, fonts,
scripts, and other resources when assessing the complete experience.

For investigating Wasm size, the CLI also offers `build --profiling` on web,
which produces an optimized build with Wasm names retained for size analysis.
This is a size-investigation mode, not a runtime frame profiler. Measure the
ordinary release build for the shipping comparison.

## Inspect an update in a working example

The repository's `examples/increment` app has a useful pair of behaviors:
clicking its rounded rectangle changes the count and starts a rotation, while
a `@pre_render` handler continuously changes the rectangle's color.

From a Pax repository checkout, start it with:

```sh
pax-cli run --path examples/increment --target web
```

With that session running, use another terminal in the same checkout:

```sh
pax-cli dev inspect tree --path examples/increment --max-depth 3
pax-cli dev logs --path examples/increment --limit 50
```

The tree shows the running instances behind the template. The log command
reads messages captured by the development session; a quiet session can
return no entries. To include renderer selection, add `?pax_log=info` to the
local application's URL and reload it, then run the log command again.
The default debug logging level includes warnings and errors. You can also
read these diagnostics in the browser console. These commands help establish
what is running; they do not measure the time spent inside each part of a frame.

Open `src/lib.pax` and `src/lib.rs` inside the example and follow a click:

1. The Group binds `@click` to `increment`.
2. The handler updates `num_clicks`, which feeds the Text's expression.
3. It starts an eased transition on `current_rotation`, which feeds the
   Group's `rotate` property.
4. The rotation settles, but `handle_pre_render` keeps updating `ticks`.
   The Rectangle's fill depends on that value, so its color keeps changing.

This app intentionally remains animated. To understand activity after an
interaction in your own application, follow the same trail: event handler,
property dependencies, active motion, and per-frame handlers.

## Measure the question you have

Separate startup, interaction, continuous motion, and resting-state work.
They can have different causes and require different checks:

- For a slow initial load, inspect resource requests and decoding, executable
  size, and scene initialization. Compare a cold load with a warm one.
- For an expensive interaction, reproduce one action and inspect the state,
  layout, and mounted-node changes it causes. Use the browser's performance
  tools or the target's native profiler to measure where time is spent.
- For motion or scrolling, test realistic content, nested surfaces, and
  representative devices. Note the target, renderer, viewport, and build mode
  alongside the result.
- For unexpected activity after the interface settles, look for active
  transitions, clock-dependent expressions, and per-frame handlers. Distinguish
  ordinary frame scheduling from repeated application or rendering work.

Screenshots and event driving are useful for repeatable behavior checks.
They do not establish frame rate, memory use, or accessibility. Keep those
measurements separate, and compare the same workload before and after a
change.

### Web startup attribution

For a slow initial mount, reload with `?pax_startup_profile=1&pax_mount_yield=1`.
This opt-in capture works in debug and release web builds with resumable startup.
A configured loading screen already enables resumable mounting. The flag does
not change the work budget, publication gates, or lifecycle ordering.

After mounting completes, inspect `window.paxStartupProfiles` in the browser's
developer console. Loader and main applications have separate entries; the
latest eight completed captures are retained across mounts and retries. For
example, export them from Chrome DevTools with:

```js
copy(JSON.stringify(window.paxStartupProfiles, null, 2))
```

Each report includes service time, CPU unit totals, accepted settlement requests,
presented checkpoints, and completed/deferred/stale mount-continuation counts.
Completed mount units are grouped by containing template component and node type.
`phases` breaks node construction and mounting into allocation, property binding,
scope setup, parent binding, projected children, settings discovery/rebinding,
property preparation, lifecycle handlers, and instance mounting. Each phase has
`count`, `total_ms`, `self_ms`, and `max_ms`. Total time includes nested phases;
self time excludes them, so it is useful for finding the work inside a costly
mount without counting child construction twice. These phases also cover root
construction before the first service call.
`reactive` separates queued effects from cutoff initialization, suppressed
propagation, and propagated changes. Category totals include all recorded work;
`top_properties` contains the 64 labels with the most evaluations, with expanded
node IDs where available. `omitted_labels` reports how many labels were
excluded from that list. `<unnamed effect>` and `<unnamed cutoff>` retain counts
for work without diagnostic names. Names are retained for the profiled graph in
release builds too, then released when its capture is consumed, the application
unmounts, or the graph closes.

These counters cover work evaluated by startup's reactive queue. A synchronous
dependency read inside a mount or another evaluator is included in its caller's
time, but is not counted as a separate queued evaluation. `inclusive_ms` measures
wall time inside queued evaluators; it overlaps the unit and service totals.
Do not add those nested timings or phase timings to unit/service totals. Host
scheduling waits, native patch application and measurement, and GPU work are
outside this capture. Cancelled or failed mounts do not produce a completed
capture. Profiling adds overhead and can change slice/checkpoint timing: use a
separate run without the query flag for baseline wall times, and record viewport
readiness separately from full mount and input publication.

### Native render phase timings

For native debug or release builds, launch the app process with
`PAX_RENDER_TIMINGS=1` to emit aggregated `[PaxRender]` diagnostics approximately
once per second. This flag is off by default and has no effect in WebAssembly.
For an installed simulator app, for example:

```sh
SIMCTL_CHILD_PAX_RENDER_TIMINGS=1 xcrun simctl launch --console-pty \
  <simulator-udid> <bundle-id>
```

Each phase reports `count/total_ms/avg_ms/max_ms`. `render` measures the runtime
render call, `lighting` collection, `lighting_upload` delivery to the backend,
and `prepare` retained-node preparation. On the GPU backend, `flush`, `encode`,
`submit`, `cleanup`, `present`, and `layer_init` distinguish drawing and renderer
initialization costs. A render call can collect several layers and flush several
surfaces, so those phase counts and averages are not per-frame counts. Nested
timings overlap: layer initialization can occur inside render or flush, and
encode/submit/present occur inside flush. Do not sum all reported phases.

`[PaxRenderTiles]` reports counters for the same window: renderer creations,
changes to the keyed tile set, matching key/host tiles discarded during those
changes, origin retargets, resizes, retained-scene resets, and reused tiles. A matching discarded
tile identifies potential reuse; it does not prove that its contents or geometry
were unchanged. Incremental additions should not discard matching survivors;
`layer_init` can now represent just a batch of missing tiles. Reuse counts include
survivors moved during set reconciliation, not every unchanged tile on every frame.
`image_texture_uploaded` and `image_texture_upload_bytes` count new image pixel
allocations/uploads observed at tile flush, while `image_texture_reused` counts
new tile bindings that reuse a sibling's resident texture. Repainting a tile with
its existing binding is not a shared-cache hit. These are work/transfer counters,
not current GPU memory usage; image bindings are tile-local even when pixels are
shared. Counts include all logical layers, and startup should be excluded
when investigating scroll-induced churn. These counters are also disabled unless
`PAX_RENDER_TIMINGS` is enabled.

These are CPU wall-clock spans, including any waits inside the measured calls,
not GPU execution timestamps or displayed-frame counts. They exclude the
chassis's tick, native-view update, and surface synchronization costs. Simulator
GPU and presentation timings do not predict performance on physical hardware.
Keep startup separate from interaction captures, and relaunch without the flag
after measuring to disable the instrumentation.

## Read more

- [State and Properties](state-properties.md) — reactive values and dependency
  management.
- [Compositing and Effects](compositing-effects.md) — graphics layers, native
  elements, and effect boundaries.
- [Scrolling and Viewports](scrolling-viewports.md) — content extents,
  scrolling, and collection costs.
- [Getting Started](getting-started.md) — workstation setup, commands, and
  build output.
- [Developer Workflow and Tools](developer-workflow.md) — hot reload,
  inspection, screenshots, and local reference.
- [Public API reference](api/index.md) — Rust types and methods.


## Shared publication and application lifetime

Application `Property<T>` storage is shared; graph computations, `LocalProperty`,
`ExpandedNode`, layout, and rendering stay on the owning UI thread. A worker
publication coalesces by shared allocation and requests a host wake. The host
validates the current application before entering the engine. The UI imports a
bounded batch of published snapshots before settling local effects; evaluators
never execute on a publishing worker. A raw shared read returns the latest
published snapshot even before a local view has imported it.

Mounted nodes own their local attachment lifetimes. Unmount detaches those
connections; retained raw shared values remain usable as data. The generated cartridge invokes one typed root initializer inside its property
graph. The host adopts that exact root allocation, attaches its wake route and
viewport, then activates staged work. Apple launchers use a real Rust `main`
with a SwiftUI library bridge; web calls `init_web` explicitly.
See [State and Properties](state-properties.md#shared-state-and-local-views) and
[Application setup](event-handling-rust.md#application-setup) for the implemented
APIs and the current limits on scoped async work. See [Entry and mounting](targets-build-deploy.md#entry-and-mounting) for standalone and embedded hosts.
