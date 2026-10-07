# engine::layer_tiling
<!-- summary: API docs for pax-runtime::engine::layer_tiling. -->
<!-- tags: api, pax-runtime -->

## Structs
### `LayerCanvasPlan`
Canvas tiling plan for one logical render layer.

#### Properties
##### `layer_id`
Type: `usize`

##### `active`
Type: `bool`

##### `surfaces`
Type: `Vec`<[`SurfaceCanvasDescriptor`](../../../../api/internal/pax-runtime/engine/layer_tiling.md#surfacecanvasdescriptor)>

---

### `ScrollerTilingPolicy`
#### Properties
##### `target_tile_viewports`
Type: `Option`<`f64`>

Optional scrolling-axis cap, in host viewports, rounded up in 256-physical-
pixel buckets. Only the explicit-coverage planner opts into this sizing.
The grid depends on the host viewport, never on its moving ancestor clip.

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

##### `replay_priority`
Type: `i32`

##### `surface_signature`
Type: `String`

##### `transform_signature`
Type: `String`

##### `host_signature`
Type: `String`

## Functions
### `scroller_canvas_plan`
<pre><code class="api-signature language-rust ignore">pub fn scroller_canvas_plan(layer_id: usize, host_signature: String, content_width: f64, content_height: f64, viewport_width: f64, viewport_height: f64, scroll_x: f64, scroll_y: f64, device_pixel_ratio: f64) -&gt; <a href="../../../../api/internal/pax-runtime/engine/layer_tiling.md#layercanvasplan">LayerCanvasPlan</a></code></pre>

Build a tile window for a scrollable vector layer.

---

### `scroller_canvas_plan_for_coverage`
<pre><code class="api-signature language-rust ignore">pub fn scroller_canvas_plan_for_coverage(layer_id: usize, host_signature: String, content: [f64; 2], viewport: [f64; 2], coverage: <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect">CoverageRect</a>, device_pixel_ratio: f64, policy: <a href="../../../../api/internal/pax-runtime/engine/layer_tiling.md#scrollertilingpolicy">ScrollerTilingPolicy</a>) -&gt; <a href="../../../../api/internal/pax-runtime/engine/layer_tiling.md#layercanvasplan">LayerCanvasPlan</a></code></pre>

Select content-anchored tiles from an already clipped preparation region.
Unlike the compatibility planner, this does not add another warm margin or
crop tile dimensions to a changing clip. Identity is the absolute grid cell,
so adding/removing optional coverage cannot rename surviving visible tiles.

---

### `single_surface_plan`
<pre><code class="api-signature language-rust ignore">pub fn single_surface_plan(layer_id: usize, host_signature: String, width: f64, height: f64) -&gt; <a href="../../../../api/internal/pax-runtime/engine/layer_tiling.md#layercanvasplan">LayerCanvasPlan</a></code></pre>

Build a one-surface plan for layers that do not need tiling.
