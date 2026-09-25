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
<pre><code class="api-signature language-rust ignore">pub fn new(layer_factory: impl Fn(usize) -&gt; (<a href="../../../../api/internal/pax-runtime/engine/piet_render_context.md#pietlayertarget">PietLayerTarget</a>&lt;S&gt;, Box&lt;dyn Fn() -&gt; <a href="../../../../api/internal/pax-runtime/engine/layer_surface.md#layersurfacelayout">LayerSurfaceLayout</a>&gt;) + &#39;static) -&gt; Self</code></pre>

Create a piet renderer with a chassis-provided logical layer factory.

## Traits
### `PietSurface`
Chassis operations missing from Piet's portable API: reusable transparent
surfaces and source-over composition with one opacity for the complete image.

## Functions
### `fill_to_piet_brush`
<pre><code class="api-signature language-rust ignore">pub fn fill_to_piet_brush(fill: &amp;<a href="../../../../api/pax-runtime-api/drawing.md#fill">Fill</a>, rect: Rect) -&gt; Option&lt;PaintBrush&gt;</code></pre>

Resolves a single paint to a Piet brush. Mixtures require the chassis paint
accumulator, since generic Piet has no additive compositing operation.
