# engine::pax_gpu_render_context
<!-- summary: API docs for pax-runtime::engine::pax_gpu_render_context. -->
<!-- tags: api, pax-runtime -->

## Structs
### `LayerCreationRequest`
Snapshot of surviving tiles supplied to a chassis factory. Return only missing surfaces;
the runtime keeps survivors ready and validates returned tiles against the current layout.

#### Properties
##### `shared_context`
Type: `Option`<`SharedGpuContext`>

Context shared by this logical layer's surviving tiles, if any.

##### `max_surface_dimension`
Type: `u32`

Adapter limit already established by the surviving tiles.

#### Implementations
##### `needs_surface`
<pre><code class="api-signature language-rust ignore">pub fn needs_surface(&amp;self, key: &amp;str, host_signature: &amp;str) -&gt; bool</code></pre>

Whether the factory must create this key and backing-surface generation.

---

### `LayerRenderer`
Retained renderer bound to one physical surface tile for a logical layer.

#### Implementations
##### `matches_surface`
<pre><code class="api-signature language-rust ignore">pub fn matches_surface(&amp;self, surface: &amp;<a href="../../../../api/internal/pax-runtime/engine/layer_surface.md#layersurfaceentry">LayerSurfaceEntry</a>) -&gt; bool</code></pre>

Whether this renderer belongs to the entry's current physical surface generation.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(key: String, host_signature: String, renderer: <a href="../../../../api/internal/pax-gpu/render_context.md#wgpurenderer">WgpuRenderer</a>&lt;&#39;static&gt;, origin_x: f32, origin_y: f32, logical_width: f32, logical_height: f32, surface_width: u32, surface_height: u32, dpr: [f32; 2]) -&gt; Self</code></pre>

Create a renderer wrapper with its current tile geometry.

##### `renderer_mut`
<pre><code class="api-signature language-rust ignore">pub fn renderer_mut(&amp;mut self) -&gt; &amp;mut <a href="../../../../api/internal/pax-gpu/render_context.md#wgpurenderer">WgpuRenderer</a>&lt;&#39;static&gt;</code></pre>

Access the underlying retained `pax-gpu` renderer.

##### `sync_layout_metadata`
<pre><code class="api-signature language-rust ignore">pub fn sync_layout_metadata(&amp;mut self, surface: &amp;<a href="../../../../api/internal/pax-runtime/engine/layer_surface.md#layersurfaceentry">LayerSurfaceEntry</a>)</code></pre>

Update the retained layout metadata after the owner has already applied matching backend
surface/view transforms.

---

### `LayerTarget`
Current renderer set for one logical layer.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(renderers: Vec&lt;<a href="../../../../api/internal/pax-runtime/engine/pax_gpu_render_context.md#layerrenderer">LayerRenderer</a>&gt;, active: bool) -&gt; Self</code></pre>

Create a layer target from physical surface renderers.

##### `renderers_mut`
<pre><code class="api-signature language-rust ignore">pub fn renderers_mut(&amp;mut self) -&gt; &amp;mut [<a href="../../../../api/internal/pax-runtime/engine/pax_gpu_render_context.md#layerrenderer">LayerRenderer</a>]</code></pre>

Mutable access to each physical renderer backing this logical layer.

---

### `PaxGpuRenderer`
Runtime `RenderContext` implementation backed by `pax-gpu`/wgpu.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(layer_factory: impl Fn(usize, <a href="../../../../api/internal/pax-runtime/engine/pax_gpu_render_context.md#layercreationrequest">LayerCreationRequest</a>) -&gt; Pin&lt;Box&lt;dyn Future&gt;&gt; + &#39;static) -&gt; Self</code></pre>

Create a renderer that lazily asks the chassis for layer backends.

## Enums
### `RenderLayerState`
Lifecycle state for a lazily-created render layer.

#### Variants
##### `Pending`
##### `Failed`
##### `Ready`(([`LayerTarget`](../../../../api/internal/pax-runtime/engine/pax_gpu_render_context.md#layertarget), `Pin`<`Box`<`dyn` `Fn`() -> [`LayerSurfaceLayout`](../../../../api/internal/pax-runtime/engine/layer_surface.md#layersurfacelayout)>>))
## Functions
### `convert_kurbo_to_lyon_path`
<pre><code class="api-signature language-rust ignore">pub fn convert_kurbo_to_lyon_path(kurbo_path: &amp;BezPath) -&gt; <a href="../../../../api/pax-std/drawing/path.md#path">Path</a></code></pre>

Convert a kurbo path emitted by primitives into a lyon path consumed by `pax-gpu`.

---

### `to_pax_gpu_color`
<pre><code class="api-signature language-rust ignore">pub fn to_pax_gpu_color(color: &amp;<a href="../../../../api/pax-runtime-api/color.md#color">Color</a>) -&gt; <a href="../../../../api/pax-runtime-api/color.md#color">Color</a></code></pre>

Convert a runtime API color into the `pax-gpu` render-context color.
