# engine::layer_surface
<!-- summary: API docs for pax-runtime::engine::layer_surface. -->
<!-- tags: api, pax-runtime -->

## Structs
### `LayerSurfaceEntry`
Desired surface geometry for one tile in a layer layout.

#### Properties
##### `key`
Type: `String`

##### `host_signature`
Type: `String`

Host ownership plus backing-surface generation; changes when the physical surface is replaced.

##### `origin_x`
Type: `f32`

##### `origin_y`
Type: `f32`

##### `replay_priority`
Type: `i32`

##### `surface`
Type: [`LayerSurfaceSize`](../../../../api/internal/pax-runtime/engine/layer_surface.md#layersurfacesize)

---

### `LayerSurfaceLayout`
Desired set of physical surfaces for a logical layer.

#### Properties
##### `surfaces`
Type: `Vec`<[`LayerSurfaceEntry`](../../../../api/internal/pax-runtime/engine/layer_surface.md#layersurfaceentry)>

##### `active`
Type: `bool`

---

### `LayerSurfaceSize`
Logical and backing-pixel dimensions for one physical surface.

#### Properties
##### `logical_width`
Type: `f32`

##### `logical_height`
Type: `f32`

##### `surface_width`
Type: `u32`

##### `surface_height`
Type: `u32`

##### `dpr`
Type: [`f32`; 2]

---

### `ReplayPriorityEntry`
Retargeted surface metadata used to choose replay order without involving backend resources.

#### Properties
##### `index`
Type: `usize`

##### `priority`
Type: `i32`

---

### `SurfaceReplayCoordinator`
Shared coordinator for physical-surface replay after a layer layout retarget.

This tracks surface indices and replay regions, not scene-node coverage. The
runtime scene index selects nodes; backends apply surface indices to their own
renderer objects.

#### Implementations
##### `remap_surfaces`
<pre><code class="api-signature language-rust ignore">pub fn remap_surfaces(&amp;mut self, layer: usize, old_indices: &amp;[usize])</code></pre>

Remap pending replay work after eviction or reordering, dropping work for removed surfaces.
`old_indices[new_index]` identifies the surviving renderer in the previous surface vector.

## Functions
### `replay_batches_by_directional_priority`
<pre><code class="api-signature language-rust ignore">pub fn replay_batches_by_directional_priority(entries: &amp;[<a href="../../../../api/internal/pax-runtime/engine/layer_surface.md#replaypriorityentry">ReplayPriorityEntry</a>]) -&gt; Vec&lt;Vec&lt;usize&gt;&gt;</code></pre>

Replay visible retargeted surfaces together, then warm tiles by priority and travel direction.
