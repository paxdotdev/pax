# engine
<!-- summary: API docs for pax-runtime::engine. -->
<!-- tags: api, pax-runtime -->

## Submodules
- [engine::layer_tiling](engine/layer_tiling.md)
- [engine::node_interface](engine/node_interface.md)
- [engine::occlusion](engine/occlusion.md)
- [engine::pax_gpu_render_context](engine/pax_gpu_render_context.md)
- [engine::piet_render_context](engine/piet_render_context.md)

## Structs
### `Globals`
Engine-wide reactive globals exposed to every component frame.

#### Properties
##### `elapsed_frames`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`u64`>

##### `elapsed_millis`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`u64`>

##### `viewport`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`TransformAndBounds`](/api/internal/pax-runtime/layout.md#transformandbounds)<[`NodeLocal`](/api/internal/pax-runtime/engine/node_interface.md#nodelocal), [`Window`](/api/pax-runtime-api/platform.md#window)>>

##### `gyro`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Gyro`](/api/pax-runtime-api/platform.md#gyro)>

##### `accel`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Accel`](/api/pax-runtime-api/platform.md#accel)>

##### `route_location`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`RouteLocation`](/api/internal/pax-runtime/router.md#routelocation)>

##### `browser_allows_scroller_vector_layers`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

##### `browser_allows_nested_scroller_vector_layers`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

##### `platform`
Type: [`Platform`](/api/pax-runtime-api/platform.md#platform)

##### `os`
Type: [`OS`](/api/pax-runtime-api/platform.md#os)

##### `get_elapsed_millis`
Type: `Rc`<`dyn` `Fn`() -> `u128`>

#### Implementations
##### `stack_frame`
<pre><code class="api-signature language-rust ignore">pub fn stack_frame(&amp;self) -&gt; Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;</code></pre>

Build the root stack frame containing built-in globals plus internal engine state.

---

### `Handler`
Runtime event handler thunk generated from template bindings.

#### Properties
##### `function`
Type: `fn`()

##### `location`
Type: [`HandlerLocation`](/api/internal/pax-runtime/engine.md#handlerlocation)

#### Implementations
##### `new_component_handler`
<pre><code class="api-signature language-rust ignore">pub fn new_component_handler(function: fn()) -&gt; Self</code></pre>

Build a handler whose `self` argument is the containing component.

##### `new_inline_handler`
<pre><code class="api-signature language-rust ignore">pub fn new_inline_handler(function: fn()) -&gt; Self</code></pre>

Build a handler whose `self` argument is the inline primitive/component.

---

### `HandlerRegistry`
Map from event key to one or more handlers registered on an instance node.

#### Properties
##### `handlers`
Type: `HashMap`<`String`, `Vec`<[`Handler`](/api/internal/pax-runtime/engine.md#handler)>>

---

### `PaxEngine`
Singleton struct storing everything related to properties computation & rendering

#### Properties
##### `runtime_context`
Type: `Rc`<[`RuntimeContext`](/api/internal/pax-runtime/properties.md#runtimecontext)>

##### `root_expanded_node`
Type: `Option`<`Rc`<`ExpandedNode`>>

##### `scroller_tiling_policy`
Type: [`ScrollerTilingPolicy`](/api/internal/pax-runtime/engine/layer_tiling.md#scrollertilingpolicy)

#### Implementations
Central instance of the PaxEngine and runtime, intended to be created by a particular chassis.
Contains all rendering and runtime logic.

##### `mount_root_component`
<pre><code class="api-signature language-rust ignore">pub fn mount_root_component(&amp;mut self, main_component_instance: Rc&lt;<a href="/api/internal/pax-runtime/component.md#componentinstance">ComponentInstance</a>&gt;) -&gt; Rc&lt;ExpandedNode&gt;</code></pre>

Mount a root component tree into an existing runtime kernel.

##### `set_viewport_size`
<pre><code class="api-signature language-rust ignore">pub fn set_viewport_size(&amp;mut self, new_viewport_size: (f64, f64))</code></pre>

Called by chassis when viewport size changes, e.g. with native window resizes

##### `tick`
<pre><code class="api-signature language-rust ignore">pub fn tick(&amp;mut self) -&gt; Vec&lt;<a href="/api/internal/pax-message/index.md#nativemessage">NativeMessage</a>&gt;</code></pre>

Workhorse methods of every tick.  Will be executed up to 240 Hz.
Three phases:
1. Expand nodes & compute properties; recurse entire instance tree and evaluate ExpandedNodes, stitching
   together parent/child relationships between ExpandedNodes along the way.
2. Compute layout (z-index & TransformAndBounds) by visiting ExpandedNode tree
   in rendering order, writing computed rendering-specific values to ExpandedNodes
3. Render:
    a. find lowest node (last child of last node)
    b. start rendering, from lowest node on-up, throughout tree

##### `unmount`
<pre><code class="api-signature language-rust ignore">pub fn unmount(&amp;mut self)</code></pre>

Detach the mounted root component tree, leaving the runtime kernel empty.

## Enums
### `HandlerLocation`
Indicates whether a handler should receive inline-node or containing-component properties.

#### Variants
##### `Inline`
##### `Component`
