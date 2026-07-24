# render_context
<!-- summary: API docs for pax-gpu::render_context. -->
<!-- tags: api, pax-gpu -->

## Structs
### `Color`
Linear RGBA color used by the low-level renderer.

#### Implementations
##### `hlca`
<pre><code class="api-signature language-rust ignore">pub fn hlca(h: f32, l: f32, c: f32, a: f32) -&gt; Self</code></pre>

Construct a color from HLC/Lab-style components plus alpha.

##### `hsva`
<pre><code class="api-signature language-rust ignore">pub fn hsva(h: f32, s: f32, v: f32, a: f32) -&gt; Self</code></pre>

Construct a color from HSV plus alpha, with hue normalized to 0.0-1.0.

##### `rgba`
<pre><code class="api-signature language-rust ignore">pub fn rgba(r: f32, g: f32, b: f32, a: f32) -&gt; Self</code></pre>

Construct a color from linear RGBA channels in the range 0.0-1.0.

---

### `DrawRange`
Normalized visible range for a stroked vector path.

#### Properties
##### `start`
Type: `f32`

##### `end`
Type: `f32`

##### `enabled`
Type: `bool`

---

### `GradientStop`
One color stop in a GPU gradient fill.

#### Properties
##### `color`
Type: [`Color`](/api/pax-runtime-api/color.md#color)

##### `stop`
Type: `f32`

---

### `Material`
Light-reactive material coefficients for a tessellated vector path.

#### Properties
##### `coefficients`
Type: [`f32`; 4]

##### `emissive`
Type: [`f32`; 4]

##### `unlit`
Type: `bool`

---

### `ResourceChurnStats`
Resource churn counters for renderer profiling.

#### Properties
##### `flushes`
Type: `u64`

##### `retained_scene_resets`
Type: `u64`

##### `vector_batch_flushes`
Type: `u64`

##### `vector_buffer_rebuilds`
Type: `u64`

##### `vector_geometry_rebuilds`
Type: `u64`

##### `vector_geometry_cache_hits`
Type: `u64`

##### `vector_geometry_cache_misses`
Type: `u64`

##### `vector_geometry_cache_evictions`
Type: `u64`

##### `vector_geometry_cache_bytes`
Type: `u64`

##### `tessellated_vertices`
Type: `u64`

##### `tessellated_indices`
Type: `u64`

##### `cached_vertices_reused`
Type: `u64`

##### `cached_indices_reused`
Type: `u64`

##### `vector_resource_creates`
Type: `u64`

##### `vector_resource_updates`
Type: `u64`

##### `vector_resource_recreates`
Type: `u64`

##### `vector_resource_create_bytes`
Type: `u64`

##### `vector_resource_update_bytes`
Type: `u64`

##### `vector_resource_cache_hits`
Type: `u64`

##### `vector_resource_cache_misses`
Type: `u64`

##### `vector_resource_cache_evictions`
Type: `u64`

##### `vector_resource_cache_bytes`
Type: `u64`

##### `texture_creates`
Type: `u64`

##### `texture_upload_bytes`
Type: `u64`

##### `retained_nodes_considered`
Type: `u64`

##### `retained_nodes_visible`
Type: `u64`

##### `retained_draw_batches`
Type: `u64`

##### `retained_draws`
Type: `u64`

##### `retained_vector_draws`
Type: `u64`

##### `retained_image_draws`
Type: `u64`

---

### `SceneLight`
A resolved scene light for the low-level renderer.

#### Properties
##### `shape`
Type: [`LightShape`](/api/pax-runtime-api/drawing.md#lightshape)

##### `position`
Type: [`f32`; 3]

##### `direction`
Type: [`f32`; 3]

##### `color`
Type: [`Color`](/api/pax-runtime-api/color.md#color)

##### `intensity`
Type: `f32`

##### `radius`
Type: `f32`

---

### `SceneLighting`
Resolved lighting state for one retained vector scene.

#### Properties
##### `active`
Type: `bool`

##### `ambient_is_authored`
Type: `bool`

##### `ambient_color`
Type: [`Color`](/api/pax-runtime-api/color.md#color)

##### `ambient_intensity`
Type: `f32`

##### `lights`
Type: `Vec`<[`SceneLight`](/api/pax-runtime-api/drawing.md#scenelight)>

---

### `Stroke`
Stroke style for a tessellated vector path.

#### Properties
##### `fill`
Type: [`Fill`](/api/pax-runtime-api/drawing.md#fill)

##### `weight`
Type: `f32`

##### `cap`
Type: [`StrokeCap`](/api/pax-runtime-api/drawing.md#strokecap)

##### `join`
Type: [`StrokeJoin`](/api/pax-runtime-api/drawing.md#strokejoin)

---

### `WgpuRenderer`
Retained scene renderer that records Pax vector/image commands and flushes them through wgpu.

#### Implementations
##### `current_transform`
<pre><code class="api-signature language-rust ignore">pub fn current_transform(&amp;self) -&gt; <a href="/api/pax-runtime-api/transform.md#transform2d">Transform2D</a></code></pre>

Current transform at the top of the render-state stack.

##### `fill_path`
<pre><code class="api-signature language-rust ignore">pub fn fill_path(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, fill: <a href="/api/pax-runtime-api/drawing.md#fill">Fill</a>)</code></pre>

Queue a filled vector path into the current retained node.

##### `fill_path_with_material_and_opacity`
<pre><code class="api-signature language-rust ignore">pub fn fill_path_with_material_and_opacity(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, fill: <a href="/api/pax-runtime-api/drawing.md#fill">Fill</a>, material: <a href="/api/pax-runtime-api/drawing.md#material">Material</a>, opacity: f32)</code></pre>

Queue a filled vector path with material response and an extra opacity multiplier.

##### `fill_path_with_material_and_opacity_and_smoothing`
<pre><code class="api-signature language-rust ignore">pub fn fill_path_with_material_and_opacity_and_smoothing(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, fill: <a href="/api/pax-runtime-api/drawing.md#fill">Fill</a>, material: <a href="/api/pax-runtime-api/drawing.md#material">Material</a>, opacity: f32, smoothing: <a href="/api/pax-runtime-api/drawing.md#pathsmoothing">PathSmoothing</a>)</code></pre>

Queue a filled vector path with material response, opacity, and optional smoothing.

##### `fill_path_with_opacity`
<pre><code class="api-signature language-rust ignore">pub fn fill_path_with_opacity(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, fill: <a href="/api/pax-runtime-api/drawing.md#fill">Fill</a>, opacity: f32)</code></pre>

Queue a filled vector path with an extra opacity multiplier.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(render_backend: <a href="/api/internal/pax-gpu/render_backend.md#renderbackend">RenderBackend</a>&lt;&#39;w&gt;) -&gt; Self</code></pre>

Create a retained renderer around a low-level `RenderBackend`.

##### `reset_retained_scene`
<pre><code class="api-signature language-rust ignore">pub fn reset_retained_scene(&amp;mut self)</code></pre>

Drop retained scene state for a surface that has been rebound to a new tile origin.

##### `set_surface_transform`
<pre><code class="api-signature language-rust ignore">pub fn set_surface_transform(&amp;mut self, transform: <a href="/api/pax-runtime-api/transform.md#transform2d">Transform2D</a>)</code></pre>

Set the base transform for the physical surface tile being rendered.

##### `share_vector_caches_from`
<pre><code class="api-signature language-rust ignore">pub fn share_vector_caches_from(&amp;mut self, other: &amp;Self)</code></pre>

Share vector resource caches with another renderer for the same logical layer.

##### `stroke_path`
<pre><code class="api-signature language-rust ignore">pub fn stroke_path(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, stroke: <a href="/api/pax-runtime-api/drawing.md#stroke">Stroke</a>)</code></pre>

Queue a stroked vector path into the current retained node.

##### `stroke_path_with_draw_range_and_material_and_opacity`
<pre><code class="api-signature language-rust ignore">pub fn stroke_path_with_draw_range_and_material_and_opacity(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, stroke: <a href="/api/pax-runtime-api/drawing.md#stroke">Stroke</a>, material: <a href="/api/pax-runtime-api/drawing.md#material">Material</a>, opacity: f32, draw_range: <a href="/api/internal/pax-gpu/render_context.md#drawrange">DrawRange</a>)</code></pre>

Queue a draw-ranged stroked vector path with material response and an extra opacity multiplier.

##### `stroke_path_with_draw_range_and_material_and_opacity_and_smoothing`
<pre><code class="api-signature language-rust ignore">pub fn stroke_path_with_draw_range_and_material_and_opacity_and_smoothing(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, stroke: <a href="/api/pax-runtime-api/drawing.md#stroke">Stroke</a>, material: <a href="/api/pax-runtime-api/drawing.md#material">Material</a>, opacity: f32, draw_range: <a href="/api/internal/pax-gpu/render_context.md#drawrange">DrawRange</a>, smoothing: <a href="/api/pax-runtime-api/drawing.md#pathsmoothing">PathSmoothing</a>)</code></pre>

Queue a draw-ranged stroked vector path with material response, opacity, and optional smoothing.

##### `stroke_path_with_material_and_opacity`
<pre><code class="api-signature language-rust ignore">pub fn stroke_path_with_material_and_opacity(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, stroke: <a href="/api/pax-runtime-api/drawing.md#stroke">Stroke</a>, material: <a href="/api/pax-runtime-api/drawing.md#material">Material</a>, opacity: f32)</code></pre>

Queue a stroked vector path with material response and an extra opacity multiplier.

##### `stroke_path_with_material_and_opacity_and_smoothing`
<pre><code class="api-signature language-rust ignore">pub fn stroke_path_with_material_and_opacity_and_smoothing(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, stroke: <a href="/api/pax-runtime-api/drawing.md#stroke">Stroke</a>, material: <a href="/api/pax-runtime-api/drawing.md#material">Material</a>, opacity: f32, smoothing: <a href="/api/pax-runtime-api/drawing.md#pathsmoothing">PathSmoothing</a>)</code></pre>

Queue a stroked vector path with material response, opacity, and optional smoothing.

##### `stroke_path_with_opacity`
<pre><code class="api-signature language-rust ignore">pub fn stroke_path_with_opacity(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, stroke: <a href="/api/pax-runtime-api/drawing.md#stroke">Stroke</a>, opacity: f32)</code></pre>

Queue a stroked vector path with an extra opacity multiplier.

##### `take_resource_churn_stats`
<pre><code class="api-signature language-rust ignore">pub fn take_resource_churn_stats(&amp;mut self) -&gt; <a href="/api/internal/pax-gpu/render_context.md#resourcechurnstats">ResourceChurnStats</a></code></pre>

Return and reset accumulated resource churn counters.

## Enums
### `Fill`
Fill style for a tessellated vector path.

#### Variants
##### `Solid`([`Color`](/api/pax-runtime-api/color.md#color))
##### `Gradient` { `gradient_type`: [`GradientType`](/api/internal/pax-gpu/render_context.md#gradienttype), `pos`: [`Point2D`](/api/internal/pax-gpu/index.md#point2d), `main_axis`: [`Vector2D`](/api/internal/pax-gpu/index.md#vector2d), `off_axis`: [`Vector2D`](/api/internal/pax-gpu/index.md#vector2d), `stops`: `Vec`<[`GradientStop`](/api/pax-runtime-api/drawing.md#gradientstop)> }
---

### `GradientType`
Shape of a GPU gradient fill.

#### Variants
##### `Linear`
##### `Radial`
---

### `LightShape`
Shape of a scene light.

#### Variants
##### `Point`
##### `Directional`
---

### `StrokeCap`
Stroke end-cap style.

#### Variants
##### `Butt`
##### `Round`
##### `Square`
---

### `StrokeJoin`
Stroke join style.

#### Variants
##### `Miter`
##### `Round`
##### `Bevel`
