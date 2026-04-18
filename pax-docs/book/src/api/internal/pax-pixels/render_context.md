# render_context
<!-- summary: API docs for pax-pixels::render_context. -->
<!-- tags: api, pax-pixels -->

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

### `GradientStop`
One color stop in a GPU gradient fill.

#### Properties
##### `color`
Type: [`Color`](/api/pax-runtime-api/color.md#color)

##### `stop`
Type: `f32`

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

##### `fill_path_with_opacity`
<pre><code class="api-signature language-rust ignore">pub fn fill_path_with_opacity(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, fill: <a href="/api/pax-runtime-api/drawing.md#fill">Fill</a>, opacity: f32)</code></pre>

Queue a filled vector path with an extra opacity multiplier.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(render_backend: <a href="/api/internal/pax-pixels/render_backend.md#renderbackend">RenderBackend</a>&lt;&#39;w&gt;) -&gt; Self</code></pre>

Create a retained renderer around a low-level `RenderBackend`.

##### `reset_retained_scene`
<pre><code class="api-signature language-rust ignore">pub fn reset_retained_scene(&amp;mut self)</code></pre>

Drop retained scene state for a surface that has been rebound to a new tile origin.

##### `set_surface_transform`
<pre><code class="api-signature language-rust ignore">pub fn set_surface_transform(&amp;mut self, transform: <a href="/api/pax-runtime-api/transform.md#transform2d">Transform2D</a>)</code></pre>

Set the base transform for the physical surface tile being rendered.

##### `stroke_path`
<pre><code class="api-signature language-rust ignore">pub fn stroke_path(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, stroke: <a href="/api/pax-runtime-api/drawing.md#stroke">Stroke</a>)</code></pre>

Queue a stroked vector path into the current retained node.

##### `stroke_path_with_opacity`
<pre><code class="api-signature language-rust ignore">pub fn stroke_path_with_opacity(&amp;mut self, path: <a href="/api/pax-std/drawing/path.md#path">Path</a>, stroke: <a href="/api/pax-runtime-api/drawing.md#stroke">Stroke</a>, opacity: f32)</code></pre>

Queue a stroked vector path with an extra opacity multiplier.

## Enums
### `Fill`
Fill style for a tessellated vector path.

#### Variants
##### `Solid`([`Color`](/api/pax-runtime-api/color.md#color))
##### `Gradient` { `gradient_type`: [`GradientType`](/api/internal/pax-pixels/render_context.md#gradienttype), `pos`: [`Point2D`](/api/internal/pax-pixels/index.md#point2d), `main_axis`: [`Vector2D`](/api/internal/pax-pixels/index.md#vector2d), `off_axis`: [`Vector2D`](/api/internal/pax-pixels/index.md#vector2d), `stops`: `Vec`<[`GradientStop`](/api/pax-runtime-api/drawing.md#gradientstop)> }
---

### `GradientType`
Shape of a GPU gradient fill.

#### Variants
##### `Linear`
##### `Radial`
---

### `StrokeCap`
Stroke end-cap style.

#### Variants
##### `Butt`
##### `Round`
##### `Square`
