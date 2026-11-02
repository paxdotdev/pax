# visibility
<!-- summary: Shared, budgeted solid/support coverage for native and GPU consumers. Keys describe mounted identities; values contain -->
<!-- tags: api, pax-runtime-api -->

Shared, budgeted solid/support coverage for native and GPU consumers. Keys describe mounted identities; values contain
no nodes or subscriptions. Only the active version of a pair is retained.

## Structs
### `CoverageContext`
Mounted geometry identity and shared cache, carried by retained drawing programs.
Contains no reactive node or platform resource ownership.

#### Properties
##### `node`
Type: (`u32`, `u64`)

Drawable ID and mount epoch.

##### `support`
Type: (`u32`, `u64`)

Certified coplanar support owner ID and mount epoch.

##### `domain`
Type: `usize`

Compositing domain containing the drawable.

##### `relations`
Type: `Rc`<`RefCell`<[`Relations`](../../api/pax-runtime-api/visibility.md#relations)>>

Runtime-wide coverage owner shared with native presentation.

---

### `RelationChart`
A renderer's computational chart. Dropping a tile/capture retires its entries
without invalidating native/window coverage or other live tiles.

#### Properties
##### `relations`
Type: `Rc`<`RefCell`<[`Relations`](../../api/pax-runtime-api/visibility.md#relations)>>

##### `id`
Type: `u64`

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(relations: Rc&lt;RefCell&lt;<a href="../../api/pax-runtime-api/visibility.md#relations">Relations</a>&gt;&gt;) -&gt; Self</code></pre>

Allocates a chart ID unique within this store for the renderer's lifetime.

##### `remove_node`
<pre><code class="api-signature language-rust ignore">pub fn remove_node(&amp;self, node: u32)</code></pre>

Tile eviction removes only this chart's dependence on a mounted node.

---

### `RelationKey`
Mounted solid/support pair in one projection chart and compositing domain.
Node identities include mount epochs so recycled numeric IDs cannot alias.

#### Properties
##### `occluder`
Type: (`u32`, `u64`)

Solid node ID and mount epoch.

##### `support`
Type: (`u32`, `u64`)

Certified coplanar support owner ID and mount epoch.

##### `domain`
Type: `usize`

Compositing domain; relations never cross its isolation boundary.

##### `chart`
Type: `u64`

Zero is the native/window chart; renderer leases own other chart IDs.

---

### `RelationStats`
Cumulative coverage work across all consumers of a relation store.

#### Properties
##### `evaluations`
Type: `u64`

##### `reuses`
Type: `u64`

##### `evictions`
Type: `u64`

##### `oversized`
Type: `u64`

---

### `Relations`
Exact coverage cache shared by native masks and GPU computational charts.

The default ceiling is 2,048 entries / 4 MiB of charged record and path data.
Eviction or an oversized result changes work, never geometric correctness.
Values contain no live nodes; mesh ownership is weak. GPU resources and
caller-owned copies are separate from this auxiliary cache budget.

#### Properties
##### `stats`
Type: [`RelationStats`](../../api/pax-runtime-api/visibility.md#relationstats)

#### Implementations
##### `front_coverage`
<pre><code class="api-signature language-rust ignore">pub fn front_coverage(&amp;mut self, key: <a href="../../api/pax-runtime-api/visibility.md#relationkey">RelationKey</a>, solid: &amp;Rc&lt;<a href="../../api/pax-runtime-api/mesh3/projection.md#projectedconvexmesh">ProjectedConvexMesh</a>&gt;, plane: <a href="../../api/pax-runtime-api/rendering.md#depthplane">DepthPlane</a>) -&gt; Rc&lt;BezPath&gt;</code></pre>

Returns projected solid coverage strictly in front of the support plane.
Validates mesh identity, projection (including camera clips), computational
viewport and plane coefficients before reusing the active pair revision.

##### `remove`
<pre><code class="api-signature language-rust ignore">pub fn remove(&amp;mut self, node: u32)</code></pre>

Retires all dependencies on an unmounted node across every chart.

##### `residency`
<pre><code class="api-signature language-rust ignore">pub fn residency(&amp;self) -&gt; (usize, usize)</code></pre>

Current entry count and charged bytes, excluding allocator overhead.

##### `with_budget`
<pre><code class="api-signature language-rust ignore">pub fn with_budget(max_entries: usize, max_bytes: usize) -&gt; Self</code></pre>

Creates one budget shared by every chart attached to this store.
