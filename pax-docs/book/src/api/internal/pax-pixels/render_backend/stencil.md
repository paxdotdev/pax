# render_backend::stencil
<!-- summary: API docs for pax-pixels::render_backend::stencil. -->
<!-- tags: api, pax-pixels -->

## Structs
### `ClipDraw`
One clip geometry instance to draw into the stencil buffer.

#### Properties
##### `clip_id`
Type: `u32`

##### `geometry_signature`
Type: `u64`

##### `geometry`
Type: &'`a` `VertexBuffers`<[`Vertex`](/api/internal/pax-pixels/render_backend/stencil.md#vertex), `u16`>

---

### `StencilRenderer`
Maintains the stencil stack used to render nested vector clips.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(device: &amp;Device, width: u32, height: u32, sample_count: u32, globals: &amp;Buffer, clip_transforms: &amp;Buffer) -&gt; Self</code></pre>

Create the stencil pipelines and backing texture.

---

### `Vertex`
Tessellated stencil vertex.

#### Properties
##### `position`
Type: [`f32`; 2]
