# hit
<!-- summary: Immutable input geometry, independent of paint opacity and effect bounds. -->
<!-- tags: api, pax-runtime-api -->

Immutable input geometry, independent of paint opacity and effect bounds.

## Enums
### `HitShape`
Geometry prepared by a primitive before a point query runs.

Values contain no live nodes or callbacks. A custom primitive describes its
local hit region with a path; it must invalidate input when that geometry
changes. Rectangles use strict edges, matching ordinary logical bounds.

#### Variants
##### `Empty`
No input area (distinct from an unknown conservative bound).

##### `Rect`(`Rect`)
A rectangle in local layout coordinates, with exclusive edges.

##### `Path` { `path`: `Rc`<`BezPath`>, `tolerance`: `f64` }
Nonzero-winding local coverage. `tolerance` also tests offsets along the
two local axes, preserving Line's historical stroke-edge convention.

##### `Mesh`(`Rc`<[`PosedConvexMesh`](../../api/pax-runtime-api/mesh3.md#posedconvexmesh)>)
A convex mesh posed in world coordinates. Available with `3d`.

#### Implementations
##### `contains_local`
<pre><code class="api-signature language-rust ignore">pub fn contains_local(&amp;self, point: Point) -&gt; bool</code></pre>

Test local planar coverage. Meshes require a projected surface query.

##### `local_bounds`
<pre><code class="api-signature language-rust ignore">pub fn local_bounds(&amp;self) -&gt; Option&lt;Rect&gt;</code></pre>

Conservative local bounds; mesh bounds are prepared after projection.
Nonfinite bounds must be treated conservatively, never as a finite box.
