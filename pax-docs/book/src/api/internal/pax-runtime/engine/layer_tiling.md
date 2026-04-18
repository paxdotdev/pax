# engine::layer_tiling
<!-- summary: API docs for pax-runtime::engine::layer_tiling. -->
<!-- tags: api, pax-runtime -->

## Structs
### `LayerCanvasPlan`
Canvas tiling plan for one logical occlusion layer.

#### Properties
##### `layer_id`
Type: `usize`

##### `active`
Type: `bool`

##### `surfaces`
Type: `Vec`<[`SurfaceCanvasDescriptor`](/api/internal/pax-runtime/engine/layer_tiling.md#surfacecanvasdescriptor)>

---

### `SurfaceCanvasDescriptor`
One physical canvas surface used to render a logical Pax layer tile.

#### Properties
##### `id`
Type: `String`

##### `key`
Type: `String`

##### `left`
Type: `f64`

##### `top`
Type: `f64`

##### `width`
Type: `f64`

##### `height`
Type: `f64`

##### `surface_signature`
Type: `String`

##### `transform_signature`
Type: `String`

##### `host_signature`
Type: `String`

## Functions
### `scroller_canvas_plan`
<pre><code class="api-signature language-rust ignore">pub fn scroller_canvas_plan(layer_id: usize, host_signature: String, content_width: f64, content_height: f64, viewport_width: f64, viewport_height: f64, scroll_x: f64, scroll_y: f64, device_pixel_ratio: f64) -&gt; <a href="/api/internal/pax-runtime/engine/layer_tiling.md#layercanvasplan">LayerCanvasPlan</a></code></pre>

Build a tile window for a scrollable vector layer.

---

### `single_surface_plan`
<pre><code class="api-signature language-rust ignore">pub fn single_surface_plan(layer_id: usize, host_signature: String, width: f64, height: f64) -&gt; <a href="/api/internal/pax-runtime/engine/layer_tiling.md#layercanvasplan">LayerCanvasPlan</a></code></pre>

Build a one-surface plan for layers that do not need tiling.
