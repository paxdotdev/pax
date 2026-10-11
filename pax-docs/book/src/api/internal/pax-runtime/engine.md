# engine
<!-- summary: API docs for pax-runtime::engine. -->
<!-- tags: api, pax-runtime -->

## Submodules
- [engine::layer_surface](engine/layer_surface.md)
- [engine::layer_tiling](engine/layer_tiling.md)
- [engine::node_interface](engine/node_interface.md)
- [engine::occlusion](engine/occlusion.md)
- [engine::surface_metrics](engine/surface_metrics.md)
- [engine::surface_preparation](engine/surface_preparation.md)
- [engine::surface_scheduling](engine/surface_scheduling.md)
- [engine::surface_work](engine/surface_work.md)
- [engine::pax_gpu_render_context](engine/pax_gpu_render_context.md)
- [engine::piet_render_context](engine/piet_render_context.md)

## Structs
### `Globals`
Engine-wide reactive globals exposed to every component frame.

#### Properties
##### `elapsed_frames`
Type: `LocalProperty`<`u64`>

##### `elapsed_millis`
Type: `LocalProperty`<`u64`>

##### `viewport`
Type: `LocalProperty`<[`TransformAndBounds`](../../../api/internal/pax-runtime/layout.md#transformandbounds)<[`NodeLocal`](../../../api/internal/pax-runtime/engine/node_interface.md#nodelocal), [`Window`](../../../api/pax-runtime-api/platform.md#window)>>

##### `gyro`
Type: `LocalProperty`<[`Gyro`](../../../api/pax-runtime-api/platform.md#gyro)>

##### `accel`
Type: `LocalProperty`<[`Accel`](../../../api/pax-runtime-api/platform.md#accel)>

##### `route_location`
Type: `LocalProperty`<[`RouteLocation`](../../../api/internal/pax-runtime/router.md#routelocation)>

##### `browser_allows_scroller_vector_layers`
Type: `LocalProperty`<`bool`>

##### `browser_allows_nested_scroller_vector_layers`
Type: `LocalProperty`<`bool`>

##### `platform`
Type: [`Platform`](../../../api/pax-runtime-api/platform.md#platform)

##### `os`
Type: [`OS`](../../../api/pax-runtime-api/platform.md#os)

##### `target`
Type: [`TargetInfo`](../../../api/pax-runtime-api/platform.md#targetinfo)

##### `get_elapsed_millis`
Type: `Rc`<`dyn` `Fn`() -> `u128`>

#### Implementations
##### `stack_frame`
<pre><code class="api-signature language-rust ignore">pub fn stack_frame(&amp;self) -&gt; Rc&lt;<a href="../../../api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;</code></pre>

Build the root stack frame containing built-in globals plus internal engine state.

---

### `Handler`
Runtime event handler thunk generated from template bindings.

#### Properties
##### `function`
Type: `fn`()

##### `location`
Type: [`HandlerLocation`](../../../api/internal/pax-runtime/engine.md#handlerlocation)

#### Implementations
##### `new_component_handler`
<pre><code class="api-signature language-rust ignore">pub fn new_component_handler(function: fn()) -&gt; Self</code></pre>

Build a handler whose `self` and context both belong to the target component.

##### `new_inline_handler`
<pre><code class="api-signature language-rust ignore">pub fn new_inline_handler(function: fn()) -&gt; Self</code></pre>

Build a handler whose `self` is the component that authored the inline binding.
The context still describes the primitive or child component receiving the call.

---

### `HandlerRegistry`
Map from event key to one or more handlers registered on an instance node.

#### Properties
##### `handlers`
Type: `HashMap`<`String`, `Vec`<[`Handler`](../../../api/internal/pax-runtime/engine.md#handler)>>

---

### `PaxEngine`
Singleton struct storing everything related to properties computation & rendering

#### Properties
##### `runtime_context`
Type: `Rc`<[`RuntimeContext`](../../../api/internal/pax-runtime/properties.md#runtimecontext)>

##### `root_expanded_node`
Type: `Option`<`Rc`<`ExpandedNode`>>

##### `scroller_tiling_policy`
Type: [`ScrollerTilingPolicy`](../../../api/internal/pax-runtime/engine/layer_tiling.md#scrollertilingpolicy)

#### Implementations
Central instance of the PaxEngine and runtime, intended to be created by a particular chassis.
Contains all rendering and runtime logic.

##### `activate_application`
<pre><code class="api-signature language-rust ignore">pub fn activate_application(&amp;self)</code></pre>

Activates the mounted application exactly once. Candidate hosts defer this
until their revision is committed.

##### `mount_root_component`
<pre><code class="api-signature language-rust ignore">pub fn mount_root_component(&amp;mut self, main_component_instance: Rc&lt;<a href="../../../api/internal/pax-runtime/component.md#componentinstance">ComponentInstance</a>&gt;) -&gt; Rc&lt;ExpandedNode&gt;</code></pre>

Mount a root component tree into an existing runtime kernel.

##### `present_settled_surfaces`
<pre><code class="api-signature language-rust ignore">pub fn present_settled_surfaces(&amp;self, rcs: &amp;mut dyn <a href="../../../api/pax-runtime-api/rendering.md#rendercontext">RenderContext</a>, budget: &amp;mut <a href="../../../api/internal/pax-runtime/engine/surface_work.md#surfaceworkbudget">SurfaceWorkBudget</a>, now_ms: impl Fn() -&gt; f64) -&gt; SurfaceWorkReport</code></pre>

Prepare surfaces from settled scene state without advancing a tick or
invoking lifecycle handlers. Supporting backends capture missing primitive
drawings in bounded units before atomically submitting a complete tile.
Capture reads current properties and inherited rendering hooks; cached
replay needs no component callbacks. Reselect after each submission while
the allowance permits. Blocked layers wait until the next pass without
preventing eligible siblings from using the remaining allowance.

##### `render_with_surface_preparation`
<pre><code class="api-signature language-rust ignore">pub fn render_with_surface_preparation(&amp;mut self, rcs: &amp;mut dyn <a href="../../../api/pax-runtime-api/rendering.md#rendercontext">RenderContext</a>, budget: &amp;mut <a href="../../../api/internal/pax-runtime/engine/surface_work.md#surfaceworkbudget">SurfaceWorkBudget</a>, now_ms: impl Fn() -&gt; f64, observe: &amp;mut dyn FnMut(&amp;&#39;static str)) -&gt; (SurfaceWorkReport, SurfaceWorkReport)</code></pre>

Service newly visible surfaces after current scene setup, before ordinary
drawing. Supporting backends also finish visible dirty layers before
unrelated drawing, preserving filtered ancestor hooks. Unclippable or
unresolved paths retain the full-tree fallback. Preparation uses the same
settled-scene guards and cooperative allowance as completion-driven work;
a dirty layer is never replayed before its whole ordinary draw completes.
The observer marks successive CPU phase boundaries, not GPU presentation.
Chassis opting in should replace their automatic preparation pass with this
opportunity, not add another per-frame allowance. Unused preparation time
services remaining work after ordinary drawing; both phases share one
allowance. Returns the visible and remaining phase reports separately.

##### `set_viewport_size`
<pre><code class="api-signature language-rust ignore">pub fn set_viewport_size(&amp;mut self, new_viewport_size: (f64, f64))</code></pre>

Called by chassis when viewport size changes, e.g. with native window resizes

##### `shutdown`
<pre><code class="api-signature language-rust ignore">pub fn shutdown(&amp;mut self, reason: <a href="../../../api/pax-runtime-api/application.md#stopreason">StopReason</a>)</code></pre>

Stops new application work, unmounts, then releases application services.

##### `tick`
<pre><code class="api-signature language-rust ignore">pub fn tick(&amp;mut self) -&gt; Vec&lt;<a href="../../../api/internal/pax-message/index.md#nativemessage">NativeMessage</a>&gt;</code></pre>

Settles reactive state and imperative update branches, dispatches lifecycle
handlers, then reconciles geometry/compositing and returns native patches.
Rendering/replay is serviced separately by the chassis.

##### `tick_with_phase_observer`
<pre><code class="api-signature language-rust ignore">pub fn tick_with_phase_observer(&amp;mut self, completed: impl FnMut(&amp;&#39;static str)) -&gt; Vec&lt;<a href="../../../api/internal/pax-message/index.md#nativemessage">NativeMessage</a>&gt;</code></pre>

Run the ordinary tick, reporting completed phase boundaries for opt-in
host timing. Observation does not change lifecycle or clock advancement.
The observer must not mutate the application or re-enter the engine.

##### `unmount`
<pre><code class="api-signature language-rust ignore">pub fn unmount(&amp;mut self)</code></pre>

Detach the mounted root component tree, leaving the runtime kernel empty.

## Enums
### `HandlerLocation`
Selects the Rust state owner independently of the handler's target `NodeContext`.

#### Variants
##### `Inline`
A binding on a template element: use the component that authored the element.

##### `Component`
A component's own settings or conventional method: use the target component.
