# render_backend
<!-- summary: API docs for pax-gpu::render_backend. -->
<!-- tags: api, pax-gpu -->

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

### `GpuContext`
GPU device, pipelines, and a weak index of resident image textures shared by sibling surfaces.

#### Implementations
##### `finish_render_turn`
<pre><code class="api-signature language-rust ignore">pub fn finish_render_turn(&amp;self, frame: RenderFrame)</code></pre>

Fence all submissions made by the admitted rendering turn on this device.
Empty turns consume no allowance. Completion never waits on the host thread.

##### `from_wgpu`
<pre><code class="api-signature language-rust ignore">pub fn from_wgpu(instance: Instance, adapter: Adapter, device: Device, queue: Queue) -&gt; Result&lt;SharedGpuContext, <a href="../../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Shares an existing WGPU device with a native compositor integration.

All four handles must originate from the same instance and adapter, and
the queue must belong to the device. Negotiate platform export extensions
before calling this constructor and request at least `required_limits()`.
Insufficient device limits return an error before creating GPU resources.
External-memory sharing and synchronization remain the bridge's job.

##### `render_queue_ready`
<pre><code class="api-signature language-rust ignore">pub fn render_queue_ready(&amp;self, frame: RenderFrame) -&gt; bool</code></pre>

Whether another rendering turn can start without building a GPU backlog.
Native devices need a nonblocking poll to deliver completion callbacks;
WebGPU delivers them through the browser's event loop.

##### `required_limits`
<pre><code class="api-signature language-rust ignore">pub fn required_limits() -&gt; Limits</code></pre>

Established renderer baseline to request when creating a shared device.

This includes storage-buffer support for gradients and mesh paints. A
platform may request higher limits, including the adapter's full limits.

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

##### `output_texture`
<pre><code class="api-signature language-rust ignore">pub fn output_texture(&amp;self) -&gt; Option&lt;Texture&gt;</code></pre>

Clones the current offscreen output handle, or returns `None` for a window surface.

Flush rendering before submitting a copy on this context's queue. The
texture is reused on subsequent frames and replaced on resize; a clone
retains allocation lifetime, not an immutable snapshot of its pixels.

##### `resize`
<pre><code class="api-signature language-rust ignore">pub fn resize(&amp;mut self, width: u32, height: u32)</code></pre>

Resize both the physical surface and logical viewport.

##### `resize_surface`
<pre><code class="api-signature language-rust ignore">pub fn resize_surface(&amp;mut self, width: u32, height: u32)</code></pre>

Resize the backing surface, clamping to device limits.

##### `set_viewport`
<pre><code class="api-signature language-rust ignore">pub fn set_viewport(&amp;mut self, width: f32, height: f32, dpr: [f32; 2])</code></pre>

Update logical viewport uniforms without reallocating the surface.

##### `to_texture`
<pre><code class="api-signature language-rust ignore">pub fn to_texture(context: SharedGpuContext, format: TextureFormat, config: <a href="../../../api/internal/pax-gpu/render_backend.md#renderconfig">RenderConfig</a>) -&gt; Result&lt;Self, <a href="../../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Renders into a persistent, single-sample texture for native composition.

Supports RGBA8/BGRA8 unorm formats with premultiplied alpha. sRGB
attachments are excluded because encoding after blending would not leave
byte-premultiplied output for a direct compositor copy. The host remains
responsible for matching the renderer's color space to its compositor.
After flushing, copy the output on the same queue into a platform-owned
export allocation. No CPU pixel transfer or window surface is required.
Resizing creates a new backing texture; previously cloned handles remain
valid for already queued copies. The host owns export synchronization and
must not hand this mutable rendering target directly to a compositor.

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

##### `with_browser_premultiplied_alpha`
<pre><code class="api-signature language-rust ignore">pub fn with_browser_premultiplied_alpha(self, enabled: bool) -&gt; Self</code></pre>

Request premultiplied browser canvas alpha for WebGPU surfaces.

wgpu's web backend currently reports only `Opaque` alpha in surface capabilities, but its
configure path accepts `PreMultiplied` and maps it to `GPUCanvasAlphaMode::Premultiplied`.
Pax needs that for transparent browser-owned scroller islands.
