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

The CLI coordinates the build. Rust application logic is compiled for the
selected target: WebAssembly for the browser, or native code for the Apple
targets. Pax's generated program data describes templates, bindings, events,
and timelines so the runtime can instantiate them.

You may encounter the word **cartridge** in build output or engine code. It
names the application-specific part that connects your program to the shared
runtime. The CLI generates this connection; you do not maintain a cartridge
file by hand. Build for each destination target—there is no single executable
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

Keep expensive data processing and repeated no-op state writes out of
per-frame handlers when they are not needed. Use a reactive expression for
a value derived from other properties, and use the motion APIs for the
transitions they describe. Neither choice makes computation free, but it
gives the runtime explicit dependencies to work with. See
[Animation and Motion](animation-motion.md).

## One scene, multiple surfaces

Drawn geometry and native elements share Pax layout, transforms, and ordering.
Their presentation is divided across graphics surfaces and platform-owned
elements. The compositor coordinates those pieces, including the places where
clipping, scrolling, and overlap cross surface boundaries.

For example, a Rectangle can be drawn into a graphics layer while a Textbox
is a browser control or Apple-native view. The runtime sends the native
element its required updates; user input returns through the chassis to the
runtime and application handlers. The platform still handles native concerns
such as editing interactions and input-method behavior.

This division preserves useful native behavior, while making backend and
element type important when choosing an effect. An effect supported on drawn
geometry does not automatically support a native control or every form of
text and image rendering. The detailed boundaries belong in
[Compositing and Effects](compositing-effects.md) and
[Accessibility and Native Controls](accessibility-native-controls.md).

### Rendering backends

On the web, Pax currently selects its WebGPU renderer where browser support
and policy allow. It uses a **Piet/CPU browser renderer** when WebGPU is
unavailable, when the current iOS Safari detection policy selects that path,
or when built explicitly with the Piet backend. Piet draws through browser
canvases; native browser elements remain native on either path.

Backend selection is not a guarantee of recovery from every graphics-device
initialization failure. When diagnosing a browser-specific problem, check the
selected renderer and startup errors rather than assuming that another path
was selected successfully.

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

The repository's `examples/src/increment` app has a useful pair of behaviors:
clicking its rounded rectangle changes the count and starts a rotation, while
a `@pre_render` handler continuously changes the rectangle's color.

From a Pax repository checkout, start it with:

```sh
pax-cli run --path examples/src/increment --target web
```

With that session running, use another terminal in the same checkout:

```sh
pax-cli dev inspect tree --path examples/src/increment --max-depth 3
pax-cli dev logs --path examples/src/increment --limit 50
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

<div class="docs-example-placeholder">
<p><strong>Visualization planned:</strong> follow a click through the existing Increment example, highlighting the count and rotation dependencies separately from its continuous color update.</p>
<!-- Production brief:
- Reuse the canonical example's source and show its rendered result alongside
  the graph. Label graph highlights as an explanatory visualization, not a
  measured performance trace.
- Three treatments: a small wiring diagram with highlighted edges; source and
  output with synchronized callouts; or a frame-by-frame annotated filmstrip.
  Prefer the wiring diagram. Include a static accessible reading order.
- No new starter, duplicate canonical app, or website gallery ownership here. -->
</div>

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
- [Runtime and cartridge design notes](architecture-runtime-cartridge.md) —
  historical maintainer context, including architectural directions that are
  not current builder-facing capabilities.
