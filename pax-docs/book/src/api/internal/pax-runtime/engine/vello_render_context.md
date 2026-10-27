# engine::vello_render_context
<!-- summary: Portable software drawing and compositing, with chassis-owned presentation. -->
<!-- tags: api, pax-runtime -->

Portable software drawing and compositing, with chassis-owned presentation.

## Structs
### `CpuLayerRenderer`
One software-backed canvas assignment in the shared surface scheduler.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(key: String, host_signature: String, surface: S, entry: &amp;<a href="../../../../api/internal/pax-runtime/engine/layer_surface.md#layersurfaceentry">LayerSurfaceEntry</a>) -&gt; Self</code></pre>

Attach a chassis presentation target to its committed surface plan entry.

---

### `CpuLayerTarget`
Current CPU surface set for one logical layer.

---

### `CpuRenderer`
`RenderContext` implementation backed by Vello CPU.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(layer_factory: impl Fn(usize) -&gt; (<a href="../../../../api/internal/pax-runtime/engine/vello_render_context.md#cpulayertarget">CpuLayerTarget</a>&lt;S&gt;, Box&lt;dyn Fn() -&gt; <a href="../../../../api/internal/pax-runtime/engine/layer_surface.md#layersurfacelayout">LayerSurfaceLayout</a>&gt;) + &#39;static) -&gt; Self</code></pre>

Create a Vello CPU renderer with a chassis-provided logical layer factory.

## Traits
### `CpuSurface`
A presentation target for Vello CPU. Rasterization and intermediate surfaces
belong to the runtime; the chassis only installs the completed pixels.
