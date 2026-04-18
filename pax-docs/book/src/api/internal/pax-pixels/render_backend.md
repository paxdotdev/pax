# render_backend
<!-- summary: API docs for pax-pixels::render_backend. -->
<!-- tags: api, pax-pixels -->

## Submodules
- [render_backend::stencil](render_backend/stencil.md)

## Structs
### `CapturedFrame`
CPU-readable screenshot payload captured from a rendered frame.

#### Properties
##### `width`
Type: `u32`

##### `height`
Type: `u32`

##### `rgba`
Type: `Vec`<`u8`>

---

### `Image`
Decoded RGBA image data ready for upload as a GPU texture.

#### Properties
##### `rgba`
Type: `Vec`<`u8`>

##### `pixel_width`
Type: `u32`

##### `pixel_height`
Type: `u32`

---

### `RenderBackend`
Low-level wgpu backend that owns surface, pipeline, and GPU buffers.

#### Implementations
##### `get_clip_depth`
<pre><code class="api-signature language-rust ignore">pub fn get_clip_depth(&amp;mut self) -&gt; u32</code></pre>

Current stencil clip depth.

##### `max_surface_dimension`
<pre><code class="api-signature language-rust ignore">pub fn max_surface_dimension(&amp;self) -&gt; u32</code></pre>

Maximum texture dimension supported by the active adapter.

##### `resize`
<pre><code class="api-signature language-rust ignore">pub fn resize(&amp;mut self, width: u32, height: u32)</code></pre>

Resize both the physical surface and logical viewport.

##### `resize_surface`
<pre><code class="api-signature language-rust ignore">pub fn resize_surface(&amp;mut self, width: u32, height: u32)</code></pre>

Resize the backing surface, clamping to device limits.

##### `set_viewport`
<pre><code class="api-signature language-rust ignore">pub fn set_viewport(&amp;mut self, width: f32, height: f32, dpr: [f32; 2])</code></pre>

Update logical viewport uniforms without reallocating the surface.

---

### `RenderConfig`
GPU resource sizing and initial surface configuration.

#### Properties
##### `debug`
Type: `bool`

##### `initial_width`
Type: `u32`

##### `initial_height`
Type: `u32`

##### `initial_dpr`
Type: [`f32`; 2]

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(_debug: bool, width: u32, height: u32, dpr: [f32; 2]) -&gt; Self</code></pre>

Construct default buffer capacities for an initial surface size.
