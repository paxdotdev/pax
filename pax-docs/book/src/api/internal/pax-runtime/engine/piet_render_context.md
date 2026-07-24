# engine::piet_render_context
<!-- summary: API docs for pax-runtime::engine::piet_render_context. -->
<!-- tags: api, pax-runtime -->

## Structs
### `PietLayerRenderer`
Retained metadata for one piet-backed browser canvas surface.

---

### `PietLayerTarget`
Current piet surface set for one logical layer.

---

### `PietRenderer`
`RenderContext` implementation backed by piet.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(layer_factory: impl Fn(usize) -&gt; (<a href="/api/internal/pax-runtime/engine/piet_render_context.md#pietlayertarget">PietLayerTarget</a>&lt;R&gt;, Box&lt;dyn Fn() -&gt; <a href="/api/internal/pax-runtime/engine/layer_surface.md#layersurfacelayout">LayerSurfaceLayout</a>&gt;) + &#39;static) -&gt; Self</code></pre>

Create a piet renderer with a chassis-provided logical layer factory.
